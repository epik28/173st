# VIEW-001 C01 source notes

Native input: configured local Beta 1.7.3 client JAR, pinned SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
The extracted original `terrain.png` is 256×256 (65,050 bytes), SHA-256
`9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf`; it remains
outside Git at the configured local path.

Native investigation identifies `uu` as the Block registry/class and `cv` as the
standard renderer. The relevant path is `cv.a(uu,x,y,z)` → `cv.b(uu,x,y,z)` →
`cv.l(uu,x,y,z)` for render type 0. It dispatches face-specific atlas lookup via
the virtual renderer-facing `uu.a(world,x,y,z,face)`. The base implementation
delegates to `uu.a(face,metadata)`, but subclasses can override the world-aware
method; the oracle calls the virtual method so those overrides are retained. For
example, grass ID 2 over air resolves tiles `2,0,3,3,3,3`; when water or snow is
above it, its side tile comes from the world-aware override. The fixture records
the above-block ID for those mappings. It checks neighbor visibility
through `uu.b(world,nx,ny,nz,face)`, and obtains local/neighbor brightness through
`uu.d(world,x,y,z)`. Face directions are bottom/top, negative/positive Z,
negative/positive X (0..5). The renderer bytecode contains fixed side shade factors
1.0, 0.5, 0.8 and 0.6, and converts the 16×16 tile into the 16×16 atlas with a
0.01-pixel inset at the upper UV edge. Native block methods are called in the
original JAR; a minimal `xp` proxy supplies scene metadata and neighbor opacity.
Bytecode inspection of native `fd.g(x,y,z)` confirms neighbor opacity comes from
the registry block's `uu.c()` result. The adapter does not execute `cv` vertex
emission; vertex output remains smoke-test/algorithm evidence, not exact native
vertex parity.
Pinned class SHA-256 values: `uu.class` =
`caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845`;
`cv.class` = `ce8f363e1f7566530ac31680eabb8c715d9ddc7c97f31cf0a62f953e069a6863`.

Workspace dependencies are exact pins in `Cargo.toml`/`Cargo.lock`:

| Crate | Version | License | MSRV | Features used |
| --- | ---: | --- | ---: | --- |
| `wgpu` | 30.0.1 | MIT OR Apache-2.0 | 1.87 | std, parking_lot, dx12, metal, gles, vulkan, wgsl; no webgpu |
| `winit` | 0.30.13 | Apache-2.0 | 1.70 | defaults: X11 and Wayland |
| `image` | 0.25.10 | MIT OR Apache-2.0 | 1.88 | PNG only; defaults off |
| `bytemuck` | 1.25.2 | Zlib OR Apache-2.0 OR MIT | — | derive |
| `glam` | 0.34.2 | MIT OR Apache-2.0 | 1.85 | defaults |
| `pollster` | 0.4.0 | Apache-2.0/MIT | — | defaults |

All selected versions support the repository Rust 1.95 toolchain. The direct crates
are permissively licensed; full transitive license inventory remains a release
packaging check, not a VIEW-001 C01 blocker.

Evidence commands (read-only, local JAR):

```text
javap -classpath <client.jar> -p uu
javap -classpath <client.jar> -p cv
javap -classpath <client.jar> -c -p cv
javap -classpath <client.jar> -c -p uu
```

`tools/oracle/view_corpus.py` executes the original `ij`, `ei`, `qj`, and `uu`
methods on two identical copied-world runs. It records 25 scene slots, native
metadata/NBT/region/block method call counts, and every source/copy hash before and
after. `live.tsv` contains 46 observed block states, 50 face-tile mapping cases
(4 world-aware context variants), and 1,617 observed neighboring state/face cases;
20 mappings meet the initial opaque full-cube, render-type-0,
fully-opaque-texture filter. Thirty mappings are explicitly unsupported.
`synthetic.tsv` separately selects metadata, above-block contexts, liquids, transparent/cutout textures,
partial models, and boundary IDs for C02 native protocol coverage. No Rust parity
has been run yet.
