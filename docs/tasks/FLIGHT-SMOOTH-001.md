# FLIGHT-SMOOTH-001 — smooth debug-flight presentation

## Scope

Fix visible 20 Hz stepping in the viewer's custom F debug-flight mode. This is a
viewer presentation/input fix, not a claim about native Minecraft flight or
player movement parity. Preserve the existing fixed-step movement rate and
control mapping.

## Observed cause and policy

The frame loop moves `Camera` only while draining 50 ms ticks. Walking renders an
interpolated previous/current player position; debug flight renders the latest
tick position directly. At render rates above 20 FPS this makes flight visibly
advance in steps.

Keep deterministic fixed 20 Hz state updates. Render translation by interpolating
between previous and current flight positions with the controller accumulator
fraction. Keep mouse yaw/pitch immediate so camera look does not lag. Seed both
positions from the current camera when entering flight; snap both to the final
camera when leaving flight, resetting, or losing focus so stale history cannot
cause a visible jump. Preserve the current nominal 8 blocks/second and vertical
control rate.

## Acceptance

- A pure unit test samples a constant input at 20, 30, 60, and 144 Hz render
  partitions. State after each fixed tick is identical, and intermediate camera
  samples advance monotonically without 50 ms plateaus.
- Toggle-in, toggle-out, reset, and focus-loss paths do not interpolate from a
  stale position. Mouse look remains immediate.
- `cargo test -p beta-viewer`, workspace fmt/clippy/check and `tools/check.py`
  pass. Run a short Xvfb viewer smoke if available.
- No changes to physics, native oracle, save files, or movement parity claims.

## Tasks

- A01 implements flight-position interpolation in the viewer/controller and
  tests only.
- C03 reviews the exact agy commit, runs integration checks, and accepts.
