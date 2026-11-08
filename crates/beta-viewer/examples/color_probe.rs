//! Atomic bounded TSV probe for native-compatible climate and color lookups.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

use beta_core::biome::{Climate, ClimateSampler, foliage_color, grass_color};
use beta_viewer::colors::{ColorMaps, load_color_maps};

const MAX_ROWS: usize = 4096;
const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_ID_BYTES: usize = 96;
const MAX_COORD: i32 = 30_000_000;

#[derive(Clone, Copy)]
enum Input {
    Climate { seed: i64, x: i32, z: i32 },
    Palette(Climate),
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

fn canonical_i64(text: &str, line: usize, field: &str) -> Result<i64, String> {
    let value = text
        .parse::<i64>()
        .map_err(|_| line_error(line, &format!("invalid {field} i64")))?;
    if value.to_string() != text {
        return Err(line_error(
            line,
            &format!("{field} must use canonical signed decimal"),
        ));
    }
    Ok(value)
}

fn canonical_i32(text: &str, line: usize, field: &str) -> Result<i32, String> {
    let value = text
        .parse::<i32>()
        .map_err(|_| line_error(line, &format!("invalid {field} i32")))?;
    if value.to_string() != text {
        return Err(line_error(
            line,
            &format!("{field} must use canonical signed decimal"),
        ));
    }
    Ok(value)
}

fn raw_f64(text: &str, line: usize, field: &str) -> Result<f64, String> {
    if text.len() != 16
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(line_error(
            line,
            &format!("{field} must be 16 lowercase hex digits"),
        ));
    }
    let bits = u64::from_str_radix(text, 16)
        .map_err(|_| line_error(line, &format!("invalid {field} raw bits")))?;
    let value = f64::from_bits(bits);
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(line_error(
            line,
            &format!("{field} must be finite and in 0..=1"),
        ));
    }
    Ok(value)
}

fn parse_record(row: &str, line: usize) -> Result<Record, String> {
    let fields: Vec<_> = row.split('\t').collect();
    if fields.is_empty() || !valid_id(fields[0]) {
        return Err(line_error(line, "invalid case ID"));
    }
    let input = match fields.get(1).copied() {
        Some("climate") if fields.len() == 5 => {
            let seed = canonical_i64(fields[2], line, "seed")?;
            let x = canonical_i32(fields[3], line, "x")?;
            let z = canonical_i32(fields[4], line, "z")?;
            if !(-MAX_COORD..=MAX_COORD).contains(&x) || !(-MAX_COORD..=MAX_COORD).contains(&z) {
                return Err(line_error(
                    line,
                    "coordinates must have absolute value <= 30000000",
                ));
            }
            Input::Climate { seed, x, z }
        }
        Some("palette") if fields.len() == 4 => Input::Palette(Climate {
            temperature: raw_f64(fields[2], line, "temperature")?,
            rainfall: raw_f64(fields[3], line, "rainfall")?,
        }),
        Some("climate") | Some("palette") => {
            return Err(line_error(line, "wrong field count for operation"));
        }
        Some(_) => return Err(line_error(line, "operation must be climate or palette")),
        None => return Err(line_error(line, "missing operation")),
    };
    Ok(Record {
        id: fields[0].to_owned(),
        input,
    })
}

fn bits64(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn color_bits(value: u32) -> String {
    format!("{value:08x}")
}

fn evaluate(record: &Record, maps: &ColorMaps) -> Result<String, String> {
    match record.input {
        Input::Climate { seed, x, z } => {
            let climate = ClimateSampler::new(seed).sample(x, z);
            let grass = grass_color(&maps.grass, climate).map_err(|e| e.to_string())?;
            let foliage0 = foliage_color(&maps.foliage, climate, 0).map_err(|e| e.to_string())?;
            let foliage1 = foliage_color(&maps.foliage, climate, 1).map_err(|e| e.to_string())?;
            let foliage2 = foliage_color(&maps.foliage, climate, 2).map_err(|e| e.to_string())?;
            let foliage3 = foliage_color(&maps.foliage, climate, 3).map_err(|e| e.to_string())?;
            Ok(format!(
                "{}\tclimate\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                record.id,
                bits64(climate.temperature),
                bits64(climate.rainfall),
                color_bits(grass),
                color_bits(foliage0),
                color_bits(foliage1),
                color_bits(foliage2),
                color_bits(foliage3)
            ))
        }
        Input::Palette(climate) => {
            let grass = grass_color(&maps.grass, climate).map_err(|e| e.to_string())?;
            let foliage = foliage_color(&maps.foliage, climate, 0).map_err(|e| e.to_string())?;
            Ok(format!(
                "{}\tpalette\t{}\t{}",
                record.id,
                color_bits(grass),
                color_bits(foliage)
            ))
        }
    }
}

fn process_with_limits<R: Read, W: Write>(
    reader: R,
    mut writer: W,
    maps: &ColorMaps,
    max_rows: usize,
    max_bytes: usize,
) -> Result<(), String> {
    let mut input = Vec::new();
    reader
        .take(max_bytes.saturating_add(1) as u64)
        .read_to_end(&mut input)
        .map_err(|e| format!("failed to read stdin: {e}"))?;
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
        output.push_str(&evaluate(record, maps)?);
        output.push('\n');
    }
    writer
        .write_all(output.as_bytes())
        .map_err(|e| format!("failed to write stdout: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("failed to flush stdout: {e}"))
}

fn process_stream<R: Read, W: Write>(reader: R, writer: W, maps: &ColorMaps) -> Result<(), String> {
    process_with_limits(reader, writer, maps, MAX_ROWS, MAX_INPUT_BYTES)
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("usage: color_probe GRASS_PNG FOLIAGE_PNG".into());
    }
    let maps = load_color_maps(Path::new(&args[1]), Path::new(&args[2]))
        .map_err(|e| format!("failed to load pinned color maps: {e}"))?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    process_stream(stdin.lock(), io::BufWriter::new(stdout.lock()), &maps)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("color_probe error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maps() -> ColorMaps {
        let grass = (0..65_536).map(|v| v as u32).collect::<Vec<_>>();
        let foliage = (0..65_536)
            .map(|v| 0x8000_0000 | v as u32)
            .collect::<Vec<_>>();
        ColorMaps {
            grass: grass.into_boxed_slice().try_into().unwrap(),
            foliage: foliage.into_boxed_slice().try_into().unwrap(),
        }
    }

    #[test]
    fn emits_both_shapes_and_raw_color_values() {
        let input = concat!(
            "c\tclimate\t0\t-1\t2\n",
            "p\tpalette\t3fe0000000000000\t3fe0000000000000\n"
        );
        let mut output = Vec::new();
        process_stream(input.as_bytes(), &mut output, &maps()).unwrap();
        let rows: Vec<_> = std::str::from_utf8(&output).unwrap().lines().collect();
        assert_eq!(rows.len(), 2);
        let climate: Vec<_> = rows[0].split('\t').collect();
        assert_eq!(climate.len(), 9);
        assert_eq!(&climate[..2], &["c", "climate"]);
        assert_eq!(climate[6], "00619961");
        assert_eq!(climate[7], "0080a755");
        assert_eq!(climate[8], "00619961");
        let palette: Vec<_> = rows[1].split('\t').collect();
        assert_eq!(palette.len(), 4);
        assert_eq!(&palette[..2], &["p", "palette"]);
    }

    #[test]
    fn malformed_late_rows_and_limits_never_emit_partial_stdout() {
        let invalid: &[&[u8]] = &[
            b"",
            b"id\tclimate\t+1\t0\t0\n",
            b"id\tclimate\t1\t-0\t0\n",
            b"id\tclimate\t1\t30000001\t0\n",
            b"id\tpalette\t7ff8000000000000\t0000000000000000\n",
            b"id\tpalette\tbfe0000000000000\t0000000000000000\n",
            b"id\tunknown\t0\n",
            b"id\tpalette\t3FE0000000000000\t3fe0000000000000\n",
            b"id\tpalette\t3fe0000000000000\t3fe0000000000000",
            b"id\tpalette\t3ff0000000000000\t0000000000000000\n\n",
            b"id\tpalette\t3ff0000000000000\t0000000000000000\r\n",
        ];
        for bytes in invalid {
            let mut output = Vec::new();
            assert!(process_stream(*bytes, &mut output, &maps()).is_err());
            assert!(output.is_empty());
        }
        let mut output = Vec::new();
        assert!(process_stream("id\té\t0\n".as_bytes(), &mut output, &maps()).is_err());
        assert!(output.is_empty());
        let mut late = b"first\tpalette\t3fe0000000000000\t3fe0000000000000\n".repeat(32);
        late.extend_from_slice(b"last\tpalette\t7ff0000000000000\t0000000000000000\n");
        let mut output = Vec::new();
        assert!(process_stream(late.as_slice(), &mut output, &maps()).is_err());
        assert!(output.is_empty());

        let duplicate = b"same\tpalette\t3fe0000000000000\t3fe0000000000000\nsame\tpalette\t3fe0000000000000\t3fe0000000000000\n";
        let mut output = Vec::new();
        assert!(process_stream(duplicate.as_slice(), &mut output, &maps()).is_err());
        assert!(output.is_empty());

        let two_rows = b"a\tpalette\t3fe0000000000000\t3fe0000000000000\nb\tpalette\t3fe0000000000000\t3fe0000000000000\n";
        let mut output = Vec::new();
        assert!(process_with_limits(two_rows.as_slice(), &mut output, &maps(), 1, 4096).is_err());
        assert!(output.is_empty());
        let mut output = Vec::new();
        assert!(process_with_limits(two_rows.as_slice(), &mut output, &maps(), 4, 16).is_err());
        assert!(output.is_empty());
    }
}
