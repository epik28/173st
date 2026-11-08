//! Strict TSV probe for Java-compatible primitive conversions and String hash.
//!
//! Input: `case_id<TAB>operation<TAB>argument`; output: `case_id<TAB>decimal_result`.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use beta_core::java_compat::{f32_to_i32, f32_to_i64, f64_to_i32, f64_to_i64, string_hash_code};

const MAX_ROWS: usize = 100_000;
const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_ID_BYTES: usize = 96;

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn parse_bits(value: &str, width: usize, line: usize) -> Result<u64, String> {
    if value.len() != width || !value.bytes().all(|byte| hex_value(byte).is_some()) {
        return Err(format!(
            "line {line}: raw bits must be exactly {width} lowercase hexadecimal digits"
        ));
    }
    u64::from_str_radix(value, 16).map_err(|_| format!("line {line}: invalid raw bits"))
}

fn decode_utf8_hex(value: &str, line: usize) -> Result<String, String> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|byte| hex_value(byte).is_some()) {
        return Err(format!(
            "line {line}: string must be lowercase hexadecimal UTF-8 with an even length"
        ));
    }
    let bytes: Vec<_> = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_value(pair[0]).unwrap() << 4) | hex_value(pair[1]).unwrap())
        .collect();
    String::from_utf8(bytes).map_err(|_| format!("line {line}: string bytes are not valid UTF-8"))
}

fn evaluate(operation: &str, argument: &str, line: usize) -> Result<String, String> {
    match operation {
        "f32_to_i32" => {
            Ok(f32_to_i32(f32::from_bits(parse_bits(argument, 8, line)? as u32)).to_string())
        }
        "f64_to_i32" => Ok(f64_to_i32(f64::from_bits(parse_bits(argument, 16, line)?)).to_string()),
        "f32_to_i64" => {
            Ok(f32_to_i64(f32::from_bits(parse_bits(argument, 8, line)? as u32)).to_string())
        }
        "f64_to_i64" => Ok(f64_to_i64(f64::from_bits(parse_bits(argument, 16, line)?)).to_string()),
        "string_hash_code" => Ok(string_hash_code(&decode_utf8_hex(argument, line)?).to_string()),
        _ => Err(format!("line {line}: unknown operation '{operation}'")),
    }
}

fn process_with_limits<R: Read, W: Write>(
    reader: R,
    mut writer: W,
    max_rows: usize,
    max_input_bytes: usize,
) -> Result<(), String> {
    let mut bytes = Vec::new();
    reader
        .take(max_input_bytes.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    if bytes.len() > max_input_bytes {
        return Err(format!("input byte limit {max_input_bytes} exceeded"));
    }
    let input = std::str::from_utf8(&bytes).map_err(|_| "input is not valid UTF-8".to_owned())?;
    if !input.is_ascii() {
        return Err("TSV input must be ASCII".to_owned());
    }
    if !input.ends_with('\n') || input.is_empty() {
        return Err("TSV input must be non-empty and end with LF".to_owned());
    }

    let mut ids = HashSet::new();
    let mut output = String::new();
    let mut rows = 0usize;
    for (line_index, row) in input[..input.len() - 1].split('\n').enumerate() {
        let line = line_index + 1;
        let fields: Vec<_> = row.split('\t').collect();
        if fields.len() != 3 {
            return Err(format!("line {line}: expected exactly three TSV fields"));
        }
        let id = fields[0];
        if !valid_id(id) {
            return Err(format!("line {line}: invalid case ID"));
        }
        if !ids.insert(id) {
            return Err(format!("line {line}: duplicate case ID '{id}'"));
        }
        rows += 1;
        if rows > max_rows {
            return Err(format!("row limit {max_rows} exceeded"));
        }

        let result = evaluate(fields[1], fields[2], line)?;
        output.push_str(id);
        output.push('\t');
        output.push_str(&result);
        output.push('\n');
    }

    writer
        .write_all(output.as_bytes())
        .map_err(|error| format!("failed to write stdout: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("failed to flush stdout: {error}"))
}

pub fn process_stream<R: Read, W: Write>(reader: R, writer: W) -> Result<(), String> {
    process_with_limits(reader, writer, MAX_ROWS, MAX_INPUT_BYTES)
}

fn run() -> Result<(), String> {
    if std::env::args_os().len() != 1 {
        return Err("unexpected command-line arguments".to_owned());
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    process_stream(stdin.lock(), io::BufWriter::new(stdout.lock()))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("java_compat_probe error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_committed_java_cases_match_reference_exactly() {
        let input = include_bytes!("../../../tests/fixtures/java/input.tsv");
        let expected = include_bytes!("../../../tests/fixtures/java/reference.tsv");
        let mut output = Vec::new();
        process_stream(input.as_slice(), &mut output).unwrap();
        assert_eq!(output.as_slice(), expected);
        assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 18_493);
    }

    #[test]
    fn accepts_all_operations_and_emits_decimal_results_with_lf() {
        let input = b"a\tf32_to_i32\tbf800000\nb\tf64_to_i32\t7ff0000000000000\nc\tf32_to_i64\t7fc00001\nd\tf64_to_i64\tc010000000000000\ne\tstring_hash_code\tf09f9880\nf\tstring_hash_code\t\n";
        let mut output = Vec::new();
        process_stream(input.as_slice(), &mut output).unwrap();
        assert_eq!(
            output,
            b"a\t-1\nb\t2147483647\nc\t0\nd\t-4\ne\t1772899\nf\t0\n"
        );
        assert!(output.ends_with(b"\n"));
        assert!(!output.contains(&b'\r'));
    }

    #[test]
    fn malformed_protocol_is_rejected_without_partial_stdout() {
        let invalid: &[&[u8]] = &[
            b"id\tf32_to_i32\n",
            b"id\tf32_to_i32\t00000000\textra\n",
            b"bad id\tf32_to_i32\t00000000\n",
            b"x\tf32_to_i32\t00000000\nx\tf32_to_i64\t00000000\n",
            b"x\tunknown\t00000000\n",
            b"x\tf32_to_i32\t0000000\n",
            b"x\tf64_to_i64\t000000000000000g\n",
            b"x\tf32_to_i32\t0x0000000\n",
            b"x\tstring_hash_code\t0\n",
            b"x\tstring_hash_code\tFF\n",
            b"x\tstring_hash_code\t80\n",
            b"\n",
            b"x\tf32_to_i32\t3f800000",
            b"x\tf32_to_i32\t3f800000\r\n",
            b"# comment\n",
        ];
        for input in invalid {
            let mut output = Vec::new();
            assert!(process_stream(*input, &mut output).is_err(), "{input:?}");
            assert!(output.is_empty(), "invalid input emitted stdout: {input:?}");
        }

        let late_error = b"good\tf32_to_i32\t3f800000\nbad\tunknown\t00\n";
        let mut output = Vec::new();
        assert!(process_stream(late_error.as_slice(), &mut output).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn enforces_row_and_byte_limits_atomically() {
        let mut output = Vec::new();
        let rows = b"a\tf32_to_i32\t00000000\nb\tf32_to_i32\t00000000\n";
        assert!(process_with_limits(rows.as_slice(), &mut output, 1, 128).is_err());
        assert!(output.is_empty());

        let mut output = Vec::new();
        assert!(
            process_with_limits(b"a\tf32_to_i32\t00000000\n".as_slice(), &mut output, 5, 8)
                .is_err()
        );
        assert!(output.is_empty());
    }
}
