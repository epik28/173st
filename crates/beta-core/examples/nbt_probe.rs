//! TSV probe for the beta_core NBT parser.
//!
//! Input: `case_id<TAB>hex_bytes` (one complete uncompressed named root per line).
//! Output: `case_id<TAB>0<TAB>round_trip_hex<TAB>consumed_bytes`.

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use beta_core::nbt::{Limits, read_named, write_named};

const MAX_CASES: usize = 100_000;
const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

fn valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return Err("hex_bytes must be non-empty and have an even number of digits".into());
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
        let high =
            hex_value(pair[0]).ok_or_else(|| format!("invalid hex at byte {}", index * 2))?;
        let low =
            hex_value(pair[1]).ok_or_else(|| format!("invalid hex at byte {}", index * 2 + 1))?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

pub fn process_stream<R: BufRead, W: Write>(reader: R, mut writer: W) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut case_count = 0usize;
    let mut input_bytes = 0usize;

    for (line_index, line_result) in reader.lines().enumerate() {
        let line_no = line_index + 1;
        let line = line_result.map_err(|error| format!("line {line_no}: read error: {error}"))?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let case_id = fields.next().unwrap_or_default();
        let hex = fields
            .next()
            .ok_or_else(|| format!("line {line_no}: expected case_id<TAB>hex_bytes"))?;
        if fields.next().is_some() {
            return Err(format!("line {line_no}: expected exactly two TSV fields"));
        }
        if !valid_case_id(case_id) {
            return Err(format!("line {line_no}: invalid case_id"));
        }
        if !seen.insert(case_id.to_owned()) {
            return Err(format!("line {line_no}: duplicate case_id {case_id:?}"));
        }
        case_count += 1;
        if case_count > MAX_CASES {
            return Err(format!("line {line_no}: case limit {MAX_CASES} exceeded"));
        }
        if hex.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(format!(
                "line {line_no}: hex_bytes cannot contain whitespace"
            ));
        }
        if hex.is_empty() || !hex.len().is_multiple_of(2) {
            return Err(format!(
                "line {line_no}: hex_bytes must be non-empty and have an even number of digits"
            ));
        }
        input_bytes = input_bytes
            .checked_add(hex.len() / 2)
            .ok_or_else(|| format!("line {line_no}: input byte count overflow"))?;
        if input_bytes > MAX_INPUT_BYTES {
            return Err(format!(
                "line {line_no}: total input limit {MAX_INPUT_BYTES} exceeded"
            ));
        }
        let bytes = decode_hex(hex).map_err(|error| format!("line {line_no}: {error}"))?;
        let (tree, consumed) = read_named(&bytes, Limits::default())
            .map_err(|error| format!("line {line_no}, case {case_id:?}: {error}"))?;
        let round_trip = write_named(&tree, Limits::default())
            .map_err(|error| format!("line {line_no}, case {case_id:?}: {error}"))?;
        writeln!(
            writer,
            "{case_id}\t0\t{}\t{consumed}",
            encode_hex(&round_trip)
        )
        .map_err(|error| format!("line {line_no}: write error: {error}"))?;
    }
    writer
        .flush()
        .map_err(|error| format!("flush error: {error}"))
}

fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    process_stream(stdin.lock(), io::BufWriter::new(stdout.lock()))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("nbt_probe error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(input: &[u8]) -> (Result<(), String>, String) {
        let mut output = Vec::new();
        let result = process_stream(input, &mut output);
        (result, String::from_utf8(output).unwrap())
    }

    #[test]
    fn round_trips_trees_in_runner_protocol() {
        // Named root Compound("root") containing Int("z")=3 and Byte("a")=-1.
        let input = b"tree\t0a0004726f6f740300017a0000000301000161ff00\n";
        let (result, output) = process(input);
        assert!(result.is_ok());
        assert_eq!(
            output,
            "tree\t0\t0a0004726f6f7401000161ff0300017a0000000300\t21\n"
        );
    }

    #[test]
    fn preserves_float_bits_and_consumed_length() {
        // Named root Float("f") with a noncanonical NaN payload.
        let (result, output) = process(b"float\t050001667fc01234\n");
        assert!(result.is_ok());
        assert_eq!(output, "float\t0\t050001667fc01234\t8\n");
    }

    #[test]
    fn accepts_empty_lines_comments_and_rejects_bad_protocol() {
        let (result, output) = process(b"\n# sample\nempty\t0a000000\n");
        assert!(result.is_ok());
        assert_eq!(output, "empty\t0\t0a000000\t4\n");
        for invalid in [
            &b"bad\n"[..],
            &b"id\t0a0000\textra\n"[..],
            &b"id\t0a0000\nid\t0a0000\n"[..],
            &b"id\t0a0 00\n"[..],
            &b"id\t0a000\n"[..],
            &b"id\t0a0000ff\n"[..],
        ] {
            assert!(process(invalid).0.is_err(), "input {invalid:?}");
        }
    }

    #[test]
    fn line_errors_do_not_leak_to_stdout_and_include_case_context() {
        let (result, output) = process(b"invalid\t0000\n");
        assert!(result.unwrap_err().contains("case \"invalid\""));
        assert!(output.is_empty());
    }
}
