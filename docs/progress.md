# Прогресс

## Текущее состояние

Открыт WORLD-LOAD-001: параметры level.dat и typed chunk data без графики/тиков.
Реализованные ранее пакеты остаются принятыми; новая очередь содержит пять карточек.

Оценка готовности **полной игры — около 7%** после REGION-001.
Методика и веса: [completion-estimate.md](completion-estimate.md).
Это приблизительная оценка возможностей, не доля закрытых задач.

- **RNG-001, RNG-002, CHUNK-001, NBT-001 и REGION-001 приняты**.
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
- Rust принадлежит роли agy. Текущий REGION реализован внутренним агентом
  `gpt-6-luna low`, runner C02 — другим агентом той же модели. Координатор
  Codex выполнил ревью, запросил исправления Rust и принял объединённый результат;
  прямых правок Rust координатором нет. Предыдущие handoff сохраняют историю моделей.
- JAR/ресурсы/дизассемблированный код и пользовательский мир в Git не включены.
  Проверки читают копии; работа с `.mcr` остаётся read-only. C01 закрепляет
  `flate2 =1.1.9` (`rust_backend`) и probe-only `sha2 =0.10.9`; lockfile проверен offline.

## Последняя независимая приёмка

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

Игровая логика, полная загрузка настоящего сохранения, графика и производительность
игры ещё не проверялись. REGION-001 извлекает сырые NBT payload из read-only
McRegion; семантическая загрузка level.dat/чанков ещё не реализована.
Текущий пакет — WORLD-LOAD-001, контракт и пять карточек открыты. Затем планируется
VIEW-001. Координатор запускает внутренних агентов по `docs/workflow.md`.
