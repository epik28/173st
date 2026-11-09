# FLUID-RENDER-001 — reduce per-frame transparent-water uploads

## Goal

Keep the viewer's fluid vertex data static between scene rebuilds and sort only
quad indices for transparent rendering. This targets recurring CPU copies and
GPU uploads during camera movement; it does not change water geometry, UVs,
colors, blend state, depth policy, or native pixel-parity claims.

## Contract

- Preserve the exact current back-to-front ordering rule for fluid quads,
  including deterministic tie behavior. Reorder six-index groups that refer to
  the original six vertices; do not copy/repack vertex payloads each frame.
- Build/update the static fluid vertex buffer only when the scene fluid mesh is
  rebuilt. Keep index data bounded by the scene vertex count and use checked
  conversions for GPU index ranges.
- On frames with unchanged scene fluid geometry, the fluid vertex upload count
  is zero. The per-frame index upload must be exactly 4 bytes per index and
  remain proportional to quad count; for six vertices per quad this is at most
  one third of the current vertex payload size.
- Preserve draw output structure: every source quad's six vertices stay in that
  quad's original order, and the sorted quad sequence matches the existing
  sorter for deterministic fixtures.
- Keep the fluid mesh cache and GPU buffer pair transactional: prepare candidate
  scene/fluid cache state and validate both packed fluid vertex data and index
  capacity before publishing the new fluid cache or either buffer handle/count.
  Opaque chunk GPU publication is outside this guarantee. Rebuild fluid vertex
  data only on an accepted scene rebuild.
- Do not change fixed-step movement, catch-up policy, water simulation,
  transparency rules, source-world data, or save files in this task.

## Work cards

### FLUID-RENDER-001-A01 — static fluid vertices and sorted indices (agy)

Implement an indexed fluid draw path in the viewer. Add fixture-driven tests
that compare sorted index groups against the current ordered quad output for
near/far, overlapping-depth, and equal-depth cases. Add a testable byte/count
accounting seam proving unchanged frames upload no fluid vertices and upload
only six `u32` indices per quad. Run viewer tests, workspace checks, clippy,
formatting, and a short Xvfb smoke if available. Record before/after p50/p95/p99
frame CPU time and fluid upload bytes for stationary and flight captures when
the local GPU instrumentation can measure them; do not make acceptance depend
on noisy timing improvement if byte-count invariants and output-order tests
pass.

### FLUID-RENDER-001-C02 — acceptance (codex)

Review the exact A01 snapshot, run the focused index-order tests and
`python3 tools/check.py`, and inspect the Xvfb smoke/measurement record. Confirm
water UV/material and blend/depth policy are unchanged, and state the exact
per-frame upload reduction supported by measured counts.

## Acceptance boundary

This is a viewer-side data-transfer optimization only. It does not claim a
specific FPS gain, fix the separate synchronous edit/rebuild path, cap fixed
tick catch-up, or alter native gameplay behavior.
