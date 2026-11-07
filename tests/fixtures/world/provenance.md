# WORLD-LOAD-001 C01 native corpus

The 749 rows are coordinates: the original level.dat plus the 748 selected slots from REGION-001. Expected outputs are produced by `tools/oracle/src/WorldOracle.java` using the pinned Beta 1.7.3 client JAR. The adapter calls native `ij.b(DataInput)`, `ei(nu)`, and `qj.a(int,int)`; `native_chunk_loads=0` and entity/tile factories are never called. `live.tsv` contains no expected answers or world bytes. Artifacts, class files, copied saves, and retained raw NBT stay ignored under `artifacts/`.

`sources.json` pins five source files. The runner compares their exact sizes and SHA-256 before copy, after copy, and in `finally` after native execution. See `audit.json` for class/JAR hashes, call counts, generated native semantics checks, and the exact live row digest.
