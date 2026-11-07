# Java-эталоны

Область Codex. Первая реализация — `src/RandomOracle.java` по
[RNG-001](../../docs/tasks/RNG-001.md). Адаптер читает TSV из stdin и вызывает
`java.util.Random.nextInt()`. Проверка: `python3 tools/oracle/verify.py`.

Java-эталон вызывает настоящие Java API или подготовленный код оригинальной Beta.
Исходники адаптера хранятся здесь; JAR, `.class` и результаты сборки — в локальных
игнорируемых каталогах. Требования к происхождению эталона — в
[методике проверок](../../docs/parity-testing.md).
