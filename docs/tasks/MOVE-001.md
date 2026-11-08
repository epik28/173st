# MOVE-001 — native entity collision and dry travel, fixed-step viewer

Base: accepted GAME-ORACLE-001, main `558a653`. No new dependencies. Native
client JAR SHA `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
Original classes `sn` (b53107a5bdd5ff92637582aaed4fd49366443a11cba96cef0a6ad719e197b7be),
`ls` (b70fbb1499cd1b37ef27dd5dec7637c7338439ef03fe13012dca93fb2326d078),
`eq`, `bt`, `in`, `uu` and original stone/ice slipperiness.

## Scope and independent native oracle

Call original `sn.b(DDD)V`, inherited `sn.a(FFF)V`, original
`ls.a_(FF)V` and protected jump `ls.R()V`. No rewritten Java physics and no
modified native class bytes. Fixture subclasses may override world collision
queries/block lookups/loaded-area checks, dry environment queries `ag/ah/p`,
walking effects `n()` and fall-damage callback `b(F)`; no motion method override.
Native World subclass may use Java Unsafe allocation to bypass I/O constructors;
initialize only fixture-owned fields/listeners. This is tooling only, Rust safe.
Record synthetic environment, native World constructors=0, inherited method
declaring classes, JAR/class/code-source hashes, actual primary invocation counts.
Capture native registry bootstrap stdout separately; TSV stdout stays atomic.

Static environment is an ordered list of axis-aligned boxes. Every native
collision query returns original `eq` boxes intersecting that queried region,
preserving input order (including support/step retry queries). Block lookup
returns the selected surface ID iff the unit voxel at (x,y,z) intersects any
fixture box, otherwise air. Surfaces: air=0, stone=1, ice=79. `ag`, `ah`, `p`
return false (water/lava/ladder excluded). Footsteps/health/damage/animation are
outside captured state; original numerical fall-distance update is retained.
Travel frame: set yaw and sneaking, invoke original jump only if requested and
currently on ground, then original `ls.a_`. Inputs are already filtered heading
values; player input preprocessing/full `ls.o()` and client loop are not claimed.

## Frozen Rust API (`beta_core::motion`)

```rust
pub struct MotionState {
    pub bbox: Aabb,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub y_offset: f32,      // sn.bf
    pub y_size: f32,        // sn.bo
    pub step_height: f32,   // sn.bp
    pub fall_distance: f32, // sn.bk
    pub yaw_degrees: f32,   // sn.aS
    pub on_ground: bool,    // sn.aX
    pub collided_horizontally: bool, // sn.aY
    pub collided_vertically: bool,   // sn.aZ
    pub collided: bool,     // sn.ba
    pub no_clip: bool,      // sn.bq
    pub in_web: bool,       // sn.bc
    pub sneaking: bool,     // native t()
    pub allow_slide: bool,  // sn.bd, true by default
}
pub enum Surface { Air, Stone, Ice }
pub struct TravelInput {
    pub strafe: f32, pub forward: f32, pub yaw_degrees: f32,
    pub jump: bool, pub sneaking: bool,
}
pub fn move_entity(state: &mut MotionState, delta: [f64;3], colliders: &[Aabb]);
pub fn dry_travel(state: &mut MotionState, input: TravelInput,
                  colliders: &[Aabb], surface: Surface);
```

All structs Copy/Clone/Debug/PartialEq. `MotionState::new([x,y,z])`: width .6f32,
height 1.8f32, bottom y, y_offset/y_size/fall/yaw zero, step_height .5f32,
velocity zero, flags false except allow_slide=true. AABB width half calculated
in f32 before f64 widening, matching native dimension/position setup.
Caller supplies complete obstacle candidates for every sweep/support/step query.
Preserve original f32/f64 operation order and Java casts, no fused math.

Important bytecode details (check original, do not trust pseudocode): Y→X→Z;
step eligibility is initialGround OR downward collision; retry gate also uses
initialGround&&sneaking OR damped y_size<.05f. BASE wins equal horizontal
distance when comparing base vs step. `bd=false` zeros current deltas after
axis clips; final velocity clearing is performed by original request comparison.
No-clip early return preserves flags/velocity and skips y_size damping. Web
scaling/reset, two-axis sneak support, y_size damping/fractional step adjustment,
position recomputation, fall distance, air/ground/ice acceleration and friction,
gravity and jump must have native evidence. No global physics framework.

## Frozen TSV protocol

One ASCII scenario per LF-terminated line, no comments/blank lines:
`case_id<TAB>mode<TAB>initial<TAB>boxes<TAB>surface_id<TAB>frames<LF>`.
IDs `[A-Za-z0-9_.:-]{1,96}`, unique. Mode `move|travel`.

State CSV has 18 components: position xyz, velocity xyz, bbox min xyz, bbox max
xyz (12 lowercase raw f64 hex16); y_offset, y_size, step_height, fall_distance,
yaw_degrees (5 lowercase raw f32 hex8); unsigned decimal flags 0..255.
Flag bits: 0 ground, 1 h-collision, 2 v-collision, 3 collided, 4 no_clip,
5 web, 6 sneak, 7 allow_slide. Output uses the identical state serialization.

`boxes`: `-` for none, otherwise colon-separated boxes, each six comma-separated
raw f64 hex16 in min xyz/max xyz order. At most 128 ordered boxes; finite,
positive extents. Surface ID decimal 0,1,79. Frames separated by semicolon:
move frame `dx16,dy16,dz16`; travel frame `strafe8,forward8,yaw8,jump,sneak`
with booleans 0/1. At most 256 frames/scenario, 4096 scenarios, 100000 total
output steps and 16 MiB input. No partial stdout on any protocol error.

Input finite-domain limits: f64 coordinates abs<=30,000,000; body box each
positive extent<=4; velocity and move deltas abs<=32; y_offset abs<=2;
y_size in [0,2]; step_height [0,1]; fall_distance [0,256]; yaw abs<=36000;
heading inputs abs<=1. Validate all components before running/outputting.
Output ordered `case_id<TAB>frame_index<TAB>state_csv<LF>`, zero-based decimal
frame index. Compare every field by exact bits, zero tolerances. Output state
may leave the initial input bounds as a result of legitimate simulation.

## Parallel cards and acceptance

- C01: MovementOracle.java (native fixture subclasses), movement_corpus.py,
  tests/fixtures/movement/*, test_movement_corpus.py. At least 5000 native states:
  collisions/contact/corners/negative coords/ceiling/fall/web/noClip/allow_slide,
  sneak ledges, step enabled/disabled/ties, support gaps, ordered obstacles;
  dry sequences with stop/start/yaw/diagonal/jump/landing and stone/ice/air.
  Two independent identical runs and sidecar audit. Pin direct native method and
  field mapping; clearly separate primary move/travel/jump counts.
- A01: motion.rs, lib.rs registration, own agy status/handoff. Full pure API,
  meaningful synthetic boundary tests; no native parity until C01/probe exist.
- A02: movement_probe.rs, own agy status/handoff; atomic bounded protocol, all
  C01 states exact, malformed/limit regressions, fix concrete native differences.
- C02: movement_run.py, movement_review.py, test_movement_runner.py; main
  dispatch `movement`, custom `--input`. Reuse general process/evidence helpers
  where useful; retain source/build inputs, classpath/code source/native audit,
  original and compiled adapters, Rust executable, logs/version/hash/revision.
  Missing adapter/probe/JAR is error or not_implemented, never pass. Saved review
  rejects changed binaries/audit/logs/commands/sources and malformed outputs.
- C04 (explicit Rust viewer delegation): beta-view scene collision accessors,
  beta-viewer controller.rs/lib.rs/main.rs and tests. Bounded visible opaque
  full-cube collisions; unsupported shapes remain explicitly excluded. Walking
  default, Space jump, Shift sneak, F toggles debug flight, R resets safe spawn.
  Fixed 20Hz ticks with Duration accumulator; retain backlog, rendering interpolates
  previous/current positions. Focus loss clears input and pauses clock. Tests
  replay same tick-indexed inputs under 20/30/60/144Hz frame partitions/stalls
  with identical final state, floor/wall/ceiling/jump/landing/sneak and bounds.
  Correct camera handedness: at yaw0(+Z), native left strafe is +X; screen right
  is -X. Use yaw-derived right vector at vertical pitch. Mouse default sensitivity
  follows sn.d(FF) yaw/pitch update (f64 *.15 then f32, pitch clamps ±90deg),
  with backend/device differences documented. Viewer applies heading preprocessing
  .98f and sneak .3f; core consumes already-filtered inputs. No dynamic save writes.
- C03: review exact commits, full workspace checks, clean native movement + saved
  protocol/mutation evidence, previous strict suites and viewer startup/screenshot.
  No full client tick/fluids/ladder/health/animation/AO parity claims. Update
  progress estimate only for implemented verified capability; detailed user
  report/GitHub push at requested cumulative 5–10pp interval.
