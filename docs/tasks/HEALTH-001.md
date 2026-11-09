# HEALTH-001 — load saved health and draw native heart states

## Goal

Replace the viewer's fixed 20/20 HUD with the player's saved Beta health and
render full, half, and empty hearts using the native atlas. This is a read-only
presentation/data slice: it does not simulate damage, healing, regeneration,
death, blink, armor, or hunger.

## Contract

- Read `Player.Health` from the already validated copied `level.dat` NBT. Native
  type is signed Short. Missing or malformed fields default to 20 for this
  viewer; accepted values clamp to `0..=20`.
- Health is represented as half-heart units, integer `0..=20`.
- Draw 10 cells in the existing native HUD positions and order. Every cell
  draws the native background UV `(16,0,9,9)`; health `>= 2*i+2` draws full
  heart `(52,0,9,9)`, health `== 2*i+1` draws half heart `(61,0,9,9)`, otherwise
  background only. Preserve the existing GUI scale, texture atlas, and pass.
- The value is saved health only. Do not expose controls that change health or
  imply live survival simulation. `R` reset returns to the pinned saved health.
- Keep the source world read-only and retain its integrity guard. Do not update
  the pinned world hash or access the user's live files outside the existing
  validated copy path.

## Work cards

### HEALTH-001-C01 — native field and heart rendering audit (Codex)

Extend the pinned HUD audit to verify `Player.Health` type/default/range facts
and original heart branch/UV behavior, including half-heart `(61,0)`. Pin source
class/resource hashes and relevant bytecode evidence. Add fixture mutation tests
for wrong JAR, health type/range, UVs, missing methods and malformed evidence.
Do not edit Rust viewer files or world data.

### HEALTH-001-A01 — load saved health and render hearts (agy)

Add a bounded saved-player health extractor and pass its value to the existing
HUD vertex builder. Implement full/half/empty heart states from the C01 fixture.
Add tests for missing/malformed NBT, `0,1,2,3,19,20`, out-of-range values,
multiple scales, and ordered raw atlas UV vertices. Preserve the reset behavior
and read-only world guard. Do not edit C01 oracle/fixture/Python tests or shared
contracts/status.

### HEALTH-001-C02 — acceptance (Codex)

Review the A01 commit against C01 evidence. Run native audit, Rust tests, full
workspace checks, and parity where applicable. Confirm no health simulation or
save writes are claimed, then update progress, roadmap, estimate and handoffs.

## Acceptance boundary

Saved health is displayed accurately, but it never changes during play. Damage,
healing, peaceful regeneration, armor, death/respawn, hurt animation and blinking
remain future work.
