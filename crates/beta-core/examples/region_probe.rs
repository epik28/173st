//! Read copied McRegion files and emit deterministic metadata TSV.
//!
//! Input: `case_id<TAB>region_file<TAB>chunk_x<TAB>chunk_z`
//! Output: `case_id<TAB>present|absent<TAB>timestamp<TAB>compression<TAB>nbt_len<TAB>nbt_sha256`

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use beta_core::region::{RegionFile, RegionLimits, local_coord, region_coord};
use sha2::{Digest, Sha256};

const INPUT_MARKER: &str = "REGION-001 copied inputs\n";
const MAX_CASES: usize = 100_000;

fn valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn parse_coord(raw: &str) -> Option<i32> {
    if raw.is_empty() || raw.starts_with('+') || raw.trim() != raw {
        return None;
    }
    if raw == "-0" {
        return Some(0);
    }
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if digits.is_empty()
        || (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    raw.parse().ok()
}

fn validate_dirs(input_dir: &Path, payload_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    let input = input_dir
        .canonicalize()
        .map_err(|e| format!("invalid input directory: {e}"))?;
    if !input.is_dir() {
        return Err("input path is not a directory".into());
    }
    let marker = input.join(".region-input-copy");
    let marker_meta =
        fs::symlink_metadata(&marker).map_err(|e| format!("missing copied-input marker: {e}"))?;
    if marker_meta.file_type().is_symlink() || !marker_meta.is_file() {
        return Err("copied-input marker must be a regular file".into());
    }
    let marker_text =
        fs::read_to_string(marker).map_err(|e| format!("cannot read copied-input marker: {e}"))?;
    if marker_text != INPUT_MARKER {
        return Err("invalid copied-input marker".into());
    }

    let prospective_payload = normalize_absolute(payload_dir)?;
    if prospective_payload.starts_with(&input) || input.starts_with(&prospective_payload) {
        return Err("payload and input directories must not overlap".into());
    }
    fs::create_dir_all(payload_dir).map_err(|e| format!("cannot create payload directory: {e}"))?;
    let payload = payload_dir
        .canonicalize()
        .map_err(|e| format!("invalid payload directory: {e}"))?;
    if payload.starts_with(&input) || input.starts_with(&payload) {
        return Err("payload and input directories must not overlap".into());
    }
    Ok((input, payload))
}

fn normalize_absolute(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("cannot resolve current directory: {e}"))?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    Ok(normalized)
}

fn resolve_region(input: &Path, name: &str) -> Result<PathBuf, String> {
    let requested = Path::new(name);
    if requested.is_absolute() || requested.file_name().and_then(|v| v.to_str()) != Some(name) {
        return Err("region_file must be a basename".into());
    }
    let path = input.join(name);
    let metadata =
        fs::symlink_metadata(&path).map_err(|e| format!("cannot stat region file: {e}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("region file must be a regular copied file".into());
    }
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("cannot resolve region file: {e}"))?;
    if canonical.parent() != Some(input) {
        return Err("region file escapes copied input directory".into());
    }
    if metadata.len() > RegionLimits::default().max_input_bytes as u64 {
        return Err("region file exceeds input limit".into());
    }
    Ok(canonical)
}

/// Process one coordinate TSV stream. Output is written in input order.
pub fn process_stream<R: BufRead, W: Write>(
    reader: R,
    mut writer: W,
    input_dir: &Path,
    payload_dir: &Path,
) -> Result<(), String> {
    let (input, payload_dir) = validate_dirs(input_dir, payload_dir)?;
    let mut seen = HashSet::new();
    let mut regions: HashMap<String, Vec<u8>> = HashMap::new();

    for (line_no, line_result) in reader.lines().enumerate() {
        let line = line_result.map_err(|e| format!("line {}: input error: {e}", line_no + 1))?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4 {
            return Err(format!("line {}: expected four TSV fields", line_no + 1));
        }
        let (case_id, region_name) = (fields[0], fields[1]);
        if !valid_case_id(case_id) || !seen.insert(case_id.to_owned()) {
            return Err(format!(
                "line {}: invalid or duplicate case_id",
                line_no + 1
            ));
        }
        if seen.len() > MAX_CASES {
            return Err(format!("line {}: case limit exceeded", line_no + 1));
        }
        let chunk_x = parse_coord(fields[2])
            .ok_or_else(|| format!("line {}: invalid chunk_x", line_no + 1))?;
        let chunk_z = parse_coord(fields[3])
            .ok_or_else(|| format!("line {}: invalid chunk_z", line_no + 1))?;
        let expected_name = format!("r.{}.{}.mcr", region_coord(chunk_x), region_coord(chunk_z));
        if region_name != expected_name {
            return Err(format!(
                "line {}: region basename does not match chunk coordinates",
                line_no + 1
            ));
        }

        if !regions.contains_key(region_name) {
            let path = resolve_region(&input, region_name)
                .map_err(|e| format!("line {}: {e}", line_no + 1))?;
            let bytes = fs::read(path)
                .map_err(|e| format!("line {}: cannot read region: {e}", line_no + 1))?;
            regions.insert(region_name.to_owned(), bytes);
        }
        let bytes = regions.get(region_name).expect("region inserted");
        let region = RegionFile::open(bytes, RegionLimits::default())
            .map_err(|e| format!("line {}: invalid region: {e}", line_no + 1))?;
        let x = local_coord(chunk_x);
        let z = local_coord(chunk_z);
        match region
            .read_local(x, z)
            .map_err(|e| format!("line {}: chunk read failed: {e}", line_no + 1))?
        {
            None => writeln!(writer, "{case_id}\tabsent\t-\t-\t-\t-")
                .map_err(|e| format!("stdout write failed: {e}"))?,
            Some(record) => {
                let hash = Sha256::digest(&record.nbt);
                let hash_hex = hash
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                let payload_path = payload_dir.join(format!("{case_id}.nbt"));
                let mut file = create_new_payload(&payload_path)
                    .map_err(|e| format!("line {}: cannot create payload: {e}", line_no + 1))?;
                file.write_all(&record.nbt)
                    .map_err(|e| format!("line {}: cannot write payload: {e}", line_no + 1))?;
                file.flush()
                    .map_err(|e| format!("line {}: cannot flush payload: {e}", line_no + 1))?;
                writeln!(
                    writer,
                    "{case_id}\tpresent\t{}\t{}\t{}\t{hash_hex}",
                    record.timestamp,
                    record.compression,
                    record.nbt.len()
                )
                .map_err(|e| format!("stdout write failed: {e}"))?;
            }
        }
    }
    Ok(())
}

fn create_new_payload(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: region_probe INPUT_COPY_DIR PAYLOAD_DIR".into());
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    process_stream(
        BufReader::new(stdin.lock()),
        stdout.lock(),
        Path::new(&args[0]),
        Path::new(&args[1]),
    )
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("region_probe: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::ZlibEncoder};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        root: PathBuf,
        input: PathBuf,
        payload: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "region-probe-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let input = root.join("input");
            let payload = root.join("payload");
            fs::create_dir_all(&input).unwrap();
            fs::write(input.join(".region-input-copy"), INPUT_MARKER).unwrap();
            let raw = b"\x0a\0\0";
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(raw).unwrap();
            let compressed = encoder.finish().unwrap();
            let mut region = vec![0; 3 * 4096];
            region[0..4].copy_from_slice(&(2_u32 << 8 | 1).to_be_bytes());
            region[4096..4100].copy_from_slice(&u32::MAX.to_be_bytes());
            region[8192..8196].copy_from_slice(&(compressed.len() as u32 + 1).to_be_bytes());
            region[8196] = 2;
            region[8197..8197 + compressed.len()].copy_from_slice(&compressed);
            fs::write(input.join("r.0.0.mcr"), region).unwrap();
            Self {
                root,
                input,
                payload,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn emits_ordered_present_and_absent_rows_and_saves_raw_payload() {
        let f = Fixture::new();
        let input = b"# comment\n\nfirst\tr.0.0.mcr\t0\t0\nempty\tr.0.0.mcr\t1\t0\n";
        let mut output = Vec::new();
        process_stream(&input[..], &mut output, &f.input, &f.payload).unwrap();
        let digest = format!("{:x}", Sha256::digest(b"\x0a\0\0"));
        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("first\tpresent\t4294967295\t2\t3\t{digest}\nempty\tabsent\t-\t-\t-\t-\n")
        );
        assert_eq!(fs::read(f.payload.join("first.nbt")).unwrap(), b"\x0a\0\0");
        assert!(!f.payload.join("empty.nbt").exists());
    }

    #[test]
    fn rejects_bad_ids_coordinates_paths_duplicates_and_existing_payloads() {
        let f = Fixture::new();
        for bad in [
            "../escape\tr.0.0.mcr\t0\t0\n",
            "x\t/etc/passwd\t0\t0\n",
            "x\tr.1.0.mcr\t0\t0\n",
            "x\tr.0.0.mcr\t+0\t0\n",
            "x\tr.0.0.mcr\t2147483648\t0\n",
            "x\tr.0.0.mcr\t0\t0\nx\tr.0.0.mcr\t0\t0\n",
            "x\tr.0.0.mcr\t0\t0\textra\n",
        ] {
            assert!(
                process_stream(bad.as_bytes(), Vec::new(), &f.input, &f.payload).is_err(),
                "accepted {bad:?}"
            );
        }
        let valid = b"existing\tr.0.0.mcr\t0\t0\n";
        process_stream(&valid[..], Vec::new(), &f.input, &f.payload).unwrap();
        assert!(process_stream(&valid[..], Vec::new(), &f.input, &f.payload).is_err());
    }

    #[test]
    fn requires_marker_and_separate_directories() {
        let f = Fixture::new();
        let bad_marker = f.root.join("bad-marker");
        fs::create_dir(&bad_marker).unwrap();
        assert!(process_stream(&b""[..], Vec::new(), &bad_marker, &f.payload).is_err());
        assert!(process_stream(&b""[..], Vec::new(), &f.input, &f.input.join("nested")).is_err());
    }

    #[test]
    fn repeated_runs_match_and_leave_input_unchanged() {
        let f = Fixture::new();
        let original = fs::read(f.input.join("r.0.0.mcr")).unwrap();
        let row = b"stable\tr.0.0.mcr\t0\t0\n";
        let mut first = Vec::new();
        process_stream(&row[..], &mut first, &f.input, &f.payload).unwrap();
        let second_payload = f.root.join("payload-second");
        let mut second = Vec::new();
        process_stream(&row[..], &mut second, &f.input, &second_payload).unwrap();
        assert_eq!(first, second);
        assert_eq!(fs::read(f.input.join("r.0.0.mcr")).unwrap(), original);
        assert_eq!(
            fs::read(f.payload.join("stable.nbt")).unwrap(),
            fs::read(second_payload.join("stable.nbt")).unwrap()
        );
    }
}
