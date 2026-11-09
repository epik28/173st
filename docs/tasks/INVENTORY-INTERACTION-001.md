# INVENTORY-INTERACTION-001 — move supported stacks with the mouse

## Goal

Allow the player to pick up and place supported stacks between the 36 currently
loaded main-inventory slots while the existing E-screen is open. Changes remain
in memory; the source world and saved `level.dat` stay read-only.

## Contract

- Keep the current `beta_core::items::Inventory` indexing: internal slots 0–8
  are hotbar and 9–35 are the three main rows. The screen draws 0–8 in its
  bottom row and 9–35 in the upper rows, matching the existing audited layout.
- Add an in-memory cursor stack to the inventory interaction state. A click on
  a slot uses the exact 18×18 native hitbox around the existing slot position.
  Left click picks up or places the whole stack; right click takes the rounded-up
  half from a slot or places one item. Empty clicks with an empty cursor do
  nothing. While carrying a stack, render its native icon next to the mouse
  cursor with the native atlas and count treatment; C01 pins the exact geometry
  and layering before A01 implements it.
- Compatible means equal item ID and damage. Merge up to the item stack limit;
  incompatible stacks swap only when the cursor stack fits the target slot.
  Supported item IDs/damage and stack limits remain exactly those already
  represented by `ItemStack`; do not synthesize unsupported stacks.
- Native closing drops the carried stack into the world; that entity behavior is
  captured by C01 but remains outside this package. The viewer uses a safe local
  policy: put the carried stack back into the first available in-memory slot. If
  there is no capacity, preserve the cursor and keep/reopen the inventory UI so
  the stack remains reachable. Focus loss follows the same no-loss policy. This
  close behavior is an explicit viewer adaptation, not native parity.
- Clicks outside the 36 supported slots are no-ops. Native outside-drop requires
  world entities and is explicitly deferred. Shift-click, crafting slots,
  crafting output, armor slots, item pickup/drop/use, and save persistence are
  also outside this package.
- Mouse interaction is suppressed while the inventory screen is closed. No
  world edit, filesystem write, synthetic item, or GUI pixel-parity claim.

## Work cards

### INVENTORY-INTERACTION-001-C01 — original client interaction corpus (Codex)

Use the pinned Beta 1.7.3 client to construct a repeatable native oracle over
supported main slots. Capture ordered operations and resulting 36 slot states,
cursor stack, selected hotbar, and held-icon draw layout. Cover left/right pickup/place, odd/even split,
compatible merge at/near limit, incompatible swap, full/unacceptable target,
empty click, screen hitbox edges, native close-time drop sink, and stable replay.
Pin JAR/classes, source evidence and fixture hashes. Keep outside-drop,
shift-click and unsupported container slots explicitly out of the oracle.

### INVENTORY-INTERACTION-001-A01 — in-memory viewer interaction (agy)

Implement the cursor and click state transitions in Rust using the committed
contract and native fixture. Draw the held item next to the mouse with the
audited atlas UVs, count placement, and pass order. Wire physical left/right clicks to the open
inventory screen and map the 36 existing screen slots to `Inventory` indexes
without changing the on-disk slot representation. Add deterministic unit tests
for fixture rows, hitboxes at multiple scales, ignored gameplay/closed-screen
input, close/focus-loss behavior, and unchanged selected-hotbar semantics. Do
not edit C01 files, saved-player parsing, source guards or persistence.

### INVENTORY-INTERACTION-001-C02 — strict integration acceptance (Codex)

Review the pinned native fixture and A01 snapshot; run the native audit/tests,
strict Rust-vs-original click-state comparison, viewer tests, full workspace
check, fmt and clippy. Confirm all changes remain in memory, the close behavior
is described as a safe viewer adaptation, and no unsupported outside-drop or
persistence behavior is claimed. Update progress, roadmap, completion estimate
and handoffs.

## Acceptance boundary

This enables basic cursor-based stack movement only for supported items in the
36 main slots. It does not implement shift transfer, crafting, armor, world
drops, item use, pickup, survival, or saving.
