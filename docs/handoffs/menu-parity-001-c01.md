# MENU-PARITY-001-C01 handoff

Result: **partial native contract; C01 is not visually complete**. The pinned client JAR provides repeatable native `qq` scaled-resolution outputs and hash-bound GUI/font/menu bytecode and resource evidence. No original GUI window was launched or captured, so this handoff does not include the paired original frames required by the card.

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
- World select: title anchor `(width/2,20)`, white. List viewport is vertical `32..height-64`; rows are 36 pixels. Name text is white at `row_y+1`, summary text gray `0x808080` at `row_y+12`. A selected enabled row gets a two-pixel gray outer and one-pixel white inner outline across `width/2±110`. Selection starts at -1; Play/Rename/Delete are disabled until a row is selected. Buttons are Select `(width/2-154,height-52,150×20)`, Rename `(width/2-154,height-28,70×20)`, Delete `(width/2-74,height-28,70×20)`, Create `(width/2+4,height-52,150×20)`, and Cancel `(width/2+4,height-28,150×20)`.
- Buttons use `/gui/gui.png` (256×256). Native button draw uses two horizontal halves, UV x=0 and `200-width/2`, UV y=`46+state*20`. State 0 is disabled, 1 normal, 2 hover. Text colors are signed `-6250336` when disabled, `14737632` normally, and `16777120` on hover; disabled buttons do not hit-test.
- Text is pinned to `/lang/en_US.lang`; `nh` loads the US English file and US stats file directly. The fixture freezes the exact English labels. `font/default.png` is a 128×128 atlas with 8×8 cells and 8-pixel row height. The 256 width entries in `native.tsv` are derived from the native `sj` constructor's alpha scan (rightmost nonempty column plus 2; space is forced to width 4). The `sj` string-width method itself skips `§` and its following formatting character and contributes zero for unsupported characters. The font renderer constructor also issues OpenGL calls, so the glyph table is explicitly a bytecode-driven bitmap derivation, not a claim of direct `sj` method invocation.
- Singleplayer opens `rq`. The list supports row selection, same-row double-click within the native 250 ms interval, and the Play button. Native `rq` has no key override for Enter, so Enter does not load the selected world. Escape is handled by inherited `da` and closes the screen/restores mouse capture; Cancel returns to the parent screen. These are pinned bytecode observations, not live input observations.

Resource evidence records hashes and dimensions only; original assets and full disassembly are not committed. Native expected output SHA-256 is `b403287e07085e8b7b04e76ccec3706e15c7d198571ad628f745d1e858bb370d`. The native scale constructor is invoked in two independent Java processes and compared byte-for-byte with this fixture on each audit run.

Exact verification performed:

```text
python3 tools/oracle/menu_parity_audit.py
PASS: pinned Beta 1.7.3 GUI bytecode/resources, 9 native scaled-resolution cases, and 256 derived font glyph metrics

python3 -m unittest tests.python.test_menu_parity_audit -v
Ran 4 tests ... OK
```

The four tests cover a clean live-native audit, rejected row-height/layout mutation, rejected translation mutation, and rejection of altered native output even if its local fixture hash is also changed. No workspace-wide checks were run.

Remaining C01 work: capture original windows at two fixed resolutions/scales, including an empty world list and an unavailable/unconvertible save, then preserve paired frames and exact client/runtime launch metadata. No pixel comparison, GPU tolerance, live resize, or live mouse/keyboard behavior was measured here. Treat this commit as geometry/resource/API guidance for A01, not as completion of the full visual C01 card.

## Prepared safe native capture

`tools/oracle/native_menu_capture.py` now prepares an offline client runtime from the pinned read-only JAR and the locally cached Prism Java 8/LWJGL/JInput files. Its only writes are under ignored `artifacts/menu-parity-capture/`: extracted native libraries, a disposable `home/.minecraft/options.txt` containing only `guiScale`, an empty `saves/` directory, logs, frames, and a JSON manifest. Java receives `-Duser.home=<that disposable home>`; the process environment's `HOME` is not changed. Java's standard HTTP and HTTPS proxies point to `127.0.0.1:9` and system proxy discovery is disabled to block standard JVM URL downloads during startup. It does not copy or open any existing save. The capture uses local X11/XTest libraries and ImageMagick; no package install or download is needed.

Offline preparation succeeded with the locally present Java 8 and six Linux native libraries. No GUI was launched in this sandbox, because it cannot create the Xvfb AF_UNIX socket. The parent can run the following exact command with X11 escalation:

```sh
env -u WAYLAND_DISPLAY xvfb-run -a -s '-screen 0 1400x900x24' python3 tools/oracle/native_menu_capture.py --runtime artifacts/menu-parity-capture/runtime --output artifacts/menu-parity-capture/frames
```

The two profiles are 854×480/GUI scale 1 and 1280×720/GUI scale 3. Each process waits at most 45 seconds for the native window, records actual X window dimensions, captures the main menu, and clicks Singleplayer through XTest. A differing second frame is recorded only as a heuristic; it may differ because menu animation changes, so the coordinator must visually verify that it shows the empty world list. The manifest includes client and local library hashes, Java version, full argument vector, disposable paths/options hash, actual window dimensions, frame hashes, timeout, elapsed times, and termination/kill/exit status. It explicitly leaves the unavailable-save case uncaptured. A successful run will provide window evidence, but C01 remains partial until the empty world list is visually verified, unavailable-save state and required review are covered.
