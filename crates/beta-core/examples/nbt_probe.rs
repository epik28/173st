//! TSV probe for the beta_core NBT parser.
//!
//! Input: `case_id<TAB>hex_bytes` (one complete uncompressed named root per line).
//! Output: `case_id<TAB>consumed_bytes<TAB>canonical_tree`.
//! String values/names are UTF-8 hex, float values are their raw IEEE-754 bits,
//! and compound entries are sorted by name for a stable canonical representation.

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use beta_core::nbt::{Limits, NamedTag, NbtError, Tag, TagType, read_named};

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

fn hex_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        use std::fmt::Write as _;
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn tag_type_name(tag_type: TagType) -> &'static str {
    match tag_type {
        TagType::End => "End",
        TagType::Byte => "Byte",
        TagType::Short => "Short",
        TagType::Int => "Int",
        TagType::Long => "Long",
        TagType::Float => "Float",
        TagType::Double => "Double",
        TagType::ByteArray => "ByteArray",
        TagType::String => "String",
        TagType::List => "List",
        TagType::Compound => "Compound",
        TagType::IntArray => "IntArray",
    }
}

fn canonical_tag(tag: &Tag) -> Result<String, NbtError> {
    Ok(match tag {
        Tag::End => "End".into(),
        Tag::Byte(value) => format!("Byte({value})"),
        Tag::Short(value) => format!("Short({value})"),
        Tag::Int(value) => format!("Int({value})"),
        Tag::Long(value) => format!("Long({value})"),
        Tag::Float(value) => format!("Float({:08x})", value.to_bits()),
        Tag::Double(value) => format!("Double({:016x})", value.to_bits()),
        Tag::ByteArray(values) => {
            let mut out = String::from("ByteArray(");
            for value in values {
                use std::fmt::Write as _;
                write!(out, "{:02x}", *value as u8).expect("writing to String cannot fail");
            }
            out.push(')');
            out
        }
        Tag::String(value) => format!("String({})", hex_string(value)),
        Tag::List {
            element_type,
            values,
        } => {
            let mut out = format!("List<{}>[", tag_type_name(*element_type));
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    out.push(',');
                }
                out.push_str(&canonical_tag(value)?);
            }
            out.push(']');
            out
        }
        Tag::Compound(fields) => {
            let mut fields: Vec<_> = fields.iter().collect();
            fields.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
            let mut out = String::from("Compound{");
            for (index, (name, value)) in fields.into_iter().enumerate() {
                if index != 0 {
                    out.push(',');
                }
                out.push_str(&hex_string(name));
                out.push(':');
                out.push_str(&canonical_tag(value)?);
            }
            out.push('}');
            out
        }
        Tag::IntArray(values) => {
            let mut out = String::from("IntArray[");
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    out.push(',');
                }
                use std::fmt::Write as _;
                write!(out, "{value}").expect("writing to String cannot fail");
            }
            out.push(']');
            out
        }
    })
}

fn canonical_named(value: &NamedTag) -> Result<String, NbtError> {
    Ok(format!(
        "{}:{}",
        hex_string(&value.name),
        canonical_tag(&value.value)?
    ))
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
        let bytes = decode_hex(hex).map_err(|error| format!("line {line_no}: {error}"))?;
        input_bytes = input_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| format!("line {line_no}: input byte count overflow"))?;
        if input_bytes > MAX_INPUT_BYTES {
            return Err(format!(
                "line {line_no}: total input limit {MAX_INPUT_BYTES} exceeded"
            ));
        }
        let (tree, consumed) = read_named(&bytes, Limits::default())
            .map_err(|error| format!("line {line_no}, case {case_id:?}: {error}"))?;
        let canonical = canonical_named(&tree)
            .map_err(|error| format!("line {line_no}, case {case_id:?}: {error}"))?;
        writeln!(writer, "{case_id}\t{consumed}\t{canonical}")
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
    fn decodes_and_canonicalizes_trees() {
        // Named root Compound("root") containing Int("z")=3 and Byte("a")=-1.
        let input = b"tree\t0a0004726f6f740300017a0000000301000161ff00\n";
        let (result, output) = process(input);
        assert!(result.is_ok());
        assert_eq!(
            output,
            "tree\t21\t726f6f74:Compound{61:Byte(-1),7a:Int(3)}\n"
        );
    }

    #[test]
    fn preserves_float_bits_and_consumed_length() {
        // Named root Float("f") with a noncanonical NaN payload.
        let (result, output) = process(b"float\t050001667fc01234\n");
        assert!(result.is_ok());
        assert_eq!(output, "float\t8\t66:Float(7fc01234)\n");
    }

    #[test]
    fn accepts_empty_lines_comments_and_rejects_bad_protocol() {
        let (result, output) = process(b"\n# sample\nempty\t0a000000\n");
        assert!(result.is_ok());
        assert_eq!(output, "empty\t4\t:Compound{}\n");
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
