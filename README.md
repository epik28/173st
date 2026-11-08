# 173st

Minecraft Java Edition **Beta 1.7.3** reimplementation in Rust. The project checks
selected behavior against the original client and uses local, hash-pinned game data.

## Current state

The Rust workspace contains verified RNG, chunk, NBT, McRegion and read-only world
data components. A local wgpu viewer renders a fixed 5×5 scene using the original
terrain atlas and a confirmed subset of full opaque blocks. This is an early visual
milestone; gameplay, world simulation, save writing and full visual parity are not
implemented yet.

The viewer includes a 20 Hz walking subset against confirmed opaque full cubes;
fluids, ladders, and other unsupported collision shapes remain outside this scope.

## Build and checks

```bash
python3 tools/doctor.py
python3 tools/check.py
python3 tools/parity/run.py view --strict
```

To run the viewer, copy `config/local.example.toml` to `config/local.toml`, set the
local Prism instance and Minecraft resource paths, then run:

```bash
cargo run --locked --offline -p beta-viewer
```

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
