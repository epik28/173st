# Native cube-face UV capture

This fixture captures the original Beta 1.7.3 standard-cube face submissions to
the CPU Tessellator buffer. The client JAR is pinned to SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.

`UvOracle.java` verifies that `cv`, `nw`, and `uu` load from that JAR, then
invokes the original `cv.a` through `cv.f` face methods with stone (`uu.m[1]`),
an explicit face tile, and both positive and negative block origins. These
methods submit through the original `nw.a(double,double,double,double,double)`
method into the Tessellator's CPU `int[] g` buffer. The capture reads the
native float32 bit patterns from slots 0–4 (x, y, z, u, v) and never calls the
Tessellator flush/draw method. No OpenGL context or framebuffer is involved.

Each face method submits six vertices as two triangles. The fixture preserves
all six submissions per case; the first and fourth positions repeat, and the
third and fifth positions repeat. Repeated positions carry identical UV bits.
There are four unique xyz corners per face. The 84 cases cover faces 0–5, atlas
tiles 0, 15, 16, 20, 37, 116, and 255, and origins `(2,3,4)` and `(-2,-3,-4)`.
Tiles 15/16 and 255 include the last atlas column/row boundaries.

The native bootstrap writes `16 achievements\n151 recipes\n`; its exact bytes
are in `bootstrap.txt` (28 bytes, SHA-256
`0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e`). The
runtime LWJGL2 helper JAR is used for class resolution only; its SHA-256 is
`833e721817f70d1445eec13d8ce5a86e12af70efac5014356302e2bbc68b3fe2`.
Native class hashes are emitted in the header of `native.tsv`:

- `cv`: `ce8f363e1f7566530ac31680eabb8c715d9ddc7c97f31cf0a62f953e069a6863`
- `nw`: `2facc45feb91d0fea0c5888ac6793a87ffa6e4cdf5b0cc4e418c6c162ca8bf33`
- `uu`: `caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845`

The checked-in `native.tsv` SHA-256 is
`6a103ab37d24e8d55ca212fb475257d42c704ad60b85d0ff93e253195941156f`; the
input `cases.tsv` SHA-256 is
`6c6e64841c0533cea561e5e155d48cfca7ae6e85bafff6e742b42e36a8b33b93`.

## Confirmed mismatch in the previous Rust UV association

For tile 37, the native bounds captured in float32 are
`u0=0x3ea00000`, `u1=0x3ebffae1`, `v0=0x3e000000`, and
`v1=0x3e3ff5c3`. At origin `(2,3,4)`, face 1's top corner
`xyz=(3,4,5)` is submitted by the native renderer with `(u1,v1)`. The previous
`append_face` association gave that same xyz `(u1,v0)`.

On face 3, corner `xyz=(2,4,5)` is submitted natively with `(u0,v0)`; the
previous Rust association gave it `(u1,v0)`. The other four standard cube
faces (0, 2, 4, 5) match the captured xyz-to-UV association in this corpus.
This accounts for textures appearing rotated on selected faces while the tile
lookup itself remains correct.

The capture intentionally stops at the native Tessellator submission buffer.
It does not render pixels, sample the atlas, test block visibility/culling, or
exercise `cv.l`'s complete block dispatch. The face UV association is independent
of those later stages; this is a CPU submission reference, not a pixel-parity
claim. Exact pre-narrowing Java double arguments are not present in the native
buffer, so this fixture records the renderer's actual submitted float32 bits.
