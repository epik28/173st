# INVENTORY-UI-001 — native inventory screen

## Goal

Add a visible, keyboard-toggleable player inventory screen to the existing
Beta viewer. Reuse the accepted in-memory `beta_core::items::Inventory`; keep
this slice read-only with respect to the world and save files.

## Contract

- Pressing E while focused and not repeating opens the inventory screen. While
  open, further E presses are ignored; Escape closes it and restores gameplay
  mouse capture. Movement, flight, hotbar selection, and block
  interaction input do not change gameplay state. Opening releases mouse capture;
  closing returns to the existing viewer mode without silently changing world
  data. Focus loss closes the panel and clears pending input.
- Render the native Beta 1.7.3 inventory background and the supported player
  inventory slots using resources extracted in memory from the SHA-pinned client
  JAR. Verify archive/resource hashes, dimensions, UV bounds, and native slot
  geometry. Do not commit proprietary PNGs or bypass the local world hash guard.
- Display the 36 main inventory slots and their supported stack icons from the
  current `Inventory` model. Do not fabricate player items: the current model
  starts empty. Unsupported item IDs/damage remain unsupported and must not be
  silently converted. Item count labels, armor/crafting interactions, stack
  movement, item use, health simulation, and persistence are outside this slice.
- Center the panel using native scaled GUI coordinates and pixel-aligned atlas
  sampling. Hide first-person HUD while the inventory screen is open. Viewer
  startup remains bound to the existing pinned read-only scene.

## Work cards

### INVENTORY-UI-001-C01 — native inventory resource/layout audit (Codex)

Inspect the pinned client for the inventory-screen key behavior, `GuiInventory`
panel dimensions/origin/UV, main-slot positions, and original `gui/inventory.png`
and `gui/items.png` hashes/dimensions. Determine native item-icon tile mapping for
the supported item IDs 1, 3, 332, and 267 from original methods/bytecode. Record
obfuscated class/method descriptors and source/JAR hashes. Add a fixture-backed
verifier and mutation tests. Do not edit viewer Rust.

### INVENTORY-UI-001-A01 — viewer inventory screen (agy)

Work in parallel with C01. Implement the focused E-to-open/Escape-to-close screen state,
capture/focus handling, native panel and 36-slot layout, and item icons for only
the supported current Inventory stacks. Use the C01 fixture for resource hashes,
coordinates, and icon mapping before finalizing rendering; do not guess native
facts. The event/state scaffold and tests may proceed while C01 is running. Add
deterministic tests for opening/closing, focus,
input suppression, scaled viewport layout, UV bounds, slot mapping, and empty vs
supported stacks. Restrict edits to viewer source/tests and own status/handoff.
Do not change core inventory semantics, native fixtures, queue/shared docs, save
handling, or the integrity guard. Run viewer tests, fmt, clippy, and guarded smoke.

### INVENTORY-UI-001-C02 — integration and acceptance (Codex)

Review exact C01/A01 commits. Run the audit, full workspace checks, relevant
previous strict suites, and a viewer smoke only if the pinned-world guard permits
it. Verify the inventory panel and item geometry from tests; document the current
empty-inventory state and any visual-smoke blocker. Update progress, roadmap, and
completion estimate; synchronize role worktrees.

## Acceptance boundaries

Native claims are limited to audited screen layout/resources and supported icon
mapping. This does not implement container click semantics, item acquisition,
crafting, armor, counts, survival, or persistence. No source save is modified.
