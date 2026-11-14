//! Bounded ray-only probe for the BLOCK-001 native input corpus.

use std::io::{self, Read, Write};

use beta_core::{
    block::{BlockScene, BlockSceneBounds, BlockSceneChunk, RayResult, RayShape, UnsupportedBlock},
    chunk::{BlockData, ChunkData, split_block_coord},
    geometry::Vec3,
};

const INPUT_HEADER: &str = "#BLOCK_CASES\t1\tray-only";
const OUTPUT_HEADER: &str =
    "#BLOCK_PROBE\t1\tray-only\n#id\tkind\tx\ty\tz\tside\tpx\tpy\tpz\tpx_bits\tpy_bits\tpz_bits\n";
const MAX_INPUT_BYTES: usize = 4096;
const MAX_CASES: usize = 32;

#[derive(Debug)]
struct Case {
    id: String,
    start: Vec3,
    end: Vec3,
    stop_on_liquid: bool,
    block_x: i32,
    block_y: i32,
    block_z: i32,
    block_id: u8,
    metadata: u8,
}

fn shape_for(id: u8, metadata: u8) -> RayShape {
    match (id, metadata) {
        (1, 0) => RayShape::FullCube,
        (44, 0) => RayShape::LowerSlab,
        (31, 0) => RayShape::TallGrass,
        (8, 0) => RayShape::Liquid,
        _ => RayShape::Unsupported,
    }
}

fn parse_case(line: &str, line_number: usize) -> Result<Case, String> {
    let fields: Vec<_> = line.split('\t').collect();
    if fields.len() != 14 {
        return Err(format!(
            "line {line_number}: expected 14 tab-separated fields"
        ));
    }
    let parse = |index: usize, name: &str| {
        fields[index]
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("line {line_number}: invalid finite {name}"))
    };
    let id = fields[0];
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(format!("line {line_number}: invalid case ID"));
    }
    let stop_on_liquid = match fields[7] {
        "0" => false,
        "1" => true,
        _ => return Err(format!("line {line_number}: invalid stop_on_liquid")),
    };
    let block_x = fields[8]
        .parse::<i32>()
        .map_err(|_| format!("line {line_number}: invalid block x"))?;
    let block_y = fields[9]
        .parse::<i32>()
        .map_err(|_| format!("line {line_number}: invalid block y"))?;
    let block_z = fields[10]
        .parse::<i32>()
        .map_err(|_| format!("line {line_number}: invalid block z"))?;
    let block_id = fields[11]
        .parse::<u8>()
        .map_err(|_| format!("line {line_number}: invalid block ID"))?;
    let metadata = fields[12]
        .parse::<u8>()
        .map_err(|_| format!("line {line_number}: invalid metadata"))?;
    if !(0..128).contains(&block_y) || metadata > 15 {
        return Err(format!(
            "line {line_number}: block coordinate or metadata out of range"
        ));
    }
    let shape = match shape_for(block_id, metadata) {
        RayShape::FullCube => "cube",
        RayShape::LowerSlab => "slab",
        RayShape::Farmland => "farmland",
        RayShape::TallGrass => "tallgrass",
        RayShape::Liquid => "liquid",
        RayShape::Unsupported => {
            return Err(format!(
                "line {line_number}: unsupported native block shape"
            ));
        }
    };
    if fields[13] != shape {
        return Err(format!(
            "line {line_number}: shape label disagrees with verified state"
        ));
    }
    Ok(Case {
        id: id.to_owned(),
        start: Vec3::new(
            parse(1, "start x")?,
            parse(2, "start y")?,
            parse(3, "start z")?,
        ),
        end: Vec3::new(parse(4, "end x")?, parse(5, "end y")?, parse(6, "end z")?),
        stop_on_liquid,
        block_x,
        block_y,
        block_z,
        block_id,
        metadata,
    })
}

fn parse_input(input: &str) -> Result<Vec<Case>, String> {
    if input.len() > MAX_INPUT_BYTES {
        return Err("input byte limit exceeded".into());
    }
    let mut lines = input.lines();
    if lines.next() != Some(INPUT_HEADER) {
        return Err("invalid BLOCK_CASES header".into());
    }
    let mut cases = Vec::new();
    for (line_number, line) in lines.enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if cases.len() == MAX_CASES {
            return Err("case count limit exceeded".into());
        }
        let case = parse_case(line, line_number + 2)?;
        if cases.iter().any(|prior: &Case| prior.id == case.id) {
            return Err(format!("line {}: duplicate case ID", line_number + 2));
        }
        cases.push(case);
    }
    if cases.is_empty() {
        return Err("input has no cases".into());
    }
    Ok(cases)
}

fn decimal(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

fn render_hit(case: &Case, hit: beta_core::block::BlockHit) -> String {
    format!(
        "{}\tblock\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}\t{:016x}\t{:016x}\n",
        case.id,
        hit.x,
        hit.y,
        hit.z,
        hit.face,
        decimal(hit.position.x),
        decimal(hit.position.y),
        decimal(hit.position.z),
        hit.position.x.to_bits(),
        hit.position.y.to_bits(),
        hit.position.z.to_bits(),
    )
}

fn process(input: &str) -> Result<String, String> {
    let cases = parse_input(input)?;
    let mut output = String::from(OUTPUT_HEADER);
    for case in cases {
        let (chunk_x, local_x) = split_block_coord(case.block_x);
        let (chunk_z, local_z) = split_block_coord(case.block_z);
        let mut data = ChunkData::new();
        data.set(
            local_x,
            case.block_y as u8,
            local_z,
            BlockData {
                id: case.block_id,
                metadata: case.metadata,
                ..BlockData::default()
            },
        )
        .map_err(|error| error.to_string())?;
        let scene = BlockScene::new(
            BlockSceneBounds::new(chunk_x, chunk_z, 1, 1).map_err(|error| error.to_string())?,
            vec![BlockSceneChunk {
                x: chunk_x,
                z: chunk_z,
                data,
            }],
        )
        .map_err(|error| error.to_string())?;
        match scene
            .raycast_segment_with_shapes(case.start, case.end, case.stop_on_liquid, &shape_for)
            .map_err(|error| error.to_string())?
        {
            RayResult::Hit(hit) => output.push_str(&render_hit(&case, hit)),
            RayResult::Miss => output.push_str(&format!(
                "{}\tmiss\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-\n",
                case.id
            )),
            RayResult::Unsupported(UnsupportedBlock { .. }) => {
                return Err(format!(
                    "{}: ray reached an unsupported block shape",
                    case.id
                ));
            }
            RayResult::Unloaded { x, y, z } => {
                return Err(format!(
                    "{}: ray entered unloaded cell ({x},{y},{z})",
                    case.id
                ));
            }
        }
    }
    Ok(output)
}

pub fn process_stream<R: Read, W: Write>(mut reader: R, mut writer: W) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(MAX_INPUT_BYTES);
    reader
        .by_ref()
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("input read failed: {error}"))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("input byte limit exceeded".into());
    }
    let input = std::str::from_utf8(&bytes).map_err(|_| "input must be UTF-8")?;
    let output = process(input)?;
    writer
        .write_all(output.as_bytes())
        .map_err(|error| format!("output write failed: {error}"))
}

fn main() -> std::process::ExitCode {
    match process_stream(io::stdin().lock(), io::stdout().lock()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("block_probe: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::process_stream;
    use std::io::Cursor;

    const INPUT: &str = include_str!("../../../tests/fixtures/block/input.tsv");
    const NATIVE: &str = include_str!("../../../tests/fixtures/block/native.tsv");

    #[test]
    fn dynamic_native_fixture_matches_all_rays_and_raw_point_words() {
        let mut output = Vec::new();
        process_stream(INPUT.as_bytes(), &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        let actual: Vec<_> = output.lines().skip(2).collect();
        let expected: Vec<_> = NATIVE
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.split('\t');
                let id = fields.next().unwrap();
                let _source = fields.next().unwrap();
                format!("{id}\t{}", fields.collect::<Vec<_>>().join("\t"))
            })
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn malformed_probe_input_writes_nothing() {
        let mut output = Vec::new();
        assert!(
            process_stream(
                Cursor::new(b"#BLOCK_CASES\t1\tray-only\nbad\n"),
                &mut output
            )
            .is_err()
        );
        assert!(output.is_empty());
    }

    #[test]
    fn duplicate_ids_unsupported_states_and_oversized_input_are_atomic_errors() {
        let row = "case\t-2.0\t0.5\t0.5\t2.0\t0.5\t0.5\t0\t0\t0\t0\t1\t0\tcube\n";
        for input in [
            format!("#BLOCK_CASES\t1\tray-only\n{row}{row}"),
            "#BLOCK_CASES\t1\tray-only\nx\t-2\t0.5\t0.5\t2\t0.5\t0.5\t0\t0\t0\t0\t3\t0\tunknown\n"
                .to_owned(),
        ] {
            let mut output = Vec::new();
            assert!(process_stream(input.as_bytes(), &mut output).is_err());
            assert!(output.is_empty());
        }
        let mut output = Vec::new();
        let oversized = vec![b'x'; super::MAX_INPUT_BYTES + 1];
        assert!(process_stream(oversized.as_slice(), &mut output).is_err());
        assert!(output.is_empty());
    }
}
