//! Beta 1.7.3 entity collision and dry player travel.
//!
//! The collision resolver follows the pinned `sn.b(DDD)` bytecode. Dry travel
//! follows `ls.a_(FF)`, `sn.a(FFF)`, and `ls.R()` with water, lava, and ladder
//! queries disabled by the frozen movement contract.

use crate::geometry::{Aabb, Axis};
use crate::math::{cos, floor_f64, sin, sqrt_f32};

const WEB_XZ_SCALE: f64 = 0.25;
const WEB_Y_SCALE: f64 = 0.050_000_000_745_058_06;
const STEP_SNEAK_INCREMENT: f64 = 0.05;
const STEP_GATE_Y_SIZE: f32 = 0.05;
const GRAVITY: f64 = 0.08;
const VERTICAL_DRAG: f64 = 0.980_000_019_073_486_3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionState {
    pub bbox: Aabb,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub y_offset: f32,
    pub y_size: f32,
    pub step_height: f32,
    pub fall_distance: f32,
    pub yaw_degrees: f32,
    pub on_ground: bool,
    pub collided_horizontally: bool,
    pub collided_vertically: bool,
    pub collided: bool,
    pub no_clip: bool,
    pub in_web: bool,
    pub sneaking: bool,
    pub allow_slide: bool,
}

impl MotionState {
    /// Creates the native player-sized box at the supplied bottom-center point.
    #[must_use]
    pub fn new(position: [f64; 3]) -> Self {
        let half_width = f64::from(0.6_f32 / 2.0_f32);
        let height = f64::from(1.8_f32);
        Self {
            bbox: Aabb::new(
                [
                    position[0] - half_width,
                    position[1],
                    position[2] - half_width,
                ],
                [
                    position[0] + half_width,
                    position[1] + height,
                    position[2] + half_width,
                ],
            ),
            position,
            velocity: [0.0; 3],
            y_offset: 0.0,
            y_size: 0.0,
            step_height: 0.5_f32,
            fall_distance: 0.0,
            yaw_degrees: 0.0,
            on_ground: false,
            collided_horizontally: false,
            collided_vertically: false,
            collided: false,
            no_clip: false,
            in_web: false,
            sneaking: false,
            allow_slide: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Air,
    Stone,
    Ice,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TravelInput {
    pub strafe: f32,
    pub forward: f32,
    pub yaw_degrees: f32,
    pub jump: bool,
    pub sneaking: bool,
}

/// Moves an entity box through ordered collision candidates.
pub fn move_entity(state: &mut MotionState, delta: [f64; 3], colliders: &[Aabb]) {
    if state.no_clip {
        state.bbox = state.bbox.offset(delta[0], delta[1], delta[2]);
        update_position(state);
        return;
    }

    state.y_size *= 0.4_f32;
    let mut requested = delta;
    if state.in_web {
        state.in_web = false;
        requested[0] *= WEB_XZ_SCALE;
        requested[1] *= WEB_Y_SCALE;
        requested[2] *= WEB_XZ_SCALE;
        state.velocity = [0.0; 3];
    }

    let initial_ground = state.on_ground;
    let sneak_support = initial_ground && state.sneaking;
    if sneak_support {
        requested[0] = reduce_to_supported_delta(state.bbox, requested[0], Axis::X, colliders);
        requested[2] = reduce_to_supported_delta(state.bbox, requested[2], Axis::Z, colliders);
    }

    let original_request = requested;
    let start_box = state.bbox;
    let base = clip_motion(start_box, requested, state.allow_slide, colliders);
    let mut chosen = base;

    let step_eligible = initial_ground || base.collided_downward;
    let step_gate = sneak_support || state.y_size < STEP_GATE_Y_SIZE;
    if state.step_height > 0.0
        && step_eligible
        && step_gate
        && (original_request[0] != base.delta[0] || original_request[2] != base.delta[2])
    {
        let step_request = [
            original_request[0],
            f64::from(state.step_height),
            original_request[2],
        ];
        let step = clip_step(
            start_box,
            step_request,
            original_request[1],
            state.step_height,
            state.allow_slide,
            colliders,
        );
        let base_horizontal = base.delta[0] * base.delta[0] + base.delta[2] * base.delta[2];
        let step_horizontal = step.delta[0] * step.delta[0] + step.delta[2] * step.delta[2];

        // Native `if (baseDistance >= stepDistance)` restores the base result,
        // so the base path wins exact ties.
        if base_horizontal >= step_horizontal {
            chosen = base;
        } else {
            chosen = step;
            let fractional_y = chosen.bbox.min[1] - f64::from(chosen.bbox.min[1] as i32);
            if fractional_y > 0.0 {
                state.y_size = (f64::from(state.y_size) + (fractional_y + 0.01)) as f32;
            }
        }
    }

    state.bbox = chosen.bbox;
    update_position(state);

    state.collided_horizontally =
        original_request[0] != chosen.delta[0] || original_request[2] != chosen.delta[2];
    state.collided_vertically = original_request[1] != chosen.delta[1];
    state.on_ground = state.collided_vertically && original_request[1] < 0.0;
    state.collided = state.collided_horizontally || state.collided_vertically;

    for (axis, requested_component) in original_request.iter().enumerate() {
        if *requested_component != chosen.delta[axis] {
            state.velocity[axis] = 0.0;
        }
    }

    if state.on_ground {
        state.fall_distance = 0.0;
    } else if chosen.delta[1] < 0.0 {
        state.fall_distance = (f64::from(state.fall_distance) - chosen.delta[1]) as f32;
    }
}

#[derive(Clone, Copy)]
struct ClipResult {
    bbox: Aabb,
    delta: [f64; 3],
    collided_downward: bool,
}

fn clip_motion(
    start: Aabb,
    mut delta: [f64; 3],
    allow_slide: bool,
    colliders: &[Aabb],
) -> ClipResult {
    let requested = delta;
    let mut bbox = start;
    let candidates = candidates_for(bbox.swept(delta[0], delta[1], delta[2]), colliders);

    for obstacle in &candidates {
        delta[1] = obstacle.clip(bbox, Axis::Y, delta[1]);
    }
    bbox = bbox.offset(0.0, delta[1], 0.0);
    if !allow_slide && requested[1] != delta[1] {
        delta = [0.0; 3];
    }
    let collided_downward = requested[1] != delta[1] && requested[1] < 0.0;

    for obstacle in &candidates {
        delta[0] = obstacle.clip(bbox, Axis::X, delta[0]);
    }
    bbox = bbox.offset(delta[0], 0.0, 0.0);
    if !allow_slide && requested[0] != delta[0] {
        delta = [0.0; 3];
    }

    for obstacle in &candidates {
        delta[2] = obstacle.clip(bbox, Axis::Z, delta[2]);
    }
    bbox = bbox.offset(0.0, 0.0, delta[2]);
    if !allow_slide && requested[2] != delta[2] {
        delta = [0.0; 3];
    }

    ClipResult {
        bbox,
        delta,
        collided_downward,
    }
}

fn clip_step(
    start: Aabb,
    mut delta: [f64; 3],
    original_y: f64,
    step_height: f32,
    allow_slide: bool,
    colliders: &[Aabb],
) -> ClipResult {
    let original = delta;
    let step = f64::from(step_height);
    let mut bbox = start;
    let candidates = candidates_for(bbox.swept(delta[0], delta[1], delta[2]), colliders);

    for obstacle in &candidates {
        delta[1] = obstacle.clip(bbox, Axis::Y, delta[1]);
    }
    bbox = bbox.offset(0.0, delta[1], 0.0);
    if !allow_slide && original_y != delta[1] {
        delta = [0.0; 3];
    }

    for obstacle in &candidates {
        delta[0] = obstacle.clip(bbox, Axis::X, delta[0]);
    }
    bbox = bbox.offset(delta[0], 0.0, 0.0);
    if !allow_slide && original[0] != delta[0] {
        delta = [0.0; 3];
    }

    for obstacle in &candidates {
        delta[2] = obstacle.clip(bbox, Axis::Z, delta[2]);
    }
    bbox = bbox.offset(0.0, 0.0, delta[2]);
    if !allow_slide && original[2] != delta[2] {
        delta = [0.0; 3];
    }

    if !allow_slide && original_y != delta[1] {
        delta = [0.0; 3];
    } else {
        delta[1] = -step;
        for obstacle in &candidates {
            delta[1] = obstacle.clip(bbox, Axis::Y, delta[1]);
        }
        bbox = bbox.offset(0.0, delta[1], 0.0);
    }

    ClipResult {
        bbox,
        delta,
        collided_downward: false,
    }
}

/// Runs the native step retry using one candidate list for ascent, horizontal
/// movement, and the final descent. The `allow_slide` resets compare the step
/// deltas with the original requested movement on each axis.
fn candidates_for(query: Aabb, colliders: &[Aabb]) -> Vec<Aabb> {
    colliders
        .iter()
        .copied()
        .filter(|collider| collider.intersects(query))
        .collect()
}

fn reduce_to_supported_delta(bbox: Aabb, mut delta: f64, axis: Axis, colliders: &[Aabb]) -> f64 {
    while delta != 0.0 {
        let (x, z) = match axis {
            Axis::X => (delta, 0.0),
            Axis::Z => (0.0, delta),
            Axis::Y => unreachable!("sneak support only probes horizontal axes"),
        };
        let support_query = bbox.offset(x, -1.0, z);
        if !candidates_for(support_query, colliders).is_empty() {
            break;
        }
        if (-STEP_SNEAK_INCREMENT..=STEP_SNEAK_INCREMENT).contains(&delta) {
            delta = 0.0;
        } else if delta > 0.0 {
            delta -= STEP_SNEAK_INCREMENT;
        } else {
            delta += STEP_SNEAK_INCREMENT;
        }
    }
    delta
}

fn update_position(state: &mut MotionState) {
    state.position = [
        (state.bbox.min[0] + state.bbox.max[0]) / 2.0,
        state.bbox.min[1] + f64::from(state.y_offset) - f64::from(state.y_size),
        (state.bbox.min[2] + state.bbox.max[2]) / 2.0,
    ];
}

/// Runs one water-free `ls.a_(FF)` movement frame.
pub fn dry_travel(
    state: &mut MotionState,
    input: TravelInput,
    colliders: &[Aabb],
    surface: Surface,
) {
    state.yaw_degrees = input.yaw_degrees;
    state.sneaking = input.sneaking;
    if input.jump && state.on_ground {
        state.velocity[1] = 0.419_999_986_886_978_15;
    }

    let acceleration_friction = ground_friction(state, colliders, surface);
    let acceleration =
        0.162_771_36_f32 / (acceleration_friction * acceleration_friction * acceleration_friction);
    let ground_acceleration = if state.on_ground {
        0.1_f32 * acceleration
    } else {
        0.02_f32
    };
    move_flying(state, input.strafe, input.forward, ground_acceleration);

    let movement = state.velocity;
    move_entity(state, movement, colliders);

    let horizontal_friction = ground_friction(state, colliders, surface);
    state.velocity[1] -= GRAVITY;
    state.velocity[1] *= VERTICAL_DRAG;
    let horizontal_drag = f64::from(horizontal_friction);
    state.velocity[0] *= horizontal_drag;
    state.velocity[2] *= horizontal_drag;
}

fn ground_friction(state: &MotionState, colliders: &[Aabb], surface: Surface) -> f32 {
    if !state.on_ground {
        return 0.91_f32;
    }

    let mut friction = 0.546_000_06_f32;
    let block_x = floor_f64(state.position[0]);
    let block_y = floor_f64(state.bbox.min[1]).wrapping_sub(1);
    let block_z = floor_f64(state.position[2]);
    let voxel = Aabb::new(
        [f64::from(block_x), f64::from(block_y), f64::from(block_z)],
        [
            f64::from(block_x) + 1.0,
            f64::from(block_y) + 1.0,
            f64::from(block_z) + 1.0,
        ],
    );
    let found_block =
        surface != Surface::Air && colliders.iter().any(|box_| box_.intersects(voxel));
    if found_block {
        let slipperiness = match surface {
            Surface::Air => unreachable!("air has no slipperiness lookup"),
            Surface::Stone => 0.6_f32,
            Surface::Ice => 0.98_f32,
        };
        friction = slipperiness * 0.91_f32;
    }
    friction
}

#[allow(clippy::approx_constant)] // Native embeds the f32 literal 3.1415927.
fn move_flying(state: &mut MotionState, mut strafe: f32, mut forward: f32, acceleration: f32) {
    let mut magnitude = sqrt_f32(strafe * strafe + forward * forward);
    if magnitude < 0.01_f32 {
        return;
    }
    if magnitude < 1.0_f32 {
        magnitude = 1.0_f32;
    }
    let scale = acceleration / magnitude;
    strafe *= scale;
    forward *= scale;

    let radians = state.yaw_degrees * 3.141_592_7_f32 / 180.0_f32;
    let sin_yaw = sin(radians);
    let cos_yaw = cos(radians);
    state.velocity[0] += f64::from(strafe * cos_yaw - forward * sin_yaw);
    state.velocity[2] += f64::from(forward * cos_yaw + strafe * sin_yaw);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_box() -> Aabb {
        Aabb::new([-10.0, 0.0, -10.0], [10.0, 1.0, 10.0])
    }

    #[test]
    fn constructor_and_no_clip_keep_native_box_and_flag_semantics() {
        let mut state = MotionState::new([1.0, 2.0, 3.0]);
        assert_eq!(
            state.bbox.min,
            [1.0 - f64::from(0.3_f32), 2.0, 3.0 - f64::from(0.3_f32)]
        );
        state.no_clip = true;
        state.on_ground = true;
        state.velocity = [1.0, 2.0, 3.0];
        state.y_size = 0.25;
        move_entity(&mut state, [2.0, -1.0, 4.0], &[]);
        assert_eq!(state.position, [3.0, 0.75, 7.0]);
        assert_eq!(state.velocity, [1.0, 2.0, 3.0]);
        assert!(state.on_ground);
        assert_eq!(state.y_size, 0.25);
    }

    #[test]
    fn movement_clips_in_y_x_z_order_and_updates_ground_and_fall() {
        let mut state = MotionState::new([0.5, 1.0, 0.5]);
        let wall = Aabb::new([1.0, 0.0, -1.0], [2.0, 3.0, 2.0]);
        move_entity(&mut state, [1.0, -0.25, 0.0], &[floor_box(), wall]);
        assert_eq!(state.bbox.max[0], 1.0);
        assert_eq!(state.bbox.min[1], 1.0);
        assert!(state.collided_horizontally);
        assert!(state.collided_vertically);
        assert!(state.on_ground);
        assert!(state.collided);
        assert_eq!(state.fall_distance, 0.0);
        assert_eq!(state.velocity[0], 0.0);

        let mut falling = MotionState::new([0.5, 10.0, 0.5]);
        falling.fall_distance = 2.0;
        move_entity(&mut falling, [0.0, -0.5, 0.0], &[]);
        assert_eq!(falling.fall_distance, 2.5);
    }

    #[test]
    fn web_scaling_zeros_velocity_and_sneak_stops_at_unsupported_edge() {
        let mut web = MotionState::new([0.5, 1.0, 0.5]);
        web.in_web = true;
        web.velocity = [2.0, -1.0, 3.0];
        move_entity(&mut web, [1.0, -1.0, 1.0], &[]);
        assert_eq!(web.position[0], 0.75);
        assert_eq!(
            web.position[1].to_bits(),
            0.949_999_999_254_941_9_f64.to_bits()
        );
        assert_eq!(web.position[2], 0.75);
        assert_eq!(web.velocity, [0.0; 3]);
        assert!(!web.in_web);

        let mut sneaking = MotionState::new([0.5, 1.0, 0.5]);
        sneaking.on_ground = true;
        sneaking.sneaking = true;
        let narrow_floor = Aabb::new([0.0, 0.0, 0.0], [0.399, 1.0, 1.0]);
        move_entity(&mut sneaking, [0.2, 0.0, 0.0], &[narrow_floor]);
        assert_eq!(sneaking.position[0], 0.65);
        assert!(!sneaking.collided_horizontally);
    }

    #[test]
    fn disallow_slide_keeps_axis_offset_but_clears_clipped_deltas() {
        let mut state = MotionState::new([0.5, 2.0, 0.5]);
        state.allow_slide = false;
        state.velocity = [1.0, 2.0, 3.0];
        let wall = Aabb::new([1.0, 0.0, 0.0], [2.0, 4.0, 1.0]);
        move_entity(&mut state, [1.0, 0.0, 0.0], &[wall]);
        assert_eq!(state.bbox.max[0], 1.0);
        assert_eq!(state.velocity, [0.0, 2.0, 3.0]);
        assert!(state.collided_horizontally);
        assert!(!state.collided_vertically);
    }

    #[test]
    fn step_retry_uses_clearance_and_base_wins_an_exact_distance_tie() {
        let mut can_step = MotionState::new([0.5, 1.0, 0.5]);
        can_step.on_ground = true;
        can_step.sneaking = true;
        let low_wall = Aabb::new([0.9, 1.0, 0.0], [1.2, 1.4, 1.0]);
        move_entity(&mut can_step, [0.8, 0.0, 0.0], &[floor_box(), low_wall]);
        assert!(can_step.position[0] > 1.0);
        assert_eq!(can_step.bbox.min[1], 1.4);
        assert_eq!(can_step.y_size, 0.41_f32);

        let mut tied = MotionState::new([0.5, 1.0, 0.5]);
        tied.on_ground = true;
        tied.sneaking = true;
        let side_wall = Aabb::new([0.9, 1.0, 0.0], [1.2, 4.0, 1.0]);
        let ceiling = Aabb::new([-1.0, 2.8, -1.0], [3.0, 3.0, 2.0]);
        move_entity(
            &mut tied,
            [0.8, 0.0, 0.0],
            &[floor_box(), side_wall, ceiling],
        );
        assert_eq!(tied.bbox.min[1], 1.0);
        assert_eq!(tied.position[0], 0.9 - f64::from(0.3_f32));
        assert!(tied.collided_horizontally);

        let mut slide_disabled = MotionState::new([0.5, 1.0, 0.5]);
        slide_disabled.on_ground = true;
        slide_disabled.sneaking = true;
        slide_disabled.allow_slide = false;
        move_entity(
            &mut slide_disabled,
            [0.8, 0.0, 0.0],
            &[floor_box(), low_wall],
        );
        assert_eq!(slide_disabled.bbox.min[1], 1.0);
        assert_eq!(slide_disabled.velocity[0], 0.0);
    }

    #[test]
    fn dry_travel_acceleration_yaw_gravity_friction_and_jump_are_separate() {
        let mut state = MotionState::new([0.5, 1.0, 0.5]);
        state.on_ground = true;
        dry_travel(
            &mut state,
            TravelInput {
                strafe: 0.0,
                forward: 1.0,
                yaw_degrees: 0.0,
                jump: true,
                sneaking: false,
            },
            &[floor_box()],
            Surface::Stone,
        );
        assert!(state.position[2] > 0.5);
        assert!(state.position[1] > 1.0);
        assert!(!state.on_ground);
        assert!(state.velocity[1] < 0.419_999_986_886_978_15);

        let mut ice = MotionState::new([0.5, 1.0, 0.5]);
        ice.on_ground = true;
        ice.velocity[0] = 1.0;
        ice.velocity[1] = -0.1;
        dry_travel(
            &mut ice,
            TravelInput {
                strafe: 0.0,
                forward: 0.0,
                yaw_degrees: 0.0,
                jump: false,
                sneaking: false,
            },
            &[floor_box()],
            Surface::Ice,
        );
        assert_eq!(
            ice.velocity[0].to_bits(),
            f64::from(0.98_f32 * 0.91_f32).to_bits()
        );

        let mut stone = MotionState::new([0.5, 1.0, 0.5]);
        stone.on_ground = true;
        stone.velocity[0] = 1.0;
        stone.velocity[1] = -0.1;
        dry_travel(
            &mut stone,
            TravelInput {
                strafe: 0.0,
                forward: 0.0,
                yaw_degrees: 0.0,
                jump: false,
                sneaking: false,
            },
            &[floor_box()],
            Surface::Stone,
        );
        assert_eq!(
            stone.velocity[0].to_bits(),
            f64::from(0.6_f32 * 0.91_f32).to_bits()
        );

        let mut air = MotionState::new([0.5, 1.0, 0.5]);
        air.on_ground = true;
        air.velocity[0] = 1.0;
        dry_travel(
            &mut air,
            TravelInput {
                strafe: 0.0,
                forward: 0.0,
                yaw_degrees: 0.0,
                jump: false,
                sneaking: false,
            },
            &[floor_box()],
            Surface::Air,
        );
        assert_eq!(air.velocity[0].to_bits(), f64::from(0.91_f32).to_bits());

        let mut grounded_air = MotionState::new([0.5, 2.0, 0.5]);
        grounded_air.on_ground = true;
        dry_travel(
            &mut grounded_air,
            TravelInput {
                strafe: 0.0,
                forward: 1.0,
                yaw_degrees: 0.0,
                jump: false,
                sneaking: false,
            },
            &[],
            Surface::Air,
        );
        let initial_ground_friction = 0.546_000_06_f32;
        let expected_acceleration = 0.1_f32
            * (0.162_771_36_f32
                / (initial_ground_friction * initial_ground_friction * initial_ground_friction));
        let expected_velocity = f64::from(expected_acceleration) * f64::from(0.91_f32);
        assert_eq!(
            grounded_air.velocity[2].to_bits(),
            expected_velocity.to_bits()
        );
    }
}
