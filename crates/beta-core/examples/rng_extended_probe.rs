//! Stdin/stdout TSV probe for extended JavaRandom parity verification.
//!
//! Conforms to the RNG-002 contract:
//! Input TSV: `case_id<TAB>seed_i64<TAB>operations`
//! Output TSV: `case_id<TAB>zero_based_index<TAB>operation_name<TAB>value`

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use beta_core::java_random::{JavaRandom, RandomError};

const MAX_TOTAL_OPERATIONS: usize = 100_000;

fn is_valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn process_stream<R: BufRead, W: Write>(reader: R, mut writer: W) -> Result<(), String> {
    let mut seen_ids = HashSet::new();
    let mut total_operations: usize = 0;

    for (line_num, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| format!("line {}: IO error: {e}", line_num + 1))?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 3 {
            return Err(format!(
                "line {}: expected 3 tab-separated fields (case_id, seed, operations), got {}",
                line_num + 1,
                parts.len()
            ));
        }

        let case_id = parts[0];
        if !is_valid_case_id(case_id) {
            return Err(format!(
                "line {}: invalid case_id '{case_id}', must be non-empty ASCII alphanumeric/underscore/hyphen",
                line_num + 1
            ));
        }

        if !seen_ids.insert(case_id.to_string()) {
            return Err(format!(
                "line {}: duplicate case_id '{case_id}'",
                line_num + 1
            ));
        }

        let initial_seed: i64 = parts[1].parse().map_err(|e| {
            format!(
                "line {}: failed to parse initial seed '{}' as signed 64-bit integer: {e}",
                line_num + 1,
                parts[1]
            )
        })?;

        let ops_raw = parts[2];
        if ops_raw.is_empty() {
            return Err(format!(
                "line {}: operations field cannot be empty",
                line_num + 1
            ));
        }

        // Check for whitespace in operations field
        if ops_raw.contains(char::is_whitespace) {
            return Err(format!(
                "line {}: operations field cannot contain whitespace",
                line_num + 1
            ));
        }

        let ops: Vec<&str> = ops_raw.split(';').collect();
        let ops_count = ops.len();
        if ops_count == 0 || ops_count > MAX_TOTAL_OPERATIONS {
            return Err(format!(
                "line {}: operation count {ops_count} out of valid range (1..={MAX_TOTAL_OPERATIONS})",
                line_num + 1
            ));
        }

        total_operations = total_operations.checked_add(ops_count).ok_or_else(|| {
            format!(
                "line {}: total operations exceeded integer limit",
                line_num + 1
            )
        })?;
        if total_operations > MAX_TOTAL_OPERATIONS {
            return Err(format!(
                "line {}: total operations {total_operations} exceeds maximum allowed limit {MAX_TOTAL_OPERATIONS}",
                line_num + 1
            ));
        }

        let mut rng = JavaRandom::new(initial_seed);

        for (op_idx, &op) in ops.iter().enumerate() {
            if op.is_empty() {
                return Err(format!(
                    "line {}: empty operation at index {op_idx}",
                    line_num + 1
                ));
            }

            if op == "int" {
                let val = rng.next_int();
                writeln!(writer, "{case_id}\t{op_idx}\tint\t{val}")
                    .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
            } else if let Some(bound_str) = op.strip_prefix("bound:") {
                let bound: i32 = bound_str.parse().map_err(|e| {
                    format!(
                        "line {}: failed to parse bound argument '{bound_str}' as signed 32-bit integer: {e}",
                        line_num + 1
                    )
                })?;

                match rng.next_int_bound(bound) {
                    Ok(val) => {
                        writeln!(writer, "{case_id}\t{op_idx}\tbound\t{val}")
                            .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
                    }
                    Err(RandomError::InvalidBound) => {
                        writeln!(writer, "{case_id}\t{op_idx}\tbound\tinvalid_bound")
                            .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
                    }
                }
            } else if op == "long" {
                let val = rng.next_long();
                writeln!(writer, "{case_id}\t{op_idx}\tlong\t{val}")
                    .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
            } else if op == "boolean" {
                let val = if rng.next_boolean() { '1' } else { '0' };
                writeln!(writer, "{case_id}\t{op_idx}\tboolean\t{val}")
                    .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
            } else if op == "float" {
                let val = rng.next_float();
                let bits = val.to_bits();
                writeln!(writer, "{case_id}\t{op_idx}\tfloat\t{bits:08x}")
                    .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
            } else if op == "double" {
                let val = rng.next_double();
                let bits = val.to_bits();
                writeln!(writer, "{case_id}\t{op_idx}\tdouble\t{bits:016x}")
                    .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
            } else if let Some(seed_str) = op.strip_prefix("seed:") {
                let new_seed: i64 = seed_str.parse().map_err(|e| {
                    format!(
                        "line {}: failed to parse seed argument '{seed_str}' as signed 64-bit integer: {e}",
                        line_num + 1
                    )
                })?;
                rng.set_seed(new_seed);
                writeln!(writer, "{case_id}\t{op_idx}\tseed\tok")
                    .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
            } else {
                return Err(format!(
                    "line {}: unknown operation token '{op}' at index {op_idx}",
                    line_num + 1
                ));
            }
        }
    }

    writer.flush().map_err(|e| format!("flush error: {e}"))?;
    Ok(())
}

fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let out = io::BufWriter::new(stdout.lock());
    process_stream(stdin.lock(), out)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("rng_extended_probe error: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_operations_stream() {
        let input = b"case1\t0\tint;bound:16;bound:7;long;boolean;float;double;seed:42;bound:0;bound:-1;int\n";
        let mut out = Vec::new();
        assert!(process_stream(&input[..], &mut out).is_ok());
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 11);
        assert_eq!(lines[0], "case1\t0\tint\t-1155484576");
        assert_eq!(lines[1], "case1\t1\tbound\t13");
        assert_eq!(lines[2], "case1\t2\tbound\t4");
        assert_eq!(lines[3], "case1\t3\tlong\t-7261648964369397258");
        assert_eq!(lines[4], "case1\t4\tboolean\t0");
        assert_eq!(lines[5], "case1\t5\tfloat\t3f0ce970");
        assert_eq!(lines[6], "case1\t6\tdouble\t3fbdf425263e2e88");
        assert_eq!(lines[7], "case1\t7\tseed\tok");
        assert_eq!(lines[8], "case1\t8\tbound\tinvalid_bound");
        assert_eq!(lines[9], "case1\t9\tbound\tinvalid_bound");
        assert_eq!(lines[10], "case1\t10\tint\t-1170105035");
    }

    #[test]
    fn test_empty_line_and_comment_skipped() {
        let input = b"\n# comment\n\n";
        let mut out = Vec::new();
        assert!(process_stream(&input[..], &mut out).is_ok());
        assert!(out.is_empty());
    }

    #[test]
    fn test_whitespace_line_rejected() {
        let mut out = Vec::new();
        assert!(process_stream(&b" \n"[..], &mut out).is_err());
        assert!(process_stream(&b"\t\n"[..], &mut out).is_err());
    }

    #[test]
    fn test_invalid_ops_syntax() {
        let mut out = Vec::new();
        // trailing semicolon
        assert!(process_stream(&b"c1\t0\tint;\n"[..], &mut out).is_err());
        // double semicolon
        assert!(process_stream(&b"c1\t0\tint;;int\n"[..], &mut out).is_err());
        // whitespace in ops
        assert!(process_stream(&b"c1\t0\tint; bound:5\n"[..], &mut out).is_err());
        // unknown op
        assert!(process_stream(&b"c1\t0\tunknown\n"[..], &mut out).is_err());
        // bad bound
        assert!(process_stream(&b"c1\t0\tbound:abc\n"[..], &mut out).is_err());
        // bad seed
        assert!(process_stream(&b"c1\t0\tseed:xyz\n"[..], &mut out).is_err());
    }

    #[test]
    fn test_duplicate_case_id_rejected() {
        let input = b"c1\t0\tint\nc1\t0\tint\n";
        let mut out = Vec::new();
        assert!(process_stream(&input[..], &mut out).is_err());
    }
}
