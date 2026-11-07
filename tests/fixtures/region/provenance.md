# REGION-001 input and native reference provenance

The four pinned local sources are described by `sources.json`. The fixture
`live.tsv` contains coordinates only: 704 occupied and 44 sampled empty slots.
No world bytes, original class files or decompiled sources are committed.

The original Beta 1.7.3 client JAR SHA-256 is
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
Native class `qj.class` SHA-256:
`6c75a123b63d555caa98e1a2619aa2483e405566de9e356585b1dc682a7da4a6`.
`RegionOracle` checks the class code source and invokes the original constructor
`qj(File)`, stream reader `a(int,int)`, and close `b()`. Timestamp/location metadata
comes from the native private fields `e`/`d`; compression is read from the record
header. Payload decompression is performed by the original returned stream.

The original constructor opens files in `rw` mode and may repair small or
unaligned files. Our wrapper requires a fresh snapshot marker, confines basenames
to the copy directory, rejects symlinks, short/unaligned/oversized files, and checks
the copies' hashes after native close. It never opens the Prism originals.
TSV/ID/path guards, metadata extraction, output cap and hashing are our wrapper,
not original-game behavior. An occupied native-null result is an error.

Reproduce C01: `python3 tools/oracle/region_corpus.py --write`.
Each run makes its own `artifacts/oracle-region/run-*/input`, records pinned source
hashes before/after copy and after execution, and saves all 704 raw NBT payloads.
Two full native runs must agree exactly. `audit.json` records native invocations
and separately labelled generated cases; Rust comparison is not claimed by C01.
`coordinates.tsv` covers i32 endpoints and -33,-32,-31,-1,0,31,32,33 on generated
files, separate from the real world.

Generated cases exercise gzip/zlib, exact sector and maximum record lengths,
sector padding, truncated streams, checksums, output cap, invalid locations,
count/length, overlap and unsupported compression. Native gzip reads consecutive
members. Both gzip and zlib ignore bytes after a complete stream. Rust retains
the contract's stricter rejection of overlaps and record lengths that exceed
the allocated sectors; original qj accepts those two generated cases.

## Dependency provenance

`flate2 =1.1.9`, MIT OR Apache-2.0, `default-features=false`, `rust_backend` only.
The pinned backend is `miniz_oxide 0.8.9` (MIT OR Zlib OR Apache-2.0), with
`adler2 2.0.1` (0BSD OR MIT OR Apache-2.0), `simd-adler32 0.3.8` (MIT),
`crc32fast 1.5.0` and `cfg-if 1.0.4` (MIT OR Apache-2.0).
[Pinned flate2 documentation](https://docs.rs/flate2/1.1.9/flate2/) confirms this
backend's pure Rust implementation and absence of a C compiler requirement.

Probe-only dev-dependency `sha2 =0.10.9` (MIT OR Apache-2.0) supplies SHA-256,
avoiding a new handwritten cryptographic implementation in the probe. Its locked
dependencies are digest 0.10.7, block-buffer 0.10.4, crypto-common 0.1.7,
generic-array 0.14.7, typenum 1.19.0, cpufeatures 0.2.17, libc 0.2.186 and
version_check 0.9.5. All declare MIT or MIT/Apache-2.0 alternatives; metadata and
license files were inspected in the local registry. No dependency code was copied
or vendored into this repository; Cargo.lock retains upstream checksums.

All dependencies resolve/build offline from the existing cache. A fresh machine
runs `cargo fetch --locked` once before `cargo check --workspace --all-targets
--locked --offline`. No code was copied from mc173 or other reference engines.
