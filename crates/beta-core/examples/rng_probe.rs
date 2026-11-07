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

fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let mut seen_ids = HashSet::new();

    for (line_num, line_res) in stdin.lock().lines().enumerate() {
        let line = line_res.map_err(|e| format!("line {}: IO error: {e}", line_num + 1))?;
        if line.starts_with('#') || line.trim().is_empty() {
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
            writeln!(out, "{case_id}\t{idx}\t{val}")
                .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
        }
    }

    out.flush().map_err(|e| format!("flush error: {e}"))?;
    Ok(())
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
