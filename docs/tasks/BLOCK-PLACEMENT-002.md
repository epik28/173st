# BLOCK-PLACEMENT-002 — native placement faces and lighting refresh

## User report

On 2026-11-09 the viewer title reported `unsupported 31 at (-13,71,-4)` while
aiming at a block face. The user reports that some block sides reject placement
even when another side of the same block can be used. The same screenshot shows
a near-black region under a stone block; the user reports that placing a block
above can sometimes break the lighting. This is a report, not yet a controlled
reproduction or proof that both symptoms share one cause.

![Placement and dark-region report](../bug-reports/block-placement-unsupported-31.png)

Screenshot SHA-256:
`ec49f04966f318ac28ea4129278a8cf145c7b2b5c1cb82560cb607d7a0ef37c6`.

## C01 — establish native placement contract (Codex)

- Pin the Rust build, world/scene inputs, target coordinates, hit face, selected
  item, camera, and edit sequence for each reproduction.
- In disposable original Beta 1.7.3 worlds, capture placement attempts against
  each relevant side and record accepted/rejected result, resulting block ID and
  metadata, inventory delta, neighbor state, and native light/height-map deltas.
- Include repeat placement, placement above/below, and chunk-edge cases. Keep
  unsupported block shapes separate from ordinary cube faces.
- Determine whether the reported `unsupported 31` is a ray-hit classification,
  a placement rule, or an unsupported target shape. Do not infer native rules
  from the viewer counter alone.

## A01 — implement only verified behavior (agy)

After C01, support the verified placement faces/blocks and reject only cases the
native contract rejects or that remain explicitly out of scope. Make edit,
light/height-map state, dirty chunks, and rebuilt meshes update consistently and
atomically. Add fixture-driven tests for every captured accept/reject and light
delta. Do not hide the issue by globally brightening the scene or swallowing
`unsupported` results.

## C02 — acceptance (Codex)

Compare Rust with the pinned native action and state corpus, exercise repeated
edits and chunk edges, and confirm visual refresh. Keep placement compatibility
claims limited to captured cases; the separate general shadow issue remains
tracked in [LIGHT-UPDATE-002](LIGHT-UPDATE-002.md).

### C01 evidence boundary (2026-11-10)

The controlled native action corpus covers 16 cases in fresh byte-array chunks.
A separate 14-ray corpus for ID 31 metadata 0 records off-center and boundary
hits/misses, face IDs, and raw hit-position words. The original registry and
bytecode identify it as tall grass with local bounds X/Z 0.1..0.9 and Y 0..0.8
(f32-converted values are pinned); it is not a full-cube or crossed-plane ray
shape. The action corpus also records six ordinary stone faces, repeat
placement, an occupied destination, the east chunk edge, and the y=127 build
limit. This is enough to start the bounded Rust follow-up; other plant states,
all item orientations, inventory/UI integration, and general lighting behavior
remain out of scope. Light values in the minimal action worlds are zero, so the
reported shadow/lighting refresh remains an independent unresolved issue under
LIGHT-UPDATE-002.
