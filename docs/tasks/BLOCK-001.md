# BLOCK-001 — native block targeting and creative interaction

## Goal and boundary

Add the first block interaction slice to the Beta viewer: trace the player's
view ray, identify the nearest block and hit face using Beta 1.7.3 rules, then
break a supported full block or place one supported full block from a test
stack. Rebuild the affected mesh so the change is visible. This is a local
creative-mode sandbox; it does not save to the user's world.

Out of scope: survival mining duration/tool strength/drops, inventory UI and
hotbar selection, entities, networking, lighting propagation, fluids, special
block placement/orientation, and pixel-perfect GPU parity. Until orientation
support is contracted, placement accepts only metadata-zero full blocks whose
native placement behavior is verified.

## Native contract

The pinned Beta 1.7.3 client JAR is identified in `config/local.toml`; the
surveyed JAR SHA-256 is
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
Native world ray tracing is `fd.a(bt, bt)` and its overloads, returning `vf`:
hit type, block coordinates, side, exact hit point, and entity. Native block
ray shape is dispatched through `uu.a(fd,x,y,z,start,end)`. C01 must verify
these mappings and class hashes against the pinned JAR before treating output
as an oracle.

The corpus must include each axis-aligned face, a miss, endpoint just before
and exactly at a face, a negative-coordinate voxel crossing, an edge/corner
tie, and a non-cube block shape (slab or ladder). Include the native
stop-on-liquid switch as paired cases if it can be invoked without changing
world state. Report hit kind, block xyz, side, and raw IEEE-754 words for hit
point coordinates. Keep native live and synthetic malformed/protocol cases
separate. Then include at least one verified creative break and one verified
placement case with before/after block ID+metadata in the local neighborhood;
placement must verify stack count/damage. The original controller's `ob.b()`
reach is 5.0 blocks and `pj` inherits it; verify against pinned bytecode and
include just-before/at/after reach cases. Placement follows
`ob.a(gs,fd,iz,x,y,z,side)` through stack/item dispatch; use the changed native
ID+metadata as the orientation result, not a guessed formula.

The oracle operates only on disposable in-memory scenes or run-specific copies.
It must not write to the PrismLauncher instance, source JAR, or source assets.

## Rust/probe contract

Add a public ray-hit result carrying block position, face, hit point, and hit
kind, and an edit API that is bounded to loaded scene data. A miss and an
unsupported target must be explicit. Breaking sets the selected block to air;
placing targets the adjacent cell indicated by the native hit side, succeeds
only when that cell is air and loaded, and updates ID/metadata atomically.
Reject invalid coordinates/IDs/metadata without partial mutation. Neighboring
mesh geometry must update after either operation. Do not mutate or persist the
loaded source save.

The line-oriented TSV probe must be bounded, deterministic, reject malformed
and over-limit input, and write machine-readable output only to stdout. Its
example target and invocation are recorded in C02. Unit tests cover exact
faces, negative coordinates, ties, misses, range edges, unsupported shapes,
placement occupancy, unloaded neighbors, metadata validation, and mesh update.

## Acceptance

1. C01 records the exact original class/method provenance, JAR and class hashes,
   live inputs/outputs, scene construction, reach, and before/after hashes.
2. C02 strict runner validates schema, hashes, live/synthetic separation,
   timeout, malformed input, missing probe, and deliberate output mutation.
   Without the Rust probe, strict mode must fail or report not-implemented.
3. A01/A02 match every supported native case exactly under the agreed raw-bit
   policy. Unsupported shapes are explicitly classified and never silently
   treated as full cubes.
4. C03 runs the complete workspace checks and all previously accepted strict
   suites, then `block --strict`; launches the viewer and verifies break/place
   plus mesh refresh on a disposable fixed scene. Evidence records known limits.
5. The source world, JAR, textures, and resources retain their original hashes.

## Evidence limitations

This package establishes CPU targeting and a bounded local creative edit loop.
It does not establish survival gameplay, persistence, network play, lighting
updates, or full client equivalence.
