# Отчёт Gemini / agy

## FLUID-001-A01 — follow-up required

Codex review found that the initial implementation does not match the C01 fixture: water-above has native side-only geometry (24 vertices), solid cardinal neighbors hide their shared face, and the emitted UV words differ from native. A01 remains pending. The repair must compare every ordered vertex and all eight native words across all 44 rows in `tests/fixtures/fluid/` before acceptance. A02 has not started.

### GAME-ORACLE-001-A01 follow-up — ray near-tie squared distance

- Codex review report `artifacts/parity/geometry/run-346l03ct/report.json` identified a concrete mismatch from custom input `/tmp/173st-ray-neartie.tsv`: for a box with X extent `[0, 1e-6]` and a reversed ray from X=`1e6` to `-1e6`, Rust selected face 4 / X=0 while native selected face 5 / X=`3eb0c70000000000`. The bytecode path is `eq.a(bt,bt)` → `bt.d(bt)`, which compares squared distance; the prior `start.length_to(position)` performed an extra MathHelper f32-narrowed square root, collapsing distinct distances into a false tie.
- Fixed `Aabb::ray_intersect` to compare `Vec3::distance_squared` and retain strict `<` update. Added a regression test for this reversed ray on thin boxes along X, Y, and Z (expected faces 5, 1, 3).
- Verification: `cargo test --locked --offline -p beta-core geometry::tests::` → 5 PASS; custom probe output exactly matches report native row; full 6278-row fixture is byte-identical; workspace clippy `-D warnings`, all-target tests, fmt and `git diff --check` pass.
- Only `geometry.rs`, this handoff and own agy status changed. Native oracle/fixtures and shared runner were not modified.

## GAME-ORACLE-001-A02 — strict geometry_probe

- Добавлен `crates/beta-core/examples/geometry_probe.rs` как thin adapter ко всем 22 операциям замороженного протокола: 6-аргументный box/vector формы, clip по трём осям, ray, distance и intermediate методы. Float параметры принимаются как ровно 16 lowercase hex digits и проверяются на finite и абсолютный диапазон `<= 30,000,000`; ID, exact arity, уникальность, ASCII, обязательный final LF, отсутствие blank rows и лимиты `100000` rows / `32 MiB` строгие. Результаты накапливаются до полного успешного разбора, ошибки идут только в stderr с ненулевым exit.
- Unit tests byte-for-byte сверяют все 6278 строк C01 `input.tsv`/`reference.tsv`; остальные cases проверяют все shapes/raw-bit output, malformed и позднюю ошибку без partial stdout, nonfinite/uppercase/out-of-range args, duplicates, final LF, IDs, row/byte limits. Native corpus не выявил расхождений в `geometry.rs`; его не менял.
- Изменены только `crates/beta-core/examples/geometry_probe.rs`, собственные `docs/tasks/status/agy.json` и этот handoff. C01 oracle/fixtures/audit, shared contract, runner, зависимости и соседний worktree не менялись.
- Проверки: `cargo test --locked --offline -p beta-core --example geometry_probe` → 4 PASS; `cargo run --quiet --locked --offline -p beta-core --example geometry_probe < tests/fixtures/geometry/input.tsv > /tmp/agy-geometry-probe.tsv` + `cmp ... tests/fixtures/geometry/reference.tsv` → exact byte match; invalid late-error subprocess → failure/empty stdout/stderr diagnostic; `python3 -m unittest tests.python.test_geometry_corpus -v` → 2 PASS; workspace `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`, `cargo test --all-targets --locked --offline`, fmt, JSON parse and `git diff --check` → PASS.
- C01 confirms native method parity only for this geometry corpus/JAR; this is not player simulation, loaded-world parity, or full game behavior. Next step: Codex C02 runner and saved-review integration.
- Model ID: **gpt-6-luna low** по назначению задачи; дополнительных агентов, ручных заимствований и зависимостей нет; token/cost telemetry недоступна.

## GAME-ORACLE-001-A01 — beta_core::geometry

- Реализован `crates/beta-core/src/geometry.rs` с `Axis`, `Vec3`, `Aabb` и `RayHit`, модуль экспортирован в `lib.rs`. API повторяет зафиксированные методы `eq`, `bt` и `vf`; новых dependencies и заимствованного кода нет.
- Сохранены детали bytecode: AABB factory хранит min/max как передано, а `Vec3::new` канонизирует `-0.0` в `+0.0`; pooled vector arithmetic сохраняет сырые знаки нуля. Length, distance и normalize используют `sqrt_f64_to_f32` и возвращают narrowed значение обратно как f64; normalize threshold — `1e-4`. Intermediate checks axis delta squared `< 1.0000000116860974E-7`, принимает fraction `[0,1]`. AABB clip использует строгий overlap на двух остальных осях, touching на axis разрешён. Raycast проверяет faces в порядке `4,5,0,1,2,3`, применяет strict closest-distance update, что сохраняет native tie face.
- Unit tests покрывают signed zero constructors, vector operations and sqrt narrowing, normalize/intermediate threshold neighbors, box sweep/expand/offset/clip/contact/intersection/containment/average, все шесть ray faces, edge tie, start-inside и miss.
- Изменены только `crates/beta-core/src/geometry.rs`, регистрация в `crates/beta-core/src/lib.rs`, собственные `docs/tasks/status/agy.json` и этот handoff. Общий API/protocol, C01 oracle/fixture, A02 probe, runner, зависимости и соседний worktree не менялись.
- Проверки: `cargo test --locked --offline -p beta-core geometry::tests::` → 4 PASS; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` → PASS; `cargo test --all-targets --locked --offline` → PASS (51 core library, 35 examples, 5 beta-view, 5 beta-viewer loader, 1 viewer binary); `cargo fmt --all -- --check` → PASS; `git diff --check` → PASS.
- Ограничение: C01 fixture и probe runner ещё не доступны, поэтому native differential parity не заявляется. Следующий шаг — после C01 fixture реализовать GAME-ORACLE-001-A02 в отдельном разрешённом файле `geometry_probe.rs`.
- Model ID: **gpt-6-luna low** по назначению задачи; дополнительных агентов, ручных заимствований и внешних зависимостей нет; token/cost telemetry недоступна.

## WORLD-LOAD-001-A01 — typed read-only loader

- Добавлен `beta_core::world` с `WorldLimits`, `WorldMetadata`, `LoadedChunk`, `WorldError`, `load_level_gzip` и `load_chunk_nbt`. Для level gzip повторно используется bounded `region::decompress(1, ...)`; эффективный decoded cap — минимум `max_level_decompressed_bytes` и `nbt.max_total_payload`, compressed cap проверяется до декодирования.
- Metadata читает только точные native scalar-типы; absent scalar defaults — 0/false/empty string, `Player` сохраняется полным `Tag::Compound`, dimension берётся из `Player.Dimension` с default 0. Boolean соответствует native policy `TAG_Byte != 0`. Ошибки malformed gzip/NBT, missing/wrong type, лимитов, chunk coordinates и массива явные.
- Chunk требует `Level`, `xPos/zPos` и все пять массивов контрактных размеров; signed NBT bytes преобразуются побитово в `u8` представление массива. Optional `LastUpdate/TerrainPopulated` используют defaults; absent lists становятся пустыми, nonempty lists требуют compounds. И root `NamedTag`, и точные decompressed/input NBT bytes остаются в результатах; unknown поля и list declarations сохраняются без записи обратно. Core не обращается к FS и не создаёт игровых объектов.
- Изменены: `crates/beta-core/src/world.rs`, экспорт в `lib.rs`, видимость существующего decoder как `pub(crate)` в `region.rs`, собственный `docs/tasks/status/agy.json` и этот handoff. Новых dependencies нет.
- Проверки: `cargo test --locked --offline -p beta-core world::tests::` → 4 PASS (metadata defaults/types, signed byte conversion, все пять lengths, coordinates, lists/unknown fields/raw player retention, gzip corruption/truncation и compressed/decompressed/NBT caps); `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` → PASS; `python3 tools/check.py` → 64 Python tests и fmt/check/clippy, 40 library + 22 example tests, doc tests PASS; `git diff --check` → PASS.
- Model ID: **gpt-6-luna low**. Заимствований, ручной помощи, новых зависимостей и изменений oracle/runner/контракта нет; стоимость/токены недоступны. Live world parity не запускался (A02 ещё не реализован).

## WORLD-LOAD-001-A02 — headless world probe

- Добавлен `crates/beta-core/examples/world_probe.rs`. CLI принимает только `INPUT_COPY_DIR RETAINED_DIR`, проверяет обе точные markers, ровно пять обычных файлов snapshot, размерные границы, канонический output parent и непересечение каталогов до создания retained directory. Region basename вычисляется через floor division; ID, дубликаты, пять TSV полей и координатная грамматика проверяются строго. Symlink и существующий output/payload отвергаются.
- Level gzip и каждый slot обрабатываются по очереди; region bytes кешируются по basename, LoadedChunk не накапливаются. Выдаются поля в порядке контракта (21 для level, 17 для chunk); absent сохраняет координаты и имеет ровно 12 `-` полей. Хеши массивов берутся из typed `ChunkData`, unknown names сортируются после UTF-8 hex-кодирования, empty name кодируется `~`, пустой набор `-`. Сохраняются точные decompressed original NBT bytes в `<id>.nbt` через `create_new`; для absent payload не создаётся.
- Изменены только `crates/beta-core/examples/world_probe.rs`, `docs/tasks/status/agy.json`, этот handoff. Cargo dependencies, oracle, runner и общий contract не менялись.
- Проверки: `cargo test --locked --offline -p beta-core --example world_probe` → 4 synthetic tests PASS; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` → PASS; `python3 tools/check.py` → 64 Python tests, fmt/check/clippy, 40 library + 26 example tests, doc tests PASS; `git diff --check` → PASS.
- CLI subprocess обработал C01 `live.tsv`: 749 rows, 704 present и 44 absent; каждый output row имел ожидаемое число полей. Полный stdout byte-identical к сохранённому C01 `native.stdout.tsv`. Отдельная ошибка basename вернула exit 1, пустой stdout и stderr diagnostic. Это прямая сверка с C01 native output; `python3 tools/parity/run.py world --strict` и C03 report review остаются приёмкой Codex.
- Model ID: **gpt-6-luna low**. Сессия A02 завершена; следующая работа — Codex C02 runner/review и C03 full package acceptance.

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

## VIEW-001-A01 — CPU scene, camera and mesh

- Реализован чистый `beta-view` API без filesystem/window/GPU кода: `SceneBounds` проверяет только конечные block coordinates внутри максимум 5×5 chunks; `Scene` хранит неизменяемые загруженные `ChunkData` и native mappings. Отсутствующие slots/neighbors считаются воздухом.
- Mesher читает только block states с точным `(id, metadata, above_id)` native mapping (для статических блоков используется `above_id=None`). Поддерживаются только C01-подтверждённые render-type-0 opaque full cubes с шестью alpha-opaque faces; unsupported states опускаются и учитываются. Грань убирается только если сосед тоже имеет подтверждённый поддержанный mapping такого куба.
- Геометрия и UV следуют default-rotation flat `cv.b` bytecode emitters `a..f`, faces 0..5: сохранён native порядок четырёх position/UV corners по каждой стороне, lower UV границы начинаются ровно на tile origin, high edges — на `+15.99` pixels. Directional shades: bottom .5, top 1, Z .8, X .6. Для каждой грани сохраняются sky/block nibbles соседнего sample position. Packed `uu.d` brightness conversion, lighting gamma и AO corner samples не реализованы; это raw lighting input, не native brightness parity.
- Добавлены hard caps scene size (25 chunks / 819200 blocks) и mesh output (5,000,000 vertices / 7,500,000 indices); callers могут выставить меньшие limits. Camera API задаёт перспективу 70°/near .05/far 128, фиксированную стартовую позицию по spawn, полёт/поворот, вертикальный look clamp и WGPU right-handed zero-to-one projection.
- Tests проверяют все 24 position/UV pairs по native emitter table, outward face winding, tile-origin/high-edge UV values, source neighbor nibbles/shade, grass above-ID context, culling двух подтверждённых кубов, unsupported block counts, limits, malformed chunk arrays, bounds/duplicates/coordinates и camera.
- Изменены `crates/beta-view/src/lib.rs`, `docs/tasks/status/agy.json` и этот handoff. Другие worktrees/branch, shared oracle, runner и contract не менялись; новых dependencies нет.
- Проверки: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`, `cargo test --workspace --all-targets --locked --offline` (40 beta-core + 5 beta-view + 26 example tests), `cargo test --workspace --doc --locked --offline`, `python3 tools/check.py` (81 Python tests и все Rust checks) и `git diff --check` → PASS.
- После C02 commit `ef16cf31ce89152478e04fa4efa2866ad43bbc33` выполнен `python3 tools/parity/run.py view --strict`: ожидаемый `not_implemented`, поскольку A02 `view_probe` пока отсутствует; Java oracle выполнил два native runs. CPU/native mapping parity и пиксельный parity не заявляются в A01.
- A01 завершена; A02 остаётся следующей задачей роли `agy` после C02.

## VIEW-001-A02 — viewer и native mapping probe

- Интегрирован `view_probe`/`loader`: headless probe читает только marked copy фиксированного мира и локальный pinned atlas; строгий формат TSV сравнивает native render types, opaque/full classification, alpha masks, face tiles и visibility для 50 mapping cases и 1,617 neighbor tuples. Probe slice — commit `1101886`, интегрирован в `agy/development` коммитом `45bd46d`.
- `beta-viewer` копирует pinned level/region исходники в уникальный ignored `artifacts/viewer/run-*/input`, проверяя SHA-256 до загрузки, а затем вызывает `load_scene_for_viewer`; исходное сохранение не изменяется. Camera берёт spawn из копии, scene — только 25 координат из fixture.
- Winit/wgpu binary загружает оригинальный 256×256 atlas с закреплённым hash, строит static GPU buffers из CPU mesh, использует nearest sampling и depth buffer, поддерживает resize, surface lost/outdated/error handling и WASD/Space/Ctrl/Shift/mouse-look/Esc. Заголовок окна показывает loaded chunks, faces и unsupported blocks. Saved block/sky nibbles подаются в shader как видимая bounded lighting approximation; это не native packed `uu.d`/AO parity.
- Локальный GUI smoke под Xvfb запустил окно и отрисовал текстурированный фиксированный ландшафт (деревья, трава, земля и камень); кадр сохранён вне Git в `/tmp/beta-viewer-smoke.png`. Software Mesa сообщила DRI3 fallback warning; окно/кадр отработали без wgpu validation error.
- После code commit `f6c2315` выполнен чистый `python3 tools/parity/run.py view --strict` → PASS, все 50 mapping cases и 1,617 neighbor tuples совпали; report `artifacts/parity/view/run-p96kzl8b/report.json`. `python3 tools/parity/verify_report.py ... --protocol` и `python3 tools/parity/view_review.py ...` → PASS; проверены scene/JAR/atlas/source/copy/output hashes, native audit и protocol rejection.
- `python3 tools/check.py`, `cargo test --all-targets --locked --offline`, `cargo check --locked --offline -p beta-viewer`, `cargo clippy --locked --offline -p beta-viewer -- -D warnings`, viewer unit tests (5 loader tests), `git diff --check` → PASS.
- A02 выполнена и закоммичена; A02 runner/mapping parity проверены, но визуальный вывод остаётся smoke-tested, без pixel-parity claims. Codex может начинать VIEW-001-C03.

### Texture minification follow-up — c98e227

- По присланному скриншоту добавлены четыре tile-isolated mip levels для terrain atlas с linear mip-level blending. Каждый mip строится независимо внутри исходных 16×16 atlas tiles, чтобы уменьшить minification aliasing без протекания цветов соседних плиток; nearest sampling внутри уровня оставляет крупные текстуры резкими.
- Добавлен unit test, который проверяет пять размеров mip chain и гарантирует, что все 256 одноцветных тестовых плиток не смешиваются между уровнями.
- `cargo test --locked --offline -p beta-viewer`, `cargo clippy --locked --offline -p beta-viewer -- -D warnings`, `cargo fmt --all -- --check`, `python3 tools/check.py` → PASS. Окно запускалось под Xvfb до таймаута с software Mesa warning; на момент повторного smoke Xvfb не смог стартовать из-за владельца `/tmp/.X11-unix`, поэтому новый screenshot после mipmap правки не сохранён.
- `python3 tools/parity/run.py view --strict` → PASS 50/50 mappings + 1617/1617 neighbor tuples; clean report `artifacts/parity/view/run-bfyl2hef/report.json`; saved protocol verification и `view_review.py` → PASS. Mapping/scene output не изменён.

## MATH-001-A01 — beta_core::math

- Добавлен чистый `beta_core::math` API: `sin`, `cos`, `sqrt_f32`, `sqrt_f64_to_f32`, `floor_f32`, `floor_f64`, `abs_f32`, `abs_max_f64`, `floor_div_i32` и `is_null_or_empty`; модуль экспортирован из `lib.rs`. Новых dependencies и probe-кода нет.
- Trig использует вычисляемую 65536-элементную таблицу и точный native LUT index/offset: sine `f2i(x * 10430.378f)`, cosine `f2i(x * 10430.378f + 16384f)`. Значения таблицы строятся по native double operation order до cast в `f32`; сравнение C01 reference прошло для каждого LUT index обоих методов.
- Floor сохраняет Java float-to-int saturation, сравнение с усечённым значением и переполнение `i32` при корректировке отрицательного результата. `abs_f32` и `abs_max_f64` повторяют разные bytecode comparison paths, включая NaN payload/sign и signed zero. `floor_div_i32` реализует bytecode формулу `x < 0 ? -((-x - 1) / y) - 1 : x / y` с wrapping промежуточными `i32`; делитель 0 возвращает `MathError::DivisionByZero`.
- Unit tests прогоняют все 131408 строк committed original corpus (`sin`/`cos` по 65536 каждое, float/double edges, abs, 169 floor-div cases и строки) и сравнивают raw bits. Отдельно проверяются деление на ноль, signed quotient behavior, negative floor, NaN/infinity cast boundaries и Unicode/empty/null strings.
- Изменены `crates/beta-core/src/math.rs`, `crates/beta-core/src/lib.rs`, `docs/tasks/status/agy.json` и этот handoff. C01 fixtures/oracle, queue, статусы Codex и shared contract не менялись в agy worktree.
- Проверки: `cargo test --locked --offline -p beta-core math::tests::` → 4 PASS; `cargo test --all-targets --locked --offline` → 81 PASS по workspace; `cargo test --doc --locked --offline` → PASS; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` → PASS; `python3 tools/check.py` → 85 Python tests и все workspace checks PASS.
- Ограничение: таблица вычисляется средствами платформенного Rust `f64::sin`; точное совпадение доказано для pinned Linux/JDK reference corpus и текущего toolchain, переносимость raw LUT bits на другие math library/platform не проверялась. Probe/live parity остаются задачей A02/C02/C03.
- Следующий шаг: выполнить MATH-001-A02 после коммита A01.

## MATH-001-A02 — math_probe

- Добавлен `crates/beta-core/examples/math_probe.rs`, thin stdin/stdout adapter к `beta_core::math` для `sin`, `cos`, обоих sqrt, обоих floor, `abs_f32`, `abs_max_f64`, `floor_div_i32` и `null_or_empty`.
- Формат на строку: `case_id<TAB>operation<TAB>arguments...` → `case_id<TAB>raw_result`. Float аргументы — ровно 8 hex digits, double — 16; результаты lowercase raw bits. Строка передаётся как hex UTF-8, `null` — отдельный токен. Деление на ноль возвращает `error_arithmetic`, `i32::MIN / -1` — `2147483646` по оригинальному bytecode.
- Parser проверяет UTF-8, arity, точные bit widths, десятичные `i32` границы, hex/UTF-8 string arguments, IDs, дубликаты, неизвестные операции; ограничивает stdin 64 MiB и 200000 cases (C01 corpus — 131408). Результат сначала накапливается в памяти и пишется в stdout только после полного успешного разбора; invalid input даёт exit failure и только stderr diagnostics.
- Regression tests включают полный committed C01 input/reference corpus и протокольные synthetic cases, позднюю ошибку после валидной строки (пустой stdout), дубликаты, unknown op, переполнение raw/integer диапазона, неверные arity, malformed UTF-8/hex, comments/blanks и оба лимита.
- Изменены только `crates/beta-core/examples/math_probe.rs`, `docs/tasks/status/agy.json` и этот handoff. Oracle, fixture, runner, общий контракт, зависимости и другие статусы не изменялись.
- Проверки: `cargo test --locked --offline -p beta-core --example math_probe` → 5 PASS; `cargo clippy --locked --offline -p beta-core --example math_probe -- -D warnings` → PASS; полный `python3 tools/check.py` → 85 Python tests и workspace fmt/check/clippy/all-target/doc PASS; `git diff --check` → PASS.
- Subprocess C01: `cargo run --quiet --locked --offline -p beta-core --example math_probe < tests/fixtures/math/input.tsv` вывел ровно 131408 LF строк и byte-identical output с committed reference. Отдельно собран committed `MathOracle.java` и запущен против закреплённого клиентского JAR; Java и Rust TSV прошли `cmp`, SHA-256 обоих outputs `feddfb35ebc8b36f1403d16ffe6f7d3190036af721d0bad840bbabe6f9ec9f3f`. Пять invalid real subprocess inputs дали ненулевой exit, пустой stdout и диагностику stderr.
- Native parity здесь ограничена именно набором C01 MathHelper-вызовов и закреплённым локальным клиентом/JDK. Это не доказывает подключение функций к игровому циклу; live saved-report review выполняет C03.

## JAVA-001-A01 — beta_core::java_compat

- Добавлен чистый `beta_core::java_compat`: `f32_to_i32`, `f64_to_i32`, `f32_to_i64`, `f64_to_i64`, `string_hash_code`; `java_compat` экспортирован из `lib.rs`. Новых зависимостей, I/O и probe-кода нет.
- Float casts используют Rust's defined float-to-integer cast semantics, которые совпадают с Java: NaN → 0, infinities/out-of-range clamp к границе target integer, конечные значения truncate toward zero. String hash проходит UTF-16 code units через `hash = hash * 31 + unit` с wrapping `i32`, включая supplementary code points.
- Unit test читает committed `tests/fixtures/java/input.tsv` и `reference.tsv`, декодирует raw float/double bits и UTF-8 hex, затем сравнивает каждую операцию по ID и decimal result. Проверены все 18493 строки (4109 на каждую из четырёх conversion operations и 2057 strings). Дополнительные assertions покрывают NaN, ±infinity, truncation, ASCII, BMP, non-BMP и Java hash value overflow.
- Изменены только `crates/beta-core/src/java_compat.rs`, `crates/beta-core/src/lib.rs`, `docs/tasks/status/agy.json` и этот handoff. Fixture, oracle, probe/runner, общий контракт, зависимости и соседний worktree не менялись.
- Проверки: `cargo test --locked --offline -p beta-core java_compat::tests::` → 3 PASS; `python3 tools/check.py` → PASS (workspace fmt/check/clippy/all-target/doc plus Python tests); `git diff --check` → PASS.
- Ограничение: подтверждено поведение для committed fixture, созданной текущей JVM; прямой probe subprocess и live parity относятся к A02/C02/C03.
- Следующий шаг: JAVA-001-A02 — TSV probe; текущий A01 commit содержит только библиотечный модуль, unit tests и собственные status/handoff.

## JAVA-001-A02 — java_compat_probe

- Добавлен `crates/beta-core/examples/java_compat_probe.rs`, thin stdin/stdout adapter к `beta_core::java_compat` для всех пяти contract operations. Строгий вход: ровно `case_id<TAB>operation<TAB>argument`; conversion arguments имеют точную ширину lowercase raw hex (8 для f32, 16 для f64), строки — lowercase hex UTF-8 (empty разрешён). Выход `case_id<TAB>decimal_result` с LF.
- Проверяются ASCII stdin с обязательным final LF, exact field count, ID `[A-Za-z0-9_.:-]{1,96}`, unique IDs, operation name, raw-bit width/characters, decoded UTF-8 validity, row limit 100000 и input limit 16 MiB. Результаты накапливаются до полного successful parse, поэтому invalid protocol не оставляет partial stdout; ошибки и неожиданные CLI arguments дают failure code и stderr diagnostics.
- Example regression tests сверяют все 18493 committed C01 rows с `reference.tsv` побайтно, проверяют все пять operations и LF/decimal formatting, wrong arity, duplicate ID, unknown operation, wrong/uppercase raw bits, malformed/odd/invalid-UTF8 string hex, blank/comment rows, missing final LF, CRLF, позднюю ошибку с empty stdout и оба лимита.
- Изменены только `crates/beta-core/examples/java_compat_probe.rs`, `docs/tasks/status/agy.json` и этот handoff. Java oracle, fixture, parity runner, shared contract, API и зависимости не менялись.
- Проверки: `cargo test --locked --offline -p beta-core --example java_compat_probe` → 4 PASS; `python3 tools/check.py` → PASS (workspace formatting/check/clippy/all-target/doc and Python checks); `git diff --check` → PASS.
- Subprocess C01: Rust probe выдал 18493 строки и byte-identical committed reference, SHA-256 `397c7c50db72608ccb863d2f00c331da8982dee4cb31cc702ddb158003ab99e8`. Java `JavaCompatOracle` был отдельно скомпилирован и запущен на тех же входах; `cmp` Java/Rust outputs прошёл с тем же SHA. Восемь malformed subprocess streams и unexpected command-line arg завершились ненулевым кодом с пустым stdout и диагностикой в stderr.
- Native parity подтверждена на 18493 conversion/hash fixture rows для pinned oracle/runtime; подключение в игровую логику и интеграционный saved-report review находятся вне A02.
# MOVE-001-A01 — native movement resolver and dry travel

Implemented `beta_core::motion` in `crates/beta-core/src/motion.rs` and exported
it from `lib.rs`. `MotionState::new` creates the frozen 0.6f32 × 1.8f32 player
box and default step height. `move_entity` follows pinned `sn.b(DDD)` ordering
(Y→X→Z), filters caller-provided ordered candidates by each native query AABB,
and handles no-clip, web scaling, two independent sneak support loops,
allow-slide delta clearing, step retry with BASE winning exact ties, position and
collision flags, velocity clipping, y-size smoothing, and fall distance.
`dry_travel` follows `ls.a_(FF)`, inherited `sn.a(FFF)`, and `ls.R()` for the
frozen dry environment: native f32 heading/acceleration, yaw, jump, collision,
gravity, and air/stone/ice friction.

Verification: `cargo test --locked --offline -p beta-core motion::tests::` passed
6 synthetic tests; `cargo clippy --workspace --all-targets --locked --offline --
-D warnings`, `cargo test --all-targets --locked --offline` (58 beta-core library,
39 examples, 11 beta-view/viewer tests), and `cargo fmt --all -- --check` passed.
These are unit/workspace checks only; no native movement parity claim until C01
fixture and A02 probe are available. No external dependencies or oracle/fixture
changes.

Scope: only `motion.rs`, `lib.rs`, `docs/tasks/status/agy.json`, and this handoff.
Next: wait for accepted C01 snapshot before MOVE-001-A02.

### MOVE-001-A01 follow-up — step descent and grounded friction defaults

Bytecode review of `sn.b(DDD)` showed that a step retry clips and offsets the
box downward by `-stepHeight` using the same step candidate list before comparing
horizontal paths. Implemented this descent, including `allow_slide=false` checks
against the original movement Y (the native comparisons at the up/down retry
boundaries), and retained BASE on equal horizontal distance. Fractional y-size
smoothing now groups `fractionalY + 0.01` before adding old `y_size`, as in the
native double-expression order.

Review of `ls.a_(FF)` showed two friction lookups: each starts at `0.91f`, or
`0.54600006f` when grounded, then replaces it with block slipperiness × `0.91f`
only when a non-air block is found. Both lookups occur before `sn.b(DDD)`; the
first controls acceleration and the second controls post-movement horizontal
drag. A02's exact corpus test caught the previous post-movement lookup order.
Regression tests cover grounded Air with no supporting voxel, partial-height
step landing, and the allow-slide-disabled step retry/base restoration.

Verification: `cargo test --locked --offline -p beta-core motion::tests::` → 6
PASS; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`,
`cargo test --all-targets --locked --offline` (58 beta-core library, 39 examples,
16 view/viewer tests), workspace doc tests, fmt, JSON validation and `git diff
--check` all pass. At the A01 stage, native parity remained pending C01/A02.

## MOVE-001-A02 — strict movement probe and native parity

Added `crates/beta-core/examples/movement_probe.rs` implementing the frozen
six-field MOVE-001 TSV protocol. It validates each full state, ordered collider,
surface and mode-specific frame before simulating any row; IDs and all resource
limits are checked, and output is buffered until all scenarios pass. The output
serializes 18 state components as raw float bits plus decimal flags. Regression
tests cover the complete native fixture, malformed input after valid rows with
empty stdout, scenario/step/byte/frame/collider limits, and protocol bounds.

The first comparison exposed a concrete `dry_travel` issue: `ls.a_(FF)` performs
both ground-friction lookups before invoking `sn.b(DDD)`. The prior implementation
looked up horizontal drag after movement, where contact flags/support could have
changed. Against the unchanged native fixture, that order produced 5376/8213
state mismatches. First repro: `travel-000-stone`, frame 0, reference Z velocity
`3fa65d39973abc40`, post-move-lookup result `3fb2a305380d1b60`. Moving the drag
lookup before `move_entity` restored exact parity. The grounded Air regression
also now asserts the native fallback drag `0.54600006f` when no support voxel is
found.

Verification: `cargo test --locked --offline -p beta-core --example
movement_probe` → 4 PASS; `cargo test --locked --offline -p beta-core --lib
motion::tests` → 6 PASS; direct probe output compares byte-for-byte with all 8213
native reference states (SHA-256
`80e3faef85ac842d756c0ac85731208db583d009ecd884627ff33e94dbc91bbd`). Full
`cargo test --all-targets --locked --offline` (58 library, 39 examples, 16
view/viewer tests), workspace clippy, and fmt passed. Oracle and fixtures were
not changed.

## UV-001-A01 — cube face UV association

Consumed C01 observations from `tests/fixtures/uv/cases.tsv` and `native.tsv`
after fast-forwarding `a936ffe`. The provenance header identifies pinned client
JAR SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`, native
`cv.a-f(uu,DDD,I) -> nw.a(DDDDD)`, and packed float32 Tessellator vertex values.
The corpus has 84 direct-face observations; each submits six triangle vertices
with four unique xyz corners.

Added a fixture-driven regression in `crates/beta-view/src/lib.rs`. It maps
native UV bits by case ID and xyz instead of assuming submission order, checks
duplicate triangle vertices have equal UV, and compares all 336 unique Rust
corners by raw f32 bits. Against the unchanged renderer this test first failed
with 112 mismatches: 56 on face 1 (top) and 56 on face 3 (+Z), across
asymmetric tiles and positive/negative origins. Faces 0, 2, 4, and 5 already
matched. Example: `f1-t0-p0` at xyz `[40400000,40800000,40a00000]` had native
UV `[3d7fd70a,3d7fd70a]` while Rust produced `[3d7fd70a,00000000]`.

Corrected only the UV association arrays for faces 1 and 3 and updated the
existing six-face UV expectation. Vertex positions, outward winding, indices,
tile selection, shade, light, and collision code are unchanged. The C01 oracle
and fixtures were not modified.

Verification: pre-fix fixture test failed as expected with the mismatch set
above; after the change `cargo test --locked --offline -p beta-view` passed (8
unit tests and doc tests), including all 84 observations / 336 unique corners.
`cargo clippy --locked --offline -p beta-view --all-targets -- -D warnings`,
`cargo fmt --all -- --check`, and `git diff --check` passed. Broader viewer
integration and screenshot acceptance remain C02 scope. No other agents were
used.

## COLOR-001-A01 — seeded biome climate and color palettes

Implemented the frozen `beta_core::biome` API in commit `9dfc4f2`: seeded scalar
climate sampling follows the native `xv.a(x,z,1,1)` path, including Java RNG
consumption, wrapped seed multipliers, simplex floor behavior, octave arithmetic
order, palette index conversion and metadata precedence. Added
`beta_viewer::colors` in commit `212c79e`: it loads the original grass/foliage
PNGs only after checking their pinned SHA-256 and exact 256×256 dimensions, then
stores row-major ARGB values. The `color_probe` example implements the frozen
ASCII TSV protocol with duplicate-ID detection, canonical numeric parsing,
4 MiB/4096-row limits and buffered all-or-nothing stdout.

After consuming C01 snapshot `6eb78c6` with a regular merge, added a core
regression that compares raw `f64` bits for every one of the 1280 native climate
rows. The full probe was run with the locally prepared original palette assets:
all 1600 output rows matched `tests/fixtures/color/native.tsv` byte-for-byte;
both files have SHA-256
`09301df08d99dfbe094b348d3e6127479090f614e2cbd9542ea1aa284d385944`. No native
oracle or fixture files were changed. The loader verifies the palette hashes
`d9b2d9073f06013b9f36b1c61a91678be478760abff31fae942ee52ad898ca1b` (grass)
and `644a527dd7daea5a195bbed056a6a41e295b6043a05621f4c5c263a25fb171cc`
(foliage).

Verification: `cargo test --locked --offline --all-targets`, workspace clippy
with `-D warnings`, `cargo fmt --all -- --check`, and `git diff --check` passed.
Probe tests cover both output shapes, late invalid rows after valid rows, duplicate
IDs, strict lowercase raw hex, missing final LF, non-ASCII input, CRLF and limits.
The fixture run uses the original local PNG assets and is recorded here; those
assets are not embedded or committed. No external dependencies were added.

## LIGHT-001-A01 — saved light and clear-weather sky arithmetic

Added `beta_core::light` and its public export in `e195649`. `brightness`
preserves provider f32 table construction for Overworld (.05) and Nether (.1);
`combined_light` implements original saved nibble subtraction and block comparison;
`sample_brightness` applies the native minimum before table lookup. The sky API
implements Java signed remainder/time normalization, provider-specific celestial
angle (Overworld smoothed with f64 `Math.cos`, Nether fixed at .5), and clear-weather
subtraction using the existing native cosine LUT. `emission_level` reflects the
original registry’s nonzero IDs, including glowstone 89 and jack-o-lantern 91 at
15. These are scalar APIs only; special neighbor-sampling block IDs, AO, weather
updates, propagation, fluids, Skylands, and rendering integration remain excluded.

Added strict bounded `light_probe` in `3e5dde0` with the frozen sample/time/
emission row shapes, canonical decimal and lowercase raw-f32 parsing, ASCII/LF
validation, unique IDs, no CLI arguments, 100,000-row/16 MiB limits, and buffered
atomic stdout. After consuming the early C01 snapshot `c610b1a`, added a dynamic
fixture test in `1da36ad`: it asserts matching input/reference line counts and a
minimum of 50,000 rows, then compares every integer and f32 raw bit. Final C01
snapshot `8c8c6d1` was merged as `c46b436`; all 72,928 rows passed. The fixtures
are from the pinned client JAR SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`; native
provenance, class hashes, bootstrap, calls and corpus scope are recorded in
`tests/fixtures/light/provenance.md` and `audit.json`. No oracle or fixture files
were changed by A01.

Verification: `cargo test --locked --offline -p beta-core
light::tests::all_native_light_fixture_rows_match_raw_values` passed for all
72,928 rows. Running `light_probe` on the complete input and comparing with the
native reference via `cmp` passed; both output files have SHA-256
`44072d43233b6265ebd5246bfe096c88bea5e35c6b2db1c52113bdf1f0fdd2f3`. Probe
protocol tests, `cargo test --locked --offline --all-targets`, doc tests, workspace
clippy with `-D warnings`, fmt, and `git diff --check` passed. No dependencies
were added. Next: Codex C02 strict runner/review and independent live verification.
## FLUID-001-A01 — complete

Pure `beta_view::fluid` geometry for water IDs 8/9 now matches all 44 committed native C01 scenes vertex-by-vertex and raw-word-by-raw-word. Coverage includes shared water material corner averaging, water above, cardinal/diagonal neighbors, solid-face visibility, native N/S/W/E order, face colors, metadata-dependent side UVs, and flow-angle top UV rotation. A01 is accepted locally; A02 remains next for bounded probe and transparent viewer integration.

Checks: `cargo test -p beta-view` PASS (17 tests, including all 44 native scenes), `cargo clippy -p beta-view --all-targets -- -D warnings` PASS, `cargo fmt --all -- --check` PASS, and `git diff --check` PASS. No oracle, fixture, runner, or viewer files changed.

## FLUID-001-A02 — probe and transparent scene integration

Added `crates/beta-viewer/examples/fluid_probe.rs`. It accepts only the bounded,
frozen one-line C01 suite on stdin, rejects arguments or any input mutation, and
buffers all output before writing. Output is exactly the 44 ordered case rows and
1,242 vertices with eight lowercase raw-u32 words per native vertex. Its corpus
regression compares the entire generated output to `native.tsv`.

Added bounded `Scene::fluid_mesh`, which scans loaded chunks for IDs 8/9, builds
the native-compatible per-cell neighborhoods, and routes resulting vertices to
a separate viewer vertex buffer. Water no longer contributes to the opaque mesh
or its unsupported-block count. The original opaque/cutout shader and pipeline
are unchanged. A second pipeline preserves sampled atlas alpha, uses alpha
blending and depth testing with depth writes disabled, and draws after opaque
geometry. C01 did not exercise native GPU state; blend/depth behavior is a viewer
choice, with no claim of pixel parity, sorted transparent overlap, lava, fluid
simulation, or FPS improvement.

The fixed local scene smoke rendered 25 chunks, 334,776 opaque vertices, and
9,768 water vertices. Screenshot: ignored local artifact
`artifacts/fluid-viewer.png` (640×480, SHA-256
`67f69d0afed0bbb3f44be16e0e074a7461c808199946340c21aaaffdb17e1009`). The five
pinned source-save hashes (`level.dat` and four region files) matched before and
after the viewer run.

Verification: `cargo test --locked --offline -p beta-view -p beta-viewer
--all-targets` → PASS (18 beta-view, 13 viewer library, 5 viewer binary, all
examples); `cargo clippy --locked --offline -p beta-view -p beta-viewer
--all-targets -- -D warnings` → PASS; `cargo fmt --all -- --check` and
`git diff --check` → PASS. `python3 tools/parity/run.py fluid --strict` → PASS:
all 44 cases / 1,242 native vertices exact. The retained report must be rerun
after the A02 commit so its Rust-revision binding is clean. No dependencies,
oracle, fixtures, runner, queue, or Codex status were changed.

## BLOCK-001-A01 — bounded block targeting and edits

Added `beta_core::block` over an owned bounded set of loaded `ChunkData`. The API
traces a five-block normalized ray using `geometry::Aabb`, reports hit/miss,
unsupported state, or an unloaded cell explicitly, and accepts the caller's
native-confirmed full-cube state predicate. Break/place validate the selected
state, metadata, occupancy, and loaded target before mutation. Successful edits
increment the scene revision and return sorted loaded chunk coordinates whose
meshes may need rebuilding. No persistence or file access was added.

Seven block unit tests and all 76 beta-core tests pass. C01-derived regressions
cover six faces, the negative-coordinate voxel, corner tie, miss, both open
endpoints, the 5.0/5.001 reach edge, and unsupported slab classification. Hit
positions are compared by raw f64 words. `cargo clippy
--locked --offline -p beta-core --all-targets -- -D warnings`, `cargo fmt --all
-- --check`, and `git diff --check` pass. C01 has no verified break or place
actions, so those operations are unit-tested only and no action parity is
claimed. Native full-cube state mapping remains supplied by the caller. No
fixture, oracle, runner, queue, shared contract, or viewer files changed.

Follow-up review correction: `raycast_segment` now counts the voxel cells the
open segment can visit before entering the DDA loop and rejects a segment whose
cell count exceeds `MAX_RAY_STEPS`. The diagonal `(0.999,0.999,0.999)` to
`(11.5,11.5,11.5)` case now returns `RayStepLimit` instead of silently missing a
block after step exhaustion. The C01 5.001 hit also asserts the native raw point
words `4014000000000000,3fe0000000000000,3fe0000000000000`.

Follow-up verification: `cargo test --locked --offline -p beta-core` → PASS
(77 library tests); `cargo clippy --locked --offline -p beta-core --all-targets
-- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check` → PASS.

## BLOCK-001-A02 — fixture probe and disposable viewer interaction

- Added `crates/beta-core/examples/block_probe.rs`. It parses the bounded C02
  input rows and derives each result from the supplied segment, stop-on-liquid
  flag, and block state. It has no case-ID result branches. The output preserves
  face, coordinates, decimal point values and all three raw binary64 words. Its
  dynamic example test reads `tests/fixtures/block/input.tsv` and
  `native.tsv` and compares all 17 rows in order. Protocol tests cover malformed
  input, duplicate IDs, unsupported states and the input byte limit atomically.
- Extended the core ray API with caller-selected verified shapes and the native
  `stop_on_liquid` option. The exact C01-backed `(44,0)` lower slab returns a
  non-editable `LowerSlab` hit; `(8,0)` source water is skipped when false and
  returns a non-editable `Liquid` hit when true. Unknown states remain explicit
  `Unsupported`, and only `FullCube` hits pass break/place validation. Hit
  coordinates/faces/raw points for slab and water are unit-tested, along with
  non-editability and unchanged revision on rejection.
- The viewer now receives owned copies of the loaded 5x5 `ChunkData` plus native
  mappings. It constructs a separate `BlockScene`, displays the aimed block and
  side in the window title, and accepts left-click break/right-click place only
  while captured. Placement uses local test state `1:0`. After a successful edit,
  the changed cell is copied into the viewer-owned chunks; the full bounded scene
  and both opaque/fluid meshes are rebuilt, and the GPU buffers are replaced.
  Edits never reach the loaded source save. C01 action captures do not exist, so
  this is local sandbox behavior only; no native break/place parity is claimed.
- Checks: `cargo test --locked --offline -p beta-core --all-targets` → PASS
  (77 library tests, 3 block probe tests and existing example tests);
  `cargo test --locked --offline -p beta-viewer --all-targets` → PASS (13 library,
  6 binary, 4 example tests); `cargo clippy --locked --offline -p beta-core
  -p beta-viewer --all-targets -- -D warnings`, `cargo fmt --all -- --check`,
  and `git diff --check` → PASS.
- `python3 tools/parity/run.py block --strict` → PASS on clean code revision
  `7b8ae7170b1ba3c5c166026fc0821016645dbe1b`: all 17 native ray rows match
  exactly, zero mismatches; strict report has `action_status=not_implemented`
  and `actions_compared=false`, and before/after source hashes match. The runner
  supplies its pinned 17-case order; the probe accepts any bounded valid rows and
  computes from those row values.
- `env -u WAYLAND_DISPLAY xvfb-run -a timeout 10s cargo run --locked --offline
  -p beta-viewer` → scene loaded and rendered under Xvfb (25 chunks, 334,776
  opaque vertices, 9,768 water vertices). The controlled timeout ended the smoke
  after startup; this environment did not automate mouse clicks, so viewer
  interaction was not manually exercised here. No oracle, fixture, runner, queue,
  shared contract or dependency files changed.

## FLIGHT-SMOOTH-001-A01 — debug-flight render interpolation

Kept debug-flight movement on fixed 50 ms ticks and added a separate previous/current
flight camera position to `MotionController`. Each simulation tick starts from the
last fixed position, advances with the existing camera movement and speed, and stores
the new fixed state. Rendering interpolates those positions with the existing
accumulator fraction. This prevents interpolation from feeding back into movement;
yaw and pitch still update immediately from mouse events. Entering/leaving flight,
reset, and focus loss synchronize the render history to the current camera.

Added controller tests that simulate one second split into 20, 30, 60, and 144 Hz
frames, assert the same fixed state and monotonic samples, and cover enter, land,
reset, and focus-loss synchronization. Changed only
`crates/beta-viewer/src/controller.rs`, `crates/beta-viewer/src/main.rs`, this
handoff, and `docs/tasks/status/agy.json`.

Verification: `cargo test --locked --offline -p beta-viewer` passed (15 library and
6 binary tests); `cargo check --workspace --locked --offline` and
`cargo clippy --workspace --all-targets --locked --offline -- -D warnings` passed.
`python3 tools/check.py` passed all 180 Python tests (18 opt-in skips), workspace
format/check/clippy/all-target tests and doc tests. The requested Xvfb smoke was
attempted with `env -u WAYLAND_DISPLAY xvfb-run -a timeout 10s cargo run --locked
--offline -p beta-viewer`, but the viewer exited before rendering because the local
`level.dat` does not match its pinned source hash. No movement, physics, fixture,
oracle, queue, or shared contract was changed. No other agents were used.

## ITEMS-001-A01 — pure bounded stack and main inventory

Imported the committed C01 snapshot from `codex/verification` (`e2cdc49`;
current shared snapshot `e9190e8`) with a fast-forward. Added
`beta_core::items` with 36 main slots, selected hotbar index 0..8, stack
identity/count/damage/verified limit, split, remove-one-by-ID, slot replacement,
compatible merge/spill, insertion, item selection, and native-direction hotbar
cycling. The supported registry is deliberately limited to native-captured IDs:
stone/dirt 1/3 (64), snowball 332 (16), and iron sword 267 (1). Nonzero damage
and other IDs fail explicitly. No GUI, armor, crafting, arbitrary NBT, I/O, or
save writes were added.

Six fixture-driven tests compare all 21 C01 states, including the ordered
contents of all 36 slots, limits, selection and native scroll results. They also
check invalid mutation atomicity. The captured fully occupied insertion failure
preserves inventory and incoming stack; behavior for unobserved partial-capacity
failure shapes is not claimed as native parity. The code does not compare item
display names because they are outside the core state model.

Verification: `cargo test --locked --offline -p beta-core --all-targets` passed
(83 library tests and all beta-core example tests);
`cargo clippy --locked --offline -p beta-core --all-targets -- -D warnings`,
`cargo fmt --all -- --check`, and `git diff --check` passed.
`python3 tools/check.py` passed all 182 Python tests (18 opt-in skips), workspace
format/check/clippy/all-target tests and doc tests. The native fixture's 21 state
rows were compared by the Rust unit tests; no separate ITEMS strict runner exists
in this card. Only `crates/beta-core/src/`, this handoff, and the agy status were
changed by A01. Next dependency is ITEMS-001-C02 before A02.

Follow-up review correction: the native rows with selected indices `-1` and `9`
were present in the fixture but were not yet exercised by `assert_native`. Added
read-only `Inventory::selected_stack_at(index)`, which returns `None` for those
raw native out-of-range lookups without permitting the safe inventory's stored
selection to leave 0..=8. The fixture test now compares both invalid-index rows,
their selected-stack results, and every ordered slot; the valid safe selection
invariant is also asserted. This closes the fixture coverage claim for all 21
rows without modeling an invalid stored selection.

Follow-up checks: `cargo test --locked --offline -p beta-core items:: -- --nocapture`
passed (6 tests); `cargo test --locked --offline -p beta-core --all-targets`
passed (83 library tests plus all beta-core example tests); beta-core clippy with
`-D warnings`, fmt check, and `git diff --check` passed. A02 was not started.

## ITEMS-001-A02 — bounded items probe and native parity

Merged the exact committed Codex C02 snapshot `9f069c081d23986bfa8cbcc1a789f2f8ec08d512`
into agy with merge commit `7bafedd`; branches had diverged after the A01 follow-up,
so fast-forward was not possible. Implemented
`crates/beta-core/examples/items_probe.rs` as a bounded, deterministic TSV adapter
over `beta_core::items`. It parses the full input before output, rejects malformed,
duplicate, unsupported, noncanonical, over-limit, or wrong-arity scenarios, stages
all result rows before writing stdout, and keeps diagnostics on stderr. It does
not consult answer rows by case ID: each scenario drives the stack/inventory API.

The probe emits the C02 six-column schema and ordered 36-slot values. Native item
display names are omitted as specified by C02. For hotbar observations `-1` and
`9`, output preserves the requested raw selection and emits `-` for the selected
stack while leaving the internal `Inventory` selected index valid and slots
unchanged. Unit tests compare every row and slot in the 21-row native fixture,
and cover late malformed input, duplicate scenarios, unsupported IDs, invalid
counts, wrong field counts, noncanonical values, and size/scenario limits with
empty output on parse failures.

Checks: `cargo test --locked --offline -p beta-core --example items_probe` → PASS
(4 tests); `cargo test --locked --offline -p beta-core --all-targets` → PASS
(83 library tests plus all examples); beta-core clippy with `-D warnings`, fmt
check and `git diff --check` → PASS. `python3 tools/check.py` → PASS (195 Python
tests, 18 opt-in skips; workspace format/check/clippy/all-target/doc checks).
`python3 tools/parity/run.py items --strict` → PASS: all 6 scenarios / 21 ordered
states match, zero mismatches, on clean probe revision
`77f968e8038c0a6c59bc4acc700e8553747a5389` (`rust_dirty=false`). Saved report
`artifacts/parity/items/run-j7vkxij2/report.json` passed
`python3 tools/parity/items_review.py`: 8 processes verified; native output SHA-256
`b4c6befa3deaa740f90eaef2e79c8a9a53f5b48eb76fe4cdbc21565604af4916`. No fixture,
oracle, runner, contract, dependency, viewer or save files changed. GUI, crafting,
armor, general item metadata, and persistence remain out of scope. A02 is complete;
do not start C03 from this worktree.
