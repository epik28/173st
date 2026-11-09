# Relight column-cache performance handoff

`BlockScene::relight` now caches the highest opaque block in each touched `(x,z)` column for the duration of one relight call. The direct-sky check previously re-read the blocks above every candidate cell. The cache scans each queried column once, using the same deterministic material callback and the existing y=1..127 sky domain. A fully transparent column uses top `-1`; an incomplete/unloaded column remains non-sky-visible. This leaves propagation, queue order, validation, dirty tracking, and commit behavior unchanged.

## Reproduction

From the repository root, with the pinned fixtures present:

```sh
cargo run --locked --offline --release --example relight_bench
```

The example builds three complete material-prestate scenes from `tests/fixtures/light_material/native_edits.tsv` and five light-update scenes from `tests/fixtures/light_update/native.tsv`. It asserts each replacement block, checks all 19 material and all 63 light-update output samples, hashes the complete ordered scene and full `RelightResult`, and runs seven repeats of 60 edits for each material scene. Set `RELIGHT_DUMP_DIR=/tmp/relight-dump` to write canonical full scene/result dumps for byte comparisons. Callback counts are measured in a separate invocation so instrumentation does not affect timings.

The baseline run used the original `ccb2b28` implementation and the same corrected harness. The optimized run used the cache. Input scene hashes, output scene hashes, and result hashes matched exactly for all eight scenarios. The full serialized scene and result files also compared byte-for-byte. The three timed scenes each visited 1,152 cells and made one block update.

## Results

Values are median p50 microseconds across the seven repeat-level p50s on this machine; each repeat contains 60 edits. Timing is sensitive to local machine load, while the digest and callback counts are deterministic.

| Scene | Before | Cached | Material calls before → cached |
| --- | ---: | ---: | ---: |
| canopy log removal | 1,237 µs | 520 µs | 37,179 → 1,702 |
| water removal | 1,273 µs | 533 µs | 37,440 → 1,737 |
| water placement | 1,286 µs | 683 µs | 37,377 → 1,736 |

No simulation parity claim is made beyond the committed material and light-update fixture outputs. The harness is a focused measurement for these edit shapes, not a general workload benchmark.
