# Отчёт Gemini / agy

## REGION-001-A01 — read-only Rust McRegion reader

- Реализован borrowed `beta_core::region::RegionFile`: проверяет выравнивание и заголовок, все location records, длины, границы и перекрытия до выдачи reader. Добавлены metadata записей, ошибки для локальных x/z вне диапазона, `region_coord/local_coord` для отрицательных координат и границ i32.
- Реализована bounded распаковка gzip (несколько members и допустимый trailing garbage по C01) и zlib. Ограничен суммарный распакованный output; ошибки потоков и неподдерживаемые compression IDs различаются. Регион не копируется целиком и входные bytes не изменяются.
- Синтетические тесты покрывают zlib NBT payload/metadata, gzip multiple members + trailing bytes, malformed header/location/length, overlap, unsupported compression, decompression failure, output cap и координатные границы. Prism save, JAR и внешние artifacts не открывались и не менялись; live parity не заявляется.
- Изменены: `crates/beta-core/src/region.rs`, `crates/beta-core/src/lib.rs`, `docs/tasks/status/agy.json`, этот handoff.
- Проверки: `cargo test --all-targets --locked --offline` → 30 library + 18 example tests PASS; `python3 tools/check.py` → 51 Python tests, fmt/check/clippy/workspace tests/doc tests PASS; `git diff --check` → PASS.
- Model ID: **gpt-6-luna low**. Заимствований нет; контракт C01 использован как входные требования. Стоимость/токены недоступны.

### A01 follow-up — stream completion regressions

- Follow-up проверка воспроизвела: flate2 read zlib decoder принимал пустой input как пустой успешный output. Zlib reader заменён на bounded `Decompress` loop, который требует `Status::StreamEnd`; incomplete/truncated streams и checksum errors дают `Decompression`, bytes после полного stream остаются допустимы по C01.
- Gzip member detection исправлена для случая, когда первый byte следующего `1f 8b` magic остаётся последним byte внутреннего 8192-byte BufReader buffer: теперь проверяется следующий byte и reader восстанавливается на точном начале member.
- Regression test перебирает все усечения валидных gzip/zlib streams (включая удаление 1–4 последних trailer bytes), повреждает каждый байт gzip (CRC32 + ISIZE) и zlib (Adler32) trailers, проверяет пустые inputs, допустимый zlib trailing garbage и concatenated gzip member с границей ровно 8191 bytes.
- Проверки: `cargo test --locked --offline -p beta-core region::tests::` → 8 PASS; `python3 tools/check.py` → 51 Python tests, fmt/check/clippy, 33 library + 22 example tests и doc tests PASS; `git diff --check` → PASS.
- Внешние файлы, сохранение Prism, JAR, oracle/runner и общий контракт не трогались. Model ID: **gpt-6-luna low**; заимствований нет, стоимость/токены недоступны.

### A01 follow-up — repeated gzip boundaries and large zlib output

- Удалён промежуточный `BufReader` и пересчёт относительных Cursor offsets: `Cursor<&[u8]>` напрямую реализует `BufRead`, поэтому каждый concatenated gzip member читается с текущего абсолютного offset, включая trailing garbage policy.
- Zlib low-level loop использует `FlushDecompress::None` с bounded output chunks и завершается только на `Status::StreamEnd`; no-progress/error отвергает незавершённый stream. Это поддерживает valid output больше 8192 байт.
- Добавлены regression cases: три gzip members, первые два encoded ровно в 8191 byte; 25,000-byte zlib output с точным output limit и limit+1; trailing gzip `0x1f` (garbage accepted) и incomplete magic `0x1f 0x8b` (decompression error). Ранее добавленные exhaustive truncation/checksum случаи остаются включены.
- Проверки: `cargo test --locked --offline -p beta-core region::tests::` → 10 PASS; `python3 tools/check.py` → 51 Python tests, fmt/check/clippy, 35 library + 22 example tests, doc tests PASS; `git diff --check` → PASS.
- Изменены только `region.rs`, собственный agy status и handoff. Model ID: **gpt-6-luna low**; зависимости, oracle, runner, общий контракт, save/JAR не изменялись.

### A01 follow-up — native gzip partial-header behavior

- Учтено фактическое наблюдение Codex original `qj`: после успешного member trailing `0x1f 0x8b` без полного header также игнорируется; полный следующий header с truncation/corrupt body или trailer остаётся ошибкой.
- `GzDecoder::new` eagerly парсит header. Когда `header()` отсутствует, первый member даёт `Decompression`, а после валидного member хвост трактуется как garbage; при полном header `read_bounded` читает body/trailer и сохраняет отказ при checksum/EOF errors.
- Добавлены tests на empty/invalid/partial first headers, trailing partial magic после valid member, full-header truncated и corrupt second-member trailer; все прежние gzip/zlib regression cases остаются.
- Проверки: `cargo test --locked --offline -p beta-core region::tests::` → 11 PASS; `cargo fmt --all -- --check` → PASS; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` → PASS; `cargo test --all-targets --locked --offline` → 36 library + 22 example tests PASS.
- Model ID: **gpt-6-luna low**. Только `region.rs` и собственные status/handoff; dependency/oracle/runner/contract/save/JAR не менялись.

## REGION-001-A02 — Rust region probe

- Добавлен `crates/beta-core/examples/region_probe.rs`. CLI принимает только `INPUT_COPY_DIR PAYLOAD_DIR`, требует обычный `.region-input-copy` marker, разделяет input/payload каталоги и читает только regular region files непосредственно в canonical input directory. TSV проверяет число полей, уникальный ASCII case ID, координаты i32 и точное имя региона `r.<floor(x/32)>.<floor(z/32)>.mcr`; запрещает абсолютные/traversal пути и symlink region files.
- Для каждой строки сохраняется порядок и печатается `present|absent` с timestamp/compression/length/SHA-256; каждый present NBT payload создаётся через `create_new`, absent файл не создаёт. Диагностика CLI идёт только в stderr с ненулевым кодом.
- Тесты probe покрывают правильный TSV, present/absent, payload hash/bytes, bad IDs/paths/coordinate syntax/range, duplicate ID, marker/dir guards, запрет перезаписи и повторяемость на неизменном синтетическом input. Live save/JAR parity не запускались; Codex C02 runner ещё не интегрирован. Все тесты выполнялись на созданных временных копиях.
- Изменены: `crates/beta-core/examples/region_probe.rs`, `docs/tasks/status/agy.json`, этот handoff.
- Проверки: `cargo test --locked --offline -p beta-core --example region_probe` → 4 PASS; `cargo test --all-targets --locked --offline` → 30 library + 22 example tests PASS; `python3 tools/check.py` → 51 Python tests и fmt/check/clippy/workspace/doc PASS.
- CLI smoke на синтетической copied input: валидный present/absent stdin → exit 0, точные 2 строки TSV в порядке входа, stderr пуст; неправильное имя/путь → exit 1, stdout пуст, diagnostics stderr. Повторный запуск с отдельной payload directory дал тот же stdout и байты payload; input не изменился.
- Model ID: **gpt-6-luna low**. Заимствований нет. Сохранение Prism, JAR и C02/C03 ветка не трогались. Стоимость/токены недоступны.

## NBT-001 — приёмка в main и завершение очереди agy

- Прочитаны закоммиченные контракт, статус C03 и handoff Codex. Приёмка находится
  в `main`: `2930029a3603632673586c90c653d4a1e370f1dc`; исправление NBT verifier
  включено отдельным коммитом `93ed4fe8356d9c30cfa9013bca111d61e75f0ebf`.
- `python3 tools/agent_session.py start agy` → сессия занята в `agy/development`.
- `git merge-base --is-ancestor HEAD main` → exit 0 перед синхронизацией.
  `git merge --ff-only main` → успешно, `541d719..2930029`.
- `git diff --exit-code main HEAD` → exit 0 после fast-forward;
  `git status --short --branch` → чистая `agy/development`.
- `python3 tools/tasks.py next agy` и `python3 tools/tasks.py next codex`
  → обе очереди `complete`, exit 3; оба snapshot равны `2930029`.
- По независимому handoff Codex: NBT 5250/5250, 86 protocol/wire/safety executions,
  пять synthetic mutations и прежние группы прошли на чистой ревизии `93ed4fe`.
  Тестовые наборы в этой сессии синхронизации повторно не запускались.
  Ранее записанное ниже ограничение NBT `verify_report.py --protocol` устранено.
- Область приёмки и отдельные проверки IntArray/native differences закреплены
  в `docs/tasks/NBT-001.md`. Карточек REGION-001 в `docs/tasks/queue.json` нет;
  следующий шаг — новый контракт и карточки от интегратора.
- Единственный собственный изменённый файл — `docs/handoffs/agy.md`; итоговый
  коммит содержит этот раздел. `git diff --check` → PASS.
- Исполнитель синхронизации — Codex в роли agy; точный model ID и метрики затрат
  недоступны. Пользователь сообщил SHA приёмки, проверенный через Git.
  Дополнительные агенты, заимствования и изменения Rust отсутствуют.

## NBT-001-A01 — Rust reader/writer

### Результат
- Реализован `beta_core::nbt` в `crates/beta-core/src/nbt.rs`, экспортирован из `lib.rs`.
- Поддержаны все wire-типы 0–11, именованный корень, big-endian числа, вложенные списки и compound, массивы, сохранение IEEE-754 битов и детерминированная запись.
- Modified UTF-8 кодирует NUL как `C0 80`, кодирует supplementary code points через UTF-16 surrogate pair и принимает также однобайтовый NUL, который допускает `DataInput.readUTF`. Одиночный surrogate отклоняется, так как `String` Rust не может его представить.
- Лимиты по умолчанию: глубина 512, строка 65535 encoded bytes, суммарно 2 000 000 элементов, полный payload 64 MiB. Проверки размера payload выполняются до накопления writer output; длина строки проверяется до временного выделения encoded буфера; reader проверяет длины до выделения коллекций.
- NBT C03 live-run 5200 cases выявил несоответствие в compound writer: оригинальный `nu` использует `java.util.HashMap` и пишет элементы в порядке `values()` iterator. Reader Rust ранее отвергал duplicate names.
- `javap -private -c ... nu` подтвердил HashMap storage/iteration; прямой вызов NbtOracle на duplicate-name compound показал `HashMap.put` semantics: duplicate принимается, последнее значение заменяет предыдущее.
- Reader теперь применяет last-value-wins, writer моделирует Java `String.hashCode`, HashMap capacity/resize, bucket iteration order и дубли. Изменение остаётся внутри NBT-001-A01.
- `Tag::Float/Double` при чтении сохраняют raw bits; writer повторяет `DataOutput.writeFloat/writeDouble` и canonicalizes noncanonical NaN payload в Java quiet-NaN bit pattern.

### Изменённые файлы
- `crates/beta-core/src/nbt.rs`
- `crates/beta-core/src/lib.rs`
- `docs/tasks/status/agy.json`
- `docs/handoffs/agy.md`

### Проверки
- `cargo test --all-targets --locked --offline` → 38 тестов пройдены (24 library, 5 `chunk_probe`, 5 `rng_extended_probe`, 4 `rng_probe`).
- `python3 tools/check.py` → 37 Python tests; fmt, cargo check, clippy `-D warnings`, workspace tests и doc tests прошли.
- `git diff --check` → без ошибок.
- Первое live-сравнение после получения Codex C01/C02 запущено на чистом SHA `9db415e` (merge snapshot C02 + agy `c325bff`): `python3 tools/parity/run.py nbt --strict` → FAIL, 5200/5200 mismatches; отчёт `artifacts/parity/nbt/run-znl4a74r/report.json`. Сырые Java/Rust выходы и первый mismatch сохранены; comparator/oracle не менялись.
- После HashMap исправления на `e8cc558` parity ещё FAIL: 473/5200; первый mismatch — noncanonical NaN payload. После Java DataOutput NaN normalization final live run на чистом SHA `099f3e5` прошёл: `python3 tools/parity/run.py nbt --strict` → PASS, 5200/5200, mismatch 0, `rust_dirty=false`; отчёт `artifacts/parity/nbt/run-9pmeibzy/report.json`.
- `cargo test --all-targets --locked --offline` → 43 tests PASS; `python3 tools/check.py` → 37 Python tests и все Rust quality checks PASS.
- `python3 tools/parity/run.py rng --strict` → PASS; `python3 tools/parity/run.py rng-extended --strict` → PASS; `python3 tools/parity/run.py chunk --strict` → PASS.
- Четыре записи `tests/fixtures/nbt/invalid.tsv` запускались отдельно против Rust `nbt_probe` и original `NbtOracle`: оба процесса отвергли все четыре; stdout Rust пустой, ошибки в stderr.
- `compare_outputs` обнаружил 5 синтетических изменений скопированного Rust output: endian, length, tag type, modified UTF-8 и payload. Сводка: `artifacts/parity/nbt/run-9pmeibzy/synthetic-mutations.json`; это sensitivity check, не дополнительный live parity.
- `python3 tools/parity/verify_report.py artifacts/parity/nbt/run-9pmeibzy/report.json --protocol` не поддерживает NBT: пытается разбирать NBT fixture как RNG и завершается `RunError: fixture line 1: invalid fields/ID`. Invalid checks выше выполнены отдельным subprocess driver. Codex C03 должен добавить/выполнить NBT protocol verification; его файлы здесь не менялись.

### Следующий шаг
- A01 исправлен и возвращён в `done`; Rust-side карточки NBT-001 роли agy завершены. Следующий шаг — Codex C03 на зафиксированном snapshot этой ветки.

## NBT-001-A02 — Rust NBT probe

### Результат
- Добавлен `crates/beta-core/examples/nbt_probe.rs` по уже закоммиченному C02 runner protocol: input `case_id<TAB>hex_bytes`; output `case_id<TAB>0<TAB>round_trip_hex<TAB>consumed_bytes`.
- Probe разбирает named root библиотечным reader, повторно записывает дерево writer-ом и выдаёт точные байты плюс consumed length. `encode:` не добавлен: этот режим не нужен C02 и его контракт требовал согласования с C01.
- Проверяются формат TSV/hex, уникальные ASCII ID, пустые/комментарии, ограничения на число случаев и объём входных payload; ошибки завершают CLI с кодом 1 и диагностикой только в stderr.

### Изменённые файлы
- `crates/beta-core/examples/nbt_probe.rs`
- `docs/tasks/status/agy.json`
- `docs/handoffs/agy.md`

### Проверки
- `cargo test --all-targets --locked --offline` до исправления A01 → 42 теста пройдены; после исправления → 43 PASS (25 library, 5 `chunk_probe`, 4 `nbt_probe`, 5 `rng_extended_probe`, 4 `rng_probe`).
- `python3 tools/check.py` → 37 Python tests; fmt, cargo check, clippy `-D warnings`, workspace tests и doc tests прошли.
- `python3 -c '…subprocess…'` запускал `cargo run --quiet --locked --offline -p beta-core --example nbt_probe`: валидный root дал точный stdout `smoke<TAB>0<TAB>0a000000<TAB>4<LF>`; неверный root завершился кодом 1 с пустым stdout и ошибкой в stderr.
- `git diff --check` → без ошибок.
- C01/C02 snapshot `07726cfa3913a4a565d8a47623f59b34bb17ae5d` получен обычным merge. Первые parity mismatches обнаружили A01 расхождения в HashMap ordering и NaN normalization; обе поправки включены в чистую ревизию `099f3e5`, на которой 5200/5200 совпали.

### Следующий шаг
- Статусы обеих agy карточек NBT-001 выставлены `done`; следующую задачу выбирает очередь. Для NBT parity нужен закоммиченный Codex C01/C02 и интеграция обеих сторон.

## Метаданные
- **Model ID:** Gemini 3.8 Flash (High) via Antigravity
- **Заимствования:** Нет (реализация контейнера чанка и probe по контракту CHUNK-001)
- **Ручная помощь:** Нет
- **Внешние зависимости:** Не добавлялись

---

## Задачи RNG-001 и RNG-002
- **RNG-001-A01, RNG-001-A02:** Выполнены (`done`), приняты в `main`.
- **RNG-002-A01, RNG-002-A02:** Выполнены (`done`), приняты в `main`.

---

## Задача: CHUNK-001-A01 — Rust API данных чанка

### Статус
Выполнена (`done`).

### Изменённые файлы
- `crates/beta-core/src/chunk.rs`:
  - `BlockData { id, metadata, block_light, sky_light }` (Clone/Copy/Debug/PartialEq/Eq/Default);
  - `ChunkError { InvalidCoordinate, InvalidNibble, InvalidLength }` (Display/Error);
  - `split_block_coord(value: i32) -> (i32, u8)` — арифметика floor(value / 16) и remainder 0..=15, проверены граничные значения включая `i32::MIN` и `i32::MAX`;
  - `block_index(x: u8, y: u8, z: u8) -> Result<usize, ChunkError>` — формула `y + z * 128 + x * 2048`, строгая проверка диапазонов x < 16, y < 128, z < 16;
  - `ChunkData`:
    - `new()` — нулевая инициализация 4 массивов (`blocks` 32768 байт, `data`, `block_light`, `sky_light` по 16384 байта);
    - `Default` — через `new()`;
    - `from_raw(...)` — валидация точных длин всех 4 массивов;
    - `get(x, y, z)` — чтение блоков и упаковка nibble (чётный индекс — младшие 4 бита, нечётный — старшие 4 бита);
    - `set(x, y, z, value)` — атомарная запись: проверка координат первично, проверка nibble `<= 15` вторично; при любой ошибке ни один байт не изменяется;
    - `raw_arrays()` — доступ к сырым срезам байтов.
  - Написаны модульные тесты: `test_split_block_coord_standard_and_bounds`, `test_block_index_computation`, `test_chunk_new_and_raw_arrays`, `test_from_raw_lengths`, `test_nibble_packing_and_neighbor_isolation` (сверка сырого байта 0xC3 / 195 с Java-эталоном), `test_set_atomic_on_error`.
- `crates/beta-core/src/lib.rs` — экспорт `pub mod chunk;`.
- `docs/tasks/status/agy.json` — отметка `CHUNK-001-A01` в состоянии `done`.

### Проверки
- `cargo test --all-targets --locked --offline` → 26 тестов Rust пройдены успешно (17 в `lib.rs`, 5 в `rng_extended_probe`, 4 в `rng_probe`).
- `python3 tools/check.py` → 32 Python-теста, `cargo fmt`, `cargo clippy -D warnings`, doc-тесты PASS.
- `python3 tools/parity/run.py rng --strict` → 96 записей совпадают с Java-эталоном.
- `python3 tools/parity/run.py rng-extended --strict` → 13227 записей совпадают с Java-эталоном.

---

## Задача: CHUNK-001-A02 — Rust probe данных чанка

### Статус
Выполнена (`done`).

### Изменённые файлы
- `crates/beta-core/examples/chunk_probe.rs`:
  - Реализован CLI probe по контракту `CHUNK-001`.
  - Читает TSV stdin (`case_id<TAB>operations`), где `operations` — список операций через `;`:
    - `coord:V`: `split_block_coord(V)` -> `chunk,local`;
    - `index:X:Y:Z`: вычисленный индекс `y + z*128 + x*2048` или `invalid_coordinate`;
    - `get:X:Y:Z`: `id,metadata,block_light,sky_light` или `invalid_coordinate`;
    - `set:X:Y:Z:ID:M:B:S`: `ok`, `invalid_coordinate` или `invalid_nibble` (проверка координат первична);
    - `byte:ARRAY:INDEX`: значение байта (0..255) из указанного массива (`blocks`, `data`, `block_light`, `sky_light`); индекс вне диапазона даёт ошибку протокола.
  - Строгая валидация протокола:
    - Пропуск строго пустых строк и комментариев (`#`);
    - Запрет пробелов внутри `operations`;
    - Запрет пустых операций (висячие или повторяющиеся `;`);
    - Уникальность `case_id` внутри входного потока;
    - Суммарный лимит не более 100 000 операций;
    - Диагностика ошибок пишется в stderr с кодом возврата 1.
  - Написаны unit-тесты внутри probe: валидный поток операций, пропуск комментариев/пустых строк, отклонение пробелов, дубликатов case_id, синтаксических ошибок.
- `docs/tasks/status/agy.json` — отметка `CHUNK-001-A02` в состоянии `done`.

### Проверки
- `cargo test --example chunk_probe --locked --offline` → 5 тестов пройдены успешно.
- `cargo test --all-targets --locked --offline` → 31 тест пройден (17 lib.rs, 5 chunk_probe, 5 rng_extended_probe, 4 rng_probe).
- `python3 tools/check.py` → 32 Python-теста, `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test` PASS.
- `python3 tools/parity/run.py rng --strict` → PASS (96 записей совпадают).
- `python3 tools/parity/run.py rng-extended --strict` → PASS (13227 записей совпадают).

### Следующий шаг
- Задачи роли `agy` для пакета CHUNK-001 выполнены. Снапшот готов для интеграции и завершения приёмки `CHUNK-001-C03` ролью `codex`.
