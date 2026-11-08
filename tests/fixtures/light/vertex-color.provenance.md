# Native packed vertex-color capture

This supplemental capture records the original Beta 1.7.3 `cv.b(uu,III,FFF)`
flat-block renderer feeding `nw.a(FFF)` and the resulting packed `nw.g` color
word. It isolates renderer arithmetic; it does not claim a native saved-light
world simulation. The `xp` dynamic proxy returns the listed f32 brightness for
the original `uu.d(xp,III)` call and false/zero for unrelated world queries.
No world/chunk load, GL draw, or GL flush occurs.

The capture makes 12 direct renderer calls: block IDs 1 (stone) and 2 (grass),
brightness raw f32 values `3e800000` (0.25), `3f000000` (0.5), and `3e85b05c`
(the original Overworld brightness table's level 8), with `cv.a` layer flag 0
and 1. Tint inputs are raw f32 `3dfbe76d`, `3ee978d5`, `3f49fbe7`, the Java
float constants 0.123f, 0.456f, and 0.789f. Each packed vertex row records block,
brightness, `cv.a`, native face emission order, base/overlay layer, vertex
ordinal, tint bits, and the packed color word. Faces are labeled in `cv.b`
bytecode emission order: down, up, north, south, west, east; the grass overlay
emits an extra quad after each of north, south, west, and east.

The bytecode establishes two distinct operation orders. The ordinary face path
forms `shade * tint` first and then multiplies by the result of `uu.d(...)`
(brightness), as seen at `cv.b` offsets 92–153 followed by 212–227, 324–339,
and the corresponding later face blocks. The grass overlay path forms
`shade * brightness` and then multiplies by tint (offsets 494–518, 662–686,
830–854, and 998–1022). `nw.a(FFF)` multiplies each channel by `255.0f` and
converts with `f2i` (truncate toward zero); `nw.a(III)` clamps each channel to
0..255 and packs the channels. For example, at brightness 0.25, stone's down
face packs `ff190e03`, while grass north base packs `ff333333` and its north
overlay packs `ff281706`.

The source adapter is [`VertexColorOracle.java`](../../../tools/oracle/src/VertexColorOracle.java).
It verifies the pinned JAR SHA and class code sources, retains original class
hashes and bootstrap bytes in the capture header, and counts actual adapter
calls. The fixture was generated twice in separate JVM invocations and compared
byte-for-byte (`cmp`); each output reports `cv_b=12`, `xp_brightness=84`,
`packed_vertices=504`, `gl_draw=0`, and `gl_flush=0`.

Reproduction from the `agy/` worktree:

```sh
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/vertex-color-oracle \
  tools/oracle/src/VertexColorOracle.java
java -cp /tmp/vertex-color-oracle:../.local/minecraft/client-b1.7.3.jar \
  VertexColorOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/vertex-color.tsv
```

Fixture SHA-256: `9b8c833645d6a8fd4fb546acbb71e7fc282ebdefd20ed1f6630258dd9fa8aa0d`.
Adapter source SHA-256: `3c7591381539035b8117fa45dc5edb84aaf7f168aa255c79289bb250bdd33922`.
Pinned client JAR SHA-256: `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
