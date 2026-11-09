# MENU-PARITY-001-C01 handoff

Result: **partial native contract; C01 is not complete**. The pinned client JAR provides repeatable native `qq` scaled-resolution outputs and hash-bound GUI/font/menu bytecode and resource evidence. The coordinator subsequently captured and inspected native main-menu and empty-world-list frames at both requested resolutions/scales; the final evidence is recorded below. Full paired Rust/native frames and remaining save/input cases are still outstanding.

The source is the PrismLauncher local-cache Beta 1.7.3 client, SHA-256 `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`, 1,465,375 bytes. The runtime oracle invokes the actual `qq(kv,int,int)` constructor loaded from that JAR; it does not port its scaling loop. The audit hashes `Minecraft`, `fu`, `rq`, `mg`, `lg`, `da`, `ke`, `qq`, `kv`, `sj`, `nh`, `ub`, and `vb` before checking short `javap` evidence excerpts.

`tests/fixtures/menu_parity/native.tsv` contains nine native scaled-resolution captures for 854×480 at GUI scales 0–3, 1280×720 at GUI scales 0–3, and 1280×800 at scale 2. The first two sizes give the required independent resolution coverage. Native scale values are Auto=0, Small=1, Normal=2, Large=3. Captured examples:

| Display | GUI scale | Native factor | Scaled integer size |
| --- | ---: | ---: | ---: |
| 854×480 | Auto | 2 | 427×240 |
| 854×480 | Small | 1 | 854×480 |
| 854×480 | Large | 2 | 427×240 |
| 1280×720 | Auto | 3 | 427×240 |
| 1280×720 | Normal | 2 | 640×360 |
| 1280×720 | Large | 3 | 427×240 |

The constructor starts with factor 1. Auto substitutes an upper bound of 1000; every factor increment requires the next integer-divided dimensions to remain at least 320×240. The exposed logical dimensions are doubles (`display/factor`); the reported pixel dimensions are their ceiling. Odd display widths therefore can have a fractional logical width alongside an integer scaled width.

The frozen geometry/API in `audit.json` and `menu_parity_audit.py` is:

- Main menu: tiled `/gui/background.png`; `/title/mclogo.png` atlas split into two 155×44 draws at screen `(width/2-137,30)`. Singleplayer, Multiplayer, and Mods use 200×20 buttons at `x=width/2-100`, `y=height/4+48,+72,+96`. The bottom Options/Quit pair is conditional on native `Minecraft.n`: false gives 98×20 buttons at `y=height/4+132`, `x=width/2-100,+2`; true gives one 200×20 Options button at `y=height/4+120`.
- World select: title anchor `(width/2,20)`, white. List viewport is vertical `32..height-64`; rows are 36 pixels. When content is shorter than the viewport, `lg.d()` centers it by halving the negative max-scroll with Java integer truncation; the first row is `top+4-initial_scroll`. Thus three rows start at y=172 logical in 854×480 and y=52 logical in 427×240. Name text is white at `row_y+1`, summary text gray `0x808080` at `row_y+12`. A selected enabled row gets a two-pixel gray outer and one-pixel white inner outline across `width/2±110`. Selection starts at -1; Play/Rename/Delete are disabled until a row is selected. Buttons are Select `(width/2-154,height-52,150×20)`, Rename `(width/2-154,height-28,70×20)`, Delete `(width/2-74,height-28,70×20)`, Create `(width/2+4,height-52,150×20)`, and Cancel `(width/2+4,height-28,150×20)`.
- Buttons use `/gui/gui.png` (256×256). Native button draw uses two horizontal halves, UV x=0 and `200-width/2`, UV y=`46+state*20`. State 0 is disabled, 1 normal, 2 hover. Text colors are signed `-6250336` when disabled, `14737632` normally, and `16777120` on hover; disabled buttons do not hit-test.
- Text is pinned to `/lang/en_US.lang`; `nh` loads the US English file and US stats file directly. The fixture freezes the exact English labels. `font/default.png` is a 128×128 atlas with 8×8 cells and 8-pixel row height. The 256 width entries in `native.tsv` are derived from the native `sj` constructor's alpha scan (rightmost nonempty column plus 2; space is forced to width 4). The `sj` string-width method itself skips `§` and its following formatting character and contributes zero for unsupported characters. The font renderer constructor also issues OpenGL calls, so the glyph table is explicitly a bytecode-driven bitmap derivation, not a claim of direct `sj` method invocation.
- Singleplayer opens `rq`. The list supports row selection, same-row double-click within the native 250 ms interval, and the Play button. Native `rq` has no key override for Enter, so Enter does not load the selected world. Escape goes through inherited `da`, which calls `Minecraft.a(null)` and then `Minecraft.g()`. With no active world, `Minecraft.a(null)` substitutes the `fu` main-menu screen before the mouse-handling call; this is a return-to-main behavior, not evidence that the native chooser displays an unavailable-save screen. Cancel returns to its parent screen. Keyboard behavior is pinned bytecode evidence pending the new live input captures below.

Resource evidence records hashes and dimensions only; original assets and full disassembly are not committed. Native expected output SHA-256 is `b403287e07085e8b7b04e76ccec3706e15c7d198571ad628f745d1e858bb370d`. The native scale constructor is invoked in two independent Java processes and compared byte-for-byte with this fixture on each audit run.

Exact verification performed:

```text
python3 tools/oracle/menu_parity_audit.py
PASS: pinned Beta 1.7.3 GUI bytecode/resources, 9 native scaled-resolution cases, and 256 derived font glyph metrics

python3 -m unittest tests.python.test_menu_parity_audit -v
Ran 4 tests ... OK
```

The four tests cover a clean live-native audit, rejected row-height/layout mutation, rejected translation mutation, and rejection of altered native output even if its local fixture hash is also changed. No workspace-wide checks were run.

Remaining C01 work: visually review the new synthetic save-list and input frames, document their exact hashes, and provide paired Rust/native comparisons if the card requires them. The accepted screenshots establish native views at two resolutions/scales but do not establish pixel parity or the complete input contract. Treat this evidence as geometry/resource/API guidance, not completion of the full C01 card.

## Synthetic native metadata and save-list contract

`tests/fixtures/menu_parity/synthetic_saves.json` defines metadata-only NBT worlds written only beneath the disposable capture runtime. `MenuCurrent` uses native version 19132, `MenuConvert` uses version 0, `MenuFallback` has corrupt `level.dat` plus a valid `level.dat_old`, and `MenuInvalid` has corrupt metadata with no fallback. These are compressed NBT metadata files only: they have no `region/` directory or chunks, and the capture never presses Play or constructs a world.

`tools/oracle/menu_save_list_audit.py` invokes the pinned native `hi` directory reader and sorts its actual `vb` outputs as `rq` does. It freezes the direct native output in `synthetic_save_list.tsv`: descending LastPlayed order, conversion=true exactly when version is not 19132, valid backup fallback, and omission of a directory whose primary and backup metadata both fail. The two corrupt-primary cases generate native `ZipException` diagnostics; the fallback still contributes a row while the no-backup directory does not. Class hashes for `hi`, `tq`, `vb`, `ei`, `as`, and `nu` bind this evidence to the pinned JAR.

This native filtering has no direct visual equivalent to the Rust chooser's unavailable-save safety state. Rust discovery keeps malformed or incomplete entries visible with a launch reason and blocks activation; the native listing drops malformed metadata folders, while valid metadata-only worlds remain listed without checking required region files. The safe native audit therefore shows those rows as the original client would, including the conversion warning, and records that this UI differs from the Rust unavailable row by design. It does not invent a native disabled row or claim parity for missing chunks.

The first synthetic GUI run showed three initial rows correctly and Cancel/Escape returned to the main menu, but its selected-row frames were not valid: the click used y=50 and missed the vertically centered rows. The harness now derives the click from `lg.d()` and records the logical and physical coordinates. Its focused regression expects row-click centers at y=190 (854×480) and y=70 logical/y=210 physical (1280×720 at GUI scale 3), and a mocked launch-path test exercises selection, Return, Cancel, reopen, and Escape at both profiles without claiming GUI evidence. A later manifest field-name error was fixed before the accepted rerun.

The reusable synthetic GUI command writes to paths separate from the accepted empty-list frames and manifest:

```sh
env -u WAYLAND_DISPLAY xvfb-run -a -s '-screen 0 1400x900x24' python3 tools/oracle/native_menu_capture.py --runtime artifacts/menu-parity-capture/synthetic-runtime --output artifacts/menu-parity-capture/synthetic-frames --save-set synthetic
```

For each resolution/GUI scale, it captures the initial list, selects the first sorted row, sends Return, clicks Cancel, reopens the chooser, and sends Escape. It records every frame hash and action coordinates in the manifest; it never activates Play. Coordinator-reviewed results follow.

## Coordinator review of synthetic list and input frames

The coordinator visually reviewed the corrected 12-frame rerun. The pinned `rq` screen has no Enter key override; `da.g()` dispatches keydown to its base handler, which only handles Escape. In both profiles, the post-Return frame is byte-for-byte identical to the selected-row frame, including selected-row outline and enabled Play/Rename/Delete buttons. Thus Return preserves the chooser selection and does not activate Play in these captures. Cancel and Escape frames show the main menu; Escape follows `da`'s `Minecraft.a(null)` path, which substitutes `fu` when no world is active.

| Profile | Initial list SHA-256 | Selected row SHA-256 | Return SHA-256 | Cancel SHA-256 | Escape SHA-256 |
| --- | --- | --- | --- | --- | --- |
| 854×480 / GUI 1 | `7aaa16e2edf7aee8f9cc151ec52a4b763d1ed2e65a4b177578253d0f191438f8` | `1f24e22c2855c7ead1985dbdd638d3b44e11d9c2a87292e6c554bf2b99135a90` | `1f24e22c2855c7ead1985dbdd638d3b44e11d9c2a87292e6c554bf2b99135a90` | `fb3416c3a6f58da695f01b2f3f55b52b4b59caab49afba6b5566166d818befd0` | `22b84b30606bec1b6dd61f78ba7b9803875f6b3f66c1e004d0c83a93bfc3788e` |
| 1280×720 / GUI 3 | `745d2fa64452e8a2d4b57395e02ec7e3b4e8bba55350591e714eb0ed82975bfa` | `9f85aa5bbcaa3109215eb474172286ee51fff89987ee461205524ea75321dfe7` | `9f85aa5bbcaa3109215eb474172286ee51fff89987ee461205524ea75321dfe7` | `e494b2a26c6b276d8a8a36f266f34e7386deb9bc8c462088b1595b99cd44b845` | `d4124b74a53f1adc7660ac3449f65b1a3cbcfd93c301d17aca94402fbcb85bde` |

The initial 854×480 list visibly contains Conversion, Current, and Backup, including the native “Must be converted!” warning. The 1280×720 initial and selected frames contain only Current and Backup, even though the disposable metadata inputs have the same five SHA-256 values across both launches, and the capture manifest records three fixture rows for both. The exact disposable tree currently contains only those four directories and five files, and all five file hashes match the manifest. Direct native `hi.b()` on that exact saves path returns all three rows.

Pinned bytecode narrows but does not explain the discrepancy. `Minecraft.b()` derives `.minecraft` from the JVM `user.home` property on Linux; Minecraft initialization stores that as `Z` and constructs its `hi` list reader with `new File(Z,"saves")`. `Minecraft.c()` returns this reader, and `rq.l()` calls its `b()` method each time the chooser initializes. The capture JVM argument points `user.home` to `/home/andrew/173st/codex/artifacts/menu-parity-capture/synthetic-runtime/home`, so the exact expected runtime list path is `/home/andrew/173st/codex/artifacts/menu-parity-capture/synthetic-runtime/home/.minecraft/saves`; the harness working directory is separate and is not consulted by this Linux path logic. I ran `MenuSaveListOracle` against that exact directory using the pinned native `hi` class on Prism Java 8; it returned Conversion, Current, and Backup in the expected order, with the expected `ZipException` diagnostics for corrupt primary/final-invalid metadata. The Java 8 probe used a temporary copy of the JAR with only its signature entries removed so the helper class could coexist in the unnamed package; native class bytes were unchanged. The current runtime tree still contains exactly four expected directories and five files with the recorded hashes.

`mg.a()` returns `rq`'s list size and `mg.b()` returns size×36. `lg` loops over every list index and draws each row whose 32-pixel body intersects the viewport. At the 1280 profile’s 427×240 logical size, three rows start at y=52, 88, and 124 and all fit inside y=32..176. The screenshot's two rows instead start at approximately y=70 and 106, exactly the centered layout for a two-row list. Thus ordinary viewport clipping cannot account for the missing row; the captured GUI list behaves as if it has two entries, despite the same native reader returning three at the exact path under Java 8. The remaining discrepancy is within GUI startup/list population or an as-yet-undetected capture-state difference. It remains unresolved; do not infer a stable native screen row contract across profiles.

An attempt to trace live JVM `openat`/`getdents64` calls used unique disposable `strace-runtime` and `strace-frames` paths and failed before launching Xvfb or Java because sandbox ptrace is denied: `PTRACE_TRACEME: Operation not permitted`, then `PTRACE_SEIZE: Operation not permitted`. No escalation was attempted, and the accepted frames plus synthetic runtime were untouched. The harness never presses Play and no world is loaded. C01 remains open for this discrepancy and paired Rust/native comparison; unavailable-save behavior still has no native UI equivalent.

## Prepared safe native capture

`tools/oracle/native_menu_capture.py` now prepares an offline client runtime from the pinned read-only JAR and the locally cached Prism Java 8/LWJGL/JInput files. Its only writes are under ignored `artifacts/menu-parity-capture/`: extracted native libraries, a disposable `home/.minecraft/options.txt` containing only `guiScale`, an empty `saves/` directory, logs, frames, and a JSON manifest. Java receives `-Duser.home=<that disposable home>`; the process environment's `HOME` is not changed. Java's standard HTTP and HTTPS proxies point to `127.0.0.1:9` and system proxy discovery is disabled to block standard JVM URL downloads during startup. It does not copy or open any existing save. The capture uses local X11/XTest libraries and ImageMagick; no package install or download is needed.

Offline preparation succeeded with the locally present Java 8 and six Linux native libraries. No GUI was launched in this sandbox, because it cannot create the Xvfb AF_UNIX socket. The parent can run the following exact command with X11 escalation:

```sh
env -u WAYLAND_DISPLAY xvfb-run -a -s '-screen 0 1400x900x24' python3 tools/oracle/native_menu_capture.py --runtime artifacts/menu-parity-capture/runtime --output artifacts/menu-parity-capture/frames
```

The two profiles are 854×480/GUI scale 1 and 1280×720/GUI scale 3. Each process waits at most 45 seconds for the native window, records actual X window dimensions, captures the main menu, and clicks Singleplayer through XTest. A differing second frame is recorded only as a heuristic; it may differ because menu animation changes, so the coordinator must visually verify that it shows the empty world list. The manifest includes client and local library hashes, Java version, full argument vector, disposable paths/options hash, actual window dimensions, frame hashes, timeout, elapsed times, and termination/kill/exit status. It explicitly leaves the unavailable-save case uncaptured. A successful run will provide window evidence, but C01 remains partial until the empty world list is visually verified, unavailable-save state and required review are covered.

The first escalated launch initialized Minecraft but timed out before candidate detection; AWT's `xwininfo -tree` line places a resource class between its title and geometry. The harness now accepts that format and saves the full tree to a `.xwininfo-tree.txt` diagnostic if detection times out again. The native process uses OpenAL Soft's `null` driver to avoid probing host audio devices. The first successful capture produced both main-menu frames, one valid empty world-list frame at 1280×720, and a second 854×480 frame that remained on the animated main menu. The cause was input positioning/focus: the smaller window could remain centered while XTest coordinates assumed the root origin, and the click had no explicit down/up dwell. The harness now always places and raises the window at `(0,0)`, sets X input focus, and holds the mouse button for 150 ms before release. This correction has not yet been exercised by another GUI capture; the existing 854×480 candidate is not an empty-world-list reference. The transition remains visually unverified at that profile, and the unavailable-save case remains uncaptured.

## Coordinator capture acceptance

After integrating `88fe7be` as `2e5a5b0`, the exact Xvfb command above exited 0.
The coordinator inspected all four PNGs: both main menus and both actual empty
Select World screens are confirmed. The earlier small-profile candidate still
showed the main menu and was rejected; moving/focusing the window and holding
the click for 150 ms fixed capture input. Full launch evidence and a separate
visual review remain in ignored `artifacts/menu-parity-capture/frames/`.

| Frame | SHA-256 |
| --- | --- |
| 854x480-gui1-main.png | `77c8d8c5c8f5c00ed957543f30380c12abc5412c8171fbe756bac3c71ad77a4f` |
| 854x480-gui1-world-list-empty.png | `66ed213328097de4565f9ea844dbe9bf54c34a991f77f9ab5ba43bfa3964341a` |
| 1280x720-gui3-main.png | `d133143037408caf2ca619c308ff67466c4d79e60b19a2ddee6fb17d3c535a96` |
| 1280x720-gui3-world-list-empty.png | `816621f46c087727b187802983ff3b838c31eb025ae724ecfac5f04ae1bfd382` |

Only disposable empty saves were opened. No user world, pixel-parity result,
or complete input contract is claimed. C01 remains open for the other cases.
