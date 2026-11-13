# Native fall-damage request

The adapter calls the original `ls.b(float)` on an in-memory synthetic living
entity from the pinned Beta 1.7.3 client and records the requested damage.
The fixture covers eleven finite, nonnegative distances including the adjacent
floating-point values around three and four blocks. It does not cover health,
armor, water, invulnerability, landing detection, or the player update loop.

```sh
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/fall-damage-classes tools/oracle/src/FallDamageOracle.java
java -cp /tmp/fall-damage-classes:../.local/minecraft/client-b1.7.3.jar FallDamageOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/fall-damage.tsv
diff -u tests/fixtures/fall_damage/native.tsv /tmp/fall-damage.tsv
cargo test --locked --offline -p beta-core fall_damage
```
