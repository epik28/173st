//! Strict TSV adapter for the pure `beta_core::math` API.
//!
//! Each input row contains one case: `case_id<TAB>operation<TAB>arguments...`.
//! Floating point arguments and results are represented by raw IEEE-754 bits.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use beta_core::math::{
    MathError, abs_f32, abs_max_f64, cos, floor_div_i32, floor_f32, floor_f64, is_null_or_empty,
    sin, sqrt_f32, sqrt_f64_to_f32,
};

const MAX_CASES: usize = 200_000;
const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
const MAX_CASE_ID_BYTES: usize = 96;

fn valid_case_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CASE_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
}

fn parse_f32_bits(value: &str, line: usize, operation: &str) -> Result<f32, String> {
    if value.len() != 8 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "line {line}: {operation} expects exactly 8 hexadecimal f32 bits"
        ));
    }
    let bits = u32::from_str_radix(value, 16)
        .map_err(|_| format!("line {line}: invalid f32 bits for {operation}"))?;
    Ok(f32::from_bits(bits))
}

fn parse_f64_bits(value: &str, line: usize, operation: &str) -> Result<f64, String> {
    if value.len() != 16 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "line {line}: {operation} expects exactly 16 hexadecimal f64 bits"
        ));
    }
    let bits = u64::from_str_radix(value, 16)
        .map_err(|_| format!("line {line}: invalid f64 bits for {operation}"))?;
    Ok(f64::from_bits(bits))
}

fn parse_i32(value: &str, line: usize, operation: &str) -> Result<i32, String> {
    value.parse::<i32>().map_err(|_| {
        format!("line {line}: invalid signed 32-bit integer for {operation}: '{value}'")
    })
}

fn decode_utf8_hex(value: &str, line: usize) -> Result<String, String> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "line {line}: string argument must be even-length hexadecimal UTF-8"
        ));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let high = (pair[0] as char)
            .to_digit(16)
            .ok_or_else(|| format!("line {line}: invalid hexadecimal string argument"))?;
        let low = (pair[1] as char)
            .to_digit(16)
            .ok_or_else(|| format!("line {line}: invalid hexadecimal string argument"))?;
        bytes.push(((high << 4) | low) as u8);
    }
    String::from_utf8(bytes).map_err(|_| format!("line {line}: string argument is not UTF-8"))
}

fn evaluate(operation: &str, arguments: &[&str], line: usize) -> Result<String, String> {
    let arity = match operation {
        "sin" | "cos" | "sqrt_f32" | "floor_f32" | "abs_f32" | "sqrt_f64_to_f32" | "floor_f64"
        | "null_or_empty" => 1,
        "abs_max_f64" | "floor_div_i32" => 2,
        _ => return Err(format!("line {line}: unknown operation '{operation}'")),
    };
    if arguments.len() != arity {
        return Err(format!(
            "line {line}: operation '{operation}' expects {arity} argument(s), got {}",
            arguments.len()
        ));
    }

    match operation {
        "sin" => Ok(format!(
            "{:08x}",
            sin(parse_f32_bits(arguments[0], line, operation)?).to_bits()
        )),
        "cos" => Ok(format!(
            "{:08x}",
            cos(parse_f32_bits(arguments[0], line, operation)?).to_bits()
        )),
        "sqrt_f32" => Ok(format!(
            "{:08x}",
            sqrt_f32(parse_f32_bits(arguments[0], line, operation)?).to_bits()
        )),
        "sqrt_f64_to_f32" => Ok(format!(
            "{:08x}",
            sqrt_f64_to_f32(parse_f64_bits(arguments[0], line, operation)?).to_bits()
        )),
        "floor_f32" => Ok(floor_f32(parse_f32_bits(arguments[0], line, operation)?).to_string()),
        "floor_f64" => Ok(floor_f64(parse_f64_bits(arguments[0], line, operation)?).to_string()),
        "abs_f32" => Ok(format!(
            "{:08x}",
            abs_f32(parse_f32_bits(arguments[0], line, operation)?).to_bits()
        )),
        "abs_max_f64" => Ok(format!(
            "{:016x}",
            abs_max_f64(
                parse_f64_bits(arguments[0], line, operation)?,
                parse_f64_bits(arguments[1], line, operation)?,
            )
            .to_bits()
        )),
        "floor_div_i32" => {
            let dividend = parse_i32(arguments[0], line, operation)?;
            let divisor = parse_i32(arguments[1], line, operation)?;
            Ok(match floor_div_i32(dividend, divisor) {
                Ok(value) => value.to_string(),
                Err(MathError::DivisionByZero) => "error_arithmetic".to_owned(),
            })
        }
        "null_or_empty" => {
            let is_empty = if arguments[0] == "null" {
                is_null_or_empty(None)
            } else {
                let value = decode_utf8_hex(arguments[0], line)?;
                is_null_or_empty(Some(&value))
            };
            Ok(if is_empty { "1" } else { "0" }.to_owned())
        }
        _ => unreachable!("operation was validated above"),
    }
}

fn process_with_limits<R: Read, W: Write>(
    reader: R,
    mut writer: W,
    max_cases: usize,
    max_input_bytes: usize,
) -> Result<(), String> {
    let mut input = Vec::new();
    reader
        .take(max_input_bytes.saturating_add(1) as u64)
        .read_to_end(&mut input)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    if input.len() > max_input_bytes {
        return Err(format!("input exceeds {max_input_bytes}-byte limit"));
    }
    let input = std::str::from_utf8(&input).map_err(|_| "input is not valid UTF-8".to_owned())?;

    let mut ids = HashSet::new();
    let mut rows = String::new();
    let mut cases = 0usize;
    for (line_index, line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() < 3 {
            return Err(format!(
                "line {line_number}: expected case_id, operation and argument field(s)"
            ));
        }
        let case_id = fields[0];
        if !valid_case_id(case_id) {
            return Err(format!("line {line_number}: invalid case_id '{case_id}'"));
        }
        if !ids.insert(case_id) {
            return Err(format!("line {line_number}: duplicate case_id '{case_id}'"));
        }
        cases += 1;
        if cases > max_cases {
            return Err(format!("case limit {max_cases} exceeded"));
        }
        let result = evaluate(fields[1], &fields[2..], line_number)?;
        rows.push_str(case_id);
        rows.push('\t');
        rows.push_str(&result);
        rows.push('\n');
    }

    writer
        .write_all(rows.as_bytes())
        .map_err(|error| format!("failed to write stdout: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("failed to flush stdout: {error}"))
}

pub fn process_stream<R: Read, W: Write>(reader: R, writer: W) -> Result<(), String> {
    process_with_limits(reader, writer, MAX_CASES, MAX_INPUT_BYTES)
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
            eprintln!("math_probe error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_bytes(input: &[u8]) -> Result<Vec<u8>, String> {
        let mut output = Vec::new();
        process_stream(input, &mut output)?;
        Ok(output)
    }

    #[test]
    fn all_c01_cases_match_committed_original_output_exactly() {
        let mut output = Vec::new();
        process_stream(
            include_bytes!("../../../tests/fixtures/math/input.tsv").as_slice(),
            &mut output,
        )
        .unwrap();
        assert_eq!(
            output.as_slice(),
            include_bytes!("../../../tests/fixtures/math/reference.tsv")
        );
    }

    #[test]
    fn supports_all_operations_and_emits_lf_raw_bits() {
        let input = b"s\tsin\t3f800000\nc\tcos\t3f800000\nsqf\tsqrt_f32\t40800000\nsqd\tsqrt_f64_to_f32\t4010000000000000\nff\tfloor_f32\tbf000000\nfd\tfloor_f64\tbfe0000000000000\na\tabs_f32\tbf800000\nam\tabs_max_f64\tc000000000000000\t4000000000000000\nd\tfloor_div_i32\t-7\t3\nz\tfloor_div_i32\t1\t0\nn\tnull_or_empty\tnull\ne\tnull_or_empty\t\nu\tnull_or_empty\te29883\n";
        let output = run_bytes(input).unwrap();
        let text = std::str::from_utf8(&output).unwrap();
        assert!(text.ends_with('\n'));
        assert_eq!(text.lines().count(), 13);
        assert!(text.lines().any(|line| line == "d\t-3"));
        assert!(text.lines().any(|line| line == "z\terror_arithmetic"));
        assert!(text.lines().any(|line| line == "n\t1"));
        assert!(text.lines().any(|line| line == "u\t0"));
    }

    #[test]
    fn rejects_bad_shapes_ids_duplicate_operations_and_ranges_atomically() {
        let invalid = [
            b"only-two-fields\tsin\n".as_slice(),
            b"bad id\tsin\t00000000\n".as_slice(),
            b"x\tsin\t00000000\nx\tcos\t00000000\n".as_slice(),
            b"x\tunknown\t0\n".as_slice(),
            b"x\tsin\t0\n".as_slice(),
            b"x\tsin\t0000000g\n".as_slice(),
            b"x\tabs_max_f64\t0000000000000000\n".as_slice(),
            b"x\tfloor_div_i32\t2147483648\t1\n".as_slice(),
            b"x\tnull_or_empty\t0g\n".as_slice(),
            b"x\tnull_or_empty\tff\n".as_slice(),
        ];
        for input in invalid {
            let mut output = Vec::new();
            assert!(process_stream(input, &mut output).is_err(), "{input:?}");
            assert!(output.is_empty(), "invalid input wrote stdout: {input:?}");
        }
        let input = b"valid\tsin\t00000000\nbad\tsin\t0000000\n";
        let mut output = Vec::new();
        assert!(process_stream(&input[..], &mut output).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn blank_lines_and_comments_are_skipped_but_whitespace_is_rejected() {
        assert_eq!(
            run_bytes(b"\n# comment\n\nx\tsin\t00000000\n").unwrap(),
            b"x\t00000000\n"
        );
        assert!(run_bytes(b" \n").is_err());
        assert!(run_bytes(b"\t\n").is_err());
    }

    #[test]
    fn enforces_case_and_input_byte_limits() {
        let mut output = Vec::new();
        assert!(
            process_with_limits(
                &b"a\tsin\t00000000\nb\tsin\t00000000\n"[..],
                &mut output,
                1,
                128,
            )
            .is_err()
        );
        assert!(output.is_empty());

        let mut output = Vec::new();
        assert!(process_with_limits(&b"a\tsin\t00000000\n"[..], &mut output, 10, 4).is_err());
        assert!(output.is_empty());
    }
}
