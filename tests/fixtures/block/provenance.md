# BLOCK-001-C01 native evidence

The client JAR is `../.local/minecraft/client-b1.7.3.jar`, SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
`audit.json` pins the SHA-256 values of the loaded `fd`, `vf`, `bt`, `uu`,
`ob`, `pj`, `iz`, and `gm` class bytes and records that every class came from
that exact JAR. It records the native methods `fd.a(bt,bt,boolean,boolean):vf`
and `uu.a(fd,int,int,int,bt,bt):vf`; `vf` exposes the hit type, block xyz,
side, and `bt` hit point. `ob.b():float` returns `5.0f`, and `pj` extends `ob`.

Reproduce with `python3 tools/oracle/block_corpus.py` (two native JVM captures)
or regenerate the reviewed fixture with `python3 tools/oracle/block_corpus.py
--write`. `BlockOracle.java` creates each scene in a fresh in-memory world using
native `fd` ray tracing and per-block native shape dispatch. The process checks
all source class code locations and hashes the class bytes. It does not open a
save, Prism instance, JAR for writing, or assets. `native.tsv` stores live rows
and raw IEEE-754 64-bit words for each hit coordinate. The tie case preserves
native side 4; slab shape hits at y=0.25; the three range cases show the block
at x=5 missed at endpoints 4.999 and 5.0 and hit at 5.001.
The `liquid_stop_false` / `liquid_stop_true` pair calls the same native ray
method against one in-memory water block and confirms the switch changes miss
to a block hit.

The regular `fd(wt,String,long,xa)` constructor attempted default spawn setup
and failed in this headless provider before a scene was available. The harness
uses the native `fd(fd,xa)` copy constructor after allocating a seed holder and
setting only its provider, dimension, and in-memory metadata; it then inserts
fixture chunks backed by private byte arrays. The audit records zero native
world constructors, meaning no loaded/generated/saved world constructor was
invoked. Every ray itself goes through the original client method.

Creative break and placement remain **not verified** in C01. No native action
output is included or implied. A complete isolated player/controller adapter
was not established, so inventory changes, neighboring block IDs/metadata,
stack count/damage, and controller action side effects have no native evidence.
This corpus establishes native targeting evidence only.
