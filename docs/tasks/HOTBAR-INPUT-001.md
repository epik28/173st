# HOTBAR-INPUT-001 — select hotbar slots from the viewer

## Goal

Connect the accepted `beta_core::items::Inventory` hotbar state to viewer input
and the native HUD selection frame. This is a bounded first-person control slice;
it does not add item sprites, inventory screen, item use, pickup/drop, or survival.

## Contract

- Pressing number keys 1 through 9 selects hotbar indices 0 through 8. Use
  physical key codes, react only to a fresh press, and leave existing movement,
  flight, reset, mouse capture, and block controls unchanged.
- Mouse-wheel scrolling changes the selection by the same direction, step count,
  and wrap behavior as the pinned Beta 1.7.3 client. Ignore zero movement. Handle
  line and pixel deltas deterministically; fractional events accumulate until a
  complete step rather than being dropped or inventing extra steps.
- Input updates the accepted `Inventory` selected index. The rendered selection
  frame follows that value on the next frame. The initial selection stays slot 0.
- Ignore gameplay input when the viewer is unfocused. Do not write the selected
  slot into the user's world or save.
- No item stack is rendered or used; the nine slots remain empty.

## Work cards

### HOTBAR-INPUT-001-C01 — native key/wheel audit (Codex)

Audit the pinned Beta client for default hotbar key bindings, scroll direction,
step handling, and wrap behavior. Preserve exact client/class hashes, obfuscated
class/method descriptors, and the bytecode evidence supporting each behavior.
Add a repeatable verifier/fixture and Python tests rejecting wrong JAR and
altered/malformed evidence. Do not edit viewer Rust files.

### HOTBAR-INPUT-001-A01 — viewer hotbar input (agy)

Use the C01 evidence and existing `beta_core::items::Inventory` API. Route
number keys 1–9 and deterministic mouse-wheel steps to selected hotbar state;
feed the selected index to the HUD geometry. Keep changes to viewer source/tests
and your own role status/handoff. Do not change core semantics, native fixtures,
queue, shared docs, save handling, or world integrity guards. Add pure unit tests
for key mapping, repeat/focus behavior, fractional and multi-step wheel events,
wrap boundaries, and HUD selection frame movement. Run viewer tests/fmt/clippy.

### HOTBAR-INPUT-001-C02 — integration and acceptance (Codex)

Review exact A01 commit and compare the tested input mapping to C01. Run the audit,
full workspace checks, and a viewer startup smoke only if the original world guard
allows it. Accept only the stated selection behavior and record the visual-smoke
limitation if the user's changed `level.dat` blocks startup.

## Acceptance boundaries

Unit tests establish every direct key mapping, wheel direction/step/wrap behavior,
fractional accumulation and focus/repeat handling. The selected slot seen by HUD
geometry must match `Inventory` state. No native gameplay or save-write parity is
claimed.
