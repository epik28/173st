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
| 39 (redstone ore) | yes | 0 | 1 |
| 50 (torch) | yes | 0 | 14 |
| 51 (fire) | yes | 0 | 15 |
| 104 (unregistered) | no | raw default 0 | raw default 0 |

The values come from the committed native TSV, not from the Rust implementation.
For an unregistered ID, `uu.m[id] == null`; zeroes in `uu.q` and `uu.s` are
static-array defaults and are not valid material records. Native chunk light
code in `eb` indexes `uu.q` while reading block IDs, confirming this is the
opacity registry used by native light updates.

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

`tests/fixtures/light_material/native_edits.tsv` contains 44 ordered rows. Its
SHA-256 is `f6bfeaa184688a48e0691c51b6ed3ca9af6f8a1c81b5b04c603e10170f72301a`;
two fresh captures match exactly. In the fixed canopy case, the sampled leaf at
`(8,66,7)` has opacity 1. Removing the log at `(8,65,8)` changes that cell's
native sky light from 0 to 13; neighboring leaf cells retain their captured
opacity and light. In the water cases, ID 9 has opacity 3 and emission 0; placing
it changes sampled sky light from 15 to 12, and removing it restores 15.
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
  synthetic edit cases; two fresh 44-row captures matched.
- `python3 tools/oracle/light_material_edit_audit.py` — two fresh edit captures
  matched the committed TSV and audit hashes.
- `python3 -m unittest tests/python/test_light_material_audit.py -v` — 6 tests
  passed, including malformed/changed registry data, provenance mutation, and
  wrong-JAR and malformed-protocol rejection.
- `git diff --check` — passed.

The oracle only reads the pinned client JAR and uses synthetic fixed registry
input. It does not open saves or alter original assets. Native edit/reproduction
evidence and visual comparison remain outstanding.
