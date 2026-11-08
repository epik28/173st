# VIEW-002: native research before contract

Read-only JAR research by gpt-6-luna low, reviewed by coordinator. This is not
an implementation, frozen protocol, or accepted parity result. Original JAR SHA:
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.

Start with leaves/alpha and grass/foliage tint; postpone water, arbitrary shapes,
AO and pixel equivalence to separate measurable work.

## Native calls

- Grass `wp.b(xp,int,int,int)` `(Lxp;III)I`; leaves same signature in `bk`.
- Leaf metadata bit1 selects `jh.a()`, bit2 selects `jh.b()`; otherwise native
  `xp.a()` biome manager `xv.a(x,z,1,1)` followed by `jh.a(double,double)`.
- Instantiate original `xv(fd)` `(Lfd;)V` with synthetic fd whose `s()J` returns
  seed. Fixture world can bypass constructors via tooling-only Unsafe; call the
  real biome-manager constructor, do not copy noise initialization into Java.
  The no-argument xv constructor leaves noise fields null and is unsuitable.
- Load original palettes with ImageIO; original setters `ia.a(int[])` and
  `jh.a(int[])`. Native lookups `ia.a(DD)I`, `jh.a(DD)I` return RGB integers.
- Leaves fancy setter `bk.a(boolean)` `(Z)V`, opacity `bk.c()Z`, native virtual
  tile mapping `bk.a(face,metadata)` `(II)I` / inherited world resolver.
- `Minecraft.x()V` enables alpha test and calls GL_GREATER with cutoff0.1f:
  source alpha must be strictly greater; RGBA8 alpha>=26. Confirm render state
  at leaf pass before implementation. This is not a promise of GL/wgpu pixel
  parity, mip/filter equivalence or lighting correctness.

## Pinned resources/classes

| Resource | SHA-256 |
| --- | --- |
| misc/grasscolor.png | d9b2d9073f06013b9f36b1c61a91678be478760abff31fae942ee52ad898ca1b |
| misc/foliagecolor.png | 644a527dd7daea5a195bbed056a6a41e295b6043a05621f4c5c263a25fb171cc |
| terrain.png | 9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf |
| bk.class | e02f35d24f4cdd895f0969d016d35038d32fbef5fe610003f8b8aec1fa38dc82 |
| wp.class | 194fac825e575ea47cbec520419a188f40e633549c6451751a254d99dbf532bf |
| xv.class | 2d6974e40ffc09c47b58ae1149dd9587513aa97dd89f569640b13603836901ee |
| jh.class | 032177bb3081b236691823927c43d92f9019c332ffa9bca9ab655f8223b074df |
| ia.class | 665cb3138a4998f85f0492f8117a12ddc51781e1fedd4eb7fa119c21430c9150 |
| ug.class | adab55a06a568cceb557065fc17be32b785905802efcd7017315b432b4c78a67 |

## Contract decisions still required

Oracle inputs should include seed, signed coordinates, metadata, fancy mode and
face. Retain exact RGB/tile/opacity output and actual method invocation audit;
bind native code sources, palettes and terrain hashes. Run repeat JVM traces.
Use actual original native biome noise; Rust must compute matching values rather
than consume a fixture lookup table. Prefer layered acceptance: palette lookup,
biome climate, then renderer integration, avoiding a large opaque package.

Cases: seeds0/1/-1/i64 extrema; negative coordinates and biome boundaries;
all leaf metadata bits and precedence; repeated coordinates across seeds;
y-coordinate invariance; every face/fancy combination; 25/26 alpha threshold
and atlas tile bleed/culling. Verify fancy tile differences from real calls,
not a guessed uniform offset. New protocol/domain needs a frozen contract.
