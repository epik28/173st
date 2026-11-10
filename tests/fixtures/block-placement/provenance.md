# BLOCK-PLACEMENT-002-C01 native placement evidence

`native.tsv` is produced by `python3 tools/oracle/placement_corpus.py`; use
`--write` only when intentionally updating the reviewed capture. The generator
checks the client JAR SHA-256 from BLOCK-001, compiles in a temporary directory,
and executes four fresh in-memory captures (two in each JVM process). Captures
must be byte-identical. `audit.json` records JAR and class hashes, Java source
hash, case count, repeat result, and isolation facts.

Each case gets a newly allocated `fd(fd, xa)` over a map provider whose chunks
are fresh byte arrays. The regular `fd(wt, String, long, xa)` path, which
initializes spawn, is never invoked. The selected stack is exactly item/block ID
1 (`new iz(uu.m[1], 4)`). The native `ob.a(gs,fd,iz,x,y,z,side)` method drives
the real item and block dispatch. The isolated player is an Unsafe-allocated
minimal subclass whose only overrides are the abstract tick and sound callback;
the latter is a no-op, so there is no Minecraft/UI/audio object. The `ob`
controller is also allocated without its Minecraft constructor; the invoked
method reads only its arguments. Save methods are no-ops and filename lookup
returns null. No Prism save, source JAR, or source assets are opened for writing.

The 16 cases cover all six faces against stone (ID 1) and tall grass (ID 31),
one repeat click, an occupied destination, placement across x=15→16, and the
top build boundary. For each case, the fixture records the original native ray
hit kind/side, action result(s), adjacent ID+metadata before/after, all 27
neighbor ID+metadata states before/after, stack count/damage, height-map value,
and sky/block light samples.

Native ray tracing hits ID 31 on all six sampled faces. Therefore `unsupported
31` in the viewer is not evidence that the original lacks a target hit; it is
the Rust viewer's current unsupported-state classification. All six original
item placements succeed in this disposable case. Clicking ID 31 replaces it
with air and consumes one item; sampled side 2 and side 4 place native ID 13 in
the adjacent cell, while the other sampled sides place ID 1. The captured
action is kept verbatim; no orientation or item mapping formula is inferred.
The second top click is rejected with no additional stack change; an occupied
destination and y=127 placement are rejected; the x=15 east-face placement
succeeds into x=16.

Lighting samples are zero in these raw, minimally initialized chunks. Height
map values are retained, but this corpus does not reproduce a normally lit
world and cannot establish the user's dark-shadow/lighting report. That native
lighting reproduction remains open under LIGHT-UPDATE-002; A01 must not claim
light parity from these values.
