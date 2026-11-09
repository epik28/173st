# HOTBAR-ITEMS-001 — render saved item stacks in the first-person hotbar

## Goal

Show supported stacks from the already loaded, read-only `Inventory` in the
first-person hotbar. The accepted HUD currently shows the frame and selection
but no items even when the saved player has supported hotbar stacks.

## Contract

- Reuse `beta_core::items::Inventory` and the existing hash-verified native
  `terrain.png`/`gui/items.png` atlases. Native `bb` binds `terrain.png` for
  block IDs below 256 and `gui/items.png` for item IDs 256 and above. Do not
  change saved-player parsing, item identity, world/save handling, or hotbar
  input semantics.
- Draw only item stacks already supported by `inventory_screen::item_icon_uv`;
  empty slots remain empty and unsupported ID/damage values stay absent. Never
  synthesize stacks or map unsupported items to a similar sprite.
- Place icons inside the nine native hotbar slots, retaining nearest sampling,
  pixel alignment, atlas alpha and correct viewport scaling. Preserve native
  HUD layering: hotbar frame, selected-slot frame, crosshair, health/armor rows,
  then item sprites and count text, as established by C01. Keep existing viewer
  geometry ordering for everything outside this new item pass.
- Display stack counts only if the original font resource and native count
  layout can be audited and implemented without a new dependency or guessed
  glyph map. Otherwise this package is icon-only and records counts as a later
  task; do not use hand-drawn digits.
- Correct the existing inventory-screen atlas binding for supported block IDs
  1 and 3 in this same renderer slice: their icon coordinates are on
  `terrain.png`; item IDs 332 and 267 remain on `gui/items.png`. Retain exact
  icon UVs and do not broaden the supported ID/damage list.
- Keep the local world integrity guard. All world reads remain bounded,
  read-only, and sourced from the verified disposable copy.

## Work cards

### HOTBAR-ITEMS-001-C01 — native hotbar item draw audit (Codex)

Use the pinned client JAR to trace the actual first-person hotbar item draw path,
its atlas coordinates, slot origin/spacing, stack-count text call and ordering
relative to the selected frame. Verify supported item UV facts against the
existing `INVENTORY-UI-001-C01` audit. Record class/method descriptors, source
and resource hashes, plus reproducible evidence. Do not edit viewer Rust or
touch the live world.

### HOTBAR-ITEMS-001-A01 — hotbar stack rendering (agy)

After C01 is committed, use the audited native geometry and existing
`Inventory`/icon mapping to render supported hotbar stacks. Add pure vertex
generation and integration tests for all nine slot locations, selected-slot
independence, empty slots, supported stacks, unsupported entries, viewport
scales, atlas bounds, and draw ordering. Implement count labels only if C01
proves the font/layout and the code can use the pinned original font resource;
otherwise leave counts outside this slice. Do not alter core items, player
save reads, native fixtures, the world guard, or add dependencies. Update agy
handoff/status and commit the result.

### HOTBAR-ITEMS-001-C02 — integration and acceptance (Codex)

Review the exact C01/A01 snapshots. Run native audit tests, focused viewer tests,
full workspace checks and applicable strict parity suites. Run a GUI smoke only
if the existing local-world pin permits it. Confirm items appear only where
the validated inventory has supported stacks, verify block/item atlas selection
in both inventory and hotbar, and state whether count labels were included.
Update progress, roadmap and completion estimate without
claiming health or survival behavior.

## Acceptance boundaries

This adds a HUD view of saved data only. It does not implement item pickup,
stack manipulation, attack/use, survival, health, counts unless separately
audited, or save persistence. Pixel parity and lighting/shadow fixes are outside
scope.
