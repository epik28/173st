//! Stdin/stdout TSV probe for JavaRandom next_int parity verification.
//!
//! Conforms to the RNG-001 contract:
//! Input TSV: `case_id<TAB>seed_i64_decimal<TAB>count`
//! Output TSV: `case_id<TAB>zero_based_index<TAB>signed_i32_decimal`

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use beta_core::java_random::JavaRandom;

const MAX_COUNT: usize = 100_000;

fn is_valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn process_stream<R: BufRead, W: Write>(reader: R, mut writer: W) -> Result<(), String> {
    let mut seen_ids = HashSet::new();

    for (line_num, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| format!("line {}: IO error: {e}", line_num + 1))?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 3 {
            return Err(format!(
                "line {}: expected 3 tab-separated fields (case_id, seed, count), got {}",
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

        let seed: i64 = parts[1].parse().map_err(|e| {
            format!(
                "line {}: failed to parse seed '{}' as signed 64-bit integer: {e}",
                line_num + 1,
                parts[1]
            )
        })?;

        let count: usize = parts[2].parse().map_err(|e| {
            format!(
                "line {}: failed to parse count '{}' as integer: {e}",
                line_num + 1,
                parts[2]
            )
        })?;

        if count == 0 || count > MAX_COUNT {
            return Err(format!(
                "line {}: count {count} out of valid range (1..={MAX_COUNT})",
                line_num + 1
            ));
        }

        let mut rng = JavaRandom::new(seed);
        for idx in 0..count {
            let val = rng.next_int();
            writeln!(writer, "{case_id}\t{idx}\t{val}")
                .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
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
            eprintln!("rng_probe error: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_line_and_comment_skipped() {
        let input = b"\n# comment\n\n";
        let mut out = Vec::new();
        assert!(process_stream(&input[..], &mut out).is_ok());
        assert!(out.is_empty());
    }

    #[test]
    fn test_space_only_line_rejected() {
        let input = b" \n";
        let mut out = Vec::new();
        let err = process_stream(&input[..], &mut out).unwrap_err();
        assert!(err.contains("expected 3 tab-separated fields"));
    }

    #[test]
    fn test_tab_only_line_rejected() {
        let input = b"\t\n";
        let mut out = Vec::new();
        let err = process_stream(&input[..], &mut out).unwrap_err();
        assert!(err.contains("expected 3 tab-separated fields"));
    }

    #[test]
    fn test_valid_input_matches() {
        let input = b"test-case\t42\t2\n";
        let mut out = Vec::new();
        assert!(process_stream(&input[..], &mut out).is_ok());
        let s = String::from_utf8(out).unwrap();
        assert_eq!(s, "test-case\t0\t-1170105035\ntest-case\t1\t234785527\n");
    }
}
