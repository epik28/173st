//! Deterministic fixed-step walking controller, independent of winit and wgpu.

use std::time::Duration;

use beta_core::{
    geometry::Aabb,
    motion::{MotionState, TravelInput, dry_travel},
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
}

impl MotionController {
    pub fn new(state: MotionState) -> Self {
        Self {
            state,
            previous: state,
            accumulator: Duration::ZERO,
            debug_flight: false,
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
        let mut strafe = u8::from(controls.left) as f32 - u8::from(controls.right) as f32;
        let mut forward = u8::from(controls.forward) as f32 - u8::from(controls.backward) as f32;
        let length = (strafe * strafe + forward * forward).sqrt();
        if length > 0.0 {
            strafe = strafe / length * 0.98_f32;
            forward = forward / length * 0.98_f32;
        }
        if controls.sneak {
            strafe *= 0.3_f32;
            forward *= 0.3_f32;
        }
        let query = collider_query(&self.state, self.state.velocity);
        let colliders = scene.colliders_in(query)?;
        let yaw_degrees = self.state.yaw_degrees;
        let surface = scene.surface_below(self.state.bbox)?;
        dry_travel(
            &mut self.state,
            TravelInput {
                strafe,
                forward,
                yaw_degrees,
                jump: controls.jump,
                sneaking: controls.sneak,
            },
            &colliders,
            surface,
        );
        scene.constrain_motion_state(&mut self.state);
        self.accumulator -= TICK;
        Ok(TickResult {
            previous: self.previous,
            current: self.state,
        })
    }

    pub fn flight_tick(&mut self) {
        debug_assert!(self.accumulator >= TICK);
        self.previous = self.state;
        self.accumulator -= TICK;
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

    fn ticks(motion: &mut MotionController, scene: &Scene, input: Controls, count: usize) {
        for _ in 0..count {
            motion.accumulate(TICK);
            motion.tick(scene, input).unwrap();
        }
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
