# Native item metadata survey

The pinned client oracle checks JAR SHA-256 and loaded class sources and probes
all 104 IDs 256–359 at metadata 0–15. It records `iz.b()` icon, `iz.f()` subtype
flag, `iz.j()` durability, `iz.c()` stack limit, and the result of inserting one
item into an `ix` inventory containing one same-ID metadata-zero item. For coal
it also records the native translation key. The subtype flag identifies only
263 and 351; 263 metadata 0 and 1 are named coal and charcoal and use the same
icon 7. Other coal metadata preserve different merge identity in this probe but
are not enabled by the bounded model. The probe does not establish all possible
metadata values, item use behavior, or rendering pixels.

```sh
mkdir -p /tmp/item-variants-classes
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/item-variants-classes tools/oracle/src/ItemVariantsOracle.java
java -cp /tmp/item-variants-classes:../.local/minecraft/client-b1.7.3.jar ItemVariantsOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/item-variants.tsv
diff -u tests/fixtures/item_variants/native.tsv /tmp/item-variants.tsv
cargo test --locked --offline -p beta-core coal_and_charcoal_metadata_match_native_insertion
```
