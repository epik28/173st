# LIGHT-001 — native flat saved-light arithmetic and clear-weather sun

Base COLOR-001 accepted d2fba16 and LEAF-001 renderer integration (C04 may still
be under root review). Original Beta1.7.3 JAR hash remains pinned. No dependencies.
This package prepares exact numerical light APIs; visual wiring is C04 after
LEAF acceptance. No AO, weather, propagation, fluids or non-cube model claims.

## Native scope

Original provider xa factory for dimensions0 (Overworld/rh) and-1 (Nether/wd),
xa.e() table, xa.a(JF) celestial angle, fd.a(F) clear-weather sky subtraction,
fd.a(IIII) brightness at a normal block coordinate, lm.c(IIIZ) saved-light
combination with original wi nibbles. Synthetic fd subclass may override block
and chunk lookup; allocate without fd constructors/disk I/O, real lm/wi/provider
and metadata/time objects. Read actual native outputs. No rewritten Java formulas.
Native dimension1/ay is unused Skylands in this version, NOT the later End, and
is excluded. Block special cases44/60/53/67 sample five neighbors; scalar helpers
here describe ordinary cube samples, not those higher-level special queries.

Original scalar values are f32 brightness, not a later-version packed lightmap.
Retain native class/code-source hashes, actual direct call counts, bootstrap,
source and inputs/outputs, two independent JVM traces. A saved-light sky-used
static flag is outside pure numerical behavior. Provider/default weather fields
must be explicitly recorded as clear0, not claimed as original-world simulation.

## Frozen pure Rust API (`beta_core::light`)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightDimension { Overworld, Nether }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightError { InvalidLevel, InvalidPartialTick }
pub fn brightness(level:u8, dimension:LightDimension)->Result<f32,LightError>;
pub fn combined_light(block:u8, sky:u8, subtraction:u8)->Result<u8,LightError>;
pub fn sample_brightness(block:u8, sky:u8, subtraction:u8, minimum:u8,
                        dimension:LightDimension)->Result<f32,LightError>;
pub fn celestial_angle(time:i64, partial:f32, dimension:LightDimension)
    ->Result<f32,LightError>;
pub fn sky_subtraction(time:i64, partial:f32, dimension:LightDimension)
    ->Result<u8,LightError>;
pub fn emission_level(block_id:u8)->u8;
```

Levels/subtraction/minimum0..15, finite partial0..1 (negative zero valid).
Preserve original f32 operation order/table generation, Java long remainder and
casts, f64 Math.cos→f32 provider smoothing, then original in.b cosine LUT in
world sky subtraction. Use existing compatible core math/RNG; no fused math.
Emission static registry for all256 IDs must come from actual native table; no
modern values guessed. No fd/Java/runtime/file dependencies in core.

## Frozen bounded TSV protocol

ASCII LF, no comments/blanks, unique IDs [A-Za-z0-9_.:-]{1,96}, canonical signed
decimals, lowercase raw hex. Max100000 rows and16MiB; atomic stdout on any error.

- `id<TAB>sample<TAB>dimension<TAB>block<TAB>sky<TAB>subtraction<TAB>minimum`
  -> `id<TAB>sample<TAB>raw_combined_level<TAB>brightness_raw_f32_hex8`.
  Raw level original lm.c, brightness original fd.a(IIII) with given minimum.
- `id<TAB>time<TAB>dimension<TAB>time_i64<TAB>partial_raw_f32_hex8`
  -> `id<TAB>time<TAB>angle_raw_f32_hex8<TAB>sky_subtraction_decimal`.
- `id<TAB>emission<TAB>block_id0..255` -> `id<TAB>emission<TAB>level0..15`.

Dimension0/-1 only. Probe `crates/beta-core/examples/light_probe.rs`, no CLI args.
Compare integer/raw bits exactly. Use all16 block/sky/subtraction combinations
across multiple minima and both dimensions, entire24000-tick clear Overworld
cycle at partial0/1, all256 emission IDs, negative/long extrema times and partial
boundaries. Keep corpus below100000 rows. Counter facts must distinguish direct
calls from inferred internal calls; no fake instrumentation counts.

## Cards

- C01 codex: native adapter/corpus/fixtures and normal fixture tests + explicit
  live tests. Actual calls only; preserve clear-world/no native constructors
  seam. Early committed fixture snapshot unblocks Rust exact comparison.
- A01 agy: module/probe and own status/handoff, exact native fixtures; meaningful
  boundary/invalid/atomic tests. No original/fixture changes, no viewer files.
- C02 codex: retained typed native runner/reviewer/custom-input with missing part
  error, clean revision, process/binary/class/source/JAR/output hashes; comparison
  mutations and malformed protocol sensitivity. May reuse established framework.
- C03 codex: exact snapshots review, full checks, clean native/parity evidence;
  previous math/Java strict, update progress honestly. C04 viewer wiring will be
  frozen separately once native APIs and LEAF accepted. No auto readiness points
  for tests alone; push after requested cumulative5–10pp gain.
