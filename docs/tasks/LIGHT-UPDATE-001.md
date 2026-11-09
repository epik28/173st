# LIGHT-UPDATE-001 — recalculate saved light after block edits

## Goal

After an in-memory full-cube break/place in the viewer's disposable scene,
update block/sky light before rebuilding the affected meshes. The source world
remains read-only. This targets the dark faces reported after edits; it does not
claim general lighting, ambient occlusion, dynamic lights, or pixel parity.

## Contract

- Use the original Beta 1.7.3 update path as the behavioral reference. The
  existing `LIGHT-001` covers scalar lookup/arithmetic only; do not infer update
  propagation from those scalar formulas.
- Only changes through the native-observed six-cardinal-neighbor update path are
  in scope. Do not add unobserved diagonal shortcuts, fluids, arbitrary direct
  light edits, new chunks, weather/time ticks, or AO. Any emission transition is
  included only if the exact operation is reachable through the viewer edit API
  and C01 captures it; otherwise it is explicitly deferred.
- Keep edits and relighting in memory in the already validated viewer scene.
  Never update the source-world pin or write to `level.dat`/region files.
- The relight API must be bounded by C01's observed fixture domains and queue
  limit. It returns exact changed block/sky nibble values, sorted dirty chunks,
  and an explicit list of unloaded boundary neighbors; unloaded chunks are not
  treated as air or mutated. Do not silently clamp propagation at a bound.
- `Viewer::interact` must treat the edit as one transaction across `BlockScene`,
  duplicated `scene_chunks`, CPU mesh/fluid caches, and buffers selected for the
  next frame. It must not call the existing mutating edit method and then relight
  as a separate fallible step. Build candidate block/light state and candidate
  meshes first; publish them only after every fallible CPU operation succeeds.
  On failure, both block stores, metadata, light arrays, revision, dirty state
  and displayed meshes remain unchanged.
- Preserve block IDs, metadata, opacity/transparency rules, and boundary
  behavior observed in C01. The dirty set is deterministic: include every chunk
  whose light changed, the edited chunk, and a conservative loaded neighbor
  superset required by opaque-face visibility and the existing fluid-corner
  sampling rule. Sort it and test the exact set; do not rely on an undocumented
  mesh dependency graph.
- Restrict implementation to native-observed block update cases. Full world
  generation, time/weather simulation, entities, fluid ticks and save writes
  remain outside this package.

## Work cards

### LIGHT-UPDATE-001-C01 — native edit and light-update oracle (Codex)

Build `tools/oracle/light_update_audit.py` and a direct original-client oracle
over synthetic in-memory chunks only. Commit `tests/fixtures/light_update/`
with an audit manifest and TSV whose rows define: scenario and operation;
chunk/cell coordinates; exact old/new block IDs and metadata; the complete
initial block/opacity/emission and block/sky nibble state for the bounded native
domain; the domain's bounds and unloaded edges; an ordered complete list of
queried cells; final block IDs and raw nibbles for every query; and combined-light
queries. Include opaque removal/placement, an opaque roof/sky boundary, and a
chunk-edge edit. Add an emissive case only if it exercises the same reachable
viewer edit API; otherwise record the exclusion. Pin JAR, class, source, input
and output hashes and verify two independent captures. Capture callback order
only if directly instrumented; never infer it from uninstrumented internal
calls. Test malformed/mutated evidence and wrong JAR. No user save, filesystem
world mutation, or viewer Rust edits.

### LIGHT-UPDATE-001-A01 — in-memory scene relighting (agy)

Implement a bounded `BlockScene` relight operation for the edit operations and
block classes covered by C01. Its frozen signature and limits must be recorded
in the C01 handoff before A01 starts. `Viewer::interact` prepares the edited
`BlockScene`, synchronized `scene_chunks`, relit light arrays, dirty meshes and
fluid cache as a candidate, then publishes them together only after successful
CPU preparation. Match every fixture query's raw block/sky nibble and
combined-light value; return the deterministic dirty chunk union and explicit
unloaded boundaries. Keep the update API independent of disk I/O. Add
fixture-driven tests for every C01 row, unloaded-edge behavior, deterministic
queue limits, dirty-set membership, mesh visibility of final light, and atomic
failure (both block stores, metadata, light arrays, revision, dirty set and
displayed meshes unchanged). Do not edit native oracle/fixtures, shared docs, or
user-world guards.

### LIGHT-UPDATE-001-C02 — acceptance (Codex)

Review the A01 snapshot against native evidence. Run
`python3 tools/oracle/light_update_audit.py --jar ../.local/minecraft/client-b1.7.3.jar`,
the fixture-driven `cargo test --locked --offline -p beta-core` test, and
`python3 tools/check.py`. Verify the local source-world manifest and source files
are unchanged, then record unsupported propagation cases. Update progress,
roadmap, estimate and handoffs.

## Acceptance boundary

The package covers only the fixture's native-observed light changes after
audited in-memory full-cube edits in loaded chunks. It does not add survival
mining, light-time updates, weather, emission updates unreachable through the
viewer edit API, direct diagonal shortcuts, fluid light updates, new chunk
loading, persistence, or a general-purpose native world simulator.
