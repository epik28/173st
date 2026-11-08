# COLOR-001 native snapshot provenance

The fixture captures calls through the pinned Minecraft Beta 1.7.3 client JAR, SHA-256 `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`. The oracle calls the JAR's original `xv.a(IIII):[Lkd;` climate method and reads its `xv.a:[D` temperature and `xv.b:[D` rainfall fields. It then calls original `wp.b(xp,III):I` and `bk.b(xp,III):I` for climate-derived colors. Separate palette rows call original `ia.a(DD):I` and `jh.a(DD):I` after invoking their original `a([I)V` setters with PNG pixels decoded by `ImageIO` in row-major ARGB order.

The synthetic world is an `fd` subclass allocated with `Unsafe.allocateInstance`, with only its seed field initialized. This avoids all world constructors; the oracle verifies that all native audited classes share the configured JAR code source and records class-byte hashes and call counts in `native.audit`. The fixture stores the code source as `<configured-original-jar>` so its bytes are portable. Runtime recapture still reports and verifies the actual code-source URI.

The corpus has 1,280 climate rows (five signed 64-bit seeds × 256 coordinates) and 320 direct palette rows. It covers zero, positive, negative, and extreme seeds; coordinate boundaries; and temperature/rainfall edge values. Two independent JVM runs produced identical output and audit bytes. The native output's SHA-256 is `09301df08d99dfbe094b348d3e6127479090f614e2cbd9542ea1aa284d385944`; the audit retains pinned JAR/resource hashes, audited class hashes, exact API signatures, dimensions, bootstrap bytes, and call counts.

Counts are measured direct oracle calls only: 1,280 explicit climate calls, 1,280 explicit grass block calls, 5,120 explicit leaf block calls, 320 explicit grass palette calls, 320 explicit foliage palette calls, and 15 seed reads across five manager constructions. `wp.b` and `bk.b` may themselves call the climate and palette methods; those internal invocations are deliberately labeled `not_instrumented` and are not included in the direct-call counts.

The native registry bootstrap emitted these exact 28 bytes, also recorded as base64 in the audit:

```text
16 achievements
151 recipes
```

Representative native rows are included below. Climate columns are raw IEEE-754 `f64` bits followed by grass ARGB and leaf ARGB for metadata 0–3.

```text
c00-000  seed=0, (0,0): temp=3fed65e1be63da72 rain=3fe085574d0ece45 grass=ff85c14b leaves=ff67b01e,00619961,0080a755,00619961
c01-002  seed=1, (-1,-1): temp=3fefd4d62721faed rain=3f6015f6a970b101 grass=ffbeb755 leaves=ffada42a,00619961,0080a755,00619961
c02-002  seed=-1, (-1,-1): temp=3fed2503a999acff rain=3fa8aaf1ee46f9b3 grass=ffb4b859 leaves=ffa2a52f,00619961,0080a755,00619961
c03-004  seed=INT64_MIN, (-30000000,-30000000): temp=3fe8fb1f197a4d84 rain=3ff0000000000000 grass=ff64c54d leaves=ff3eb521,00619961,0080a755,00619961
c04-003  seed=INT64_MAX, (30000000,30000000): temp=3fece43df42c4191 rain=3fe3a25aaa2259ae grass=ff7bc349 leaves=ff5ab21c,00619961,0080a755,00619961
```

Snapshot hashes:

- `cases.tsv`: `3287b59d88ea22487b08451031ad08e982fa00663c2e4c910abaa740134baff2`
- `native.tsv` and `native-repeat.tsv`: `09301df08d99dfbe094b348d3e6127479090f614e2cbd9542ea1aa284d385944` (identical)
- `native.audit` and `native-repeat.audit`: `d91434066c270e4e76c55db98e6973b2ae18bff8fa81e8384cd4cc008dbf7dde` (identical after source-path normalization)
- Grass resource: `d9b2d9073f06013b9f36b1c61a91678be478760abff31fae942ee52ad898ca1b`
- Foliage resource: `644a527dd7daea5a195bbed056a6a41e295b6043a05621f4c5c263a25fb171cc`
