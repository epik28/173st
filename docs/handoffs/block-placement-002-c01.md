# BLOCK-PLACEMENT-002-C01 handoff

State: native placement corpus captured; lighting regression not reproduced.

- Added a pinned-JAR native oracle and 16 exact, repeatable cases at
  `tests/fixtures/block-placement/`.
- Original `fd.a(bt,bt,...)` ray tracing hits ID 31 from all six sampled sides.
  Thus the reported `unsupported 31` is the viewer's unsupported-state handling,
  not a native ray miss for the controlled ID-31 shape.
- Original `ob.a(gs,fd,iz,x,y,z,side)` accepts all six placement sides in the
  controlled scenes. Results include native ID 13 on ID-31 sides 2/4; preserve
  this snapshot and do not generalize it to other metadata/scenes.
- Repeat, occupied-cell, y=127, and chunk-edge cases are captured. User source
  saves and assets were not accessed or changed.
- Sky/block-light samples are all zero in this raw in-memory setup, so the
  screenshot lighting symptom remains unverified. Keep that limitation open.

See `tests/fixtures/block-placement/provenance.md` for reproduction details and
exact isolation limitations.
