//! Deterministic fixed-step walking controller, independent of winit and wgpu.

use std::time::Duration;

use beta_core::{
    geometry::Aabb,
    motion::{MotionState, TravelInput, dry_travel, water_contact, water_living_step},
};
use beta_view::{Scene, ViewError};

pub const TICK: Duration = Duration::from_nanos(50_000_000);
const QUERY_MARGIN: f64 = 3.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Controls {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
}

/// The native inventory does not pause the world, but opening it releases the
/// movement keys. Keep simulation running with neutral input while it is open.
pub fn controls_for_inventory(controls: Controls, inventory_open: bool) -> Controls {
    if inventory_open {
        Controls::default()
    } else {
        controls
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MouseDeltaAccumulator {
    residual_x: f64,
    residual_y: f64,
}

impl MouseDeltaAccumulator {
    /// Converts raw motion to native integer-pixel deltas without losing fractions
    /// that arrive across separate window-system events.
    pub fn take_integer_deltas(&mut self, dx: f64, dy: f64) -> (i32, i32) {
        fn take(residual: &mut f64, delta: f64) -> i32 {
            if !delta.is_finite() {
                *residual = 0.0;
                return 0;
            }
            let total = *residual + delta;
            let whole = total.trunc();
            if whole >= f64::from(i32::MAX) {
                *residual = 0.0;
                i32::MAX
            } else if whole <= f64::from(i32::MIN) {
                *residual = 0.0;
                i32::MIN
            } else {
                *residual = total - whole;
                whole as i32
            }
        }
        (
            take(&mut self.residual_x, dx),
            take(&mut self.residual_y, dy),
        )
    }

    pub fn clear(&mut self) {
        self.residual_x = 0.0;
        self.residual_y = 0.0;
    }
}

pub fn apply_native_mouse_delta(yaw: f32, pitch: f32, dx: i32, dy: i32) -> (f32, f32) {
    let yaw = (f64::from(yaw) + f64::from(dx) * 0.15_f64) as f32;
    let pitch = (f64::from(pitch) + f64::from(dy) * 0.15_f64) as f32;
    (yaw, pitch.clamp(-90.0, 90.0))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickResult {
    pub previous: MotionState,
    pub current: MotionState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotionController {
    pub state: MotionState,
    previous: MotionState,
    accumulator: Duration,
    pub debug_flight: bool,
    flight_previous: [f64; 3],
    flight_current: [f64; 3],
}

impl MotionController {
    pub fn new(state: MotionState) -> Self {
        Self {
            state,
            previous: state,
            accumulator: Duration::ZERO,
            debug_flight: false,
            flight_previous: state.position,
            flight_current: state.position,
        }
    }

    /// Adds wall time without clamping or dropping it; callers drain every due tick.
    pub fn accumulate(&mut self, elapsed: Duration) {
        self.accumulator = self.accumulator.saturating_add(elapsed);
    }

    pub fn ticks_due(&self) -> u128 {
        self.accumulator.as_nanos() / TICK.as_nanos()
    }

    pub fn tick(&mut self, scene: &Scene, controls: Controls) -> Result<TickResult, ViewError> {
        debug_assert!(self.accumulator >= TICK);
        self.previous = self.state;
        // Native input signs are opposite the camera's screen-right axis:
        // left strafe is positive and points +X when yaw is zero.
        // Scale axes independently; dry_travel applies native diagonal normalization.
        let raw_strafe = u8::from(controls.left) as f32 - u8::from(controls.right) as f32;
        let raw_forward = u8::from(controls.forward) as f32 - u8::from(controls.backward) as f32;
        let mut strafe = raw_strafe * 0.98_f32;
        let mut forward = raw_forward * 0.98_f32;
        if controls.sneak {
            strafe *= 0.3_f32;
            forward *= 0.3_f32;
        }
        let query = collider_query(&self.state, self.state.velocity);
        let colliders = scene.colliders_in(query)?;
        let yaw_degrees = self.state.yaw_degrees;
        let travel_input = TravelInput {
            strafe,
            forward,
            yaw_degrees,
            jump: controls.jump,
            sneaking: controls.sneak,
        };
        let water_cells = scene.water_cells_in(self.state.bbox.expanded(1.0, 1.0, 1.0))?;
        if water_contact(&self.state, &water_cells).map_err(beta_view::ViewError::WaterMotion)? {
            let sneak_factor = if controls.sneak { 0.3_f32 } else { 1.0 };
            water_living_step(
                &mut self.state,
                TravelInput {
                    strafe: raw_strafe * sneak_factor,
                    forward: raw_forward * sneak_factor,
                    ..travel_input
                },
                &colliders,
                &water_cells,
            )
            .map_err(beta_view::ViewError::WaterMotion)?;
        } else {
            let surface = scene.surface_below(self.state.bbox)?;
            dry_travel(&mut self.state, travel_input, &colliders, surface);
        }
        scene.constrain_motion_state(&mut self.state);
        self.accumulator -= TICK;
        Ok(TickResult {
            previous: self.previous,
            current: self.state,
        })
    }

    pub fn begin_flight(&mut self, position: [f32; 3]) {
        self.sync_flight_position(position);
        self.debug_flight = true;
    }

    pub fn flight_tick(&mut self, position: [f32; 3]) {
        debug_assert!(self.accumulator >= TICK);
        self.previous = self.state;
        self.flight_previous = self.flight_current;
        self.flight_current = position.map(f64::from);
        self.accumulator -= TICK;
    }

    /// Synchronizes both render samples after a discontinuous camera change.
    pub fn sync_flight_position(&mut self, position: [f32; 3]) {
        let position = position.map(f64::from);
        self.flight_previous = position;
        self.flight_current = position;
    }

    pub fn interpolated_flight_position(&self) -> [f64; 3] {
        let alpha = self.interpolation();
        std::array::from_fn(|i| {
            self.flight_previous[i] * (1.0 - alpha) + self.flight_current[i] * alpha
        })
    }

    pub fn flight_position(&self) -> [f32; 3] {
        self.flight_current.map(|value| value as f32)
    }

    pub fn clear_clock(&mut self) {
        self.accumulator = Duration::ZERO;
    }

    pub fn interpolation(&self) -> f64 {
        (self.accumulator.as_secs_f64() / TICK.as_secs_f64()).clamp(0.0, 1.0)
    }

    pub fn interpolated_position(&self) -> [f64; 3] {
        let alpha = self.interpolation();
        std::array::from_fn(|i| {
            self.previous.position[i] * (1.0 - alpha) + self.state.position[i] * alpha
        })
    }

    pub fn reset(&mut self, safe_spawn: [f64; 3]) {
        self.state = MotionState::new(safe_spawn);
        self.previous = self.state;
        self.accumulator = Duration::ZERO;
        self.debug_flight = false;
        self.flight_previous = [safe_spawn[0], safe_spawn[1] + 1.62, safe_spawn[2]];
        self.flight_current = self.flight_previous;
    }

    pub fn land_from_flight(&mut self, eye: [f32; 3], yaw_degrees: f32) {
        let mut state = MotionState::new([
            f64::from(eye[0]),
            f64::from(eye[1]) - 1.62,
            f64::from(eye[2]),
        ]);
        state.yaw_degrees = yaw_degrees;
        self.state = state;
        self.previous = state;
        self.sync_flight_position(eye);
        self.debug_flight = false;
    }
}

pub fn collider_query(state: &MotionState, displacement: [f64; 3]) -> Aabb {
    state
        .bbox
        .swept(displacement[0], displacement[1], displacement[2])
        .expanded(QUERY_MARGIN, QUERY_MARGIN, QUERY_MARGIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flight_interpolation_is_frame_partition_independent_at_common_rates() {
        let mut fixed_tick_positions = Vec::new();
        for rate in [20_u128, 30, 60, 144] {
            let mut motion = MotionController::new(MotionState::new([0.0, 0.0, 0.0]));
            motion.begin_flight([0.0; 3]);
            let mut previous_nanos = 0_u128;
            let mut samples = Vec::new();
            for frame in 1..=rate {
                let frame_nanos = frame * 1_000_000_000 / rate;
                motion.accumulate(Duration::from_nanos((frame_nanos - previous_nanos) as u64));
                previous_nanos = frame_nanos;
                while motion.ticks_due() > 0 {
                    let mut p = motion.flight_position();
                    p[0] += 0.5;
                    motion.flight_tick(p);
                }
                samples.push(motion.interpolated_flight_position()[0]);
            }
            assert_eq!(motion.flight_position()[0], 10.0, "{rate} Hz fixed state");
            assert!(
                samples.windows(2).all(|pair| pair[1] >= pair[0]),
                "{rate} Hz: {samples:?}"
            );
            if rate > 20 {
                assert!(samples.iter().any(|sample| *sample > 0.0 && *sample < 10.0));
            }
            fixed_tick_positions.push(motion.flight_position());
        }
        assert!(
            fixed_tick_positions
                .windows(2)
                .all(|pair| pair[0] == pair[1])
        );
    }

    #[test]
    fn flight_transitions_reset_and_focus_sync_both_samples() {
        let mut motion = MotionController::new(MotionState::new([2.0, 3.0, 4.0]));
        motion.begin_flight([10.0, 20.0, 30.0]);
        assert_eq!(motion.interpolated_flight_position(), [10.0, 20.0, 30.0]);

        motion.accumulate(TICK);
        motion.flight_tick([10.5, 20.0, 30.0]);
        motion.accumulate(Duration::from_millis(25));
        assert_eq!(motion.interpolated_flight_position(), [10.25, 20.0, 30.0]);

        motion.sync_flight_position([11.0, 21.0, 31.0]);
        motion.clear_clock();
        assert_eq!(motion.interpolated_flight_position(), [11.0, 21.0, 31.0]);

        motion.land_from_flight([12.0, 22.0, 32.0], 45.0);
        assert!(!motion.debug_flight);
        assert_eq!(motion.interpolated_flight_position(), [12.0, 22.0, 32.0]);

        motion.begin_flight([13.0, 23.0, 33.0]);
        motion.reset([1.0, 2.0, 3.0]);
        assert_eq!(motion.interpolated_flight_position(), [1.0, 3.62, 3.0]);
        assert!(!motion.debug_flight);
    }

    fn test_mapping() -> beta_view::BlockMapping {
        beta_view::BlockMapping {
            key: beta_view::BlockStateKey {
                id: 1,
                metadata: 0,
                above_id: None,
            },
            render_type: 0,
            opaque: true,
            full_cube: true,
            alpha_opaque_faces: [true; 6],
            tiles: [1; 6],
            supported: true,
        }
    }

    fn test_scene(floor_depth: std::ops::Range<u8>, wall: bool, ceiling: bool) -> Scene {
        let mut data = beta_core::chunk::ChunkData::new();
        for x in 0..16 {
            for z in floor_depth.clone() {
                data.set(
                    x,
                    0,
                    z,
                    beta_core::chunk::BlockData {
                        id: 1,
                        ..Default::default()
                    },
                )
                .unwrap();
            }
        }
        if wall {
            for x in 0..16 {
                for y in 1..5 {
                    data.set(
                        x,
                        y,
                        5,
                        beta_core::chunk::BlockData {
                            id: 1,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
            }
        }
        if ceiling {
            for x in 7..10 {
                for z in 7..10 {
                    data.set(
                        x,
                        4,
                        z,
                        beta_core::chunk::BlockData {
                            id: 1,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
            }
        }
        Scene::new(
            beta_view::SceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![beta_view::SceneChunk { x: 0, z: 0, data }],
            vec![test_mapping()],
        )
        .unwrap()
    }

    fn water_test_scene() -> Scene {
        let mut data = beta_core::chunk::ChunkData::new();
        for x in 7..10 {
            for y in 1..4 {
                for z in 7..10 {
                    data.set(
                        x,
                        y,
                        z,
                        beta_core::chunk::BlockData {
                            id: 9,
                            metadata: 0,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
            }
        }
        Scene::new(
            beta_view::SceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![beta_view::SceneChunk { x: 0, z: 0, data }],
            vec![],
        )
        .unwrap()
    }

    fn ticks(motion: &mut MotionController, scene: &Scene, input: Controls, count: usize) {
        for _ in 0..count {
            motion.accumulate(TICK);
            motion.tick(scene, input).unwrap();
        }
    }

    #[test]
    fn open_inventory_keeps_gravity_and_momentum_without_movement_input() {
        let scene = test_scene(0..16, false, false);
        let mut state = MotionState::new([8.5, 6.0, 8.5]);
        state.velocity = [0.1, -0.2, 0.0];
        let mut inventory = MotionController::new(state);
        let mut neutral = MotionController::new(state);
        let held = Controls {
            forward: true,
            jump: true,
            ..Controls::default()
        };
        assert_eq!(controls_for_inventory(held, false), held);
        let gated = controls_for_inventory(held, true);
        assert_eq!(gated, Controls::default());
        ticks(&mut inventory, &scene, gated, 4);
        ticks(&mut neutral, &scene, Controls::default(), 4);
        assert_eq!(inventory.state, neutral.state);
        assert!(inventory.state.position[1] < state.position[1]);
        assert!(inventory.state.position[0] > state.position[0]);
    }

    #[test]
    fn wall_floor_jump_landing_and_ceiling_collisions_are_bounded() {
        let scene = test_scene(0..16, true, true);
        let mut walking = MotionController::new(MotionState::new([8.5, 1.0, 2.5]));
        walking.state.on_ground = true;
        ticks(
            &mut walking,
            &scene,
            Controls {
                forward: true,
                ..Controls::default()
            },
            100,
        );
        assert!(walking.state.position[2] <= 4.700_001);
        assert!(walking.state.on_ground);

        let mut jumping = MotionController::new(MotionState::new([8.5, 1.0, 8.5]));
        jumping.state.on_ground = true;
        ticks(
            &mut jumping,
            &scene,
            Controls {
                jump: true,
                ..Controls::default()
            },
            1,
        );
        let jumped = jumping.state.position[1] > 1.0;
        ticks(&mut jumping, &scene, Controls::default(), 120);
        assert!(jumped);
        assert!(jumping.state.on_ground);
        assert!((jumping.state.position[1] - 1.0).abs() < 1e-9);
        assert!(jumping.state.bbox.max[1] <= 4.0);
    }

    #[test]
    fn water_controller_uses_native_drag_and_fixed_tick_partitioning() {
        fn simulate(rate: u128) -> MotionState {
            let scene = water_test_scene();
            let mut motion = MotionController::new(MotionState::new([8.5, 1.5, 8.5]));
            let mut previous_nanos = 0_u128;
            for frame in 1..=rate {
                let frame_nanos = frame * 1_000_000_000 / rate;
                motion.accumulate(Duration::from_nanos((frame_nanos - previous_nanos) as u64));
                previous_nanos = frame_nanos;
                while motion.ticks_due() > 0 {
                    motion
                        .tick(
                            &scene,
                            Controls {
                                forward: true,
                                ..Controls::default()
                            },
                        )
                        .unwrap();
                }
            }
            motion.state
        }

        let states: Vec<_> = [20_u128, 30, 60, 144].into_iter().map(simulate).collect();
        assert!(states.windows(2).all(|pair| pair[0] == pair[1]));

        let mut dry = MotionController::new(MotionState::new([8.5, 1.5, 8.5]));
        let dry_scene = test_scene(0..0, false, false);
        ticks(
            &mut dry,
            &dry_scene,
            Controls {
                forward: true,
                ..Controls::default()
            },
            20,
        );
        assert!(states[0].position[2] > 8.5);
        assert!(states[0].velocity[1] > dry.state.velocity[1]);
    }

    #[test]
    fn sneaking_stops_at_platform_edge_and_scene_bounds_clamp_player() {
        let scene = test_scene(1..2, false, false);
        let mut motion = MotionController::new(MotionState::new([8.5, 1.0, 1.5]));
        motion.state.on_ground = true;
        ticks(
            &mut motion,
            &scene,
            Controls {
                forward: true,
                sneak: true,
                ..Controls::default()
            },
            60,
        );
        assert!(motion.state.position[2] <= 2.301);

        motion.state = MotionState::new([-20.0, -4.0, 400.0]);
        scene.constrain_motion_state(&mut motion.state);
        assert!(motion.state.bbox.min[0] >= 0.0);
        assert!(motion.state.bbox.max[0] <= 16.0);
        assert!(motion.state.bbox.min[2] >= 0.0);
        assert!(motion.state.bbox.max[2] <= 16.0);
        assert!(motion.state.bbox.min[1] >= 0.0);
        assert_eq!(
            motion.state.position[0],
            (motion.state.bbox.min[0] + motion.state.bbox.max[0]) / 2.0
        );
        assert_eq!(
            motion.state.position[1],
            motion.state.bbox.min[1] + f64::from(motion.state.y_offset)
                - f64::from(motion.state.y_size)
        );
        assert_eq!(
            motion.state.position[2],
            (motion.state.bbox.min[2] + motion.state.bbox.max[2]) / 2.0
        );
        assert_eq!(motion.state.velocity, [0.0; 3]);
        assert!(
            motion.state.collided_horizontally
                && motion.state.collided_vertically
                && motion.state.collided
        );
    }

    #[test]
    fn controller_heading_scalars_match_native_preprocessed_inputs() {
        let scene = test_scene(0..16, false, false);
        for (controls, strafe, forward, sneaking) in [
            (
                Controls {
                    forward: true,
                    left: true,
                    ..Controls::default()
                },
                0.98_f32,
                0.98_f32,
                false,
            ),
            (
                Controls {
                    forward: true,
                    ..Controls::default()
                },
                0.0_f32,
                0.98_f32,
                false,
            ),
            (
                Controls {
                    left: true,
                    ..Controls::default()
                },
                0.98_f32,
                0.0_f32,
                false,
            ),
            (
                Controls {
                    forward: true,
                    left: true,
                    sneak: true,
                    ..Controls::default()
                },
                0.98_f32 * 0.3_f32,
                0.98_f32 * 0.3_f32,
                true,
            ),
        ] {
            let initial = MotionState {
                on_ground: true,
                ..MotionState::new([8.5, 1.0, 8.5])
            };
            let mut actual = MotionController::new(initial);
            actual.accumulate(TICK);
            actual.tick(&scene, controls).unwrap();

            let mut expected = initial;
            let candidates = scene
                .colliders_in(collider_query(&expected, expected.velocity))
                .unwrap();
            let surface = scene.surface_below(expected.bbox).unwrap();
            let yaw_degrees = expected.yaw_degrees;
            dry_travel(
                &mut expected,
                TravelInput {
                    strafe,
                    forward,
                    yaw_degrees,
                    jump: false,
                    sneaking,
                },
                &candidates,
                surface,
            );
            scene.constrain_motion_state(&mut expected);
            assert_eq!(actual.state, expected, "input {controls:?}");
        }
    }

    #[test]
    fn mouse_accumulator_preserves_fractional_pixels_and_sign_cancellation() {
        let mut accumulator = MouseDeltaAccumulator::default();
        let mut yaw = 0.0_f32;
        let mut pitch = 0.0_f32;
        let mut deltas = Vec::new();
        for _ in 0..4 {
            let (dx, dy) = accumulator.take_integer_deltas(0.25, 0.25);
            deltas.push((dx, dy));
            (yaw, pitch) = apply_native_mouse_delta(yaw, pitch, dx, dy);
        }
        assert_eq!(deltas, [(0, 0), (0, 0), (0, 0), (1, 1)]);
        assert_eq!((yaw, pitch), apply_native_mouse_delta(0.0, 0.0, 1, 1));

        let mut accumulator = MouseDeltaAccumulator::default();
        assert_eq!(accumulator.take_integer_deltas(0.75, -0.75), (0, 0));
        assert_eq!(accumulator.take_integer_deltas(-0.75, 0.75), (0, 0));
        assert_eq!(accumulator.take_integer_deltas(0.25, -0.25), (0, 0));
        assert_eq!(accumulator.take_integer_deltas(-0.25, 0.25), (0, 0));
        accumulator.take_integer_deltas(0.75, -0.75);
        accumulator.clear();
        assert_eq!(accumulator.take_integer_deltas(0.25, 0.25), (0, 0));
    }

    #[test]
    fn accumulator_retains_stalls_and_interpolation_fraction() {
        let scene = Scene::new(
            beta_view::SceneBounds::new(0, 0, 1, 1).unwrap(),
            vec![],
            vec![],
        )
        .unwrap();
        let mut motion = MotionController::new(MotionState::new([8.5, 80.0, 8.5]));
        motion.accumulate(Duration::from_millis(375));
        assert_eq!(motion.ticks_due(), 7);
        let controls = Controls::default();
        for _ in 0..7 {
            motion.tick(&scene, controls).unwrap();
        }
        assert_eq!(motion.accumulator, Duration::from_millis(25));
        assert!((motion.interpolation() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn frame_partition_and_stall_replays_have_equal_core_state() {
        fn replay(frames: &[Duration]) -> MotionState {
            let scene = Scene::new(
                beta_view::SceneBounds::new(0, 0, 1, 1).unwrap(),
                vec![],
                vec![],
            )
            .unwrap();
            let mut motion = MotionController::new(MotionState::new([8.5, 80.0, 8.5]));
            let mut tick_index = 0;
            for frame in frames {
                motion.accumulate(*frame);
                while motion.ticks_due() > 0 {
                    let controls = match tick_index / 20 {
                        0 => Controls {
                            forward: true,
                            ..Controls::default()
                        },
                        1 => Controls {
                            left: true,
                            forward: true,
                            ..Controls::default()
                        },
                        2 => Controls {
                            jump: tick_index == 40,
                            ..Controls::default()
                        },
                        _ => Controls::default(),
                    };
                    motion.tick(&scene, controls).unwrap();
                    tick_index += 1;
                }
            }
            motion.state
        }
        fn partition(hz: u64, ticks: usize) -> Vec<Duration> {
            let frames = ticks * hz as usize / 20;
            let total_ns = ticks as u128 * TICK.as_nanos();
            let base_ns = total_ns / frames as u128;
            let extra_ns = total_ns % frames as u128;
            (0..frames)
                .map(|i| {
                    Duration::from_nanos((base_ns + u128::from((i as u128) < extra_ns)) as u64)
                })
                .collect()
        }
        let expected = replay(&partition(20, 120));
        for hz in [30, 60, 144] {
            assert_eq!(replay(&partition(hz, 120)), expected);
        }
        let mut with_stall = vec![Duration::from_millis(50); 20];
        with_stall[4] += Duration::from_millis(900);
        assert_eq!(
            replay(&with_stall),
            replay(&vec![Duration::from_millis(50); 38])
        );
    }
}
