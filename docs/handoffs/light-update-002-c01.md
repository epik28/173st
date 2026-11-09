# LIGHT-UPDATE-002 C01 material registry evidence

This handoff records the native material portion of C01. It does not establish
the reported screenshot's reproduction or the complete cause of its shadow.

## Source and capture

- Base revision: `468630060608e52d2cc750755923b53483ff72a1` on
  `agents/light-update-002`.
- Original client JAR: `../.local/minecraft/client-b1.7.3.jar`, SHA-256
  `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
- The Java adapter loads `uu` from that exact JAR and reads all 256 entries of
  `uu.m`, `uu.q`, and `uu.s`: registration, raw opacity, and raw emission.
  Registry bootstrap stdout is captured and checked against SHA-256
  `0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e`.
- The fixture `tests/fixtures/light_material/native.tsv` contains IDs 0 through
  255 in order. Its output SHA-256 is
  `735a11b759bb35257ba029bfe2b087248df82ffc08c264ddb35952d2dab78f2a`.
  Two independently launched JVM captures matched byte-for-byte. The audit
  manifest records JAR, Java source, fixed request, native `uu.class`, bootstrap,
  and both output hashes.

## Findings

Native material properties are represented as one table entry per block ID;
the captured interface has no metadata dimension. The relevant entries are:

| ID | Registered | Opacity | Emission |
| ---: | :---: | ---: | ---: |
| 1 (stone) | yes | 255 | 0 |
| 8 (flowing water) | yes | 3 | 0 |
| 9 (water) | yes | 3 | 0 |
| 18 (leaves) | yes | 1 | 0 |
| 31 (tall grass) | yes | 0 | 0 |
| 39 (brown mushroom) | yes | 0 | 1 |
| 50 (torch) | yes | 0 | 14 |
| 51 (fire) | yes | 0 | 15 |
| 104 (unregistered) | no | raw default 0 | raw default 0 |

The values come from the committed native TSV, not from the Rust implementation.
For an unregistered ID, `uu.m[id] == null`; zeroes in `uu.q` and `uu.s` are
static-array defaults and are not valid material records. Bytecode inspection
confirms the native light propagation helper is `st`: it reads `uu.q` and
`uu.s` and calls the world light accessors. `eb` is the light-kind enum passed
to that helper, not the propagation class.

This confirms a concrete mismatch in the callback reported by the coordinator:
it supplies opacity 255 to every non-air ID, while native registered leaves use
1 and water uses 3. This is a verified material-property mismatch and a strong
candidate for the symptom. It does not prove the user's exact edit sequence or
exclude independent light-queue, height-map, or mesh publication issues.

## Native synthetic edit probes

The separate `LightMaterialEditOracle` reuses the existing bounded native
`LightUpdateOracle` world, setter, and light-queue helpers without changing that
adapter or its fixtures. It performs three fixed synthetic cases: remove a log
under registered leaves, remove still water from an isolated stone-floor cell,
and place still water into that cell. All seed and edit operations call the
native block setter and drain the native update queue. Samples record block ID,
metadata, raw opacity/emission, sky light, block light, and combined light on
both sides of the edit. Both adapters verify their original classes load from
the pinned JAR; the audit records both Java source hashes, original class hashes,
request/output hashes, and the registry bootstrap hash.

`tests/fixtures/light_material/native_edits.tsv` contains 8,159 ordered rows.
For each of the three cases it includes the complete pre-edit exceptions for
all nine loaded chunks, followed by the samples before and after the edit. State
rows use `state<TAB>name<TAB>x<TAB>y<TAB>z<TAB>id<TAB>meta<TAB>blocklight<TAB>skylight`;
height rows use `height<TAB>name<TAB>x<TAB>z<TAB>height`. The domain descriptor
declares defaults of block/meta/block-light 0, sky 0 at y=0 and 15 at y=1..127,
and height 0 from the native all-air initialization. The pre-edit digest covers
the complete native state. Output SHA-256 is
`52184b840b178cb00a3e6b442c54659d6c6dafb0db7eda05ce9ca09558901f80`; two fresh
captures match exactly.

In the fixed canopy case, the sampled leaf at `(8,66,7)` has opacity 1. Removing
the log at `(8,65,8)` changes that cell's native sky light from 0 to 13. The
post-edit native sample also records metadata 8 on the neighboring leaf at
`(8,66,7)`, showing native block metadata/neighbor marking during this edit.
The Rust work under review is light-only and does not implement that block
metadata behavior; this capture makes no claim of full block/metadata parity.
In the water cases, ID 9 has opacity 3 and emission 0; placing it changes sampled
sky light from 15 to 12, and removing it restores 15.
These results demonstrate native behavior for these synthetic inputs only.
They do not reproduce the user's exact save, edit sequence, or screenshot, and
do not rule out other queue, height-map, boundary, or publication defects.

## Files and validation

Files added: `tools/oracle/src/LightMaterialOracle.java`,
`tools/oracle/light_material_audit.py`,
`tests/fixtures/light_material/native.tsv`,
`tests/fixtures/light_material/audit.json`,
`tools/oracle/src/LightMaterialEditOracle.java`,
`tools/oracle/light_material_edit_audit.py`,
`tests/fixtures/light_material/native_edits.tsv`,
`tests/fixtures/light_material/edit_audit.json`,
`tests/python/test_light_material_audit.py`, and this handoff.

Commands run:

- `python3 tools/oracle/light_material_audit.py --write` — two fresh captures
  matched and generated the 256-row fixture.
- `python3 tools/oracle/light_material_audit.py` — two fresh captures matched
  the committed TSV and audit hashes.
- `python3 tools/oracle/light_material_edit_audit.py --write` — three native
  synthetic edit cases; two fresh 8,159-row captures matched.
- `python3 tools/oracle/light_material_edit_audit.py` — two fresh edit captures
  matched the committed complete-state TSV and audit hashes (8,159 rows,
  including all native pre-edit block/light and height exceptions).
- `python3 -m unittest tests/python/test_light_material_audit.py -v` — 6 tests
  passed, including malformed/changed registry data, provenance mutation, and
  wrong-JAR and malformed-protocol rejection.
- `git diff --check` — passed.

The oracle only reads the pinned client JAR and uses synthetic fixed input. It
does not open saves or alter original assets. The user's exact edit/save
reproduction and visual comparison remain outstanding.

## V2 full-state native edits and coordinator comparison

Commit `f0dcc3e` adds an independent V2 corpus; V1 `native_edits.tsv` remains
unchanged. `LightMaterialEditV2Oracle` captures four synthetic scenarios with
three ordered native phases (`before`, `after-place`, `after-remove`): the
canopy/log sequence, a leaf opacity placement/removal, a water attenuation
placement/removal, and glowstone emission at x=15 crossing into loaded chunk
x=1. Every phase records a SHA-256 over all nine chunks' raw block, metadata,
sky-light, block-light, and height arrays. Sparse `state` and `height` rows
encode deviations from the declared all-air defaults; `delta` and `hdelta` rows
encode complete transitions between adjacent native phases. Each `edit` row
precedes its resulting snapshot.

The V2 fixture `tests/fixtures/light_material/native_edits_v2.tsv` is 2,769,192
bytes and 44,806 ordered rows, SHA-256
`d12c5ba682813944647dbe3ac74e7d61235684d122cf40e8d9bc5440b78d8222`. The
manifest `tests/fixtures/light_material/edit_v2_audit.json` records the pinned
JAR, adapter and helper source, request, native class, bootstrap, output, and
two identical capture hashes. The audit reconstructs each complete raw native
snapshot and checks its digest, then verifies every cell and height delta
against the adjacent snapshots. Mutation tests cover those links, declared
all-air defaults, seed rows, wrong-JAR handling, and bounded protocol input.

Native glowstone ID 89 at `(15,64,8)` has block light 15 and sky light 0.
Native cells `(14,64,8)` and `(16,64,8)` each have block light 14 while their
sky light remains 15; cells at x=13 and x=17 have block light 13. Removal
restores the neighboring block-light samples to zero. This is synthetic native
emission behavior only.

The coordinator ran the full-state Rust comparison using the V2 fixture and
reported the following counts. Each phase compares all 294,912 cells in the
nine loaded chunks. The command exited 1 because it found mismatches:

| Scenario and phase | Light mismatches | Block ID mismatches | Metadata mismatches |
| --- | ---: | ---: | ---: |
| canopy/log after-place | 0 | 0 | 0 |
| canopy/log after-remove | 0 | 0 | 12 |
| leaf after-place | 2,107 | 0 | 0 |
| leaf after-remove | 0 | 0 | 0 |
| water after-place | 0 | 0 | 0 |
| water after-remove | 0 | 0 | 0 |
| glowstone after-place | 6,195 | 0 | 0 |
| glowstone after-remove | 3,852 | 0 | 0 |

This comparison is reported by the coordinator; C01 did not run the Rust
comparator. For each edit, the comparator starts from that phase's complete
native predecessor snapshot before applying the edit, so these counts do not
measure a cumulative Rust edit sequence. Its light counts are over block- and
sky-light cells. It makes no height-map, metadata parity, renderer, GPU, or
screenshot reproduction claim. The canopy metadata differences reflect native
leaf neighbor marking outside the light-only Rust contract. The coordinator
also reported a glowstone-removal stale-light discrepancy at `(1,64,8)` (Rust
block light 1, native 0) and a y=0 sky-light discrepancy in the glowstone
comparison; those details are captured in its diagnostic output, not inferred
from the native fixture.

The comparator command reported by the coordinator was:

```text
cargo run --quiet --locked --offline -p beta-core --example light_snapshot_probe < tests/fixtures/light_material/native_edits_v2.tsv > artifacts/light-full-state-comparison.tsv 2> artifacts/light-full-state-comparison.log
```

V2 checks run in C01:

- `python3 tools/oracle/light_material_edit_v2_audit.py --write` — two native
  JVM captures matched; 44,806 rows; output SHA-256 above.
- `python3 tools/oracle/light_material_edit_v2_audit.py` — two new native
  captures matched the frozen fixture and manifest.
- `python3 -m unittest tests.python.test_light_material_audit tests.python.test_light_material_edit_v2_audit -v`
  — 12 focused Python tests passed at capture commit; the added V2 integrity
  hardening was separately checked by all 6 V2 tests.
- `git diff --check` — passed for the V2 capture commit.
