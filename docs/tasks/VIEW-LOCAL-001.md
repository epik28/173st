# VIEW-LOCAL-001 — hash-pinned preview of the local post-play world

## Goal

Allow the graphical viewer to load the user's current local save snapshot while
preserving the historical WORLD-LOAD-001 and VIEW-001 fixtures as immutable
parity baselines.

## Contract

- The viewer reads only the five configured level/region inputs. Before copying,
  each must be a regular non-symlink file whose size and SHA-256 match the
  ignored machine-local `artifacts/user-world-postplay.sha256` manifest.
- Copy inputs into a unique ignored artifacts run directory. Load and render only
  the copy. Never open or write the original save in the viewer/oracle path.
- Validate the preview copy against its exact five-file manifest, strict names,
  bounded sizes, markers, hashes, and absence of extra files/symlinks.
- Keep `view_probe`, `view --strict`, and all committed historic source hashes
  bound to the original fixtures. The local preview probe is separate and must
  compare against two byte-identical native runs from the pinned original JAR.
- Do not commit the local save manifest, copied world data, or captured output.

## Work cards

### VIEW-LOCAL-001-C01 — fresh current-scene native comparison (Codex)

Capture the current selected 5×5 scene from the pinned original client using only
a verified temporary copy. Repeat in a second JVM and compare Rust output
byte-for-byte. Add any newly observed native mappings as narrowly scoped facts;
do not rewrite the historic VIEW fixture or publish local save hashes.

### VIEW-LOCAL-001-C02 — preview loader and acceptance (Codex)

Add a separate machine-local pin path for the graphical viewer, with strict
manifest/path validation and a dedicated current-snapshot probe. Keep the
historical parity loader unchanged. Run full workspace checks, current native
comparison, and a GUI smoke only if a display server is available. Verify
source hashes remain unchanged after all reads.

## Boundaries

This still loads only the configured fixed 5×5 area. It is not a world chooser,
general save browser, full block catalog, complete visual parity, or save writer.
