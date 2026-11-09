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

## A01 — read-only world catalog and snapshots (agy)

Implement a tested catalog API for immediate save directories under the configured
Prism `minecraft/saves` root. Read only regular non-symlink `level.dat` files,
parse metadata through the existing bounded NBT loader, and sort by `last_played`
descending with folder name as deterministic tie-breaker. Keep folder identity
separate from display name. Mark malformed or incomplete saves with a launch
reason; never mutate source files.

Only mark a save launchable when its spawn-centered 5×5 chunks are present in
the required dynamically computed McRegion files and loadable by the bounded
viewer. Resolve region names from spawn coordinates, including negative values.
Provide a fresh snapshot API that pins sizes and hashes at discovery, rechecks
sources immediately before and after copying, validates copied hashes, and
writes only disposable artifacts. Reject traversal, symlinked roots/directories/files,
unexpected entries, absent chunks, malformed metadata, and paths outside the
configured saves root.

Do not implement graphical menu/input in A01. Keep historical strict world/view
parity fixtures and source pins unchanged. Do not write source saves,
`level.dat`, `.mcr`, `level.dat_old`, or user configuration.

## A02 — visible chooser and selected-world startup (agy)

Integrate the A01 catalog into a visible main-menu / world-list flow before any
world scene or GPU mesh is loaded. Provide Singleplayer, row selection, disabled
invalid rows with a reason, Select/double-click or Enter activation, and Cancel.
The selected folder's disposable snapshot must drive the loaded scene; do not
auto-select the first valid save and do not fall back to the old fixed world.

Generalize the bounded loader to use the selected save's spawn-centered 5×5
chunks and dynamically pinned McRegion files. Remove the hidden dependency on the
historical fixed `scene.tsv` from normal selected-world startup while preserving
it for parity tools. Preserve source hash/symlink/path guards, and keep invalid
world refresh/error behavior visible and safe.

Tests must cover menu state/input, dynamic positive and negative region
coordinates, selected-world scene loading, malformed/missing worlds, changed
source rejection, cancel, and no source writes. User saves remain read-only.

## C02 — acceptance (Codex)

Review A01 and A02 against C01 and the security boundary. Test discovery/sorting,
selection, activation, cancel, changed-source rejection, malformed/missing worlds,
dynamic region coordinates (including negative spawn coordinates), and read-only
source behavior. Run the native audit, Rust tests, and full
`python3 tools/check.py`. Verify the existing local source-world manifest and
files remain unchanged. Document the fixed 5×5 window and unsupported worlds.

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
