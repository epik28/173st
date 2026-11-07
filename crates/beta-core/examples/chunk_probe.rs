//! Stdin/stdout TSV probe for chunk coordinate and data storage parity verification.
//!
//! Conforms to the CHUNK-001 contract:
//! Input TSV: `case_id<TAB>operations`
//! Output TSV: `case_id<TAB>zero_based_index<TAB>operation_name<TAB>value`

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use beta_core::chunk::{BlockData, ChunkData, ChunkError, block_index, split_block_coord};

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
        if parts.len() != 2 {
            return Err(format!(
                "line {}: expected 2 tab-separated fields (case_id, operations), got {}",
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

        let ops_raw = parts[1];
        if ops_raw.is_empty() {
            return Err(format!(
                "line {}: operations field cannot be empty",
                line_num + 1
            ));
        }

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

        let mut chunk = ChunkData::new();

        for (op_idx, &op) in ops.iter().enumerate() {
            if op.is_empty() {
                return Err(format!(
                    "line {}: empty operation at index {op_idx}",
                    line_num + 1
                ));
            }

            let op_parts: Vec<&str> = op.split(':').collect();
            let op_name = op_parts[0];

            match op_name {
                "coord" => {
                    if op_parts.len() != 2 {
                        return Err(format!(
                            "line {}: expected coord:VALUE at index {op_idx}, got '{op}'",
                            line_num + 1
                        ));
                    }
                    let val: i32 = op_parts[1].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse coord value '{}' as i32 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[1]
                        )
                    })?;
                    let (c, l) = split_block_coord(val);
                    writeln!(writer, "{case_id}\t{op_idx}\tcoord\t{c},{l}")
                        .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
                }
                "byte" => {
                    if op_parts.len() != 3 {
                        return Err(format!(
                            "line {}: expected byte:ARRAY:INDEX at index {op_idx}, got '{op}'",
                            line_num + 1
                        ));
                    }
                    let (blocks, data, block_light, sky_light) = chunk.raw_arrays();
                    let array: &[u8] = match op_parts[1] {
                        "blocks" => blocks,
                        "data" => data,
                        "block_light" => block_light,
                        "sky_light" => sky_light,
                        other => {
                            return Err(format!(
                                "line {}: unknown byte array '{other}' at index {op_idx}",
                                line_num + 1
                            ));
                        }
                    };
                    let pos: usize = op_parts[2].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse byte index '{}' as usize at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[2]
                        )
                    })?;
                    if pos >= array.len() {
                        return Err(format!(
                            "line {}: byte index {pos} out of range (0..{}) at index {op_idx}",
                            array.len(),
                            line_num + 1
                        ));
                    }
                    let b = array[pos];
                    writeln!(writer, "{case_id}\t{op_idx}\tbyte\t{b}")
                        .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
                }
                "index" => {
                    if op_parts.len() != 4 {
                        return Err(format!(
                            "line {}: expected index:X:Y:Z at index {op_idx}, got '{op}'",
                            line_num + 1
                        ));
                    }
                    let x: u8 = op_parts[1].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse X coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[1]
                        )
                    })?;
                    let y: u8 = op_parts[2].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse Y coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[2]
                        )
                    })?;
                    let z: u8 = op_parts[3].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse Z coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[3]
                        )
                    })?;
                    let res = match block_index(x, y, z) {
                        Ok(idx) => idx.to_string(),
                        Err(ChunkError::InvalidCoordinate) => "invalid_coordinate".to_string(),
                        Err(e) => {
                            return Err(format!("line {}: unexpected error: {e}", line_num + 1));
                        }
                    };
                    writeln!(writer, "{case_id}\t{op_idx}\tindex\t{res}")
                        .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
                }
                "get" => {
                    if op_parts.len() != 4 {
                        return Err(format!(
                            "line {}: expected get:X:Y:Z at index {op_idx}, got '{op}'",
                            line_num + 1
                        ));
                    }
                    let x: u8 = op_parts[1].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse X coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[1]
                        )
                    })?;
                    let y: u8 = op_parts[2].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse Y coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[2]
                        )
                    })?;
                    let z: u8 = op_parts[3].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse Z coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[3]
                        )
                    })?;
                    let res = match chunk.get(x, y, z) {
                        Ok(b) => {
                            format!("{},{},{},{}", b.id, b.metadata, b.block_light, b.sky_light)
                        }
                        Err(ChunkError::InvalidCoordinate) => "invalid_coordinate".to_string(),
                        Err(e) => {
                            return Err(format!("line {}: unexpected error: {e}", line_num + 1));
                        }
                    };
                    writeln!(writer, "{case_id}\t{op_idx}\tget\t{res}")
                        .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
                }
                "set" => {
                    if op_parts.len() != 8 {
                        return Err(format!(
                            "line {}: expected set:X:Y:Z:ID:M:B:S at index {op_idx}, got '{op}'",
                            line_num + 1
                        ));
                    }
                    let x: u8 = op_parts[1].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse X coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[1]
                        )
                    })?;
                    let y: u8 = op_parts[2].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse Y coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[2]
                        )
                    })?;
                    let z: u8 = op_parts[3].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse Z coord '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[3]
                        )
                    })?;
                    let id: u8 = op_parts[4].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse block ID '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[4]
                        )
                    })?;
                    let metadata: u8 = op_parts[5].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse metadata '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[5]
                        )
                    })?;
                    let block_light: u8 = op_parts[6].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse block_light '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[6]
                        )
                    })?;
                    let sky_light: u8 = op_parts[7].parse().map_err(|e| {
                        format!(
                            "line {}: failed to parse sky_light '{}' as u8 at index {op_idx}: {e}",
                            line_num + 1,
                            op_parts[7]
                        )
                    })?;

                    let res = match chunk.set(
                        x,
                        y,
                        z,
                        BlockData {
                            id,
                            metadata,
                            block_light,
                            sky_light,
                        },
                    ) {
                        Ok(()) => "ok",
                        Err(ChunkError::InvalidCoordinate) => "invalid_coordinate",
                        Err(ChunkError::InvalidNibble) => "invalid_nibble",
                        Err(e) => {
                            return Err(format!("line {}: unexpected error: {e}", line_num + 1));
                        }
                    };
                    writeln!(writer, "{case_id}\t{op_idx}\tset\t{res}")
                        .map_err(|e| format!("line {}: write error: {e}", line_num + 1))?;
                }
                other => {
                    return Err(format!(
                        "line {}: unknown operation '{other}' at index {op_idx}",
                        line_num + 1
                    ));
                }
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
            eprintln!("chunk_probe error: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_chunk_operations_stream() {
        let input = b"case1\tcoord:0;coord:-1;coord:-17;index:0:0:0;index:16:0:0;get:0:0:0;set:0:0:0:1:3:5:7;get:0:0:0;byte:blocks:0;byte:data:0;set:16:0:0:1:2:3:4;set:0:0:0:1:16:0:0\n";
        let mut out = Vec::new();
        assert!(process_stream(&input[..], &mut out).is_ok());
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 12);
        assert_eq!(lines[0], "case1\t0\tcoord\t0,0");
        assert_eq!(lines[1], "case1\t1\tcoord\t-1,15");
        assert_eq!(lines[2], "case1\t2\tcoord\t-2,15");
        assert_eq!(lines[3], "case1\t3\tindex\t0");
        assert_eq!(lines[4], "case1\t4\tindex\tinvalid_coordinate");
        assert_eq!(lines[5], "case1\t5\tget\t0,0,0,0");
        assert_eq!(lines[6], "case1\t6\tset\tok");
        assert_eq!(lines[7], "case1\t7\tget\t1,3,5,7");
        assert_eq!(lines[8], "case1\t8\tbyte\t1");
        assert_eq!(lines[9], "case1\t9\tbyte\t3");
        assert_eq!(lines[10], "case1\t10\tset\tinvalid_coordinate");
        assert_eq!(lines[11], "case1\t11\tset\tinvalid_nibble");
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
        assert!(process_stream(&b"c1\tcoord:0;\n"[..], &mut out).is_err());
        // double semicolon
        assert!(process_stream(&b"c1\tcoord:0;;coord:1\n"[..], &mut out).is_err());
        // whitespace in ops
        assert!(process_stream(&b"c1\tcoord:0; coord:1\n"[..], &mut out).is_err());
        // unknown op
        assert!(process_stream(&b"c1\tunknown\n"[..], &mut out).is_err());
        // bad byte index
        assert!(process_stream(&b"c1\tbyte:blocks:32768\n"[..], &mut out).is_err());
        // bad coord number
        assert!(process_stream(&b"c1\tcoord:not_num\n"[..], &mut out).is_err());
    }

    #[test]
    fn test_duplicate_case_id_rejected() {
        let input = b"c1\tcoord:0\nc1\tcoord:0\n";
        let mut out = Vec::new();
        assert!(process_stream(&input[..], &mut out).is_err());
    }
}
