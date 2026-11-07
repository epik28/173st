# Java-эталоны

Область Codex. Первая реализация — `src/RandomOracle.java` по
[RNG-001](../../docs/tasks/RNG-001.md). Адаптер читает TSV из stdin и вызывает
`java.util.Random.nextInt()`. Проверка: `python3 tools/oracle/verify.py`.

Java-эталон вызывает настоящие Java API или подготовленный код оригинальной Beta.
Исходники адаптера хранятся здесь; JAR, `.class` и результаты сборки — в локальных
игнорируемых каталогах. Требования к происхождению эталона — в
[методике проверок](../../docs/parity-testing.md).

RNG-002: `src/RandomExtendedOracle.java`, входы `tests/fixtures/rng/extended.tsv`.
`python3 tools/oracle/extended_corpus.py` проверяет корпус, компиляцию,
детерминированность, границы/ошибки и отдельную синтетическую rejection-мутацию.
`--write` явно воспроизводит входной fixture, не сохраняет ожидаемые ответы.

CHUNK-001: `src/ChunkOracle.java` reflection-вызов оригинального класса `wi`.
`python3 tools/oracle/chunk_corpus.py` проверяет корпус и настоящий JAR из
`paths.client_jar`; `--write` воспроизводит входы. Имена и хеши опубликованы в
`tests/fixtures/chunk/provenance.md`; JAR и disassembly остаются вне Git.
