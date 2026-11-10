# MENU-PARITY-001-A01 follow-up

Result: fixed the confirmed small-font geometry mismatch and added the native
version/copyright decorations to the main menu. This work is in the isolated
`agents/menu-parity-followup` branch; the coordinator has not accepted or
integrated it.

The evidence is the pinned read-only Beta 1.7.3 client JAR
`../.local/minecraft/client-b1.7.3.jar` (SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`). The
`sj` constructor creates each glyph display list using a 7.99×7.99 quad from
the full 8×8 atlas cell, then translates by the separately alpha-scanned glyph
width. The Rust renderer had scaled the image quad to that advance width,
visibly squeezing narrow glyphs. It now draws the native full cell and advances
by the native metric. For a fully transparent cell, native scanning ends at
column -1 and stores width 1; the Rust fallback now does the same. Shadow color
uses the native `0xfcfcfc >> 2` channel operation.

Pinned `fu.a` bytecode also confirms the main-menu decorations: shadowed
`Minecraft Beta 1.7.3` at `(2,2)` in `0x505050`, and shadowed
`Copyright Mojang AB. Do not distribute.` at
`(scaledWidth-stringWidth-2, scaledHeight-10)` in white. They are now drawn
after the buttons as in the native method. The native JAR was inspected only;
no original resources were extracted or committed.

Verification:

- `cargo test --locked --offline -p beta-viewer --bin beta-viewer menu::tests`:
  6 focused menu tests passed, including exact glyph draw/advance geometry,
  transparent-cell advance, and title/footer coordinates and colors.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --offline -p beta-viewer --all-targets -- -D warnings`:
  passed.
- `git diff --check`: passed.

Remaining: the date-dependent/random splash text drawn over the logo is not
implemented or claimed; its deterministic runtime selection/animation is not
captured by the current C01 contract. The unresolved conversion-world row at
GUI scale 3 is a separate C01 question and was not altered or treated as fixed.
No paired native/Rust pixel comparison was performed, so this change does not
claim full pixel parity. Other native menu interactions and unimplemented
button actions remain as recorded in the parent A01 handoff.
