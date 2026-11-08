# Прогресс

## Текущее состояние

WORLD-LOAD-001 принят: read-only metadata и typed chunk data из Beta 1.7.3 save.
VIEW-001 принят: реализован CPU mesher и локальный wgpu viewer фиксированной
5×5 сцены с оригинальной terrain-текстурой; окно показывает рельеф и поддержанные
opaque cube блоки. Это первый графический milestone, не полная визуальная parity.
MATH-001 и JAVA-001 приняты. JAVA-001 добавил Java-совместимые float/double
casts в int/long и UTF-16 `String.hashCode`, сверенные с прямым JVM oracle.
GAME-ORACLE-001 принят: оригинальные AABB/Vec3/ray методы доступны агентам через
CLI с пользовательскими входами, Rust geometry сверена по raw bits.
EVIDENCE-001 усилил сохранённые доказательства. Следующий пакет — MOVE-001.

Оценка готовности **полной игры — около 12%** после GAME-ORACLE-001.
Методика и веса: [completion-estimate.md](completion-estimate.md).
Это приблизительная оценка возможностей, не доля закрытых задач.

- **RNG-001, RNG-002, CHUNK-001, NBT-001, REGION-001, WORLD-LOAD-001,
  VIEW-001, MATH-001, JAVA-001 и GAME-ORACLE-001 приняты**.
  Проверенная ревизия REGION — `1e076324e992412d396a15da72f808facb00632f`;
  коммит приёмки содержит запись C03 и переносится в `main` fast-forward.
- RNG-001: 96 результатов Java API nextInt.
- RNG-002: 13227 смешанных операций, общее состояние, reseed, rejection sampling,
  raw IEEE-биты float/double.
- CHUNK-001: 4182 операции с контейнером 16×128×16 и оригинальным nibble-классом.
- NBT-001: 5250 live round-trips оригинальных ij/nu, точные raw bytes,
  canonical tree и consumed length; 86 дополнительных protocol/wire/safety
  executions и пять синтетических мутаций проверены отдельно.
- REGION-001: 704 occupied + 44 absent slot точно совпали с оригинальным qj;
  37 synthetic cases/74 reader executions проверены отдельно от live parity.
- WORLD-LOAD-001: 749 live rows (level.dat + 748 slots) совпали с native Java;
  704 chunk payload present, 44 absent. Отдельно проверены 22 protocol executions
  и 42 generated edge cases/84 executions. Native audit: qj=748, ij=705, ei=1,
  full native chunk/world loads=0. Это read-only data extraction, без генерации,
  тиков, создания игровых объектов и записи.
- Rust принадлежит роли agy. Текущий REGION реализован внутренним агентом
  `gpt-6-luna low`, runner C02 — другим агентом той же модели. Координатор
  Codex выполнил ревью, запросил исправления Rust и принял объединённый результат;
  прямых правок Rust координатором нет. Предыдущие handoff сохраняют историю моделей.
- JAR/ресурсы/дизассемблированный код и пользовательский мир в Git не включены.
  Проверки читают копии; работа с `.mcr` остаётся read-only. C01 закрепляет
  `flate2 =1.1.9` (`rust_backend`) и probe-only `sha2 =0.10.9`; lockfile проверен offline.

## Последняя независимая приёмка

GAME-ORACLE-001 принят на clean объединённой ревизии `9347ae5`. Все 6284
native geometry cases совпали точно. Custom differential input обнаружил
реальную ошибку выбора ray face на тонком box при близких расстояниях; Rust
исправлен и native corpus расширен для X/Y/Z в обоих направлениях. Saved review
проверяет процессы, команды, code source, compiled adapter, Rust binary, source
и input/output hashes; 14 malformed executions отклонены. Полный workspace check
и все предыдущие strict suites прошли. EVIDENCE-001 независимо усилил
math/java evidence и прошёл live mutations/protocol. Геометрия готова к
использованию в физике; игрового шага пока нет. Оценка полной игры — около 12%.

JAVA-001: snapshot agy `73dac4598dd2d596c1158b640a2bf9f630950b3d` принят.
Rust casts и `String.hashCode` побайтно совпали с direct JVM oracle на 18 493
строках; `python3 tools/check.py` прошёл с 95 Python-тестами и всеми Rust
workspace checks. Saved review обнаружил 5 синтетических изменений и отклонил
10 некорректных JVM/Rust protocol inputs. Семь предыдущих strict suites также
прошли. Это узкий набор базовой Java-совместимости, поэтому округлённая оценка
полной игры остаётся около 11%.

MATH-001: agy snapshot `aaebed54d22a9149894792e305ed264d9a2b1a36` объединён и
принят. Rust MathHelper совпал с исходным классом Beta 1.7.3 по 131 408
операциям и raw IEEE-754 bits; `sin` и `cos` покрывают каждый из 65 536 LUT
индексов. Saved report `artifacts/parity/math/run-lgbtghq2/report.json` имеет
`rust_dirty=false`; protocol review проверил 10 отказов некорректных входов и
синтетические изменения type/bit/order/row count/source. `python3 tools/check.py`
прошёл с 89 Python-тестами и всеми Rust workspace, clippy, fmt и doc checks.
Семь предыдущих strict suites также прошли. Набор функций не включает игровой
цикл, генерацию или физику; готовность полной игры увеличена примерно на один
пункт до 11%.

VIEW-001 принят с clean agy snapshot `973de0be1bc36f20ce5d136475cd7226ff884ebb`.
`python3 tools/check.py` и все Rust all-target/doc, fmt/check/clippy проверки прошли.
Все семь strict suites на принятом снимке прошли: RNG 96, RNG-extended 13227,
chunk 4182, NBT 5250, region 748, world 749, view 50 mappings + 1617 neighbor
tuples. Для view выполнены saved protocol/hash review и Xvfb GUI smoke; локальный
скриншот `/tmp/beta-viewer-smoke.png` просмотрен. Renderer пока ограничен
фиксированной сценой и подтверждёнными полными opaque cubes; native brightness,
AO и pixel parity остаются вне покрытия. По пользовательскому кадру добавлены
tile-isolated atlas mipmaps для уменьшения minification aliasing; тесты и повторная
view parity прошли. Xvfb sandbox после этого не позволил сохранить новый кадр.

## Последняя независимая приёмка

WORLD-LOAD-001: clean Rust snapshot `a7aa6856f81da7f3e008b4c1d7ad9ac89cffac18`,
объединённая ревизия `8292470d5cdcba84ad42527496226e2c1ca33f8d`; Rust↔Java
749/749 exact. `python3 tools/check.py` — 75 Python + 66 Rust tests,
fmt/check/clippy/doc PASS. Все пять прежних strict parity suites и новый world
suite PASS; полный world protocol/saved review и generated edges PASS. Отчёты и
артефакты перечислены в `docs/tasks/status/codex.json` и `docs/handoffs/codex.md`.

REGION-001: snapshot agy `4f6c41c6a1edef399ac4bfa001f6be13d5089158`,
C02 `e388657`, объединённая ревизия `1e076324e992412d396a15da72f808facb00632f`.
60 Python + 58 Rust tests, fmt/check/clippy/doc PASS. Все пять live-групп
PASS на этой чистой ревизии: RNG 96, extended 13227, chunk 4182, NBT 5250,
region 748. Region report: `artifacts/parity/region/run-pq6ftpqm/report.json`.
16 TSV executions, шесть report mutations, missing-probe/wrong-JAR/timeout
paths и 37 synthetic cases проверены отдельно. Исходные четыре `.mcr`, JAR
и обе копии входов неизменны. Подробные пути — `docs/handoffs/codex.md`.

## Предыдущая приёмка NBT-001

Snapshot agy: `541d719e83e32ad13e9448e102c8cc1ed16bcf40`.
Чистая проверенная объединённая ревизия:
`93ed4fe8356d9c30cfa9013bca111d61e75f0ebf` (включает отдельное исправление
инструментария Codex). Финальные live-отчёты имеют `rust_dirty=false`.

- `python3 tools/check.py`: 47 Python + 43 Rust all-targets,
  fmt/check/clippy `-D warnings` и отдельный doc — PASS.
- `cargo test --all-targets --locked --offline`: 43 PASS.
- `python3 tools/parity/run.py rng --strict`: 96/96 PASS,
  `artifacts/parity/rng/run-cxelcb7r/report.json`.
- `python3 tools/parity/run.py rng-extended --strict`: 13227/13227 PASS,
  `artifacts/parity/rng-extended/run-w6ixr3u6/report.json`.
- `python3 tools/parity/run.py chunk --strict`: 4182/4182 PASS,
  `artifacts/parity/chunk/run-rohs2w09/report.json`.
- `python3 tools/parity/run.py nbt --strict`: 5250/5250 PASS,
  `artifacts/parity/nbt/run-a56fm2ot/report.json`.
- `python3 tools/parity/verify_report.py artifacts/parity/nbt/run-a56fm2ot/report.json --protocol`:
  hashes/audit/canonical/consumed PASS, пять синтетических подмен обнаружены,
  86 дополнительных executions PASS. Native differences не считаются parity.

Оригинал NBT/CHUNK: клиент Beta 1.7.3, JAR SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
NBT реально вызывает `ij.b(DataInput)` и `ij.a(ij,DataOutput)` из JAR.
Корпус усилен 50 граничными случаями без удаления прежних 5200. Подробности:
`tests/fixtures/nbt/provenance.md`, `docs/handoffs/codex.md`.

## Ограничения и следующий шаг

NBT parity относится к общей области оригинального API: типы 0–10,
представимые в Rust String строки. IntArray — отдельное wire extension.
Пустой список, отрицательные длины, End/unknown list types и одиночные surrogate
имеют явно измеренные отличия от оригинала; требования безопасного Rust API
сохранены. Java DataOutput нормализует NaN; входные биты остаются в evidence.
Root-End/trailing-byte/TSV guards принадлежат адаптеру, не оригиналу.

Игровая логика, симуляция мира, запись сохранения, большая часть графики,
производительности и игровой цикл ещё не реализованы. VIEW-001 показывает только
фиксированный участок и не загружает игровой World в полноценный renderer.
Следующий пакет нужно выбирать после принятия VIEW-001 и синхронизации веток.
