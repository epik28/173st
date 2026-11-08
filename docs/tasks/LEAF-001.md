# LEAF-001 — fancy leaf cubes and native grass/foliage tint integration

Base COLOR-001 contract d0e06f3, native COLOR APIs frozen there. Original client
JAR/terrain hashes remain pinned. Depends on COLOR-001 acceptance for final merge.

## Native evidence before implementation

Root live experiment `/tmp/GraphicsBehaviorProbe.java` calls unchanged bk setters/
face methods and cv.b(grass,x,y,z,r,g,b), reads original nw.g packed vertices.
It disproved earlier speculative bytecode notes; those are not expected values.
Record this behavior with actual-call fixture adapter, not a copied Java formula:

- Constructor-default leaf c()=true and tile52. `bk.a(true)` explicitly selects
  fancy: opaque=false, tiles52 (or132 when meta&3==1). `bk.a(false)` fast:
  opaque=true, tiles53/133. Metadata3 does not select pine tile, but tint bit1
  still takes precedence over bit2, as independently tested COLOR-001.
- With xp.g reflecting original neighbor c(), fancy leaves show a shared face
  next to leaves; fast leaves suppress it. Air/glass show, stone occludes.
- Leaf collision full cube must be verified independently from opacity.
- Original flat cv.b grass: bottom white, TOP tinted supplied RGB; side base tile3
  white, side overlay tile38 tinted. Capture all actual packed colors/UVs/xyz for
  primary RGB inputs and both fancy-grass overlay settings, document flat-light
  fixture world brightness=1. No whole-world or GPU pixel-parity claim.

C01 oracle should capture leaf mode/meta0..15/face0..5 against neighbor air/stone/
leaves/glass (both modes), plus original collision bounds. Grass submission cases
must establish face/tile/tint association from native buffers. Record cv/nw/bk/nr/
uu class hashes/code sources, actual invocations, bootstrap capture and inputs/
outputs. No original class modifications/GL draw/flush or assets in Git.

## Frozen renderer API and compatibility

Explicit Rust delegation C04 to separate agents/leaf-viewer worktree. Own
beta-view/src/lib.rs, beta-viewer/src/main.rs, loader.rs, shader.wgsl, README
controls/graphics notes. Do not modify core biome/colors/probe/exports owned A01.
No new dependencies. Safe Rust.

- `Scene::enable_fancy_leaves(&mut self)` opts into native fancy leaves for the
  viewer. Default Scene behavior and `mesh()` remain legacy opaque-only compatible
  with existing VIEW oracle/probe when this method is not called.
- `Scene::mesh_with_tint<F>(&self, limits:MeshLimits, tint:F)->Result<Mesh,ViewError>
  where F:FnMut(u8,u8,i32,i32)->u32` accepts block ID/metadata/world x,z and returns
  actual native palette ARGB/RGB bits. Caller supplies exact COLOR-001 climate/
  palettes; mesh has no seed/noise/I/O. RGB channels are low24 bits, alpha ignored.
- Add `tint:[f32;3]` to Vertex, white by default. Legacy mesh/UV tests still apply.
  New path colors grass top and side overlays, leaves all faces, other faces white.
  Grass side overlay emitted only when native base tile==3, faces2..5; tile38,
  same xyz and corrected UV orientation, tint inherited native biome color.
- Fancy leaves use cube faces, native tile selection52/132, metadata0..15 and
  validated cube bounds. Neighbors occlude only as actual native visibility does;
  do not cull opaque-cube faces against nonopaque fancy leaves. Leaf-leaf faces
  remain visible. Avoid reusing render opacity as collision predicate.
- Scene colliders/safe spawn/surface queries must treat enabled leaves as verified
  solid unit cubes independently of alpha. Keep query bounds and scene limits.
- GPU adds tint attribute, shader multiplies RGB by tint and existing shade/light.
  Strict texture alpha cutoff `alpha > 0.1f`; discard alpha<=0.1, preserve depth
  writes. Coplanar overlays require native-compatible depth comparison; verify
  original GL_LEQUAL setting before selecting LessEqual. No blend-alpha workaround
  that makes leaves translucent or changes material colors.
- Existing saved-light approximation remains explicit, no AO/packed-light/pixel
  equivalence claim. Preserve UV-001 native fixture tests and fixed-step motion.

Viewer load path: keep existing load_scene_for_viewer API for old probe/tests;
add `load_scene_for_viewer_with_seed(input,terrain,scene_file)` returning
`Result<(Scene,[i32;3],i64),ProbeError>`. Main uses saved level seed, loads verified
COLOR color maps from terrain parent `grasscolor.png`/`foliagecolor.png`, enables
fancy leaves, computes exact scalar climate per column (may cache), invokes
mesh_with_tint. Missing/mismatched resources are explicit actionable errors.
Asset extraction tool supplied by root writes local copies only, never modifies
JAR/Prism save. Runtime game never invokes Java/Python.

## Cards and acceptance

- C01 codex delegated native leaf/grass geometry/material fixture adapter above,
  deterministic two JVM runs, normal pure fixture checks and explicit opt-in live
  original tests (missing originals fail opt-in, not pass).
- C04 codex explicitly delegated Rust renderer integration, native-derived tests
  (leaf geometry/occlusion/collision; grass face/overlay tint; alpha threshold and
  mesh limits). Exact core color values come from COLOR-001, no guessed constants.
- C03 codex final review exact commits, full workspace check, fresh COLOR/native
  leaf evidence and UV regression, legacy view parity, GUI screenshot that shows
  green grass and tree canopies, startup read-only resource checks. Update estimate
  conservatively; graphics improvement alone need not meet requested5–10pp push.
