# WORLD-SELECT-001 — select an existing local save

## Goal

The viewer currently starts directly in one hard-coded `New World` preview.
Add a small native-informed main-menu/world-selection flow that lets the user
choose an existing supported save, then opens a disposable read-only snapshot
centered on that save's player spawn. The source world must never be modified.

The first slice only reads existing saves. It does not create, rename, delete,
convert, or save worlds, and it does not add chunk streaming or world generation.

## C01 — native menu and save-list contract (Codex)

Audit the pinned Beta 1.7.3 client menu classes and record provenance for the
Singleplayer transition, world-list ordering/selection, row metadata, selection
activation, cancel behavior, and relevant button geometry. Pin the JAR, class
files, inspected methods, and audit source. The reference facts currently
identified are obfuscated classes `fu` (main menu), `rq` (select world), `mg`
(world list), `vb` (save summary), and `tq` (legacy save format); C01 must verify
them reproducibly before A01 relies on them.

Exclude create-world (`fj`), rename (`jk`), delete confirmation (`qt`),
multiplayer/options/mods, title panorama animation, and any native save mutation.

## A01 — read-only catalog and selected-world preview (agy)

Implement a main-menu Singleplayer entry and an existing-world chooser. List
immediate save directories under the configured Prism `minecraft/saves` root;
read only regular non-symlink `level.dat` files, parse metadata through the
existing bounded NBT loader, and sort by `last_played` descending with folder
name as deterministic tie-breaker. Keep folder identity separate from display
name. Render enough native-informed metadata to identify each save and support
row selection plus Select/double-click activation and Cancel.

Only enable launch for saves whose spawn-centered 5×5 chunks are present in the
required McRegion files and are loadable by the current bounded viewer. Resolve
the needed region-file names from the selected spawn; do not assume `r.-1.*` or
`r.0.*`. For launch, snapshot the selected `level.dat` and required region files
into a fresh artifact directory, pin their sizes and hashes at discovery, and
recheck every source file immediately before and after copying. A changed save
must produce a visible refresh/error state, never a silent fallback. The loader
must consume only the disposable copy.

Reject traversal, symlinked directories/files, unexpected copy entries, absent
chunks, malformed metadata, and paths outside the configured saves root. Invalid
saves may remain visible with a disabled launch reason. Keep historical strict
world/view parity fixtures and source pins unchanged. Do not write to source
saves, `level.dat`, `.mcr`, `level.dat_old`, or user configuration.

## C02 — acceptance (Codex)

Review the implementation against C01, test discovery/sorting, selection,
double-click, cancel, changed-source rejection, malformed/missing worlds,
dynamic region coordinates (including negative spawn coordinates), and the
read-only source boundary. Run the relevant native audit, Rust tests, and full
`python3 tools/check.py`. Verify the existing local source-world manifest and
files remain unchanged. Document the fixed 5×5 window and unsupported worlds.

## Acceptance boundary

This adds a usable chooser for existing saves whose spawn-centered 5×5 scene is
already represented by complete, supported chunks. It does not implement the
native title panorama pixel-for-pixel, create/rename/delete, save persistence,
arbitrary camera coordinates, streaming, simulation, or survival behavior.
