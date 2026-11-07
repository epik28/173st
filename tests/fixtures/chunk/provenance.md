# CHUNK-001 reference provenance

Original: user's local Minecraft Java Edition Beta 1.7.3 client JAR, previously
copied from PrismLauncher and checked against its version metadata. Local path is
`paths.client_jar` in config/local.toml; no original binary is committed.

- JAR SHA-256: `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`
- Class: `wi`; class bytes SHA-256:
  `ef5df5cdcd5ba63d8802f5bc539c8e164681eb173ef74586c369ae483415bb50`
- Constructor `wi(int)` invoked with 32768; original raw field `a:byte[]`.
- Original getter `a(int,int,int):int`, setter `a(int,int,int,int):void`.
- Inspected using `javap -private -c -classpath <client-jar> wi`; local disassembly
  remains ignored under artifacts/chunk-research/wi.javap.txt. Class discovered by
  scanning classfile descriptors for `[B`, `(III)I`, `(IIII)V`; no decompiler output
  or copied original implementation is added to Git.

ChunkOracle uses an isolated URLClassLoader, verifies JAR SHA and class code source,
then calls constructor/get/set and reads the raw field through reflection. Its
self-test writes 3 and 12 to adjacent nibbles and observes original raw byte 195.
Actual fixture invocations are counted separately in native-invocations.json.
No LWJGL or other runtime dependencies are needed by this original class.

Our wrapper supplies bounds/argument validation and atomic rejection, Blocks bytes,
zeroed container initialization, Java `>>4`/`&15` coordinate split and contracted
block_index arithmetic. These are wrapper/API checks; only nibble calls and raw
nibble arrays are observations of the original client. This is not a game/world test.

Reproduce inputs: `python3 tools/oracle/chunk_corpus.py --write`.
Audit without changing fixture: `python3 tools/oracle/chunk_corpus.py`.
4182 operations cover coordinates including i32 limits; x/z/y boundaries; both halves
of bytes; all nibble values; IDs 0/127/128/255; adjacent columns; repeated/failed writes;
neighbor/raw reads and a fresh zeroed container. No expected answers come from Rust.
