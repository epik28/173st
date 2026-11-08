# LIGHT-001 native fixture provenance

The fixture was captured twice in separate JVM processes from the pinned
`client-b1.7.3.jar` (SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`). Both
stdout TSV traces and byte-for-byte audit JSON matched. The checked-in audit
replaces only the machine-specific `code_source` path with
`<configured-original-jar>`; the live adapter still verifies the resolved class
source is the configured, hash-pinned JAR before emitting output.

Capture command, from `codex/`:

```sh
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/light-oracle-classes tools/oracle/src/LightOracle.java
python3 tools/oracle/light_corpus.py > /tmp/light-input.tsv
java -cp /tmp/light-oracle-classes:../.local/minecraft/client-b1.7.3.jar LightOracle ../.local/minecraft/client-b1.7.3.jar /tmp/light-audit.json < /tmp/light-input.tsv > /tmp/light-native.tsv
```

The adapter calls original `xa.a(int)` for dimensions 0/−1, the provider's
original `e()` table initializer (base `xa.e()` for Overworld, native override
`wd.e()` for Nether), and `xa.a(long,float)`. It sets original `ei` time and
calls `fd.a(float)` for clear-weather sky subtraction. For each sample it writes
the actual `wi` sky/block nibbles, calls original `lm.c(int,int,int,int)`, then
calls original `fd.a(int,int,int,int)` at the ordinary air block coordinate.
All 256 emission results are read directly from original `uu.s[]` after the
native registry bootstrap. The `fd` instance is Unsafe-allocated without calling
either its native or synthetic Java constructor; its real provider, metadata,
chunk, and nibble arrays are attached, with clear weather fields left at their
Java zero defaults. The capture contains 8,224 direct `lm.c` calls, 8,224 direct
`fd.a(IIII)` calls, 48,064 direct calls each to provider angle, sky subtraction,
and metadata time setter, two provider factories/table initializers, and 256
emission table reads. Calls internal to original native methods are documented
as inferred, never instrumented counts.

The corpus includes all 4,096 block/sky/subtraction triples in each supported
dimension, three rotating minimum-light values plus explicit probes for every
table level, all 24,000 Overworld ticks at partial 0 and 1, long/time boundary
probes in both dimensions, and all emission IDs including glowstone 89 and
jack-o-lantern 91. It excludes special five-neighbor block queries 44/53/60/67,
dimension 1 (Skylands in this release), weather, fluids, propagation, and AO.
The original `lm.a` saved-sky-used static flag may be set by the native call and
is outside the returned numerical behavior.
