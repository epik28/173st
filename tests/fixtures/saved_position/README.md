# Native saved player position

`native.tsv` contains raw f64 words from an in-memory native player NBT
roundtrip. The oracle constructs real `gs` players using a synthetic `fd`
instance allocated without a world constructor; it writes and reads native
`nu` data without opening or modifying a world. The oracle checks the pinned
JAR SHA-256 and native class code sources. It records the saved `Pos`, loaded
bounding box Y bounds, and native y offset. This single case verifies the
conversion for the sampled player state; it does not cover every entity state.

```sh
mkdir -p /tmp/saved-position-classes
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/saved-position-classes tools/oracle/src/SavedPositionOracle.java
java -cp /tmp/saved-position-classes:../.local/minecraft/client-b1.7.3.jar SavedPositionOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/saved-position.tsv
diff -u tests/fixtures/saved_position/native.tsv /tmp/saved-position.tsv
cargo test --locked --offline -p beta-viewer saved_position
```
