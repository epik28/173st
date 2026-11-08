//! Strict, atomic TSV adapter for `beta_core::geometry`.
//!
//! Input rows are `case_id<TAB>operation<TAB>raw_f64_bits...`; output rows
//! preserve input order and encode all floating point results as lowercase bits.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use beta_core::geometry::{Aabb, Axis, Vec3};

const MAX_ROWS: usize = 100_000;
const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_ID_BYTES: usize = 96;
const MAX_ABS_ARGUMENT: f64 = 30_000_000.0;

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
}

fn parse_f64(value: &str, line: usize, operation: &str) -> Result<f64, String> {
    if value.len() != 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "line {line}: {operation} expects exactly 16 lowercase hexadecimal f64 bits"
        ));
    }
    let bits = u64::from_str_radix(value, 16)
        .map_err(|_| format!("line {line}: invalid f64 bits for {operation}"))?;
    let number = f64::from_bits(bits);
    if !number.is_finite() {
        return Err(format!("line {line}: {operation} argument must be finite"));
    }
    if number.abs() > MAX_ABS_ARGUMENT {
        return Err(format!(
            "line {line}: {operation} argument exceeds {MAX_ABS_ARGUMENT}"
        ));
    }
    Ok(number)
}

fn axis_for(operation: &str) -> Axis {
    match operation {
        "box_clip_x" | "vec_intermediate_x" => Axis::X,
        "box_clip_y" | "vec_intermediate_y" => Axis::Y,
        "box_clip_z" | "vec_intermediate_z" => Axis::Z,
        _ => unreachable!("axis operation was validated"),
    }
}

fn arity(operation: &str) -> Option<usize> {
    Some(match operation {
        "box_new" | "box_average_edge" => 6,
        "box_sweep" | "box_expand" | "box_offset" => 9,
        "box_clip_x" | "box_clip_y" | "box_clip_z" => 13,
        "box_intersects" => 12,
        "box_contains" => 9,
        "box_ray" => 12,
        "vec_new" | "vec_normalize" | "vec_length" => 3,
        "vec_to" | "vec_cross" | "vec_add" | "vec_distance" | "vec_distance_sq" => 6,
        "vec_intermediate_x" | "vec_intermediate_y" | "vec_intermediate_z" => 7,
        _ => return None,
    })
}

fn parse_values(operation: &str, arguments: &[&str], line: usize) -> Result<Vec<f64>, String> {
    let expected =
        arity(operation).ok_or_else(|| format!("line {line}: unknown operation '{operation}'"))?;
    if arguments.len() != expected {
        return Err(format!(
            "line {line}: operation '{operation}' expects {expected} argument(s), got {}",
            arguments.len()
        ));
    }
    arguments
        .iter()
        .map(|argument| parse_f64(argument, line, operation))
        .collect()
}

fn box_from(values: &[f64]) -> Aabb {
    Aabb::new(
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
    )
}

fn vector_from(values: &[f64]) -> Vec3 {
    Vec3::new(values[0], values[1], values[2])
}

fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn vector_text(vector: Vec3) -> String {
    format!("{},{},{}", bits(vector.x), bits(vector.y), bits(vector.z))
}

fn box_text(bounds: Aabb) -> String {
    format!(
        "{},{},{},{},{},{}",
        bits(bounds.min[0]),
        bits(bounds.min[1]),
        bits(bounds.min[2]),
        bits(bounds.max[0]),
        bits(bounds.max[1]),
        bits(bounds.max[2])
    )
}

fn evaluate(operation: &str, arguments: &[&str], line: usize) -> Result<String, String> {
    let values = parse_values(operation, arguments, line)?;
    let result = match operation {
        "box_new" => box_text(box_from(&values)),
        "box_sweep" => box_text(box_from(&values).swept(values[6], values[7], values[8])),
        "box_expand" => box_text(box_from(&values).expanded(values[6], values[7], values[8])),
        "box_offset" => box_text(box_from(&values).offset(values[6], values[7], values[8])),
        "box_clip_x" | "box_clip_y" | "box_clip_z" => {
            bits(box_from(&values).clip(box_from(&values[6..]), axis_for(operation), values[12]))
        }
        "box_intersects" => {
            if box_from(&values).intersects(box_from(&values[6..])) {
                "1".to_owned()
            } else {
                "0".to_owned()
            }
        }
        "box_contains" => {
            if box_from(&values).contains(vector_from(&values[6..])) {
                "1".to_owned()
            } else {
                "0".to_owned()
            }
        }
        "box_average_edge" => bits(box_from(&values).average_edge()),
        "box_ray" => match box_from(&values)
            .ray_intersect(vector_from(&values[6..]), vector_from(&values[9..]))
        {
            Some(hit) => format!("{},{}", hit.face, vector_text(hit.position)),
            None => "none".to_owned(),
        },
        "vec_new" => vector_text(vector_from(&values)),
        "vec_to" => vector_text(vector_from(&values).vector_to(vector_from(&values[3..]))),
        "vec_cross" => vector_text(vector_from(&values).cross(vector_from(&values[3..]))),
        "vec_add" => vector_text(vector_from(&values).add_xyz(values[3], values[4], values[5])),
        "vec_normalize" => vector_text(vector_from(&values).normalize()),
        "vec_length" => bits(vector_from(&values).length()),
        "vec_distance" => bits(vector_from(&values).distance(vector_from(&values[3..]))),
        "vec_distance_sq" => bits(vector_from(&values).distance_squared(vector_from(&values[3..]))),
        "vec_intermediate_x" | "vec_intermediate_y" | "vec_intermediate_z" => {
            let start = vector_from(&values);
            let end = vector_from(&values[3..]);
            match start.intermediate(end, axis_for(operation), values[6]) {
                Some(point) => vector_text(point),
                None => "none".to_owned(),
            }
        }
        _ => unreachable!("operation arity was validated"),
    };
    Ok(result)
}

fn process_with_limits<R: Read, W: Write>(
    reader: R,
    mut writer: W,
    max_rows: usize,
    max_input_bytes: usize,
) -> Result<(), String> {
    let mut input = Vec::new();
    reader
        .take(max_input_bytes.saturating_add(1) as u64)
        .read_to_end(&mut input)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    if input.len() > max_input_bytes {
        return Err(format!("input byte limit {max_input_bytes} exceeded"));
    }
    if input.is_empty() || input.last() != Some(&b'\n') {
        return Err("TSV input must be non-empty and end with LF".to_owned());
    }
    if input.iter().any(|byte| !byte.is_ascii()) {
        return Err("TSV input must be ASCII".to_owned());
    }

    let mut ids = HashSet::new();
    let mut output = String::new();
    let mut row_count = 0usize;
    for (index, row) in input[..input.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        let line = index + 1;
        if row.is_empty() {
            return Err(format!("line {line}: blank rows are not allowed"));
        }
        let row = std::str::from_utf8(row)
            .map_err(|_| format!("line {line}: input row is not valid UTF-8"))?;
        let fields: Vec<_> = row.split('\t').collect();
        if fields.len() < 2 {
            return Err(format!("line {line}: expected case ID and operation"));
        }
        let id = fields[0];
        if !valid_id(id) {
            return Err(format!("line {line}: invalid case ID"));
        }
        if !ids.insert(id) {
            return Err(format!("line {line}: duplicate case ID '{id}'"));
        }
        row_count += 1;
        if row_count > max_rows {
            return Err(format!("row limit {max_rows} exceeded"));
        }
        let result = evaluate(fields[1], &fields[2..], line)?;
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
            eprintln!("geometry_probe error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_native_corpus_matches_exactly() {
        let input = include_bytes!("../../../tests/fixtures/geometry/input.tsv");
        let expected = include_bytes!("../../../tests/fixtures/geometry/reference.tsv");
        let mut output = Vec::new();
        process_stream(input.as_slice(), &mut output).unwrap();
        assert_eq!(output.as_slice(), expected);
        assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 6_278);
    }

    #[test]
    fn malformed_input_is_rejected_atomically() {
        let invalid: &[&[u8]] = &[
            b"",
            b"id\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000",
            b"id\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\n\n",
            b"id\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\r\n",
            b"bad id\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\n",
            b"id\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\nid\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\n",
            b"id\tunknown\t0000000000000000\n",
            b"id\tvec_new\t0000000000000000\t0000000000000000\n",
            b"id\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\textra\n",
            b"id\tvec_new\t000000000000000G\t0000000000000000\t0000000000000000\n",
            b"id\tvec_new\t000000000000000A\t0000000000000000\t0000000000000000\n",
            b"id\tvec_new\t7ff0000000000000\t0000000000000000\t0000000000000000\n",
            b"id\tvec_new\t7ff8000000000000\t0000000000000000\t0000000000000000\n",
            b"id\tvec_new\t41dc6bf400000001\t0000000000000000\t0000000000000000\n",
            b"# comment\n",
            b"\xff\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\n",
        ];
        for input in invalid {
            let mut output = Vec::new();
            assert!(process_stream(*input, &mut output).is_err(), "{input:?}");
            assert!(output.is_empty(), "invalid input emitted stdout: {input:?}");
        }

        let late_error = b"ok\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\nbad\tvec_new\t0000000000000000\n";
        let mut output = Vec::new();
        assert!(process_stream(late_error.as_slice(), &mut output).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn configured_row_and_byte_limits_are_enforced_atomically() {
        let rows = b"a\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\nb\tvec_new\t0000000000000000\t0000000000000000\t0000000000000000\n";
        let mut output = Vec::new();
        assert!(process_with_limits(rows.as_slice(), &mut output, 1, 512).is_err());
        assert!(output.is_empty());

        let mut output = Vec::new();
        assert!(process_with_limits(rows.as_slice(), &mut output, 5, 16).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn all_contract_operations_have_exact_shapes_and_raw_bit_output() {
        let input = concat!(
            "box\tbox_new\t0000000000000000\t0000000000000000\t0000000000000000\t3ff0000000000000\t3ff0000000000000\t3ff0000000000000\n",
            "ray\tbox_ray\t0000000000000000\t0000000000000000\t0000000000000000\t3ff0000000000000\t3ff0000000000000\t3ff0000000000000\tbff0000000000000\t3fe0000000000000\t3fe0000000000000\t4000000000000000\t3fe0000000000000\t3fe0000000000000\n",
            "none\tvec_intermediate_x\t0000000000000000\t0000000000000000\t0000000000000000\t3ff0000000000000\t0000000000000000\t0000000000000000\t4000000000000000\n"
        );
        let mut output = Vec::new();
        process_stream(input.as_bytes(), &mut output).unwrap();
        assert_eq!(
            output,
            concat!(
                "box\t0000000000000000,0000000000000000,0000000000000000,3ff0000000000000,3ff0000000000000,3ff0000000000000\n",
                "ray\t4,0000000000000000,3fe0000000000000,3fe0000000000000\n",
                "none\tnone\n"
            )
            .as_bytes()
        );
    }
}
