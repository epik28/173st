# Прогресс

## Текущее состояние

- **RNG-001, RNG-002 и CHUNK-001 приняты**; все карточки текущей очереди выполнены.
  Коммит приёмки RNG-002 `e11362c` включён в main fast-forward. Приёмка CHUNK-001
  `2c5cb4a` также включена в main fast-forward; очередь Codex вернула complete.
- RNG-001: Java API nextInt на шести seed, 96 результатов.
- RNG-002: методы bound/long/boolean/float/double/setSeed, общее состояние и
  13227 операций; raw IEEE-биты, invalid_bound/reseed, rejection sampling.
- CHUNK-001: контейнер 16×128×16, индексация, координаты, nibble-массивы,
  from_raw и атомарные отказы. 4182 live-операции проверены по оригинальному классу.
- Rust-код написан agy. Codex подготовил Java-эталоны/корпуса/runner,
  обнаружил прежний дефект whitespace (исправлен agy) и исправил собственный
  verifier отдельным коммитом `660008d`. Прямых правок Rust от Codex нет.
- JAR/ресурсы/дизассемблированный код не включены в Git. Новые зависимости
  не добавлялись, новые этапы за пределами очереди не начинались.

## Последняя независимая приёмка

Snapshot agy: `14b50482bc1fa9ef4850d7983780ceda876bb80a`.
Проверенная чистая объединённая ревизия: `6b0bdfc26a5c53934da9980c4a3ed5051b7910bc`.

- `python3 tools/check.py`: 37 Python + 31 Rust (all-targets),
  fmt/check/clippy с -D warnings и отдельный doc — PASS.
- `cargo test --all-targets --locked --offline`: 31 PASS.
  В обычную проверку теперь включены unit-тесты examples.
- `python3 tools/parity/run.py rng --strict`: 96/96 PASS,
  `artifacts/parity/rng/run-5_dm3mel/report.json`.
- `python3 tools/parity/run.py rng-extended --strict`: 13227/13227 PASS,
  `artifacts/parity/rng-extended/run-epqhsc13/report.json`.
- `python3 tools/parity/run.py chunk --strict`: 4182/4182 PASS,
  `artifacts/parity/chunk/run-jvj35_be/report.json`.
- `python3 tools/parity/verify_report.py <соответствующий-report.json> --protocol`:
  rng 36 PASS, rng-extended 44 PASS; chunk 46 TSV-запусков + missing JAR и
  явно синтетическая ошибка reflection (48 всего) PASS. Синтетические копии
  с подменой значения/nibble-порядка/индекса обнаружены; оригинальные выходы сохранены.

Оригинал CHUNK-001: локальный клиент Beta 1.7.3 из Prism, JAR SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
Через reflection реально вызваны wi(int), get/set a(...), raw поле a:byte[].
Аудит fixture: constructors=6, native get=3606, native set=1746, raw reads=1752.
Имена/хеш класса и границы обёртки — `tests/fixtures/chunk/provenance.md`.
Координаты/index/ID/API guards — наша обёртка; nibble allocation/get/set/raw — оригинал.

## Ограничения и следующий шаг

RNG — Java API parity, не генерация мира. CHUNK — совместимость контейнера,
не NBT/McRegion, загрузка настоящего мира или рендер. Игровая логика, движение,
графика и производительность игры ещё не проверялись.

Текущая очередь расширена контрактом NBT-001. Следующие REGION-001/WORLD-LOAD-001
и затем VIEW-001 пока остаются планом; начинать их после отдельной приёмки NBT.
Перед новой задачей обе роли обновляют ветки до принятого main.
