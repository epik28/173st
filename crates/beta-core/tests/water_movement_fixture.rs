use std::collections::BTreeMap;

use beta_core::{
    geometry::Aabb,
    motion::{MotionState, TravelInput, WaterMotionCell, water_living_step, water_travel},
};

const INPUT: &str = include_str!("../../../tests/fixtures/water_movement/input.tsv");
const REFERENCE: &str = include_str!("../../../tests/fixtures/water_movement/reference.tsv");

#[test]
fn invalid_and_oversized_water_inputs_are_rejected_atomically() {
    let input = TravelInput {
        strafe: 0.5,
        forward: 1.0,
        yaw_degrees: 90.0,
        jump: true,
        sneaking: true,
    };
    let collider = Aabb::new([0.0; 3], [1.0; 3]);

    let mut state = MotionState::new([0.5, 1.0, 0.5]);
    let original = state;
    let mut invalid = BTreeMap::new();
    invalid.insert((0, 0, 0), WaterMotionCell { id: 1, metadata: 0 });
    assert_eq!(
        water_travel(&mut state, input, &[], &invalid),
        Err(beta_core::motion::WaterMotionError::InvalidWaterCell((
            0, 0, 0
        )))
    );
    assert_eq!(state, original);

    let mut oversized_cells = BTreeMap::new();
    for x in 0..513 {
        oversized_cells.insert((x, 0, 0), WaterMotionCell { id: 8, metadata: 0 });
    }
    assert_eq!(
        water_travel(&mut state, input, &[], &oversized_cells),
        Err(beta_core::motion::WaterMotionError::TooManyWaterCells)
    );
    assert_eq!(state, original);

    // Dense but still local viewer queries must not fail at the former
    // 128-collider cutoff while the total remains within the bounded limit.
    let dense_local_colliders = vec![collider; 129];
    let mut dense_state = original;
    assert!(
        water_travel(
            &mut dense_state,
            TravelInput {
                jump: false,
                ..input
            },
            &dense_local_colliders,
            &BTreeMap::new(),
        )
        .is_ok()
    );

    let oversized_colliders = vec![collider; 513];
    assert_eq!(
        water_travel(&mut state, input, &oversized_colliders, &BTreeMap::new()),
        Err(beta_core::motion::WaterMotionError::TooManyColliders)
    );
    assert_eq!(state, original);
}

#[test]
fn native_water_movement_matches_all_ordered_states_and_raw_float_words() {
    let actual = run(INPUT).unwrap();
    let actual_rows: Vec<_> = actual.lines().collect();
    let expected_rows: Vec<_> = REFERENCE.lines().collect();
    assert_eq!(
        actual_rows.len(),
        expected_rows.len(),
        "state row count differs"
    );
    for (index, (actual, expected)) in actual_rows.iter().zip(expected_rows).enumerate() {
        assert_eq!(
            actual, &expected,
            "first state mismatch at ordered row {index}"
        );
    }
}

#[test]
fn native_living_swim_matches_full_motion_state() {
    let actual = run_mode(
        include_str!("../../../tests/fixtures/water_swim/input.tsv"),
        true,
    )
    .unwrap();
    let expected = include_str!("../../../tests/fixtures/water_swim/reference.tsv");
    assert_eq!(actual, expected);
}

fn run(input: &str) -> Result<String, String> {
    run_mode(input, false)
}
fn run_mode(input: &str, living: bool) -> Result<String, String> {
    let mut output = String::new();
    for line in input.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 5 {
            return Err(format!("{}: expected five input fields", fields[0]));
        }
        let mut state = parse_state(fields[1])?;
        let water = parse_water(fields[2])?;
        let colliders = parse_colliders(fields[3])?;
        for (index, raw_frame) in fields[4].split(';').enumerate() {
            let values: Vec<_> = raw_frame.split(',').collect();
            if values.len() != 5 {
                return Err(format!(
                    "{} frame {index}: expected five input values",
                    fields[0]
                ));
            }
            let travel = if living {
                water_living_step
            } else {
                water_travel
            };
            let observation = travel(
                &mut state,
                TravelInput {
                    strafe: parse_f32(values[0])?,
                    forward: parse_f32(values[1])?,
                    yaw_degrees: parse_f32(values[2])?,
                    jump: values[3] == "1",
                    sneaking: values[4] == "1",
                },
                &colliders,
                &water,
            )
            .map_err(|error| format!("{} frame {index}: {error:?}", fields[0]))?;
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                fields[0],
                index,
                format_state(&state),
                u8::from(observation.in_water),
                u8::from(observation.eyes_in_water),
                observation.native_water_queries,
                observation.water_contact_queries,
            ));
        }
    }
    Ok(output)
}

fn parse_state(raw: &str) -> Result<MotionState, String> {
    let fields: Vec<_> = raw.split(',').collect();
    if fields.len() != 18 {
        return Err("state must contain 18 fields".to_owned());
    }
    let mut state = MotionState::new([0.0; 3]);
    state.position = [
        parse_f64(fields[0])?,
        parse_f64(fields[1])?,
        parse_f64(fields[2])?,
    ];
    state.velocity = [
        parse_f64(fields[3])?,
        parse_f64(fields[4])?,
        parse_f64(fields[5])?,
    ];
    state.bbox = Aabb::new(
        [
            parse_f64(fields[6])?,
            parse_f64(fields[7])?,
            parse_f64(fields[8])?,
        ],
        [
            parse_f64(fields[9])?,
            parse_f64(fields[10])?,
            parse_f64(fields[11])?,
        ],
    );
    state.y_offset = parse_f32(fields[12])?;
    state.y_size = parse_f32(fields[13])?;
    state.step_height = parse_f32(fields[14])?;
    state.fall_distance = parse_f32(fields[15])?;
    state.yaw_degrees = parse_f32(fields[16])?;
    let flags: u8 = fields[17].parse().map_err(|_| "invalid flags")?;
    state.on_ground = flags & 1 != 0;
    state.collided_horizontally = flags & 2 != 0;
    state.collided_vertically = flags & 4 != 0;
    state.collided = flags & 8 != 0;
    state.no_clip = flags & 16 != 0;
    state.in_web = flags & 32 != 0;
    state.sneaking = flags & 64 != 0;
    state.allow_slide = flags & 128 != 0;
    Ok(state)
}

fn parse_water(raw: &str) -> Result<BTreeMap<(i32, i32, i32), WaterMotionCell>, String> {
    let mut cells = BTreeMap::new();
    if raw == "-" {
        return Ok(cells);
    }
    for row in raw.split(';') {
        let values: Vec<_> = row.split(',').collect();
        if values.len() != 5 {
            return Err("water cell must contain five values".to_owned());
        }
        let position = (
            parse_int(values[0])?,
            parse_int(values[1])?,
            parse_int(values[2])?,
        );
        cells.insert(
            position,
            WaterMotionCell {
                id: values[3].parse().map_err(|_| "invalid water id")?,
                metadata: values[4].parse().map_err(|_| "invalid water metadata")?,
            },
        );
    }
    Ok(cells)
}

fn parse_colliders(raw: &str) -> Result<Vec<Aabb>, String> {
    if raw == "-" {
        return Ok(Vec::new());
    }
    raw.split(':')
        .map(|row| {
            let values: Vec<_> = row.split(',').collect();
            if values.len() != 6 {
                return Err("collision box must contain six values".to_owned());
            }
            Ok(Aabb::new(
                [
                    parse_f64(values[0])?,
                    parse_f64(values[1])?,
                    parse_f64(values[2])?,
                ],
                [
                    parse_f64(values[3])?,
                    parse_f64(values[4])?,
                    parse_f64(values[5])?,
                ],
            ))
        })
        .collect()
}

fn parse_f64(raw: &str) -> Result<f64, String> {
    u64::from_str_radix(raw, 16)
        .map(f64::from_bits)
        .map_err(|_| format!("invalid raw f64 {raw}"))
}

fn parse_f32(raw: &str) -> Result<f32, String> {
    u32::from_str_radix(raw, 16)
        .map(f32::from_bits)
        .map_err(|_| format!("invalid raw f32 {raw}"))
}

fn parse_int(raw: &str) -> Result<i32, String> {
    raw.parse().map_err(|_| format!("invalid integer {raw}"))
}

fn format_state(state: &MotionState) -> String {
    let flags = u8::from(state.on_ground)
        | (u8::from(state.collided_horizontally) << 1)
        | (u8::from(state.collided_vertically) << 2)
        | (u8::from(state.collided) << 3)
        | (u8::from(state.no_clip) << 4)
        | (u8::from(state.in_web) << 5)
        | (u8::from(state.sneaking) << 6)
        | (u8::from(state.allow_slide) << 7);
    [
        format!("{:016x}", state.position[0].to_bits()),
        format!("{:016x}", state.position[1].to_bits()),
        format!("{:016x}", state.position[2].to_bits()),
        format!("{:016x}", state.velocity[0].to_bits()),
        format!("{:016x}", state.velocity[1].to_bits()),
        format!("{:016x}", state.velocity[2].to_bits()),
        format!("{:016x}", state.bbox.min[0].to_bits()),
        format!("{:016x}", state.bbox.min[1].to_bits()),
        format!("{:016x}", state.bbox.min[2].to_bits()),
        format!("{:016x}", state.bbox.max[0].to_bits()),
        format!("{:016x}", state.bbox.max[1].to_bits()),
        format!("{:016x}", state.bbox.max[2].to_bits()),
        format!("{:08x}", state.y_offset.to_bits()),
        format!("{:08x}", state.y_size.to_bits()),
        format!("{:08x}", state.step_height.to_bits()),
        format!("{:08x}", state.fall_distance.to_bits()),
        format!("{:08x}", state.yaw_degrees.to_bits()),
        flags.to_string(),
    ]
    .join(",")
}
