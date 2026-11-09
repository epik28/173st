# 173st

Minecraft Java Edition **Beta 1.7.3** reimplementation in Rust. The project checks
selected behavior against the original client and uses local, hash-pinned game data.

## Unofficial project and third-party rights

173st is an independent, unofficial project. It is not created, approved,
endorsed, sponsored, or supported by Mojang Studios or Microsoft, and is not an
official Minecraft product or service. “Minecraft” and related names, marks,
game code, artwork, textures, audio, and other game content belong to their
respective rights holders. The name is used only to identify the game and version
this project studies; it does not imply affiliation or endorsement.

This repository does not include the Minecraft client, its proprietary source
code, or original game assets. Local tools may read files from a user's own game
installation for testing and rendering; they do not grant rights to those files.
Do not commit or distribute client files, extracted assets, or modified game files
through this repository, releases, or other downloads. The GPL-3.0-only license
in this repository applies to the project's own code and does not license
third-party game content or trademarks. Users are responsible for obtaining and
using local game files under the terms that apply to them.

This notice describes the project's intent; it is not a legal opinion or a
guarantee of compliance. See the official
[Minecraft EULA](https://www.minecraft.net/en-us/eula) and
[Minecraft Usage Guidelines](https://www.minecraft.net/en-us/usage-guidelines).

## Current state

The Rust workspace contains verified RNG, chunk, NBT, McRegion and read-only world
data components. A local wgpu viewer renders a selected save's spawn-centered
5×5 scene using the original
terrain atlas, native seeded grass/foliage palettes, and fancy cutout leaf cubes.
The current sandbox includes limited movement, block edits and inventory controls;
world simulation, save writing and full visual parity remain unfinished.

The viewer includes a 20 Hz walking subset against confirmed solid full cubes and
the full-cube leaves enabled for rendering;
fluids, ladders, and other unsupported collision shapes remain outside this scope.

## Build and checks

```bash
python3 tools/doctor.py
python3 tools/check.py
python3 tools/parity/run.py view --strict
```

To run the viewer, copy `config/local.example.toml` to `config/local.toml`, set the
local Prism instance and Minecraft resource paths, and prepare the hash-verified
palette PNGs beside `terrain.png`:

```bash
python3 tools/prepare_assets.py
cargo run --locked --offline -p beta-viewer
```

Choose **Singleplayer**, select an available save, then press **Select** or Enter.
Unavailable entries show a reason. Cancel returns to the menu. The current menu
is temporary; original-menu presentation is tracked in MENU-PARITY-001.
The loaded window stays bounded to 5×5 chunks; chunk streaming is not implemented.

### Controls

Press Enter to capture the mouse. Move with W/A/S/D, jump with Space, sneak
with Shift, toggle debug flight with F, and return to the nearest safe loaded
floor with R. Escape releases the mouse. Walking uses a fixed 20 Hz simulation
step; flight is a debugging view and ignores collision.

Mouse look follows the native 0.15-degree-per-pixel sensitivity using integer
pixel deltas. Window-system and mouse-device backends can report different
motion deltas, so exact look parity is not guaranteed.

The viewer reads the original save, verifies pinned hashes, and renders from a local
working copy under ignored `artifacts/`. Original JARs, resources and saves are not
stored in Git.

The fixed scene includes fancy leaves and original biome grass/foliage colors.
Flat lighting uses saved time and native brightness tables. Weather, ambient
occlusion, general light propagation and loading Nether regions remain future
work. Edits have bounded relighting using native opacity values; the reported
post-edit shadow is still tracked in LIGHT-UPDATE-002.

For an opt-in X11 smoke check under an isolated Xvfb display:

```bash
env -u WAYLAND_DISPLAY xvfb-run -a python3 tools/viewer_smoke.py
```

The default checks menu/list/cancel/quit. Add `--enter-world` to explicitly open
the first enabled world's disposable preview. Captures stay in `artifacts/`;
this checks Rust UI integration, not visual parity with the original client.

## Project notes

- [Requirements and stages](docs/project-requirements.md)
- [Agent workflow](docs/workflow.md)
- [Parity testing](docs/parity-testing.md)
- [Current progress](docs/progress.md)
- [Task queue](docs/tasks/queue.json)
- [Task cards](docs/tasks/)
- [References](docs/references.md)

The workspace uses separate `codex/verification` and `agy/development` worktrees
under a shared `173st/` directory. Agent-specific instructions are in [AGENTS.md](AGENTS.md)
and [GEMINI.md](GEMINI.md).

License: [GPLv3](LICENSE).
