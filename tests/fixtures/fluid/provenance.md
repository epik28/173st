# FLUID-001 C01 native source and geometry corpus

`native.tsv` was captured twice by `FluidOracle` against the pinned Beta 1.7.3
client JAR (`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`).
The adapter directly calls `cv.b(uu,int,int,int)` for 44 isolated synthetic water
scenes and records the actual eight integer words written per vertex in original
`nw.g`. These rows are native output, not values computed by a Java reimplementation.

The matrix covers still/flowing water IDs 8/9 at metadata 0..15 and 12 additional
neighbor arrangements (cardinal source/flow/solid, diagonal water, four sources,
water above, and a falling column). It contains 1,242 submitted vertices. Both
independent JVM executions matched byte-for-byte; the output SHA-256 is recorded in
`audit.json`. Original class hashes and registry bootstrap output are embedded in
the fixture.

The synthetic `xp` proxy provides a tiny block/material/light environment. It does
not instantiate a native World or Chunk. The adapter does not call GL draw or flush;
it resets only the original Tessellator's private batch counters between direct
renderer calls so the native vertex words can be read without submitting a frame.
This corpus establishes renderer geometry and vertex attributes only. It does not
establish GPU blending, full-scene water parity, fluid simulation, or lava behavior.
