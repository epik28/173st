//! Strict atomic TSV adapter for the frozen MOVE-001 movement contract.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use beta_core::geometry::Aabb;
use beta_core::motion::{MotionState, Surface, TravelInput, move_entity, travel_with_ladder_query};

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_SCENARIOS: usize = 4096;
const MAX_STEPS: usize = 100_000;
const MAX_BOXES: usize = 128;
const MAX_FRAMES: usize = 256;
const MAX_COORD: f64 = 30_000_000.0;

#[derive(Clone, Copy)]
enum Frame {
    Move([f64; 3]),
    Travel(TravelInput),
}

struct Scenario {
    id: String,
    state: MotionState,
    colliders: Vec<Aabb>,
    surface: Surface,
    frames: Vec<Frame>,
}

fn parse_error(line: usize, message: &str) -> String {
    format!("line {line}: {message}")
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

fn f64_bits(value: &str, line: usize, name: &str) -> Result<f64, String> {
    if value.len() != 16
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(parse_error(
            line,
            &format!("{name} must be 16 lowercase hex digits"),
        ));
    }
    let value = f64::from_bits(
        u64::from_str_radix(value, 16).map_err(|_| parse_error(line, "invalid f64 bits"))?,
    );
    if !value.is_finite() || value.abs() > MAX_COORD {
        return Err(parse_error(
            line,
            &format!("{name} is non-finite or exceeds {MAX_COORD}"),
        ));
    }
    Ok(value)
}

fn f32_bits(value: &str, line: usize, name: &str) -> Result<f32, String> {
    if value.len() != 8
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(parse_error(
            line,
            &format!("{name} must be 8 lowercase hex digits"),
        ));
    }
    let value = f32::from_bits(
        u32::from_str_radix(value, 16).map_err(|_| parse_error(line, "invalid f32 bits"))?,
    );
    if !value.is_finite() {
        return Err(parse_error(line, &format!("{name} must be finite")));
    }
    Ok(value)
}

fn exact_parts<'a>(
    text: &'a str,
    separator: char,
    count: usize,
    line: usize,
    name: &str,
) -> Result<Vec<&'a str>, String> {
    let values: Vec<_> = text.split(separator).collect();
    if values.len() != count {
        return Err(parse_error(
            line,
            &format!("{name} expects {count} components"),
        ));
    }
    Ok(values)
}

fn bounded_f32(value: f32, low: f32, high: f32, line: usize, name: &str) -> Result<f32, String> {
    if !(low..=high).contains(&value) {
        return Err(parse_error(
            line,
            &format!("{name} is outside [{low}, {high}]"),
        ));
    }
    Ok(value)
}

fn parse_state(text: &str, line: usize) -> Result<MotionState, String> {
    let fields = exact_parts(text, ',', 18, line, "initial state")?;
    let mut f64s = [0.0; 12];
    for (index, field) in fields[..12].iter().enumerate() {
        f64s[index] = f64_bits(field, line, "state coordinate")?;
    }
    let mut f32s = [0.0; 5];
    for (index, field) in fields[12..17].iter().enumerate() {
        f32s[index] = f32_bits(field, line, "state scalar")?;
    }
    if fields[17].is_empty() || !fields[17].bytes().all(|b| b.is_ascii_digit()) {
        return Err(parse_error(line, "flags must be unsigned decimal digits"));
    }
    let flags = fields[17]
        .parse::<u8>()
        .map_err(|_| parse_error(line, "flags must be decimal 0..255"))?;
    let bbox_min = [f64s[6], f64s[7], f64s[8]];
    let bbox_max = [f64s[9], f64s[10], f64s[11]];
    for axis in 0..3 {
        let extent = bbox_max[axis] - bbox_min[axis];
        if !(extent > 0.0 && extent <= 4.0) {
            return Err(parse_error(
                line,
                "body box extents must be positive and <= 4",
            ));
        }
        if f64s[axis + 3].abs() > 32.0 {
            return Err(parse_error(line, "velocity magnitude exceeds 32"));
        }
    }
    let y_offset = bounded_f32(f32s[0], -2.0, 2.0, line, "y_offset")?;
    let y_size = bounded_f32(f32s[1], 0.0, 2.0, line, "y_size")?;
    let step_height = bounded_f32(f32s[2], 0.0, 1.0, line, "step_height")?;
    let fall_distance = bounded_f32(f32s[3], 0.0, 256.0, line, "fall_distance")?;
    let yaw_degrees = bounded_f32(f32s[4], -36000.0, 36000.0, line, "yaw")?;
    Ok(MotionState {
        bbox: Aabb::new(bbox_min, bbox_max),
        position: [f64s[0], f64s[1], f64s[2]],
        velocity: [f64s[3], f64s[4], f64s[5]],
        y_offset,
        y_size,
        step_height,
        fall_distance,
        yaw_degrees,
        on_ground: flags & 1 != 0,
        collided_horizontally: flags & 2 != 0,
        collided_vertically: flags & 4 != 0,
        collided: flags & 8 != 0,
        no_clip: flags & 16 != 0,
        in_web: flags & 32 != 0,
        sneaking: flags & 64 != 0,
        allow_slide: flags & 128 != 0,
    })
}

fn parse_boxes(text: &str, line: usize) -> Result<Vec<Aabb>, String> {
    if text == "-" {
        return Ok(Vec::new());
    }
    let rows: Vec<_> = text.split(':').collect();
    if rows.len() > MAX_BOXES {
        return Err(parse_error(line, "box limit 128 exceeded"));
    }
    rows.into_iter()
        .map(|row| {
            let parts = exact_parts(row, ',', 6, line, "box")?;
            let mut values = [0.0; 6];
            for (i, part) in parts.iter().enumerate() {
                values[i] = f64_bits(part, line, "box coordinate")?;
            }
            let bounds = Aabb::new(
                [values[0], values[1], values[2]],
                [values[3], values[4], values[5]],
            );
            if (0..3).any(|axis| {
                let extent = bounds.max[axis] - bounds.min[axis];
                !(extent > 0.0 && extent.is_finite())
            }) {
                return Err(parse_error(
                    line,
                    "collider extents must be finite and positive",
                ));
            }
            Ok(bounds)
        })
        .collect()
}

fn parse_frames(mode: &str, text: &str, line: usize) -> Result<Vec<Frame>, String> {
    let rows: Vec<_> = text.split(';').collect();
    if rows.is_empty() || rows.len() > MAX_FRAMES {
        return Err(parse_error(line, "frame count must be 1..256"));
    }
    rows.into_iter()
        .map(|row| match mode {
            "move" => {
                let values = exact_parts(row, ',', 3, line, "move frame")?;
                let mut delta = [0.0; 3];
                for (i, value) in values.iter().enumerate() {
                    delta[i] = f64_bits(value, line, "move delta")?;
                    if delta[i].abs() > 32.0 {
                        return Err(parse_error(line, "move delta magnitude exceeds 32"));
                    }
                }
                Ok(Frame::Move(delta))
            }
            "travel" => {
                let values = exact_parts(row, ',', 5, line, "travel frame")?;
                let strafe = f32_bits(values[0], line, "strafe")?;
                let forward = f32_bits(values[1], line, "forward")?;
                let yaw_degrees = f32_bits(values[2], line, "yaw")?;
                if strafe.abs() > 1.0 || forward.abs() > 1.0 || yaw_degrees.abs() > 36000.0 {
                    return Err(parse_error(
                        line,
                        "heading or yaw input exceeds frozen limit",
                    ));
                }
                if !matches!(values[3], "0" | "1") || !matches!(values[4], "0" | "1") {
                    return Err(parse_error(line, "jump and sneak must be 0 or 1"));
                }
                Ok(Frame::Travel(TravelInput {
                    strafe,
                    forward,
                    yaw_degrees,
                    jump: values[3] == "1",
                    sneaking: values[4] == "1",
                }))
            }
            _ => Err(parse_error(line, "mode must be move or travel")),
        })
        .collect()
}

fn parse_scenario(row: &str, line: usize) -> Result<Scenario, String> {
    let fields: Vec<_> = row.split('\t').collect();
    if fields.len() != 6 {
        return Err(parse_error(line, "expected six tab-separated fields"));
    }
    if !valid_id(fields[0]) {
        return Err(parse_error(line, "invalid case ID"));
    }
    if !matches!(fields[1], "move" | "travel") {
        return Err(parse_error(line, "mode must be move or travel"));
    }
    let state = parse_state(fields[2], line)?;
    let colliders = parse_boxes(fields[3], line)?;
    let surface = match fields[4] {
        "0" => Surface::Air,
        "1" => Surface::Stone,
        "79" => Surface::Ice,
        _ => return Err(parse_error(line, "surface ID must be 0, 1, or 79")),
    };
    let frames = parse_frames(fields[1], fields[5], line)?;
    Ok(Scenario {
        id: fields[0].to_owned(),
        state,
        colliders,
        surface,
        frames,
    })
}

fn bits64(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}
fn bits32(value: f32) -> String {
    format!("{:08x}", value.to_bits())
}

fn state_text(state: MotionState) -> String {
    let mut values = Vec::with_capacity(18);
    for value in state
        .position
        .into_iter()
        .chain(state.velocity)
        .chain(state.bbox.min)
        .chain(state.bbox.max)
    {
        values.push(bits64(value));
    }
    for value in [
        state.y_offset,
        state.y_size,
        state.step_height,
        state.fall_distance,
        state.yaw_degrees,
    ] {
        values.push(bits32(value));
    }
    let flags = u8::from(state.on_ground)
        | (u8::from(state.collided_horizontally) << 1)
        | (u8::from(state.collided_vertically) << 2)
        | (u8::from(state.collided) << 3)
        | (u8::from(state.no_clip) << 4)
        | (u8::from(state.in_web) << 5)
        | (u8::from(state.sneaking) << 6)
        | (u8::from(state.allow_slide) << 7);
    values.push(flags.to_string());
    values.join(",")
}

// The synthetic Java oracle maps a voxel to the configured block only when a
// collider overlaps that voxel. Real gameplay supplies the sampled block directly.
fn synthetic_surface(state: &MotionState, colliders: &[Aabb], surface: Surface) -> Surface {
    let x = beta_core::math::floor_f64(state.position[0]);
    let y = beta_core::math::floor_f64(state.bbox.min[1]).wrapping_sub(1);
    let z = beta_core::math::floor_f64(state.position[2]);
    let voxel = Aabb::new(
        [f64::from(x), f64::from(y), f64::from(z)],
        [f64::from(x) + 1.0, f64::from(y) + 1.0, f64::from(z) + 1.0],
    );
    if colliders.iter().any(|collider| collider.intersects(voxel)) {
        surface
    } else {
        Surface::Air
    }
}

fn process_with_limits<R: Read, W: Write>(
    reader: R,
    mut writer: W,
    max_scenarios: usize,
    max_steps: usize,
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
    if input.iter().any(|b| !b.is_ascii()) {
        return Err("TSV input must be ASCII".into());
    }
    let mut scenarios = Vec::new();
    let mut ids = HashSet::new();
    let mut total_steps = 0usize;
    for (index, row) in input[..input.len() - 1].split(|b| *b == b'\n').enumerate() {
        let line = index + 1;
        if row.is_empty() {
            return Err(parse_error(line, "blank rows are not allowed"));
        }
        let row = std::str::from_utf8(row).map_err(|_| parse_error(line, "invalid UTF-8"))?;
        let scenario = parse_scenario(row, line)?;
        if !ids.insert(scenario.id.clone()) {
            return Err(parse_error(line, "duplicate case ID"));
        }
        if scenarios.len() >= max_scenarios {
            return Err(format!("scenario limit {max_scenarios} exceeded"));
        }
        total_steps = total_steps
            .checked_add(scenario.frames.len())
            .ok_or_else(|| "step count overflow".to_owned())?;
        if total_steps > max_steps {
            return Err(format!("output step limit {max_steps} exceeded"));
        }
        scenarios.push(scenario);
    }
    let mut output = String::new();
    for mut scenario in scenarios {
        for (index, frame) in scenario.frames.into_iter().enumerate() {
            let surface = synthetic_surface(&scenario.state, &scenario.colliders, scenario.surface);
            match frame {
                Frame::Move(delta) => move_entity(&mut scenario.state, delta, &scenario.colliders),
                Frame::Travel(input) => travel_with_ladder_query(
                    &mut scenario.state,
                    input,
                    &scenario.colliders,
                    surface,
                    |_| true,
                ),
            }
            output.push_str(&scenario.id);
            output.push('\t');
            output.push_str(&index.to_string());
            output.push('\t');
            output.push_str(&state_text(scenario.state));
            output.push('\n');
        }
    }
    writer
        .write_all(output.as_bytes())
        .map_err(|e| format!("failed to write stdout: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("failed to flush stdout: {e}"))
}

pub fn process_stream<R: Read, W: Write>(reader: R, writer: W) -> Result<(), String> {
    process_with_limits(reader, writer, MAX_SCENARIOS, MAX_STEPS, MAX_INPUT_BYTES)
}

fn run() -> Result<(), String> {
    if std::env::args_os().len() != 1 {
        return Err("unexpected command-line arguments".into());
    }
    process_stream(io::stdin().lock(), io::BufWriter::new(io::stdout().lock()))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("movement_probe error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_native_corpus_matches_every_state_exactly() {
        let input = include_bytes!("../../../tests/fixtures/ladder_movement/input.tsv");
        let expected = include_bytes!("../../../tests/fixtures/ladder_movement/reference.tsv");
        let mut output = Vec::new();
        process_stream(input.as_slice(), &mut output).unwrap();
        assert_eq!(output.as_slice(), expected);
        let records = expected.iter().filter(|b| **b == b'\n').count();
        assert!(records >= 5_000);
        assert_eq!(output.iter().filter(|b| **b == b'\n').count(), records);
    }

    #[test]
    fn malformed_and_late_invalid_rows_are_atomic() {
        let good = include_bytes!("../../../tests/fixtures/movement/input.tsv");
        let mut late = good.to_vec();
        late.extend_from_slice(b"bad\tmove\t\n");
        let invalid: &[&[u8]] = &[b"", b"x\tmove\t\n", b"x\tmove\tbad\t-\t0\t0000000000000000,0000000000000000,0000000000000000\n", b"x\tmove\t0000000000000000\t-\t0\t0000000000000000,0000000000000000,0000000000000000\n"];
        for bytes in invalid
            .iter()
            .copied()
            .chain(std::iter::once(late.as_slice()))
        {
            let mut output = Vec::new();
            assert!(process_stream(bytes, &mut output).is_err());
            assert!(output.is_empty());
        }
    }

    #[test]
    fn scenario_step_and_input_limits_are_atomic() {
        let row = include_bytes!("../../../tests/fixtures/movement/input.tsv")
            .split(|b| *b == b'\n')
            .next()
            .unwrap();
        let mut input = row.to_vec();
        input.push(b'\n');
        let mut output = Vec::new();
        assert!(
            process_with_limits(input.as_slice(), &mut output, 0, usize::MAX, usize::MAX).is_err()
        );
        assert!(output.is_empty());
        let mut output = Vec::new();
        assert!(
            process_with_limits(input.as_slice(), &mut output, usize::MAX, 1, usize::MAX).is_err()
        );
        assert!(output.is_empty());
        let mut output = Vec::new();
        assert!(
            process_with_limits(
                input.as_slice(),
                &mut output,
                usize::MAX,
                usize::MAX,
                input.len() - 1
            )
            .is_err()
        );
        assert!(output.is_empty());
    }

    #[test]
    fn per_scenario_frame_and_collider_limits_are_enforced() {
        let template = std::str::from_utf8(
            include_bytes!("../../../tests/fixtures/movement/input.tsv")
                .split(|b| *b == b'\n')
                .next()
                .unwrap(),
        )
        .unwrap();
        let fields: Vec<_> = template.split('\t').collect();
        let move_frame = fields[5].split(';').next().unwrap();
        let frames = std::iter::repeat_n(move_frame, 257)
            .collect::<Vec<_>>()
            .join(";");
        let too_many_frames = format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            fields[0], fields[1], fields[2], fields[3], fields[4], frames
        );
        let boxes = std::iter::repeat_n(fields[3], 129)
            .collect::<Vec<_>>()
            .join(":");
        let too_many_boxes = format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            fields[0], fields[1], fields[2], boxes, fields[4], move_frame
        );
        for invalid in [too_many_frames, too_many_boxes] {
            let mut output = Vec::new();
            assert!(process_stream(invalid.as_bytes(), &mut output).is_err());
            assert!(output.is_empty());
        }
    }
}
