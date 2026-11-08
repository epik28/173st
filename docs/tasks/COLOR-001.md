# COLOR-001 — seeded native climate and grass/foliage palettes

Base UV-001 accepted c0b03f9. JAR af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c.
Goal: exact scalar climate and palette behavior usable without Java at runtime.
Renderer integration/leaf geometry follows separate LEAF-001 contract.

## Frozen API and ownership

No dependencies added. `beta_core::biome` owns:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climate { pub temperature: f64, pub rainfall: f64 }
pub struct ClimateSampler { /* private original-compatible seeded noise state */ }
impl ClimateSampler {
    pub fn new(seed: i64) -> Self;
    pub fn sample(&mut self, x: i32, z: i32) -> Climate;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorError { InvalidClimate }
pub fn grass_color(palette: &[u32; 65536], climate: Climate) -> Result<u32, ColorError>;
pub fn foliage_color(palette: &[u32; 65536], climate: Climate, metadata: u8)
    -> Result<u32, ColorError>;
```

Sample must match original xv.a(x,z,1,1), not batch16x16. Preserve operation order,
Java RNG consumption, i64 seed multiplication wrapping, native noise permutation,
floor/cast and f64 arithmetic. Public sample accepts i32; native parity scope
coordinates abs<=30,000,000. No fixture lookup table or rewritten Java noise
oracle. Validate finite climate components0..1 in palette API (also special leaf
metadata), use original metadata precedence. Return full original int bits as u32;
RGB extraction belongs renderer. Palette input is native ImageIO ARGB pixel order.

Rust role also owns `beta-viewer/src/colors.rs`, its lib.rs module export and
`beta-viewer/examples/color_probe.rs`; no edits loader/main/shader in this package.
`colors::ColorMaps { pub grass:Box<[u32;65536]>, pub foliage:Box<[u32;65536]> }` and
`load_color_maps(grass:&Path,foliage:&Path)->Result<ColorMaps,Box<dyn Error>>` use
existing image PNG decoder, verify exact hashes/dimensions256x256, convert RGBA8
pixels to ARGB u32. No assets embedded or committed.

Resource SHA: grasscolor.png d9b2d9073f06013b9f36b1c61a91678be478760abff31fae942ee52ad898ca1b;
foliagecolor.png 644a527dd7daea5a195bbed056a6a41e295b6043a05621f4c5c263a25fb171cc.

## Frozen protocol

ASCII TSV, one LF-terminated record, no comments/blank lines, unique IDs matching
[A-Za-z0-9_.:-]{1,96}. `id<TAB>climate<TAB>seed_i64<TAB>x_i32<TAB>z_i32` or
`id<TAB>palette<TAB>temperature_raw_f64_hex16<TAB>rainfall_raw_f64_hex16`.
Canonical signed decimals, lowercase raw hex, finite palette args in0..1,
coordinates abs<=30m. Max4096 rows and4MiB input; parse all before output.
Probe CLI `color_probe GRASS_PNG FOLIAGE_PNG`. Atomic stdout on invalid input.

Output climate: `id<TAB>climate<TAB>temperature16<TAB>rainfall16<TAB>grass8<TAB>foliage0_8<TAB>foliage1_8<TAB>foliage2_8<TAB>foliage3_8`.
Output palette: `id<TAB>palette<TAB>grass8<TAB>foliage8`.
All colors raw original32bits, zero tolerance. Climate colors call native block
methods wp.b/bk.b with metadata0..3 at y64; direct palette rows call ia/jh.a(DD).

## Cards and acceptance

- C01 codex: unchanged native xv(fd) constructed from synthetic seed-only World
  (no native World constructors), native palette setters loaded ImageIO original
  resources, native grass/leaf calls via xp. Record class/code sources/hashes,
  actual primary invocation counts, resource/bootstrap/source/output hashes.
  Generate at least1000 seeded climate cases incl long extrema/wrapping/negative
  coords, boundaries and repeated coordinates/seeds, plus at least256 palette
  boundary/random cases incl0/1/subnormal/nextafter. Repeat two independent JVMs.
- A01 agy: pure core noise/climate/palette APIs and colors/probe above, own status/
  handoff. Meaningful synthetic RNG/permutation/boundary tests and all native
  fixtures exact; malformed protocol/late error/limits. No oracle/fixture edits.
- C02 codex: focused live CLI `tools/parity/color_run.py`, retained input/source/
  adapter classes/executable/commands/logs/resource and output hashes, source
  revision; missing parts error/not_implemented, never pass. Comparator exact
  ordered state types/counts; verify output mutations and malformed subprocesses.
  Asset preparation helper may extract validated original PNGs to local configured
  directory without overwriting files or writing original JAR/save.
- C03 codex: integrate exact Rust commit, full workspace check, fresh clean native
  color run, independent saved evidence/protocol review and previous math/java/RNG
  strict suites affected by new noise use. Record limits/no pixel parity claim.
  Estimate only for capability; push/report at requested cumulative5–10pp interval.
