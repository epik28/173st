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

HUD-001: `python3 tools/oracle/hud_audit.py` verifies the pinned Beta 1.7.3
client JAR, exact hashes and PNG dimensions for `gui/gui.png` and
`gui/icons.png`, then checks the `uq.a(FZII)V` native HUD method and stored
bytecode excerpts against `tests/fixtures/hud/audit.json`. The fixture records
JAR/class provenance and hotbar, selected slot, crosshair, and full-health heart
layout. Original assets and disassembly are not copied into Git. `GuiIngame` is
the MCP name; the pinned JAR itself contains obfuscated class `uq` and no usable
Java source file.

WATER-SIM-001 captures the original `om`/`nx` liquid block updates through
`tools/oracle/water_tick_corpus.py`. Its nine C01 cases remain frozen separately
from the source-formation extension in `tests/fixtures/water_tick_sources/`.
With the pinned client JAR configured, `--live --write` recaptures the original
C01 fixture and `--source-live --write` captures the extension with two
independent runs; `--sources` audits the committed extension without starting
Java.
