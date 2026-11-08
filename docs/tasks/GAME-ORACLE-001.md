# GAME-ORACLE-001 — native geometry and agent-facing differential CLI

First reusable headless game-method scenario: original client `eq` (AABB),
`bt` (Vec3), `vf` (ray result), and their original MathHelper dependency.
Pinned client JAR SHA-256:
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
This verifies geometry, not player simulation or a loaded native World.

## Shared protocol (frozen before parallel implementation)

ASCII TSV, final LF required, no comments or blank rows. A row is
`case_id<TAB>operation<TAB>arg...<LF>`. IDs match `[A-Za-z0-9_.:-]{1,96}` and
are unique. Numeric args are lowercase 16-digit raw IEEE f64 hex. All args must
be finite with absolute value <= 30,000,000. Signed zero and degenerate/inverted
boxes are permitted. At most 100,000 rows / 32 MiB. Malformed input is a process
error, stderr diagnostic, nonzero exit, no partial stdout for either probe.

Output one ordered `case_id<TAB>value<LF>` per input. Values: hex scalar,
comma-separated raw-hex coordinates (3 or 6), boolean `0|1`, optional vector
`none|x,y,z`, ray `none|face,x,y,z`. Faces are native 0..5. All fields match
exactly by raw bits, no epsilon, normalization or sorting. Native constructors'
zero canonicalization must be preserved only where the original does so.

| Operations | Arguments | Output | Original method |
| --- | --- | --- | --- |
| box_new | min xyz, max xyz (6) | box (6) | eq.a(DDDDDD)Leq; |
| box_sweep / box_expand / box_offset | box (6), xyz (3) | box (6) | eq.a / b / c(DDD)Leq; |
| box_clip_x / y / z | obstacle box (6), moving box (6), delta (1) | scalar | eq.a / b / c(Leq;D)D |
| box_intersects | box (6), other (6) | boolean | eq.a(Leq;)Z |
| box_contains | box (6), point (3) | boolean | eq.a(Lbt;)Z |
| box_average_edge | box (6) | scalar | eq.c()D |
| box_ray | box (6), start (3), end (3) | ray | eq.a(Lbt;Lbt;)Lvf; |
| vec_new | xyz (3) | vector | bt.a(DDD)Lbt; |
| vec_to | self (3), other (3) | vector (other-self) | bt.a(Lbt;)Lbt; |
| vec_cross | self (3), other (3) | vector | bt.b(Lbt;)Lbt; |
| vec_add | self (3), delta (3) | vector | bt.c(DDD)Lbt; |
| vec_normalize | xyz (3) | vector | bt.c()Lbt; |
| vec_length | xyz (3) | scalar | bt.d()D |
| vec_distance / vec_distance_sq | self (3), other (3) | scalar | bt.c / d(Lbt;)D |
| vec_intermediate_x / y / z | start (3), end (3), plane (1) | optional vector | bt.a / b / c(Lbt;D)Lbt; |

## Rust API (beta_core::geometry)

`Axis::{X,Y,Z}`. `Vec3 { pub x:f64, pub y:f64, pub z:f64 }`, Copy.
Methods: `new(x,y,z)`, `vector_to(self,other)`, `cross(self,other)`,
`add_xyz(self,x,y,z)`, `normalize(self)`, `length(self)`,
`distance(self,other)`, `distance_squared(self,other)`,
`intermediate(self,end,axis,plane)->Option<Vec3>`.
Use original f32 sqrt narrowing and near-zero thresholds, no fused math.

`Aabb { pub min:[f64;3], pub max:[f64;3] }`, Copy. `new(min,max)` preserves
coordinates exactly. Methods: `swept(self,x,y,z)`, `expanded(self,x,y,z)`,
`offset(self,x,y,z)`, `clip(self,moving:Self,axis,delta)->f64`,
`intersects(self,other:Self)`, `contains(self,point:Vec3)`,
`average_edge(self)`, `ray_intersect(self,start:Vec3,end:Vec3)->Option<RayHit>`.
`RayHit { pub face:u8, pub position:Vec3 }`. No external dependencies.

## Parallel ownership and acceptance

- C01: GeometryOracle.java, geometry_corpus.py, fixtures/geometry/* and
  test_geometry_corpus.py. Direct native reflection calls, resolved method
  signatures, actual invocation counts, JAR/class/code-source hashes; two
  independent identical runs. At least 5,000 cases, every operation exercised,
  touching/overlap/gaps/negative coordinates/zero/inverted boxes, every ray face,
  edge/corner ties and start-inside, threshold neighbors and non-axis rays.
  Fixtures contain generated input and native output, no game binaries/source.
- A01: geometry.rs and lib.rs registration only. Implement frozen API using
  original bytecode research (not a reference project's copied implementation).
  Unit tests include exact contact, eps thresholds, signed-zero and ray ties.
- A02: geometry_probe.rs only. Strict atomic protocol; synthetic protocol tests
  and whole C01 fixture exact-bit regression when C01 is integrated.
- C02: geometry_run.py, geometry_review.py, test_geometry_runner.py. Add suite
  `geometry` to main dispatch only at coordinator integration. Runner executes
  live native oracle twice and Rust probe, hashes sources/artifacts, validates
  native sidecar audit and fixed expected output. Saved review checks process
  status/log hashes, native JAR/classes/code source, Java compiled adapter, Rust
  executable and outputs before any protocol reruns. Negative cases: wrong JAR,
  timeout, missing probe, malformed protocol, changed output/source/binary/audit.
- C03: integrate exact C01/C02/A01/A02 commits, workspace check, clean strict
  geometry + saved protocol/mutation review, prior accepted strict suites.
  Agent CLI documents `list`, live suite, saved verification and custom input.
  A custom-input command uses the same parsers/oracle/probe and does not rewrite
  committed fixtures. It reports synthetic custom cases separately.

Custom CLI: `python3 tools/parity/geometry_run.py --input FILE --strict`.
Normal `python3 tools/parity/run.py geometry --strict` uses pinned fixtures.
No new MCP/server/dependency is needed for agents to invoke a local CLI.
