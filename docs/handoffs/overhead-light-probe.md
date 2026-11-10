# Overhead opaque-block lighting probe

## Scope and result

Added one disposable synthetic native scenario to the V2 light-material corpus:
stone floor at `(8,63,8)`, then place and remove opaque stone at `(8,65,8)` in
an otherwise all-air 9-chunk scene. This is a bounded probe of an opaque block
above an illuminated cell, **not a reproduction of the user's screenshot or
world**.

The pinned Beta 1.7.3 native setter and settled light queue report sky light 15
at `(8,64,8)` before placement, 14 after placing the overhead stone, and 15
after removal. Neighbor cells at `(7,64,8)` and `(9,64,8)` remain at 15. Native
height-map column `(8,8)` changes 64 -> 66 -> 64. This small scene does not
produce the black region in the screenshot.

The Rust `light_snapshot_probe` compared each resulting state against native
over all 294,912 cells in the loaded 9-chunk domain. For this case both edits
had 0 block/sky-light mismatches, 0 block-ID mismatches, and 0 metadata
mismatches. The probe does not compare height-map arrays or mesh/GPU vertex
lighting, and reconstructs each edit from its native preceding snapshot; it is
not cumulative engine-state or screenshot parity.

The same diagnostic run retained existing corpus failures: glowstone and leaf
placement each show 2,107 full-domain light differences at `y=0`, all outside
the edit's ±8-cell neighborhood; canopy-log removal has 12 metadata-only leaf
differences. Those are not caused by this new overhead scenario and remain
outside its acceptance claim.

## Evidence and commands

- Pinned client JAR SHA-256: `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
- V2 fixture SHA-256 after extension: `ad1f583d56a5d267a64c935778db2998b5e8f934f77947c945d01f61089e3d18`.
- Two native captures were byte-identical: 3,180,453 bytes, 51,380 rows.
- `python3 tools/oracle/light_material_edit_v2_audit.py --write` — PASS; generated fixture and provenance manifest from two native runs.
- `python3 -m unittest tests/python/test_light_material_edit_v2_audit.py -v` — PASS, 7 tests.
- `cargo run --locked --offline -p beta-core --example light_snapshot_probe < tests/fixtures/light_material/native_edits_v2.tsv` — overhead place/remove each 0/294,912 light, ID, and metadata mismatches; diagnostic exit remains FAIL overall because of the pre-existing differences listed above.
- Existing settled native capture audit — PASS, 1,434,350 bytes, 21,776 rows, SHA-256 `7582860cf08809550eebe8adb4db307ad9857f58a45863a39da8e42b96399d4c`.

No user world was opened or modified. No engine implementation, queue/status,
progress, or shared task contract was changed.
