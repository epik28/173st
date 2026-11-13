# Native fall-damage request

The adapter calls the original `ls.b(float)` on an in-memory synthetic living
entity from the pinned Beta 1.7.3 client and records the requested damage.
The fixture covers eleven finite, nonnegative distances including the adjacent
floating-point values around three and four blocks. It does not cover health,
armor, water, invulnerability, landing detection, or the player update loop.
`native_landing.tsv` separately records seven successive calls to the native
entity fall accumulator, including two landings and repeated grounded input.
`native_health.tsv` records five damage events on an unarmored player. The
cooldown is set explicitly by the oracle; timer progression is not captured.
`native_health_ticks.tsv` captures one landing and 21 `ls.U` ticks in an
isolated player setup; water and lava checks are stubbed and this is not a full
player-update or death simulation.

```sh
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/fall-damage-classes tools/oracle/src/FallDamageOracle.java
java -cp /tmp/fall-damage-classes:../.local/minecraft/client-b1.7.3.jar FallDamageOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/fall-damage.tsv
diff -u tests/fixtures/fall_damage/native.tsv /tmp/fall-damage.tsv
cargo test --locked --offline -p beta-core fall_damage
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/fall-damage-classes tools/oracle/src/LandingOracle.java
java -cp /tmp/fall-damage-classes:../.local/minecraft/client-b1.7.3.jar LandingOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/landing.tsv
diff -u tests/fixtures/fall_damage/native_landing.tsv /tmp/landing.tsv
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/fall-damage-classes tools/oracle/src/FallHealthOracle.java
java -cp /tmp/fall-damage-classes:../.local/minecraft/client-b1.7.3.jar FallHealthOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/fall-health.tsv
diff -u tests/fixtures/fall_damage/native_health.tsv /tmp/fall-health.tsv
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/fall-damage-classes tools/oracle/src/FallHealthOracle.java tools/oracle/src/FallHealthTickOracle.java
java -cp /tmp/fall-damage-classes:../.local/minecraft/client-b1.7.3.jar FallHealthTickOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/fall-health-ticks.tsv
diff -u tests/fixtures/fall_damage/native_health_ticks.tsv /tmp/fall-health-ticks.tsv
```
