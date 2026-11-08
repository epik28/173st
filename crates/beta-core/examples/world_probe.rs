//! Read-only adapter for copied Beta world inputs.
//!
//! Input rows are `case_id<TAB>level|chunk<TAB>basename<TAB>x<TAB>z`.
//! This example owns filesystem access; `beta_core::world` remains pure.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use beta_core::nbt::{self, NamedTag, Tag};
use beta_core::region::{self, RegionFile, RegionLimits};
use beta_core::world::{self, WorldLimits};
use sha2::{Digest, Sha256};

const WORLD_MARKER: &str = "WORLD-LOAD-001 copied inputs\n";
const REGION_MARKER: &str = "REGION-001 copied inputs\n";
const MAX_CASES: usize = 100_000;
const MAX_LEVEL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_REGION_BYTES: u64 = 256 * 1024 * 1024;
const DATA_FIELDS: &[&str] = &[
    "RandomSeed",
    "SpawnX",
    "SpawnY",
    "SpawnZ",
    "Time",
    "LastPlayed",
    "SizeOnDisk",
    "LevelName",
    "version",
    "rainTime",
    "raining",
    "thunderTime",
    "thundering",
    "Player",
];
const LEVEL_FIELDS: &[&str] = &[
    "xPos",
    "zPos",
    "Blocks",
    "Data",
    "BlockLight",
    "SkyLight",
    "HeightMap",
    "LastUpdate",
    "TerrainPopulated",
    "Entities",
    "TileEntities",
];

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn parse_coord(value: &str) -> Option<i32> {
    if value.is_empty() || value.starts_with('+') || value.trim() != value {
        return None;
    }
    if value == "-0" {
        return Some(0);
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty()
        || (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    value.parse().ok()
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn encoded_names<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let mut names: Vec<String> = names
        .map(|name| {
            if name.is_empty() {
                "~".to_owned()
            } else {
                hex(name.as_bytes())
            }
        })
        .collect();
    names.sort_unstable();
    if names.is_empty() {
        "-".into()
    } else {
        names.join(",")
    }
}

fn unknown_names(root: &NamedTag, compound_name: &str, known: &[&str]) -> Result<String, String> {
    let fields = match &root.value {
        Tag::Compound(fields) => fields,
        _ => return Err("NBT root must be a compound".into()),
    };
    let compound = fields
        .iter()
        .find(|(name, _)| name == compound_name)
        .map(|(_, tag)| tag)
        .ok_or_else(|| format!("missing {compound_name}"))?;
    let fields = match compound {
        Tag::Compound(fields) => fields,
        _ => return Err(format!("{compound_name} must be a compound")),
    };
    Ok(encoded_names(
        fields
            .iter()
            .map(|(name, _)| name.as_str())
            .filter(|name| !known.contains(name)),
    ))
}

fn required_child<'a>(root: &'a NamedTag, name: &str) -> Result<&'a Tag, String> {
    let Tag::Compound(fields) = &root.value else {
        return Err("NBT root must be a compound".into());
    };
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
        .ok_or_else(|| format!("missing {name}"))
}

fn list_count(value: Option<&Tag>, name: &str) -> Result<usize, String> {
    match value {
        None => Ok(0),
        Some(Tag::List { values, .. }) => Ok(values.len()),
        Some(_) => Err(format!("{name} must be a list")),
    }
}

fn named_list(root: &NamedTag, name: &str) -> Result<usize, String> {
    let level = required_child(root, "Level")?;
    let Tag::Compound(fields) = level else {
        return Err("Level must be a compound".into());
    };
    let value = fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, tag)| tag);
    let count = list_count(value, name)?;
    if let Some(Tag::List { values, .. }) = value
        && !values.is_empty()
        && values.iter().any(|tag| !matches!(tag, Tag::Compound(_)))
    {
        // Empty lists may use any supported element type; non-empty lists
        // must consist only of compounds, as the typed loader guarantees.
        return Err(format!("{name} contains a non-compound value"));
    }
    Ok(count)
}

fn canonical_output(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("cannot resolve output path: {e}"))?
            .join(path)
    };
    let parent = absolute
        .parent()
        .ok_or_else(|| "output directory must have a parent".to_owned())?;
    let name = absolute
        .file_name()
        .ok_or_else(|| "output directory must have a basename".to_owned())?;
    Ok(parent
        .canonicalize()
        .map_err(|e| format!("output parent must already exist: {e}"))?
        .join(name))
}

fn copied_input(input_dir: &Path, output_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    let input = input_dir
        .canonicalize()
        .map_err(|e| format!("invalid input directory: {e}"))?;
    if !input.is_dir() {
        return Err("input path is not a directory".into());
    }
    for (marker, expected) in [
        (".world-input-copy", WORLD_MARKER),
        (".region-input-copy", REGION_MARKER),
    ] {
        let path = input.join(marker);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| format!("missing copied-input marker {marker}: {e}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!(
                "copied-input marker {marker} must be a regular file"
            ));
        }
        if fs::read_to_string(path).map_err(|e| format!("cannot read marker: {e}"))? != expected {
            return Err(format!("invalid copied-input marker {marker}"));
        }
    }

    let allowed = [
        "level.dat",
        "r.-1.-1.mcr",
        "r.-1.0.mcr",
        "r.0.-1.mcr",
        "r.0.0.mcr",
        ".world-input-copy",
        ".region-input-copy",
    ];
    let entries = fs::read_dir(&input).map_err(|e| format!("cannot list input: {e}"))?;
    let mut seen = HashSet::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot inspect input: {e}"))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err("input contains a non-UTF-8 basename".into());
        };
        if !allowed.contains(&name) || !seen.insert(name.to_owned()) {
            return Err(format!(
                "unexpected or duplicate copied-input entry {name:?}"
            ));
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|e| format!("cannot inspect input file: {e}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!("copied input {name:?} must be a regular file"));
        }
        let cap = if name == "level.dat" {
            MAX_LEVEL_BYTES
        } else {
            MAX_REGION_BYTES
        };
        if metadata.len() > cap {
            return Err(format!("copied input {name:?} exceeds size limit"));
        }
    }
    if seen.len() != allowed.len() {
        return Err("copied input must contain level.dat and exactly four regions".into());
    }

    let output = canonical_output(output_dir)?;
    if output.starts_with(&input) || input.starts_with(&output) {
        return Err("input and retained directories must not overlap".into());
    }
    if output.exists() || fs::symlink_metadata(&output).is_ok() {
        return Err("retained directory must not already exist".into());
    }
    fs::create_dir(&output).map_err(|e| format!("cannot create retained directory: {e}"))?;
    let output = output
        .canonicalize()
        .map_err(|e| format!("cannot resolve retained directory: {e}"))?;
    if output.starts_with(&input) || input.starts_with(&output) {
        return Err("input and retained directories must not overlap".into());
    }
    Ok((input, output))
}

fn region_path(input: &Path, name: &str) -> Result<PathBuf, String> {
    let path_name = Path::new(name);
    if path_name.is_absolute() || path_name.file_name().and_then(|part| part.to_str()) != Some(name)
    {
        return Err("region filename must be a basename".into());
    }
    let path = input.join(name);
    let metadata =
        fs::symlink_metadata(&path).map_err(|e| format!("cannot stat region file: {e}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("region path must be a regular copied file".into());
    }
    if metadata.len() < 8192 || metadata.len() % 4096 != 0 || metadata.len() > MAX_REGION_BYTES {
        return Err("region file size/alignment guard failed".into());
    }
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("cannot resolve region: {e}"))?;
    if canonical.parent() != Some(input) {
        return Err("region file escapes copied input directory".into());
    }
    Ok(canonical)
}

fn retain(output: &Path, id: &str, bytes: &[u8], line: usize) -> Result<(), String> {
    let path = output.join(format!("{id}.nbt"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("line {line}: cannot create retained payload: {e}"))?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .map_err(|e| format!("line {line}: cannot write retained payload: {e}"))
}

fn level_row(id: &str, raw: Vec<u8>, output: &Path, line: usize) -> Result<String, String> {
    let metadata = world::load_level_gzip(&raw, WorldLimits::default())
        .map_err(|e| format!("line {line}: invalid level data: {e}"))?;
    let player_entry_count = match &metadata.player {
        None => 0,
        Some(Tag::Compound(fields)) => fields.len(),
        Some(_) => return Err(format!("line {line}: Player must be a compound")),
    };
    let name = if metadata.name.is_empty() {
        "-".into()
    } else {
        hex(metadata.name.as_bytes())
    };
    let unknown = unknown_names(&metadata.root, "Data", DATA_FIELDS)
        .map_err(|e| format!("line {line}: {e}"))?;
    retain(output, id, &metadata.raw_nbt, line)?;
    Ok(format!(
        "{id}\tlevel\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{player_entry_count}\t{unknown}\t{}\t{}",
        metadata.seed,
        metadata.spawn[0],
        metadata.spawn[1],
        metadata.spawn[2],
        metadata.time,
        metadata.last_played,
        metadata.size_on_disk,
        metadata.dimension,
        metadata.version,
        metadata.rain_time,
        u8::from(metadata.raining),
        metadata.thunder_time,
        u8::from(metadata.thundering),
        u8::from(metadata.player.is_some()),
        metadata.raw_nbt.len(),
        sha(&metadata.raw_nbt),
    ))
}

fn chunk_row(
    id: &str,
    x: i32,
    z: i32,
    raw: Vec<u8>,
    output: &Path,
    line: usize,
) -> Result<String, String> {
    let loaded = world::load_chunk_nbt(raw, (x, z), nbt::Limits::default())
        .map_err(|e| format!("line {line}: invalid chunk: {e}"))?;
    let unknown = unknown_names(&loaded.root, "Level", LEVEL_FIELDS)
        .map_err(|e| format!("line {line}: {e}"))?;
    let arrays = loaded.data.raw_arrays();
    let entities = named_list(&loaded.root, "Entities").map_err(|e| format!("line {line}: {e}"))?;
    let tiles =
        named_list(&loaded.root, "TileEntities").map_err(|e| format!("line {line}: {e}"))?;
    retain(output, id, &loaded.raw_nbt, line)?;
    Ok(format!(
        "{id}\tchunk\tpresent\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{entities}\t{tiles}\t{unknown}\t{}\t{}",
        loaded.x,
        loaded.z,
        loaded.last_update,
        u8::from(loaded.terrain_populated),
        sha(arrays.0),
        sha(arrays.1),
        sha(arrays.2),
        sha(arrays.3),
        sha(&loaded.height_map),
        loaded.raw_nbt.len(),
        sha(&loaded.raw_nbt),
    ))
}

pub fn process_stream<R: BufRead, W: Write>(
    reader: R,
    mut writer: W,
    input_dir: &Path,
    retained_dir: &Path,
) -> Result<(), String> {
    let (input, output) = copied_input(input_dir, retained_dir)?;
    let mut seen = HashSet::new();
    let mut regions: HashMap<String, Vec<u8>> = HashMap::new();
    let mut cases = 0usize;
    for (index, line_result) in reader.lines().enumerate() {
        let line_no = index + 1;
        let line = line_result.map_err(|e| format!("line {line_no}: input read failed: {e}"))?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 5 {
            return Err(format!("line {line_no}: expected exactly five TSV fields"));
        }
        let (id, kind, basename) = (fields[0], fields[1], fields[2]);
        if !valid_id(id) || !seen.insert(id.to_owned()) {
            return Err(format!("line {line_no}: invalid or duplicate case_id"));
        }
        cases += 1;
        if cases > MAX_CASES {
            return Err(format!("line {line_no}: case limit exceeded"));
        }
        let row = match kind {
            "level" => {
                if basename != "level.dat" || fields[3] != "-" || fields[4] != "-" {
                    return Err(format!("line {line_no}: invalid level row"));
                }
                let path = input.join("level.dat");
                let bytes = fs::read(&path)
                    .map_err(|e| format!("line {line_no}: cannot read level.dat: {e}"))?;
                level_row(id, bytes, &output, line_no)?
            }
            "chunk" => {
                let x = parse_coord(fields[3])
                    .ok_or_else(|| format!("line {line_no}: invalid x coordinate"))?;
                let z = parse_coord(fields[4])
                    .ok_or_else(|| format!("line {line_no}: invalid z coordinate"))?;
                let expected = format!(
                    "r.{}.{}.mcr",
                    region::region_coord(x),
                    region::region_coord(z)
                );
                if basename != expected {
                    return Err(format!(
                        "line {line_no}: region basename does not match coordinates"
                    ));
                }
                if !regions.contains_key(basename) {
                    let path = region_path(&input, basename)
                        .map_err(|e| format!("line {line_no}: {e}"))?;
                    let bytes = fs::read(path)
                        .map_err(|e| format!("line {line_no}: cannot read region: {e}"))?;
                    regions.insert(basename.to_owned(), bytes);
                }
                let bytes = regions.get(basename).expect("region cached");
                let file = RegionFile::open(bytes, RegionLimits::default())
                    .map_err(|e| format!("line {line_no}: invalid region: {e}"))?;
                match file
                    .read_local(region::local_coord(x), region::local_coord(z))
                    .map_err(|e| format!("line {line_no}: chunk read failed: {e}"))?
                {
                    None => {
                        format!("{id}\tchunk\tabsent\t{x}\t{z}\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-")
                    }
                    Some(record) => chunk_row(id, x, z, record.nbt, &output, line_no)?,
                }
            }
            _ => return Err(format!("line {line_no}: unknown case kind")),
        };
        writeln!(writer, "{row}").map_err(|e| format!("stdout write failed: {e}"))?;
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: world_probe INPUT_COPY_DIR RETAINED_DIR".into());
    }
    process_stream(
        BufReader::new(io::stdin().lock()),
        io::BufWriter::new(io::stdout().lock()),
        Path::new(&args[0]),
        Path::new(&args[1]),
    )
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("world_probe: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder, write::ZlibEncoder};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        root: PathBuf,
        input: PathBuf,
        output: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "world-probe-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let input = root.join("input");
            fs::create_dir_all(&input).unwrap();
            fs::write(input.join(".world-input-copy"), WORLD_MARKER).unwrap();
            fs::write(input.join(".region-input-copy"), REGION_MARKER).unwrap();
            for name in ["r.-1.-1.mcr", "r.-1.0.mcr", "r.0.-1.mcr", "r.0.0.mcr"] {
                let mut region = vec![0; 8192];
                if name == "r.0.0.mcr" {
                    let raw = nbt::write_named(&test_chunk(), nbt::Limits::default()).unwrap();
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(&raw).unwrap();
                    let compressed = encoder.finish().unwrap();
                    let sectors = (compressed.len() + 5).div_ceil(4096);
                    region.resize((2 + sectors) * 4096, 0);
                    region[0..4].copy_from_slice(&(2_u32 << 8 | sectors as u32).to_be_bytes());
                    region[4096..4100].copy_from_slice(&123_u32.to_be_bytes());
                    region[8192..8196]
                        .copy_from_slice(&(compressed.len() as u32 + 1).to_be_bytes());
                    region[8196] = 2;
                    region[8197..8197 + compressed.len()].copy_from_slice(&compressed);
                }
                fs::write(input.join(name), region).unwrap();
            }
            let level_raw = nbt::write_named(&test_level(), nbt::Limits::default()).unwrap();
            let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
            gzip.write_all(&level_raw).unwrap();
            fs::write(input.join("level.dat"), gzip.finish().unwrap()).unwrap();
            Self {
                root: root.clone(),
                input,
                output: root.join("retained"),
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn test_level() -> NamedTag {
        NamedTag {
            name: String::new(),
            value: Tag::Compound(vec![(
                "Data".into(),
                Tag::Compound(vec![
                    ("RandomSeed".into(), Tag::Long(-9)),
                    ("SpawnX".into(), Tag::Int(-1)),
                    ("SpawnY".into(), Tag::Int(64)),
                    ("SpawnZ".into(), Tag::Int(2)),
                    ("Time".into(), Tag::Long(99)),
                    ("LastPlayed".into(), Tag::Long(12)),
                    ("SizeOnDisk".into(), Tag::Long(45)),
                    ("LevelName".into(), Tag::String("hé".into())),
                    ("version".into(), Tag::Int(19132)),
                    ("rainTime".into(), Tag::Int(0)),
                    ("raining".into(), Tag::Byte(-128)),
                    ("thunderTime".into(), Tag::Int(2)),
                    ("thundering".into(), Tag::Byte(0)),
                    (
                        "Player".into(),
                        Tag::Compound(vec![
                            ("Dimension".into(), Tag::Int(-1)),
                            ("mystery".into(), Tag::Byte(1)),
                        ]),
                    ),
                    ("future☃".into(), Tag::Int(1)),
                    ("".into(), Tag::Byte(0)),
                ]),
            )]),
        }
    }

    fn test_chunk() -> NamedTag {
        NamedTag {
            name: String::new(),
            value: Tag::Compound(vec![(
                "Level".into(),
                Tag::Compound(vec![
                    ("xPos".into(), Tag::Int(0)),
                    ("zPos".into(), Tag::Int(0)),
                    ("Blocks".into(), Tag::ByteArray(vec![0x80_u8 as i8; 32768])),
                    ("Data".into(), Tag::ByteArray(vec![0xff_u8 as i8; 16384])),
                    ("BlockLight".into(), Tag::ByteArray(vec![1; 16384])),
                    ("SkyLight".into(), Tag::ByteArray(vec![2; 16384])),
                    ("HeightMap".into(), Tag::ByteArray(vec![3; 256])),
                    ("LastUpdate".into(), Tag::Long(7)),
                    ("TerrainPopulated".into(), Tag::Byte(-1)),
                    (
                        "Entities".into(),
                        Tag::List {
                            element_type: nbt::TagType::Compound,
                            values: vec![Tag::Compound(vec![(
                                "id".into(),
                                Tag::String("future".into()),
                            )])],
                        },
                    ),
                    (
                        "TileEntities".into(),
                        Tag::List {
                            element_type: nbt::TagType::Byte,
                            values: vec![],
                        },
                    ),
                    ("future☃".into(), Tag::Int(1)),
                    ("".into(), Tag::Byte(0)),
                ]),
            )]),
        }
    }

    #[test]
    fn emits_exact_level_chunk_and_absent_rows_retaining_original_nbt() {
        let f = Fixture::new();
        let input = b"# rows\nmeta\tlevel\tlevel.dat\t-\t-\nchunk\tchunk\tr.0.0.mcr\t-0\t0\nempty\tchunk\tr.0.0.mcr\t1\t0\n";
        let mut out = Vec::new();
        process_stream(&input[..], &mut out, &f.input, &f.output).unwrap();
        let rows = String::from_utf8(out).unwrap();
        let lines: Vec<_> = rows.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].split('\t').count(), 21);
        assert_eq!(lines[1].split('\t').count(), 17);
        assert_eq!(
            lines[2],
            "empty\tchunk\tabsent\t1\t0\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-"
        );
        assert!(lines[0].contains("\t-1\t68c3a9\t19132\t0\t1\t2\t0\t1\t2\t667574757265e29883,~\t"));
        assert!(lines[1].contains("\t7\t1\t"));
        assert!(lines[1].contains("\t1\t0\t667574757265e29883,~\t"));
        assert_eq!(
            fs::read(f.output.join("meta.nbt")).unwrap(),
            nbt_bytes(&test_level())
        );
        assert_eq!(
            fs::read(f.output.join("chunk.nbt")).unwrap(),
            nbt_bytes(&test_chunk())
        );
        assert!(!f.output.join("empty.nbt").exists());
    }

    fn nbt_bytes(tag: &NamedTag) -> Vec<u8> {
        nbt::write_named(tag, nbt::Limits::default()).unwrap()
    }

    #[test]
    fn rejects_invalid_protocol_paths_markers_and_duplicates() {
        let f = Fixture::new();
        for row in [
            "../x\tlevel\tlevel.dat\t-\t-\n",
            "x\tlevel\tlevel.dat\t0\t-\n",
            "x\tchunk\tr.0.0.mcr\t+0\t0\n",
            "x\tchunk\tr.0.0.mcr\t00\t0\n",
            "x\tchunk\tr.0.0.mcr\t2147483648\t0\n",
            "x\tchunk\tr.1.0.mcr\t0\t0\n",
            "x\tunknown\tfoo\t0\t0\n",
            "x\tlevel\tlevel.dat\t-\t-\nx\tlevel\tlevel.dat\t-\t-\n",
            "x\tlevel\tlevel.dat\t-\t-\textra\n",
            " \n",
        ] {
            let result = process_stream(row.as_bytes(), Vec::new(), &f.input, &f.output);
            assert!(result.is_err(), "accepted {row:?}");
            let _ = fs::remove_dir_all(&f.output);
        }
        assert!(process_stream(&b""[..], Vec::new(), &f.input, &f.input.join("nested")).is_err());
        let bad = f.root.join("bad-input");
        fs::create_dir(&bad).unwrap();
        fs::write(bad.join(".world-input-copy"), WORLD_MARKER).unwrap();
        assert!(process_stream(&b""[..], Vec::new(), &bad, &f.output).is_err());
    }

    #[test]
    fn rejects_corrupt_chunk_without_retaining_partial_payload() {
        let f = Fixture::new();
        // A present zlib record whose NBT coordinates do not match the slot is an error,
        // not an absent row and not a retained payload.
        let mut region = fs::read(f.input.join("r.0.0.mcr")).unwrap();
        let mut malformed = test_chunk();
        if let Tag::Compound(root) = &mut malformed.value {
            if let Tag::Compound(level) = &mut root[0].1 {
                level[0].1 = Tag::Int(5);
            }
        }
        let raw = nbt::write_named(&malformed, nbt::Limits::default()).unwrap();
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&raw).unwrap();
        let compressed = enc.finish().unwrap();
        let sectors = (compressed.len() + 5).div_ceil(4096);
        region.resize((2 + sectors) * 4096, 0);
        region[0..4].copy_from_slice(&(2_u32 << 8 | sectors as u32).to_be_bytes());
        region[4096..4100].copy_from_slice(&1_u32.to_be_bytes());
        region[8192..8196].copy_from_slice(&(compressed.len() as u32 + 1).to_be_bytes());
        region[8196] = 2;
        region[8197..8197 + compressed.len()].copy_from_slice(&compressed);
        fs::write(f.input.join("r.0.0.mcr"), region).unwrap();
        let mut out = Vec::new();
        assert!(
            process_stream(
                b"bad\tchunk\tr.0.0.mcr\t0\t0\n".as_slice(),
                &mut out,
                &f.input,
                &f.output
            )
            .is_err()
        );
        assert!(out.is_empty());
        assert!(!f.output.join("bad.nbt").exists());
    }

    #[test]
    fn encoded_unknown_names_are_hex_sorted_and_empty_is_tilde() {
        assert_eq!(encoded_names(["z", "", "☃"].into_iter()), "7a,e29883,~");
        assert_eq!(encoded_names(std::iter::empty()), "-");
    }
}
