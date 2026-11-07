# Прогресс

## Текущее состояние

- **RNG-001, RNG-002, CHUNK-001 и NBT-001 приняты**. Все карточки согласованной
  очереди выполнены; коммит приёмки NBT — содержащий этот раздел коммит в main.
- RNG-001: 96 результатов Java API nextInt.
- RNG-002: 13227 смешанных операций, общее состояние, reseed, rejection sampling,
  raw IEEE-биты float/double.
- CHUNK-001: 4182 операции с контейнером 16×128×16 и оригинальным nibble-классом.
- NBT-001: 5250 live round-trips оригинальных ij/nu, точные raw bytes,
  canonical tree и consumed length; 86 дополнительных protocol/wire/safety
  executions и пять синтетических мутаций проверены отдельно.
- Весь Rust-код — agy. Codex подготовил независимые Java-адаптеры, корпусы,
  runner, тесты и приёмку. Прямых правок Rust от Codex нет.
- JAR/ресурсы/дизассемблированный код в Git не включены. Новых зависимостей нет.

## Последняя независимая приёмка

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

Игровая логика, загрузка настоящего сохранения, графика и производительность
игры ещё не проверялись. REGION-001, WORLD-LOAD-001 и VIEW-001 остаются планом:
в текущей queue.json их карточек нет. Следующий пакет требует отдельного
контракта и добавления в очередь до начала реализации.
