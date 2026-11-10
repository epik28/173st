# Сравнительные проверки

Область Codex. `python3 tools/parity/run.py list` показывает доступные группы.
`python3 tools/parity/run.py rng --strict` выполняет [RNG-001](../../docs/tasks/RNG-001.md):
собирает Java-эталон и Rust-probe, подаёт один сохранённый вход, сравнивает точные
байты и структуру TSV. `--timeout 60` задаёт тайм-аут каждого процесса в секундах.

Каждый запуск создаёт отдельный каталог `artifacts/parity/rng/run-*/` (корень
переопределяется `paths.artifacts` в `config/local.toml`). Там сохраняются вход,
все stdout/stderr, исполняемый probe, Java-класс и `report.json` с хешами, версиями,
ревизией, dirty-статусом и первым расхождением. При изменении исходников или входа
во время проверки результат становится `error`.

Отсутствующий Rust-probe — `not_implemented`; Java всё равно реально запускается.
В strict-режиме любой статус кроме `pass` даёт ненулевой код. Неизвестная группа,
ошибка сборки/запуска и тайм-аут также завершаются ненулевым кодом.
Тесты в `tests/python/test_rng_runner.py` используют явно синтетические данные;
они не подтверждают совпадение реальных процессов.

`python3 tools/parity/verify_report.py <report.json> --protocol` проверяет хеши
сохранённых входа и выходов, обнаружение подмены одного значения в отдельной
синтетической копии и обработку неверного TSV обоими сохранёнными адаптерами.
Дополнительные отчёты лежат рядом с исходным; его live-результат не перезаписывается.

Методика и статусы: [parity-testing.md](../../docs/parity-testing.md).
`tools/check.py` проверяет workspace и не заменяет этот runner.

RNG-002: `python3 tools/parity/run.py rng-extended --strict` использует
`RandomExtendedOracle` и `rng_extended_probe`, сохраняет точные IEEE-биты
float/double и общее состояние цепочки. Проверка сохранённого отчёта/протокола:
`python3 tools/parity/verify_report.py <rng-extended-report.json> --protocol`.
Она проверяет синтаксис, лимиты, invalid_bound/reseed и подмену копии результата.
Стандартный `tools/check.py` теперь выполняет all-targets (включая example-тесты)
и отдельный cargo test --doc.

CHUNK-001: `python3 tools/parity/run.py chunk --strict` читает `paths.client_jar`
из config/local.toml, проверяет JAR и вызывает оригинальный класс через ChunkOracle.
В отчёте отдельно сохраняются native metadata/invocation counts и wrapper scope;
JAR и исходники проверяются на неизменность до/после запуска.
`python3 tools/parity/verify_report.py <chunk-report.json> --protocol` подтверждает
хеш JAR, реальные вызовы, обнаружение синтетической перестановки nibble/индекса,
протокол/лимиты, missing JAR и ошибку reflection на явно синтетической копии нашей
обёртки. Оригинальный JAR и успешные live-выходы не изменяются.

NBT-001: `python3 tools/parity/run.py nbt --strict` вызывает оригинальные `ij.b`
и `ij.a`, сохраняет round-trip bytes, consumed length и `canonical-trees.json`.
Деревья сравниваются с учётом типов, IEEE-битов, порядка списков и last-value-wins
для compound; независимо проверяется сохранность входного дерева при записи,
с единственным явным преобразованием NaN по Java DataOutput.
`python3 tools/parity/verify_report.py <nbt-report.json> --protocol` проверяет
хеши входа/выходов/JAR/класса/probe/деревьев, успешность процессов, audit вызовов,
пять синтетических мутаций, TSV и invalid NBT, missing JAR/classes.
IntArray, границы глубины и защитные ограничения Rust проверяются отдельно.
`protocol-review.json` отдельно перечисляет подтверждённые native differences:
пустой список теряет объявленный тип в оригинальном writer; отрицательная длина
списка, End-элементы и unknown type пустого списка допускаются оригинальным reader;
Java String допускает одиночный surrogate. Эти проверки не считаются совпавшей parity
и не меняют safe Rust API. Границы общей области описаны в NBT-001 и provenance.

REGION-001: `python3 tools/parity/run.py region --strict` сначала проверяет четыре
закреплённых source hash, создаёт отдельную копию входов и запускает Java `qj`
только на ней. Далее `region_probe` получает независимую копию. Отчёт сравнивает
presence, timestamp, compression, длину и SHA-256 decompressed NBT; каждый present
payload остаётся в `java-payloads/` и `rust-payloads/`. Источники, обе копии, JAR,
процессы, вызовы native reader и payloads сверяются до/после запуска.
`python3 tools/parity/verify_report.py <region-report.json> --protocol` повторно
проверяет сохранённые хеши, native audit, payload bytes, отсутствие изменений
источников/копий и синтетически меняет payload для проверки чувствительности.
Fixture с 704 занятыми и 44 пустыми слотами отделён от синтетических malformed
region tests; последние проверяют safe Rust reader, а не расширяют native parity.

`python3 tools/parity/region_edges.py <region-report.json>` запускает обе стороны
на 37 generated cases: 23 C01 wire cases, 10 coordinate boundaries и четыре gzip
tail/header/trailer cases. Сохраняет отдельный `edge-review-*/review.json` с 74
executions, stdout/stderr и проверкой неизменности inputs. Overlap и oversized
record отдельно отмечаются как native differences: Java принимает, safe Rust
отвергает. Эти результаты не увеличивают счётчик 704 настоящих чанков.

VIEW-001: `python3 tools/parity/run.py view --strict` запускает оригинальный
`uu`/`cv` mapping oracle на отдельной hash-verified 5×5 scene copy дважды и
сравнивает точные TSV bytes с `view_probe` в
`crates/beta-viewer/examples/view_probe.rs`, когда probe появится. Пока его нет,
отчёт честно получает `not_implemented`, а strict команда завершится ненулевым
кодом. Проверка сохранённого отчёта:
`python3 tools/parity/verify_report.py <view-report.json> --protocol`.
Она проверяет JAR/atlas/world/source/class/output hashes, вызовы native mapping,
повторяемость, malformed/duplicate TSV и искусственную однобайтовую мутацию.
Вершины, live brightness и pixel parity не измеряются.
# Agent-facing native geometry checks

The `geometry` suite calls the original Beta 1.7.3 AABB (`eq`), Vec3 (`bt`)
and ray-intersection methods. It compares all f64 results by raw bits, including
signed zeros. Configure the local original client JAR in `config/local.toml`.

```bash
python3 tools/parity/run.py list
python3 tools/parity/run.py geometry --strict
python3 tools/parity/verify_report.py artifacts/parity/geometry/run-.../report.json --protocol
```

Agents can supply their own cases to the same original-vs-Rust runner:

```bash
printf 'agent-normalize\tvec_normalize\t4008000000000000\t4010000000000000\t0000000000000000\n' > /tmp/geometry-case.tsv
python3 tools/parity/geometry_run.py --input /tmp/geometry-case.tsv --strict
```

Custom runs are labelled `scenario: custom`; they invoke the original methods
twice, retain independent audits, and never overwrite committed fixtures. Saved
verification also supports custom reports. Protocol and operation definitions:
[GAME-ORACLE-001](../../docs/tasks/GAME-ORACLE-001.md). This verifies geometry;
player ticks, World simulation and rendering require their own suites.

## Bounded water tick parity

The command python3 tools/parity/run.py water-tick --strict verifies the pinned
native scheduled-tick traces, then feeds each recorded whole due-time batch to
the Rust water_tick_probe. It compares ordered edit/schedule/neighbor events,
the complete resulting queue, and all 100 block-ID/metadata cells byte-for-byte.
The report retains every probe input/output and binds the runner, sources,
fixtures, client JAR, and Rust revision. Native evidence covers only synthetic
loaded chunks and the captured water IDs 8/9 scenes; it does not claim unloaded
neighbor rules, general source thresholds, lava, or player swimming physics.
