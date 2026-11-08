//! Strict atomic probe for Beta 1.7.3 saved-light arithmetic.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use beta_core::light::{
    LightDimension, LightError, celestial_angle, combined_light, emission_level, sample_brightness,
    sky_subtraction,
};

const MAX_ROWS: usize = 100_000;
const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_ID_BYTES: usize = 96;

#[derive(Clone, Copy)]
enum Input {
    Sample {
        dimension: LightDimension,
        block: u8,
        sky: u8,
        subtraction: u8,
        minimum: u8,
    },
    Time {
        dimension: LightDimension,
        time: i64,
        partial: f32,
    },
    Emission(u8),
}

struct Record {
    id: String,
    input: Input,
}

fn line_error(line: usize, message: &str) -> String {
    format!("line {line}: {message}")
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_BYTES
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
}

fn canonical_i64(text: &str, line: usize, label: &str) -> Result<i64, String> {
    let value = text
        .parse::<i64>()
        .map_err(|_| line_error(line, &format!("invalid {label} i64")))?;
    if value.to_string() != text {
        return Err(line_error(
            line,
            &format!("{label} must use canonical signed decimal"),
        ));
    }
    Ok(value)
}

fn bounded_u8(text: &str, line: usize, label: &str, maximum: u8) -> Result<u8, String> {
    let value = canonical_i64(text, line, label)?;
    let value = u8::try_from(value)
        .map_err(|_| line_error(line, &format!("{label} must be in 0..={maximum}")))?;
    if value > maximum {
        return Err(line_error(
            line,
            &format!("{label} must be in 0..={maximum}"),
        ));
    }
    Ok(value)
}

fn dimension(text: &str, line: usize) -> Result<LightDimension, String> {
    match text {
        "0" => Ok(LightDimension::Overworld),
        "-1" => Ok(LightDimension::Nether),
        _ => Err(line_error(line, "dimension must be canonical 0 or -1")),
    }
}

fn partial_f32(text: &str, line: usize) -> Result<f32, String> {
    if text.len() != 8
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(line_error(line, "partial must be 8 lowercase hex digits"));
    }
    let bits =
        u32::from_str_radix(text, 16).map_err(|_| line_error(line, "invalid partial raw bits"))?;
    let value = f32::from_bits(bits);
    if !value.is_finite() || !(0.0_f32..=1.0_f32).contains(&value) {
        return Err(line_error(line, "partial must be finite and in 0..=1"));
    }
    Ok(value)
}

fn parse_record(row: &str, line: usize) -> Result<Record, String> {
    let fields: Vec<_> = row.split('\t').collect();
    if fields.is_empty() || !valid_id(fields[0]) {
        return Err(line_error(line, "invalid case ID"));
    }
    let input = match fields.get(1).copied() {
        Some("sample") if fields.len() == 7 => Input::Sample {
            dimension: dimension(fields[2], line)?,
            block: bounded_u8(fields[3], line, "block", 15)?,
            sky: bounded_u8(fields[4], line, "sky", 15)?,
            subtraction: bounded_u8(fields[5], line, "subtraction", 15)?,
            minimum: bounded_u8(fields[6], line, "minimum", 15)?,
        },
        Some("time") if fields.len() == 5 => Input::Time {
            dimension: dimension(fields[2], line)?,
            time: canonical_i64(fields[3], line, "time")?,
            partial: partial_f32(fields[4], line)?,
        },
        Some("emission") if fields.len() == 3 => {
            Input::Emission(bounded_u8(fields[2], line, "block_id", u8::MAX)?)
        }
        Some("sample") | Some("time") | Some("emission") => {
            return Err(line_error(line, "wrong field count for operation"));
        }
        Some(_) => return Err(line_error(line, "unknown operation")),
        None => return Err(line_error(line, "missing operation")),
    };
    Ok(Record {
        id: fields[0].to_owned(),
        input,
    })
}

fn light_error(error: LightError) -> String {
    error.to_string()
}

fn evaluate(record: &Record) -> Result<String, String> {
    match record.input {
        Input::Sample {
            dimension,
            block,
            sky,
            subtraction,
            minimum,
        } => {
            let combined = combined_light(block, sky, subtraction).map_err(light_error)?;
            let value = sample_brightness(block, sky, subtraction, minimum, dimension)
                .map_err(light_error)?;
            Ok(format!(
                "{}\tsample\t{}\t{:08x}",
                record.id,
                combined,
                value.to_bits()
            ))
        }
        Input::Time {
            dimension,
            time,
            partial,
        } => {
            let angle = celestial_angle(time, partial, dimension).map_err(light_error)?;
            let subtraction = sky_subtraction(time, partial, dimension).map_err(light_error)?;
            Ok(format!(
                "{}\ttime\t{:08x}\t{}",
                record.id,
                angle.to_bits(),
                subtraction
            ))
        }
        Input::Emission(block_id) => Ok(format!(
            "{}\temission\t{}",
            record.id,
            emission_level(block_id)
        )),
    }
}

fn process_with_limits<R: Read, W: Write>(
    reader: R,
    mut writer: W,
    max_rows: usize,
    max_bytes: usize,
) -> Result<(), String> {
    let mut input = Vec::new();
    reader
        .take(max_bytes.saturating_add(1) as u64)
        .read_to_end(&mut input)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    if input.len() > max_bytes {
        return Err(format!("input byte limit {max_bytes} exceeded"));
    }
    if input.is_empty() || input.last() != Some(&b'\n') {
        return Err("TSV input must be non-empty and end with LF".into());
    }
    if input.iter().any(|byte| !byte.is_ascii()) {
        return Err("TSV input must be ASCII".into());
    }

    let mut records = Vec::new();
    let mut ids = HashSet::new();
    for (index, row) in input[..input.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        let line = index + 1;
        if row.is_empty() {
            return Err(line_error(line, "blank rows are not allowed"));
        }
        let row = std::str::from_utf8(row).map_err(|_| line_error(line, "invalid UTF-8"))?;
        let record = parse_record(row, line)?;
        if !ids.insert(record.id.clone()) {
            return Err(line_error(line, "duplicate case ID"));
        }
        if records.len() >= max_rows {
            return Err(format!("row limit {max_rows} exceeded"));
        }
        records.push(record);
    }

    let mut output = String::new();
    for record in &records {
        output.push_str(&evaluate(record)?);
        output.push('\n');
    }
    writer
        .write_all(output.as_bytes())
        .map_err(|error| format!("failed to write stdout: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("failed to flush stdout: {error}"))
}

fn process<R: Read, W: Write>(reader: R, writer: W) -> Result<(), String> {
    process_with_limits(reader, writer, MAX_ROWS, MAX_INPUT_BYTES)
}

fn run() -> Result<(), String> {
    if std::env::args_os().len() != 1 {
        return Err("light_probe takes no command-line arguments".into());
    }
    let mut stdin = io::stdin().lock();
    let stdout = io::stdout();
    process(&mut stdin, io::BufWriter::new(stdout.lock()))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("light_probe error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_all_three_frozen_output_shapes() {
        let input = concat!(
            "s\tsample\t0\t1\t15\t0\t0\n",
            "t\ttime\t0\t0\t3f800000\n",
            "e\temission\t50\n"
        );
        let mut output = Vec::new();
        process(input.as_bytes(), &mut output).unwrap();
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            concat!(
                "s\tsample\t15\t3f800000\n",
                "t\ttime\t3f48d8fd\t0\n",
                "e\temission\t14\n"
            )
        );
    }

    #[test]
    fn malformed_late_rows_duplicate_ids_and_limits_are_atomic() {
        let invalid: &[&[u8]] = &[
            b"",
            b"id\tsample\t2\t0\t0\t0\t0\n",
            b"id\tsample\t0\t16\t0\t0\t0\n",
            b"id\ttime\t-1\t+1\t3f000000\n",
            b"id\ttime\t0\t0\t3F000000\n",
            b"id\ttime\t0\t0\t7fc00000\n",
            b"id\temission\t256\n",
            b"id\tunknown\t0\n",
            b"id\temission\t1\n\n",
            b"id\temission\t1\r\n",
            b"id\temission\t1",
        ];
        for input in invalid {
            let mut output = Vec::new();
            assert!(process(*input, &mut output).is_err());
            assert!(output.is_empty());
        }
        let mut late = (0..16)
            .map(|index| format!("valid-{index}\temission\t50\n"))
            .collect::<String>()
            .into_bytes();
        late.extend_from_slice(b"last\ttime\t0\t0\t7fc00000\n");
        let mut output = Vec::new();
        assert!(process(late.as_slice(), &mut output).is_err());
        assert!(output.is_empty());

        let duplicate = b"same\temission\t50\nsame\temission\t51\n";
        let mut output = Vec::new();
        assert!(process(duplicate.as_slice(), &mut output).is_err());
        assert!(output.is_empty());

        let rows = b"a\temission\t1\nb\temission\t2\n";
        let mut output = Vec::new();
        assert!(process_with_limits(rows.as_slice(), &mut output, 1, 1024).is_err());
        assert!(output.is_empty());
        let mut output = Vec::new();
        assert!(process_with_limits(rows.as_slice(), &mut output, 4, 8).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn rejects_non_ascii_and_noncanonical_numbers() {
        for input in [
            "id\témission\t1\n",
            "id\temission\t+1\n",
            "id\temission\t01\n",
        ] {
            let mut output = Vec::new();
            assert!(process(input.as_bytes(), &mut output).is_err());
            assert!(output.is_empty());
        }
    }
}
