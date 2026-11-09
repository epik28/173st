# Отчёт Codex

## LIGHT-UPDATE-001-C02 — принята bounded relight реализация

- Принят agy snapshot `73ce3121ad09d207e7ef0c7df8ea964febd0465b` после
  независимого просмотра Rust diff, fixture тестов и bytecode-аудита `fd/lm/st`.
  Повторный native audit прошёл: два запуска, 2,322 строки, SHA-256 выхода
  `effa63e2c14cb76962b1023a07c60f4e9fcf6e55f4db62695f78b7013e5d02cb`.
- `cargo test --locked --offline -p beta-core --test light_update_fixture` —
  4/4 тестовых функции прошли и сверяют все пять C01 сценариев, включая
  `edge-x31`; отдельный regression проверяет native skip при отсутствующем
  диагональном chunk footprint. `cargo test --locked --offline -p beta-core
  --lib block::tests` — 11/11. `cargo clippy --locked --offline -p beta-core
  --all-targets -- -D warnings`, `cargo fmt --all`, `git diff --check` и
  полный `python3 tools/check.py` прошли.
- Проверено read-only, что `level.dat` и четыре pinned `.mcr` из
  `/home/andrew/.local/share/PrismLauncher/instances/b1.7.3/minecraft/saves/New World`
  точно совпадают с `artifacts/user-world-postplay.sha256`. Исходный мир,
  native fixture и oracle не менялись.
- Принимается только bounded C01 parity: native LIFO/coalescing queue и общая
  height-map propagation не реализованы полностью; также вне scope emission
  transitions, fluid lighting, persistence и world simulation. GUI screenshot
  и latency/FPS benchmark не получены.
- Оценка полной игры поднята с ≈34% до ≈35%. Интервал от 30% достиг 5 пунктов;
  после интеграции проверенного snapshot публикуется обновлённый main.

## LIGHT-UPDATE-001-C01 — native edit and light-update oracle (accepted)

The initial `fd.d` fixture was discarded. The accepted synthetic-only oracle
drives `fd.a(eb,...)`, drains `fd.j()` to quiescence and captures the explicit
unloaded edge. It is pinned and repeatable as documented below. The A01 API and
limits are frozen in the final C01 handoff section; implementation may proceed
against that contract.

## HEALTH-001-C01 — native Player.Health and heart audit

- Расширены `tools/oracle/hud_audit.py` и `tests/fixtures/hud/audit.json`: закреплены
  `ls.Y:I` (Player health), signed-short NBT read/write, native missing default 10,
  viewer fallback 20/clamp 0..20, а также exact ordered heart UV branches:
  background `(16,0)`, full `(52,0)`, half `(61,0)`, all `9x9`.
- Добавлена provenance/hash-проверка `ls.class` (`b70fbb1499cd1b37ef27dd5dec7637c7338439ef03fe13012dca93fb2326d078`)
  и bytecode evidence для Health field, `nu.d(String):S`, `nu.a(String,S)` и
  missing-field default. Fixture tests проверяют wrong JAR, altered health type/range,
  UVs, missing evidence/methods and malformed data.
- Проверки: `python3 tools/oracle/hud_audit.py --jar ../.local/minecraft/client-b1.7.3.jar`
  PASS; `python3 -m unittest tests.python.test_hud_audit -v` 5 PASS; `git diff --check`
  PASS. Rust/world/contract/queue не изменялись. Native pixel parity не заявляется.
- Commit SHA записывается после коммита C01; следующий шаг — HEALTH-001-A01 на agy,
  затем HEALTH-001-C02 acceptance.

## HOTBAR-ITEMS-001-C02 — принята отрисовка предметов

- C01 native audit `076de31fb40a59bafafae559787b8a041c1ff4f0` и agy renderer
  snapshot `6a303dba7ae9e81bdfbaf190c2168e526b232b7c` интегрированы в main.
- `python3 tools/oracle/hotbar_items_audit.py ../.local/minecraft/client-b1.7.3.jar`
  → PASS; две original JVM записи совпали. Проверены `uq/bb/sj`, atlas dispatch,
  icon UV, digit widths, позиция/count/shadow; GL pixel parity не заявлена.
- `python3 -m unittest tests.python.test_hotbar_items_audit -v` → 3 PASS.
- `python3 tools/parity/run.py items --strict` → PASS: все упорядоченные
  inventory states совпали с native эталоном.
- `python3 tools/check.py` → PASS: полный Python/Rust workspace, fmt, clippy,
  all-target и doc tests; focused viewer tests: 17 library + 36 viewer + 4 probes.
- GUI smoke намеренно остановлен на прежнем guard несовпавшего `level.dat`; мир
  не открывался и не менялся. Новый визуальный кадр не подтверждён.
- Изменён HUD notice: сообщает о показе saved hotbar items и фиксированном
  здоровье. Оценка поднята с ≈30% до ≈31% за видимые поддержанные предметы и
  counts. Push пока не делался: это +1pp от текущего GitHub интервала.
- Следующий контракт INVENTORY-INTERACTION-001 охватывает мышиный перенос между
  36 слотами. `inventory_native_audit` подтвердил bytecode-семантику left/right,
  merge/swap и Escape возврата cursor; отдельный строгий C01 corpus ещё предстоит.

## INVENTORY-INTERACTION-001 — контракт опубликован локально

- Зафиксированы внутренние индексы `0..8` hotbar, `9..35` main rows, click
  semantics, cursor close/focus-loss, поддержанные stacks и ограничения.
- C01 native fixture/oracle и A01 Rust implementation открыты параллельно;
  C02 примет объединённую ревизию. Вне scope пока shift-click, armor/crafting,
  world drops, use/pickup и persistence.
- Рекомендации по отложенной визуальной диагностике получены read-only:
  листва — full-cube faces/alpha-test/static light; тёмная грань после edits
  вероятно требует LIGHT-UPDATE; остаточные flight spikes следует профилировать
  CPU/GPU buffer work. Эти визуальные дефекты пока не исправлялись.

## BLOCK-001-C03 — принят, 2026-11-08

- В `main` fast-forward интегрирован чистый agy snapshot `170033d`.
- `python3 tools/check.py` → PASS: 180 Python tests (18 opt-in skips), fmt,
  workspace check, clippy `-D warnings`, all-target Rust и doc tests.
- `python3 tools/parity/run.py block --strict` → PASS: 17/17 native ray cases.
  Старые strict suites `rng`, `rng-extended`, `chunk`, `nbt`, `region`, `world`,
  `view`, `math`, `java`, `geometry`, `movement`, `color`, `light`, `fluid`
  прошли. Одна параллельная NBT попытка выдала scenario-protocol error; отдельный
  последовательный запуск прошёл exact.
- Agy Xvfb screenshot smoke: 640×480 RGB scene визуально проверена; viewer
  сообщает 25 chunks, 334776 opaque и 9768 water vertices. GUI break/place click
  automation недоступна, поэтому действие не подтверждалось ручным вводом.
- В C01 не получилось безопасно построить изолированный native controller
  oracle. Поэтому ray имеет native parity, break/place — только Rust test
  coverage и viewer wiring; action parity не заявлена. Изменения локальны и не
  сохраняются в пользовательский мир.
- Estimate повышена примерно с 21% до 23% за прицеливание и редактирование мира.
  GitHub push остаётся отложен до 30% по указанию пользователя.
- Workspace и status карточки обновлены; следующая задача требует нового
  контракта. За BLOCK-001 commit здесь принят `170033d`.

## REGION-001-C03 — принят, внутренние агенты и интеграция

- Исполнители: `gpt-6-luna low`, отдельные роли Rust/runner, отдельные worktree.
  Координатор Codex принял управление после подтверждения пользователя, что
  прежний внешний диалог остановлен. Пользователь явно разрешил внутреннее
  делегирование; инструкции и стартовая памятка обновлены. Метрики токенов/стоимости
  исполнителей недоступны. Оригинал остаётся Java qj из закреплённого JAR,
  одинаковая модель исполнителей не заменяет независимость native reference.
- Snapshot agy: `4f6c41c6a1edef399ac4bfa001f6be13d5089158`.
  Snapshot C02: `e388657`; проверенная чистая объединённая ревизия:
  `1e076324e992412d396a15da72f808facb00632f`. Все пять live-отчётов
  относятся к ней и имеют `rust_dirty=false`. Коммит приёмки содержит эту запись
  и переносится в main fast-forward; сам SHA не вписывается в собственный коммит.
- Ревью координатора выявило неполную проверку завершения zlib, gzip buffer
  boundary, неверный FlushDecompress::Finish для больших выходов и частичный
  following gzip header. Исполнитель исправил Rust отдельными коммитами;
  прямых правок движка координатором нет. Нюанс following header измерен настоящим
  qj: `artifacts/region-native-edge/run-x_devjhd/results.json`; первоначальная
  ошибочно построенная проба `run-6sf586vr` завершалась wrapper alignment guard
  и не использовалась как native evidence. Повторяемые проверки теперь в region_edges.
- Координатор reviewed C02, запросил binary/copy/TSV/audit tampering checks;
  отдельно добавил повторяемый protocol review, проверку JAR в error paths и
  generated edge acceptance. Эталон и допуски не ослаблялись.

### Фактические проверки на объединённой ревизии

- `python3 tools/check.py` → PASS: 60 Python, 58 Rust all-targets (36 lib,
  22 examples), fmt/check/clippy `-D warnings`, doc. Отдельно повторять тот же
  cargo all-targets набор после этой проверки не потребовалось.
- `python3 tools/parity/run.py rng --strict` → 96/96 PASS;
  `artifacts/parity/rng/run-k0gadkkv/report.json`.
- `python3 tools/parity/run.py rng-extended --strict` → 13227/13227 PASS;
  `artifacts/parity/rng-extended/run-19hbifxk/report.json`.
- `python3 tools/parity/run.py chunk --strict` → 4182/4182 PASS;
  `artifacts/parity/chunk/run-x5vuuna9/report.json`.
- `python3 tools/parity/run.py nbt --strict` → 5250/5250 PASS;
  `artifacts/parity/nbt/run-gdk50ebx/report.json`.
- `python3 tools/parity/run.py region --strict` → 748/748 PASS,
  704 present + 44 absent, mismatch_count=0;
  `artifacts/parity/region/run-pq6ftpqm/report.json`.
- `python3 tools/parity/verify_report.py artifacts/parity/region/run-pq6ftpqm/report.json --protocol`
  → native audit/source/copy/JAR/class/binary/log/payload hashes PASS,
  16 TSV executions PASS. Повторный вызов на том же отчёте PASS.
- `python3 tools/parity/region_edges.py artifacts/parity/region/run-pq6ftpqm/report.json`
  → 37 separately labelled synthetic cases/74 reader executions PASS;
  `artifacts/parity/region/run-pq6ftpqm/edge-review-aer9hznw/review.json`.
  23 C01 wire cases, 10 coordinate boundaries и четыре дополнительных gzip
  header/trailer cases. Native overlap/oversized-length отличаются от safe Rust
  и не выдаются за равную parity. Все generated copies неизменны.
- Шесть metadata mutations через полный verify_region отвергнуты:
  binary hash, native count, payload length, process failure, source/copy hashes;
  `artifacts/parity/region/run-pq6ftpqm/report-mutation-review.json`.
- Изолированный test root без Rust source probe выполнил original Java 748 reads,
  вернул `not_implemented`, strict exit 1. Неверный JAR hash и process timeout
  дали `error`, copy/source verification не пропущена;
  `artifacts/region-error-review/run-sjwzzg38/review.json`.
- Четыре source SHA до/после и две input copies совпали с закреплёнными,
  JAR неизменён. `git diff --check` PASS. Бинарники/миры/декомпиляция не добавлены.

### Область и дальнейшая работа

Принято read-only извлечение raw NBT payload из McRegion. Семантическая загрузка
мира, level.dat, сохранение и рендер не заявляются. Очередь текущих 25 карточек
завершена; следующий плановый WORLD-LOAD-001 требует отдельного контракта.
Постоянный фоновый наблюдатель после завершения диалога не работает; в этом запуске
координатор дождался сообщений обоих агентов и выполнил приёмку.

## REGION-001-C01 — done, dependency и native corpus готовы для agy

- Исходная ревизия: `312a41e88bec2d68d3e31b2d6e58907bafa16173` (main и обе роли).
  Итоговая — коммит, содержащий этот раздел; точный SHA берётся из очереди.
  Agy должен получить snapshot Codex с C01 done через собственный merge, затем A01.
  Его worktree/ветка здесь не менялись; C01 в main до приёмки пакета не переносится.
- Cargo.toml / crates/beta-core/Cargo.toml / Cargo.lock: flate2 **=1.1.9**,
  default-features=false, rust_backend → miniz_oxide 0.8.9. Для хешей probe только
  dev-dependency sha2 **=0.10.9**. Лицензии/версии записаны в provenance; локальный
  registry содержит все 15 dependency packages. Новая машина: cargo fetch --locked.
- Native `qj(File)`, `a(int,int): DataInputStream`, `b()`; private d/e для
  locations/timestamps. Class hash 6c75a123b63d555caa98e1a2619aa2483e405566de9e356585b1dc682a7da4a6.
  Проверен code source оригинального JAR и pinned JAR hash. Ручной reader/decompressor
  вместо native метода не используется. Constructor rw запускается только на копиях
  с guard против ремонта short/unaligned files; все копии хешируются после закрытия.
- `python3 tools/oracle/region_corpus.py --write` — PASS,
  `artifacts/oracle-region/run-m_k99qsa`: 704 occupied + 44 absent в двух одинаковых
  native запусках; 23 synthetic cases и 10 coordinate-boundary cases отдельно.
  snapshot.json содержит before/after_copy/after_run, все четыре pinned source SHA
  совпали; копии тоже неизменны. Raw NBT остаётся в artifacts/payloads.
- Подтверждены ids 1=gzip / 2=zlib, trailing garbage после завершённого stream,
  несколько gzip members → concatenated output. CRC/truncation/output cap дают error.
  Native допускает overlap и oversized length на 1–4 байта; safe Rust должен их
  отвергать. Контракт уточнён этими измерениями до начала A01.
- CLI закреплён: `region_probe INPUT_COPY_DIR PAYLOAD_DIR`; TSV по контракту;
  `PAYLOAD_DIR/<case_id>.nbt` для каждого present, без перезаписи; marker в input.
- Остальные файлы: tools/oracle/region_corpus.py, tools/oracle/src/RegionOracle.java,
  tests/fixtures/region/{sources.json,live.tsv,coordinates.tsv,audit.json,provenance.md},
  tests/python/test_region_corpus.py, docs/tasks/REGION-001.md, progress/status/handoff.
- `cargo generate-lockfile --offline`, `cargo check --workspace --all-targets --locked --offline`,
  `cargo tree --locked --offline -e features` — PASS; pure Rust compression backend.
  `python3 -m unittest discover -s tests/python -p test_region_corpus.py -v` — 4 PASS.
  `python3 tools/check.py` — 51 Python + 43 Rust all-targets, fmt/check/clippy/doc PASS.
  `git diff --check` — PASS.
- C01 доказывает оригинальный adapter, а не Rust parity. Сохранение не изменено,
  игровые bytes не включены в Git. Rust implementation agy не редактировалась;
  Codex менял только dependency manifests. Заимствований из mc173 нет.
  Codex на основе GPT-6; точный runtime model ID/стоимость недоступны. Других агентов нет.
- Следующий шаг Codex: C02 runner; agy: A01 по этому зафиксированному контракту.

## NBT-001-C03 — независимая приёмка завершена

- Snapshot agy `541d719e83e32ad13e9448e102c8cc1ed16bcf40` интегрирован fast-forward.
  Исправления инструментария отдельно: `93ed4fe8356d9c30cfa9013bca111d61e75f0ebf`.
  Все финальные live-отчёты проверяют именно эту ревизию с `rust_dirty=false`.
  Итоговая ревизия приёмки — коммит, содержащий этот раздел.
- `python3 tools/check.py` — 47 Python + 43 Rust all-targets, fmt/check/clippy/doc PASS.
  `cargo test --all-targets --locked --offline` — 43 PASS.
- `python3 tools/parity/run.py rng --strict` — 96/96 PASS,
  `artifacts/parity/rng/run-cxelcb7r/report.json`.
- `python3 tools/parity/run.py rng-extended --strict` — 13227/13227 PASS,
  `artifacts/parity/rng-extended/run-w6ixr3u6/report.json`.
- `python3 tools/parity/run.py chunk --strict` — 4182/4182 PASS,
  `artifacts/parity/chunk/run-rohs2w09/report.json`.
- `python3 tools/parity/run.py nbt --strict` — 5250/5250 PASS,
  `artifacts/parity/nbt/run-a56fm2ot/report.json`; canonical tree/consumed/raw match,
  исходные payload bits и NaN write normalization сохранены в evidence.
- `python3 tools/parity/verify_report.py artifacts/parity/nbt/run-a56fm2ot/report.json --protocol`
  — 86 executions PASS, пять synthetic mutations обнаружены. Хеши дерева/входа/
  выходов/JAR/adapter/probe, audit оригинальных named read/write подтверждены.
- Параллельный запуск JVM в этом окружении вызвал предупреждения hsperfdata lock
  прямо в stdout: `run-qp8se6aj`/`run-59fyhuar` корректно получили error протокола.
  Две группы повторены последовательно; предупреждения не фильтровались, эталон
  не ослаблен. Эти отклонённые запуски не объявляются parity PASS.
- Область приёмки: несжатый NBT, original types 0–10 в общей области;
  IntArray и пять native differences проверены отдельно и описаны в контракте.
  Нет gzip/zlib/McRegion/world loading. Rust — полностью agy, прямых правок Codex нет.
- Изменения финального коммита: этот handoff, собственный status JSON,
  docs/progress.md и статус docs/tasks/NBT-001.md. Перенос: `git switch main`,
  `git merge --ff-only codex/verification`, `git switch codex/verification`;
  затем проверить main и снова вызвать `python3 tools/tasks.py next codex`.
- Следующий шаг выбирается только очередью. REGION-001 пока не имеет карточек
  в queue.json; самостоятельно начинать новый пакет за пределами очереди нельзя.

## NBT-001-C03 — исправление инфраструктуры Codex, приёмка продолжается

- Исходная Codex `07726cfa3913a4a565d8a47623f59b34bb17ae5d`; snapshot agy
  `541d719e83e32ad13e9448e102c8cc1ed16bcf40` взят из очереди, изучены его
  handoff/status/diff. `git merge --ff-only <этот SHA>` выполнен без конфликтов.
- Исправлен собственный пробел C02: `verify_report.py --protocol` теперь вызывает
  NBT verifier, сохраняет деревья, сверяет consumed length и исходную семантику,
  NaN DataOutput, JAR/class/binary hashes и audit. Неправильные hex/TSV/IDs/duplicates
  отвергаются также Java-обёрткой; разрешены любые named roots кроме End.
- Файлы: tools/oracle/{nbt_corpus.py,src/NbtOracle.java}, tools/parity/{run.py,
  verify_report.py,nbt_review.py,README.md}, tests/python/test_nbt_runner.py,
  tests/fixtures/nbt/{corpus.tsv,invalid.tsv,audit.json,provenance.md}, контракт,
  собственные status/handoff. Корпус усилен без удаления прежних 5200 cases.
- Предварительная проверка: `python3 tools/check.py` — 47 Python + 43 Rust,
  fmt/check/clippy/doc PASS; regression tests отдельно:
  `python3 -m unittest discover -s tests/python -p test_nbt_runner.py -v` — 10 PASS.
  `cargo test --all-targets --locked --offline` — 43 PASS.
- `python3 tools/parity/run.py nbt --strict` — 5250/5250 PASS,
  `artifacts/parity/nbt/run-amwvx0tt/report.json` (dirty, ещё не финальная приёмка).
  `python3 tools/parity/verify_report.py artifacts/parity/nbt/run-amwvx0tt/report.json --protocol`
  — 86 executions PASS: 12 common invalid inputs по одному на процесс, TSV,
  missing JAR/classes, 5 copied-output mutations, отдельный IntArray/safety/depth.
- Исследован оригинальный bytecode `ij`/`nu`/`sp`. Native differences записаны
  отдельно, без выдачи за equal-output parity: empty-list type→Byte, negative list
  length→empty, End-list, unknown empty-list type, isolated surrogate. В Rust
  сохраняются требования контракта, Java-ответы не менялись. Прежний invalid ID
  negative-list оказался trailing bytes после End и переименован по содержимому.
- При расширении генератора исправлена собственная ошибка terminators вложенного
  compound; первоначальный `run-pp8uqhx9` честно дал Java error, повторный прошёл.
- Rust полностью agy; Codex Rust не редактировал. Модель Codex на основе GPT-6,
  точный runtime model ID/стоимость не предоставлены; делегирования/заимствований нет.
- Следующий шаг: после отдельного коммита инструментария повторить все live-группы
  на чистой ревизии, завершить C03 и fast-forward main; затем снова прочитать очередь.

## RNG-001-C01 — done

- Роль: Codex, текущая модель GPT-6; точные model ID, настройки и стоимость
  инструментами сессии не предоставлены. Делегирования не было.
- Исходная ревизия: `ae897194183194a276516c862189602ecc7dd900`.
  Итоговая ревизия: коммит C01, содержащий этот раздел (SHA показывает Git).
- Файлы: `tools/oracle/src/RandomOracle.java`, `tools/oracle/verify.py`,
  `tools/oracle/README.md`, `docs/tasks/status/codex.json`, этот handoff.
- Java-адаптер вызывает настоящий `java.util.Random`, принимает TSV и выдаёт LF TSV.
  Формула RNG вручную не реализуется; сторонний код не заимствован.
- `python3 tools/doctor.py` → инструменты доступны, Java/javac 17.0.21.0.3;
  серверный JAR не настроен и для Java API не нужен.
- `python3 tools/oracle/verify.py` → PASS: компиляция javac, 96 записей,
  два одинаковых выхода, границы i64, 14 неверных входов отвергнуты,
  count=100000 принят. Выход: `artifacts/oracle-check/fixture.stdout.tsv`.
- `python3 tools/check.py` → PASS: 13 Python-тестов, Rust fmt/check/clippy/test;
  в текущем ядре 0 Rust-тестов.
- Ограничения: это проверка Java-адаптера; сравнений с Rust и игровой Beta не было.
  Gemini в C01 не участвовал, Codex Rust-движок не менял.
- Следующий шаг: выбрать C02 через очередь и реализовать runner.

## RNG-001-C02 — done

- Исходная ревизия: `7d435e67c2650cd385ce19e573a7ed45549fb101` (C01).
  Итоговая: коммит C02, содержащий этот раздел.
- Файлы: `tools/parity/run.py`, `tools/parity/README.md`,
  `tests/python/test_rng_runner.py`, собственный status JSON и handoff.
- Runner собирает обе стороны по одному разу, сохраняет один вход, сырые выходы,
  ошибки и коды/тайм-ауты процессов. Отчёт содержит версии, ревизию/dirty,
  хеши исходников до/после, адаптера, бинарников, входа и выходов. Сравнение точное.
- `python3 tools/check.py` → PASS: 27 Python-тестов, включая 14 тестов runner,
  fmt/check/clippy/test; в checkout до интеграции 0 Rust-тестов.
- `python3 tools/parity/run.py rng --strict` → exit 1, `not_implemented`:
  Java действительно собрана и выполнена (96 записей), Rust-probe отсутствует.
  Отчёт: `artifacts/parity/rng/run-_drfc8ip/report.json`.
- `python3 tools/parity/run.py list` → только rng; `git diff --check` → PASS.
- Тесты обнаруживают подмену, пропуск/добавление, порядок, отсутствие данных,
  неверный TSV/i32. Реальные subprocess-тесты проверяют exit 7 и тайм-аут с сохранением
  частичного stdout. Синтетическая pipeline проверяет ошибки/тайм-ауты сборки и
  выполнения обеих сторон, отсутствие probe и изменение исходников во время запуска.
- Найденная при разработке ошибка сериализации expected_key (tuple/list) исправлена
  в runner; ожидания и fixture не ослаблялись. Синтетические тесты явно помечены,
  их pass не является результатом реального Rust↔Java.
- Codex не менял Rust-движок; помощи Gemini в C02 не было. Очередь уже видит
  закоммиченный A01 `236752f79cdcd7cd54a5b2610d6e354814339424`, A02 ещё pending.
- Следующий шаг: перечитать `docs/tasks/queue.json`; C03 после закоммиченного A02, ожидание по `docs/workflow.md`.

## RNG-001-C03 — blocked, запрос исправления agy

### Ревизии и вклад

- Исходная Codex: `185b8287a6497023580d18de8b8fad7900ff1dca`.
- Snapshot agy из `tasks.py next codex`:
  `ac6ce46c7ac872362c09e2fefc34c39dd79a4331`. Прочитаны handoff, status и diff
  именно этого коммита через Git; изменения только в разрешённых файлах agy.
- Интеграция: `git merge --no-ff ac6ce46c7ac872362c09e2fefc34c39dd79a4331`
  (сообщение merge: `merge: integrate agy RNG implementation and probe for C03 verification`).
  Получен `c485934358335cf2535300b5293b616ed71e9cae`, конфликтов нет.
- Итоговый отчёт/блокировка: коммит C03, содержащий этот раздел.
- C03 изменяет `tools/parity/verify_report.py`, `tools/parity/README.md`,
  `tests/python/test_rng_runner.py`, собственные status/handoff,
  `docs/progress.md`, статус `docs/tasks/RNG-001.md`.
- Gemini реализовал весь Rust RNG и probe. Codex написал Java-эталон,
  инфраструктуру проверки и выполнил независимую приёмку; Rust-код Codex не правил.

### Фактические результаты

- `python3 tools/check.py` → PASS: 27 Python-тестов, 7 Rust-тестов,
  fmt/check/clippy с `-D warnings`.
- `python3 tools/parity/run.py rng --strict` → PASS, exit 0, 96/96 значений,
  оба процесса exit 0, шесть seed включая min/max i64. Объединённая ревизия
  `c485934358335cf2535300b5293b616ed71e9cae`, rust_dirty=false.
- Полный вход, исходные stdout/stderr, бинарник Rust, Java-класс и метаданные:
  `artifacts/parity/rng/run-lwytrhsv/report.json` и соседние файлы.
- `python3 tools/parity/verify_report.py artifacts/parity/rng/run-lwytrhsv/report.json --protocol`
  → exit 1: целостность отчёта и чувствительность компаратора PASS; live-проверка
  протокола выявила 2 нарушения из 36 запусков (16 неверных входов и 2 допустимых,
  каждый выполнен обеими сторонами). Это не отменяет совпадение 96 RNG-значений,
  но блокирует полную приёмку контракта.
- `comparator-sensitivity.json` явно помечает синтетическую подмену копии выхода;
  основной live-отчёт не менялся. Изменено ровно одно число — mismatch_count=1,
  first_mismatch.record=0. `protocol-review.json` содержит настоящие коды/диагностику.
- `git diff --check` → PASS. В main ничего не интегрировано.

### Конкретный запрос agy для отдельной согласованной карточки

В `crates/beta-core/examples/rng_probe.rs` условие `line.trim().is_empty()`
пропускает непустые строки с whitespace, хотя контракт разрешает пропускать
только пустые строки и строки с `#` в начале.

Воспроизведение сохранёнными адаптерами: указанная команда `verify_report.py --protocol`.
Минимальные входы: байты `20 0a` (`" \n"`) и `09 0a` (`"\t\n"`).
Ожидание: ненулевой exit и stderr о неправильном TSV (Java: exit 1,
`RandomOracle: line 1: invalid fields/ID`). Факт Rust: exit 0, stdout/stderr пусты.
Реально пустая строка `0a` и комментарий `# comment` принимаются обеими сторонами.

Предлагаемая узкая карточка исправления agy (ещё не добавлена в очередь):
проверять именно пустоту строки, сохранить отклонение пробела/табуляции обычной
проверкой числа полей, добавить регрессию на оба входа; не менять RNG и fixture.
Сделать отдельный коммит со своими проверками и handoff. По правилам C03
карточка должна быть согласована отдельно; чужие статусы/очередь Codex не менял.

После согласованного исправления: взять точный новый SHA agy, интегрировать,
выполнить check.py, rng --strict и verify_report.py --protocol, обновить C03 и принять
результат в main. Текущая область проверки — Java API, не игровая Beta.

## RNG-001-C03 — повторная приёмка, done

Пользователь передал готовое исправление agy и поручил завершить C03.
Предыдущий раздел blocked сохранён как история; запрос исправления закрыт.
Новые карточки и этапы не добавлялись, чужой status JSON вручную не редактировался.

### Ревизии и изменения

- Начальная ревизия Codex: `1983689bd0d1263f675fd4f33ff404849ba5d0f6`.
- Snapshot agy подтверждён через `python3 tools/tasks.py next codex`:
  `80dee8a5296d0769d29a938b0aea2795d4f0e556`. Очередь ещё показывала прежний
  собственный blocked; возобновление разрешено сообщением пользователя.
- Через Git прочитаны handoff/status и diff snapshot относительно `ac6ce46`.
  В diff только Rust probe и файлы отчёта/status agy; новых зависимостей нет.
- `git merge --no-ff 80dee8a5296d0769d29a938b0aea2795d4f0e556 -m 'merge: integrate agy probe validation fix for RNG-001-C03'`
  → успешно, без конфликтов. Объединённая ревизия:
  `544e9d2c0adb3412939ce203a278d56f97c904f9`.
- Исправление Rust и четыре регрессионных теста написаны agy. Codex повторил
  независимую проверку, не меняя Rust-код, эталоны или fixture.
- Текущие изменения Codex: этот handoff, `docs/tasks/status/codex.json`,
  `docs/progress.md`, статус `docs/tasks/RNG-001.md`.
- Итоговая ревизия приёмки — коммит, содержащий этот раздел. Его точный SHA
  устанавливается через Git; этот же коммит переносится в `main` fast-forward.

### Точные команды и результаты

- `python3 tools/doctor.py` → PASS, инструменты доступны, Java/javac 17.0.21.0.3.
- `python3 tools/check.py` → PASS: 27 Python-тестов, Rust fmt/check/clippy,
  7 Rust-тестов ядра. Эта команда не запускает unit-тесты examples.
- `cargo test --locked --offline -p beta-core --example rng_probe` → PASS,
  отдельно выполнены все 4 регрессионных теста probe.
- `python3 tools/parity/run.py rng --strict` → PASS, exit 0, оба процесса exit 0,
  96/96 значений, шесть seed (включая min/max i64), mismatch_count=0.
  Проверен чистый merge `544e9d2c0adb3412939ce203a278d56f97c904f9`.
- Отчёт, полный вход/выходы, бинарник Rust и Java-класс:
  `artifacts/parity/rng/run-oajcevsr/report.json` и соседние файлы.
- `python3 tools/parity/verify_report.py artifacts/parity/rng/run-oajcevsr/report.json --protocol`
  → PASS, exit 0: все 36 проверок TSV (16 неверных и 2 допустимых входа
  обеими сторонами), включая бывшие проблемные `20 0a` и `09 0a`.
  Хеши сохранённых данных подтверждены. Синтетическая подмена одного значения
  обнаружена (mismatch_count=1, first_mismatch.record=0), оригиналы не менялись.
- `git diff --check` → PASS.

Интеграция завершена: коммит приёмки `fa875dd937a0c5c6d4cf7dd2f8a232c1a9c6b978`.
`git switch main` → успешно; `git merge --ff-only codex/verification` → успешно,
main обновлён с `ae89719` до `fa875dd`. `git switch codex/verification` → успешно.
`git rev-parse main codex/verification` → одинаковые SHA `fa875dd...`;
`git diff --exit-code main codex/verification` → exit 0;
`python3 tools/tasks.py next codex` → complete; `git status --short` → пусто.
Этот финальный отчёт дополняет коммит приёмки и также включается в main fast-forward.
Файлы кода после успешных проверок не менялись.

Ограничения: принят только Java RNG API nextInt на фиксированных seed, игровая
Beta и другие методы RNG не проверялись. Следующий этап без нового контракта
не начинать. Модель Codex и настройки сессии не менялись, делегирования не было.


## 2026-11-07 — подробные этапы и следующие контракты

Запрос пользователя: расширить этапы после отчёта agy о RNG-001 и объяснить,
как убрать подтверждение каждого изменения в Antigravity. Исходная ревизия обеих
ролей/main: `44a8bf9fa3bffcdc1ced3e34e7f5cca783713c50`; agy свободен, деревья чистые.

Codex подготовил `docs/roadmap.md`: 29 пакетов с результатом, обязанностями agy,
независимыми проверками Codex и критериями приёмки; первый видимый рубеж VIEW-001,
короткий игровой цикл — SAVE-001. В очередь добавлены только два конкретных
контракта RNG-002 и CHUNK-001, по пять карточек. Дальние этапы не исполняются без
контракта. Результаты RNG-001 сохранены; новые записи обеих ролей инициализированы
pending в общем коммите подготовки, а не отмечены как выполненные за исполнителя.
Обновлены инструкции/прогресс; описан возврат ошибок через собственные статусы
и закоммиченный handoff без ручной передачи пользователем. Код движка не менялся.

Проверки исходной ревизии, выполненные заново:

- `python3 tools/check.py`: PASS, 27 Python, fmt/check/clippy, 7 Rust.
- `cargo test --all-targets --locked --offline`: PASS, 11 Rust, включая 4 example.
- `python3 tools/parity/run.py rng --strict`: PASS, 96/96; отчёт
  `artifacts/parity/rng/run-p7anmxvs/report.json`.
- `python3 tools/parity/verify_report.py artifacts/parity/rng/run-p7anmxvs/report.json --protocol`:
  PASS, хеши, одна синтетическая подмена, 36 live protocol-проверок.
- После правок документов: 7 тестов `test_task_queue.py` PASS; статическая проверка
  DAG из 15 карточек, владельцев, всех прежних результатов, десяти pending,
  заголовков карточек и локальных ссылок PASS; `git diff --check` PASS.

В прежней версии handoff хранились настройки Antigravity CLI, больше не используемого
в этом workflow. Настройки пользователя не изменялись. Модель текущей сессии Codex;
точный model ID/токены/стоимость
не предоставлены средой. Дополнительные модели не запускались, сторонний код
и оригинальные ресурсы не добавлялись.

Следующий запуск: RNG-002-C01 для Codex и RNG-002-A01 для agy, параллельно.
Подготовка контрактов сама по себе эти реализации не выполняет.

## RNG-002-C01 — done

- Старт: `0fc4ba9b879bc524fdd5472c7e458921dfc965d8`; итог — коммит с этим разделом.
  Роль Codex, модель/настройки не менялись, дополнительных агентов нет.
- Ветка уже на main: `git merge --ff-only main` → Already up to date.
- Файлы: `tools/oracle/src/RandomExtendedOracle.java`, `RejectionMutation.java`,
  `tools/oracle/extended_corpus.py`, README oracle, `tests/fixtures/rng/extended.tsv`,
  `extended.md`, собственные status/handoff. Rust не изменялся.
- `python3 tools/oracle/extended_corpus.py --write` → PASS: javac, два идентичных
  выхода (25 случаев / 13227 операций), 17 неверных входов отклонены, 100000
  операций принято. Проверены неизменность состояния после invalid_bound и
  эквивалентность reseed новому Random на шести seed. Все операции вызывает
  настоящий java.util.Random; LCG в Java не переписывался.
- 2048 последовательных bound=1073741825 при seed=0 отличают синтетический sampler
  без rejection: 2048 несовпадений, первое (record=0) — Java 516548029 против
  synthetic 495999535. Мутация в отдельном классе, не используется как live-эталон.
  Доказательства: `artifacts/oracle-extended-check/audit.json`, reference.stdout.tsv
  и synthetic-no-rejection.tsv. `git diff --check` → PASS.
- Заимствований/помощи agy в Java нет; Rust↔Java по RNG-002 ещё не сравнивались.
  Следующий шаг: RNG-002-C02, расширить runner и стандартный запуск example-тестов.

## RNG-002-C02 — done

- Старт `6fb5a6d`; итог — коммит с этим разделом.
- Изменены run.py/verify_report.py/README в tools/parity, tools/check.py,
  tests/python/test_rng_extended.py, собственные status/handoff. Rust и fixture не менялись.
- Общий runner сохранён: добавлена rng-extended с типами/порядком/количеством записей,
  raw IEEE-битами и аргументами bound/reseed. Метаданные и тайм-ауты общие для групп.
- `python3 tools/check.py` → PASS: 31 Python-тест, 11 Rust-тестов (включая examples),
  fmt/check/clippy и отдельный --doc. Тесты процесса/timeout/ошибок используют общий
  механизм runner; новые синтетические тесты проверяют типы, лимиты и порчу состояния.
- `python3 tools/parity/run.py rng --strict` → PASS 96/96,
  `artifacts/parity/rng/run-v37op4wi/report.json`.
- `python3 tools/parity/run.py rng-extended --strict` → not_implemented, exit 1;
  Java выполнена (13227 записей), probe отсутствует в текущем checkout.
  Отчёт `artifacts/parity/rng-extended/run-e1c9a148/report.json` — не parity pass.
- Точная команда для C03: `python3 tools/parity/verify_report.py <report.json> --protocol`.
  Для rng-extended она проверяет неизвестные/пустые операции, аргументы/ID/whitespace,
  лимиты на строку/весь вход, валидный invalid_bound/reseed и максимальные 100000 операций;
  сравнивает выходы обеих сторон. Синтетическая подмена сохраняется отдельно.
- `python3 tools/parity/verify_report.py artifacts/parity/rng/run-v37op4wi/report.json --protocol`
  → PASS: хеши, подмена и 36 live-проверок прежней группы.
- `git diff --check` → PASS. Следующий шаг: очередь C03; agy A01 уже закоммичен,
  A02 ещё pending. Codex Rust-код не правил, эталоны не ослаблял.

### RNG-002-C03 — отдельное исправление инфраструктуры Codex

- Снимок agy `f16d280c4f0a48016d0357669e6f56661013dcac` проверен по diff/handoff/status;
  все изменения в пределах роли. Merge без конфликтов: `1a554b1dd88a3ba47d9f2d18944c7c50069104b4`.
- `python3 tools/check.py` → PASS: 31 Python + 20 Rust (all-targets), fmt/check/clippy/doc.
- Первые live-отчёты: rng `artifacts/parity/rng/run-k1b8hjrc/report.json` → 96/96;
  rng-extended `artifacts/parity/rng-extended/run-6830ekct/report.json` → 13227/13227.
- При запуске verify_report.py на расширенном отчёте обнаружена ошибка Codex:
  мутация читала колонку operation_name вместо value. Исправлено на последнюю колонку;
  добавлен CLI-регрессионный тест с явно синтетическим четырёхколоночным отчётом.
  Это исправление инструмента, не изменение эталона/fixture/Rust или допусков.
- `python3 -m unittest discover -s tests/python -p test_rng_extended.py -v` → 5 PASS.
- `python3 tools/parity/verify_report.py artifacts/parity/rng-extended/run-6830ekct/report.json --protocol`
  → PASS: целостность, подмена и 44 live-проверки. Старый rng verifier → 36 PASS.
- `python3 tools/oracle/extended_corpus.py` → аудит Java/rejection PASS;
  `cargo test --all-targets --locked --offline` → 20 PASS.
- Изменены только verify_report.py, tests/python/test_rng_extended.py и этот handoff.
  Исправление оформляется отдельным коммитом, затем чистая повторная приёмка C03.

## RNG-002-C03 — done

- Snapshot agy: `f16d280c4f0a48016d0357669e6f56661013dcac` из очереди,
  просмотрены diff/status/handoff, границы роли соблюдены.
- Merge `1a554b1d`; отдельное исправление инфраструктуры Codex `660008d`.
  Чистая проверенная ревизия: `660008d6e93b751e7b604cda92f8f27249a7ed0b`.
- `python3 tools/check.py` → 32 Python + 20 Rust (all-targets), fmt/check/clippy/doc PASS.
  `cargo test --all-targets --locked --offline` → 20 PASS.
- `python3 tools/parity/run.py rng --strict` → 96/96, оба процесса exit 0;
  `artifacts/parity/rng/run-ebou8vei/report.json`.
- `python3 tools/parity/run.py rng-extended --strict` → 13227/13227,
  оба процесса exit 0, dirty=false; `artifacts/parity/rng-extended/run-8kwu2qxa/report.json`.
- `python3 tools/parity/verify_report.py artifacts/parity/rng-extended/run-8kwu2qxa/report.json --protocol`
  → 44 live-проверки PASS, сохранённые хеши и синтетическая подмена PASS.
- `python3 tools/parity/verify_report.py artifacts/parity/rng/run-ebou8vei/report.json --protocol`
  → 36 live-проверок и синтетическая подмена PASS.
- `python3 tools/oracle/extended_corpus.py` → Java API/rejection audit PASS,
  2048 synthetic no-rejection mismatches. Синтетические данные отдельно помечены;
  настоящие входы/выходы не перезаписывались. `git diff --check` → PASS.
- Изменены только свой status/handoff, docs/progress.md и статус RNG-002.md.
  Итоговый SHA — коммит с этим разделом, включаемый в main fast-forward.
- Вклад: Rust методы/probe — agy; Java/corpus/runner/review/integration — Codex.
  Codex не правил Rust. Покрытие — только Java RNG API, не генерация мира.
- После main: самостоятельно выбрать CHUNK-001-C01. Сессия agy завершена;
  следующий пакет требует её повторного запуска пользователем, файлы не запускают агент.

## CHUNK-001-C01 — done

- Предыдущая приёмка реально включена в main: `e11362c518ad75c83edda89076ea1ef44b256415`.
  `git switch main; git merge --ff-only codex/verification; git switch codex/verification`
  → PASS; `git diff --exit-code main HEAD` → 0; очередь открыла CHUNK-001-C01.
  `git merge --ff-only main` перед новым пакетом → Already up to date.
- Исходная ревизия: `e11362c`; итог — коммит с этим разделом.
- Изменены tools/oracle/src/ChunkOracle.java, chunk_corpus.py, README,
  tests/fixtures/chunk/layout.tsv и provenance.md, собственные status/handoff.
- Оригинальный класс найден сканированием сигнатур classfile и javap:
  `javap -private -c -classpath <configured-client-jar> wi`. JAR SHA256
  `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`;
  wi.class SHA256 `ef5df5cdcd5ba63d8802f5bc539c8e164681eb173ef74586c369ae483415bb50`.
  Имена: wi(int), get a(int,int,int), set a(int,int,int,int), raw a:byte[].
  Disassembly только artifacts/chunk-research/wi.javap.txt, не в Git.
- `python3 tools/oracle/chunk_corpus.py --write` → PASS: javac, reflection self-test,
  два одинаковых выхода по 4182 записи, 16 неверных входов отклонены,
  100000 операций принято, missing/wrong JAR отвергнуты.
  Фактические native вызовы: constructors=6, get=3606, set=1746, raw reads=1752.
  Отчёт artifacts/oracle-chunk-check/audit.json; исходный JAR не менялся.
- Оригинальные вызовы исполняет изолированный URLClassLoader, проверяются hash и
  code source. Дополнительных runtime-зависимостей wi нет. Обёртка отдельно
  реализует контракт координат/индекса/ID/валидации, не выдаёт их за игровой код.
- Корпус охватывает i32-границы, nibble 0..15, оба полубайта/разные x/z,
  ID 0/127/128/255, повторные записи/соседей/raw, отказы без изменения данных.
- `git diff --check` → PASS. Rust↔Java контейнера пока не сравнивались.
  Rust не изменялся; сторонний код не переносился. Следующий шаг CHUNK-001-C02.

## CHUNK-001-C02 — done

- Старт `f5cf5c9`; итог — коммит с этим разделом.
- Изменены tools/parity/run.py, verify_report.py, README,
  tests/python/test_chunk_runner.py, собственные status/handoff. Rust/fixture не менялись.
- Группа chunk выполняет настоящий ChunkOracle на закреплённом JAR, сохраняет
  reference_metadata, reference_invocations, wrapper scope, версии, исходные данные
  и хеш JAR до/после. Отсутствие JAR → error; probe → not_implemented strict exit 1.
- Синтетические тесты проверяют типы/порядок/количество, error priority,
  nibble-order/index, ошибку отсутствующего JAR. Ошибка при отсутствующем ключе
  исходника в минимальном checkout найдена и исправлена в runner до коммита;
  ожидания не ослаблялись. Общие tests ошибок процессов/timeout продолжают работать.
- `python3 tools/check.py` → PASS: 37 Python + 20 Rust, fmt/check/clippy/all-targets/doc.
- `python3 tools/parity/run.py chunk --strict` → not_implemented exit 1,
  реальный Java-адаптер и оригинальный nibble выполнены (4182 записи), Rust отсутствует.
  Отчёт artifacts/parity/chunk/run-7uuqez7z/report.json — не parity pass.
- `python3 tools/parity/run.py rng --strict` → 96/96 PASS,
  artifacts/parity/rng/run-xg341uy5/report.json.
- `python3 tools/parity/run.py rng-extended --strict` → 13227/13227 PASS,
  artifacts/parity/rng-extended/run-mtp76uod/report.json.
- Точная команда для C03:
  `python3 tools/parity/verify_report.py <chunk-report.json> --protocol`.
  Она проверяет исходный JAR/native calls, синтетические копии с изменёнными
  nibble-order/index/значением, TSV/лимиты, missing JAR, ошибку reflection на
  синтетической копии нашего адаптера с отсутствующим методом; оригинал не меняется.
  Исполнение этого chunk-верификатора требует настоящего успешного live-отчёта C03.
- `git diff --check` → PASS. Очередь показывает agy CHUNK-001-A01 ready,
  но его snapshot ещё f16d280 (только RNG-002). Следующий шаг C03 после A01/A02;
  ожидание по `docs/workflow.md`. Дополнительные агенты не запускались.

## CHUNK-001-C03 — done

- Из очереди получен точный snapshot agy `14b50482bc1fa9ef4850d7983780ceda876bb80a`.
  Просмотрены diff, status и handoff именно этого коммита: только chunk.rs,
  chunk_probe.rs, экспорт модуля и собственные файлы agy; новых зависимостей нет.
- `git merge --no-ff 14b50482bc1fa9ef4850d7983780ceda876bb80a -m 'merge: integrate CHUNK-001 container and TSV probe'`
  → PASS без конфликтов. Чистая объединённая проверенная ревизия:
  `6b0bdfc26a5c53934da9980c4a3ed5051b7910bc`.
- `python3 tools/check.py` → PASS: 37 Python + 31 Rust all-targets,
  fmt/check/clippy с -D warnings и отдельный doc.
- `cargo test --all-targets --locked --offline` → 31 PASS, включая from_raw,
  атомарность set, соседние nibble и пять тестов chunk_probe.
- `python3 tools/parity/run.py chunk --strict` → PASS, оба процесса exit 0,
  4182/4182, dirty=false. `artifacts/parity/chunk/run-jvj35_be/report.json`.
- `python3 tools/parity/run.py rng --strict` → PASS 96/96,
  `artifacts/parity/rng/run-5_dm3mel/report.json`.
- `python3 tools/parity/run.py rng-extended --strict` → PASS 13227/13227,
  `artifacts/parity/rng-extended/run-epqhsc13/report.json`.
- `python3 tools/parity/verify_report.py artifacts/parity/chunk/run-jvj35_be/report.json --protocol`
  → PASS: хеши и native audit; отдельные синтетические подмены значения,
  nibble-order и index обнаружены. 46 настоящих TSV-запусков + missing JAR и
  ошибка reflection на явно синтетической копии НАШЕЙ обёртки — 48 всего PASS.
  Исходный JAR, адаптер и live-выходы не менялись.
- `python3 tools/parity/verify_report.py artifacts/parity/rng/run-5_dm3mel/report.json --protocol`
  → 36 PASS; аналогично `artifacts/parity/rng-extended/run-epqhsc13/report.json` → 44 PASS.
- Подлинность nibble-эталона: JAR hash и code source проверены; original wi
  реально получил constructors=6/get=3606/set=1746/raw reads=1752. Имена и хеши
  опубликованы в tests/fixtures/chunk/provenance.md; заимствованного Java-кода нет.
  Наша обёртка отдельно реализует контракт координат/index/ID/безопасных отказов.
- Изменены собственный status/handoff, docs/progress.md и статус CHUNK-001.md.
  Итоговая ревизия приёмки — коммит с этим разделом, включаемый в main fast-forward.
- Вклад: Rust контейнер/probe — agy, Java/research/corpus/runner/review/integration — Codex.
  Codex Rust-код не правил. Область — контейнер без NBT/world load/render.
- После main перечитать очередь: ожидается complete; новых этапов не создавать.

### Фактическое завершение очереди и main

- RNG-002 принят в main коммитом `e11362c518ad75c83edda89076ea1ef44b256415`.
- CHUNK-001 принят в main коммитом `2c5cb4a6256acb4a29a7beb7b9e0dc9f1197ed4c`.
  `git switch main; git merge --ff-only codex/verification; git switch codex/verification`
  → успешно, main e11362c..2c5cb4a. Чужая ветка/worktree не перемещались.
- `git rev-parse main HEAD` → одинаковые 2c5cb4a...;
  `git diff --exit-code main HEAD` → 0; `git status --short` → пусто.
- `python3 tools/tasks.py next codex` → state complete; незавершённой приёмки нет.
- Этот финальный отчёт дополняет принятую ревизию только документацией/своим статусом
  и включается в main тем же fast-forward. Проверенный код остаётся идентичным
  ревизии `6b0bdfc26a5c53934da9980c4a3ed5051b7910bc`.
- Все шесть новых карточек Codex выполнены, новые этапы вне очереди не открывались.
  Следующий шаг — отдельные контракты по roadmap; обновить ветку agy до main
  в её следующей собственной сессии. Публикация удалённых веток не выполнялась.
## NBT-001-C01 — done

- Исследован оригинальный Beta 1.7.3 client JAR `../.local/minecraft/client-b1.7.3.jar`, SHA-256 `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`. Найдены obfuscated классы `ij` (tag factory/named I/O) и `nu` (compound).
- Добавлены `tools/oracle/src/NbtOracle.java`, `tools/oracle/nbt_corpus.py`, NBT fixtures и provenance. Адаптер вызывает `ij.b(DataInput)` и `ij.a(ij,DataOutput)` оригинала.
- `python3 tools/oracle/nbt_corpus.py` → 5200 детерминированных valid cases, original tag ids 0..10, supplementary Unicode/NUL, signed boundaries, IEEE payloads, empty/nested compound/list и arrays; 4 invalid cases. TAG_Int_Array (11) явно вынесен в wire-only, поскольку Beta factory оригинала поддерживает только 0..10.
- `javac -cp ../.local/minecraft/client-b1.7.3.jar -d artifacts/oracle-nbt-check/classes tools/oracle/src/NbtOracle.java` → PASS. Live adapter на corpus → 5200 round-trips, `cases=5200`, JAR hash подтверждён. Invalid fixture тем же адаптером → ненулевой код и stderr.
- `git diff --check` → PASS. Rust parity не заявляется; следующий шаг — C02 runner/canonical comparator после появления agy A01/A02 по очереди.
- NBT-001-C02: `tools/parity/run.py` расширен группой `nbt`, с exact
  round-trip bytes/consumed-length comparator, сохранением raw logs/hash и
  strict `not_implemented` при отсутствии `nbt_probe`.
- `python3 tools/check.py` → PASS (37 Python, 31 Rust, fmt/check/clippy/doc).
  `python3 tools/parity/run.py nbt --strict` → ожидаемый `not_implemented`
  exit 1, Java выполнила 5200 случаев: `artifacts/parity/nbt/run-mfmj868x/report.json`.
- Предыдущая группа `rng` повторно PASS 96/96; следующий шаг — ожидать agy
  NBT A01/A02 и затем C03 по точному snapshot.
## 2026-11-08 — открыт REGION-001 после приёмки NBT

Проверил snapshot Codex/main `2930029` (NBT-001-C03 принят) и agy `38ded3c`
(handoff agy добавлен fast-forward в Codex перед изменениями). Обе сессии были
свободны, деревья чистые. Подготовлен read-only контракт McRegion и пять карточек
для Codex/agy; старые выполненные статусы сохранены. Новая рабочая область — чтение
region-записей и bounded decompression; записи в `.mcr`, `level.dat`, генерация
и рендер не включены.

Локальный Prism-мир в каталоге сохранений экземпляра Beta 1.7.3: `minecraft/saves/New World`.
содержит четыре `.mcr`, 704 непустых location slots (везде compression id 2).
В read-only scan границ обнаружено 0 malformed entries и 0 пересечений. Размеры
и SHA-256 каждого файла записаны в `docs/tasks/REGION-001.md`. Были прочитаны только
region headers/record headers и хеши файлов; save не изменялся. Raw сохранение не
копировалось в Git.

Для A01 выбран `flate2` pure-Rust backend `rust_backend`; C01 добавит разрешённую
версию и lockfile прежде, чем agy начнёт implementation. Источник backend:
[официальные docs.rs docs](https://docs.rs/flate2/latest/flate2/).

Изменены контракт, roadmap/progress, карточки ролей, queue и pending entries.
Зафиксированные next tasks: Codex C01 (оригинальный RegionFile adapter, dependency,
snapshot fixture); agy A01 откроется после C01; затем обе очереди выбирают C02/A02
и C03. Сохранение остаётся локальным; parity helper должен тестировать копии и
проверять неизменность источников.

Проверки этого docs-only этапа: `git diff --check`; JSON parsing queue/status files;
очередь валидируется `tools/tasks.py` после синхронизации обеих веток. Полный Cargo,
Python и parity наборы не запускались повторно, код движка не менялся.

## REGION-001-C02 — runner и protocol checks

- Рабочая ветка `agents/region-runner`, исходный C01 `dcca8e3`; Rust для C02
  fast-forward до `033aa449b96f49414258069d9970243892b0d805`. Ветка C02
  закоммичена двумя изменениями: `5c7655093c70112111b0e1125f1e5a81630b6bb9`
  (runner/review/tests) и `c76113e79e6928d16bad9764157deb6296739d6f`
  (усиление saved-report verifier по review). Rust-файлы не менялись.
- Добавлена группа `region`, использующая закреплённый `RegionOracle`/`qj` и
  `region_probe` на двух независимых снимках. Оригинальное сохранение не открывается
  Java: helper проверяет четыре source SHA до и после; Java и Rust получают только
  копии с marker. Сохраняются stdout/stderr, audits, отчет и отдельные decompressed
  payload-файлы. Компаратор сравнивает порядок, present/absent, timestamp,
  compression, длину и SHA-256.
- `python3 tools/check.py` → PASS: 60 Python, 57 Rust (all-targets), fmt/check/
  clippy/doc. `python3 -m unittest discover -s tests/python -p test_region_runner.py
  -v` → 9 PASS, включая invalid header, overlap, unknown compression, truncated
  zlib, output limit, path/symlink, payload/audit/copy tamper и timeout.
- Чистый live run после commit: `python3 tools/parity/run.py region --strict` →
  PASS, 748/748 ordered cases: 704 present + 44 absent, mismatch=0,
  `rust_dirty=false`; `artifacts/parity/region/run-sfw68p9c/report.json`.
  Java/Rust output SHA-256 одинаков: `b816543713a272313d46f071d9e376ca0e9ae8f239ae57b3dab749ccc6bc7f4d`.
- `python3 tools/parity/verify_report.py
  artifacts/parity/region/run-sfw68p9c/report.json --protocol` → PASS: pinned
  JAR/class, native qj audit, process logs, fixture, source/copy hashes, TSV-linked
  payload bytes; 16 synthetic valid/invalid protocol executions и payload mutation.
  Все четыре world SHA до/после равны manifest, обе copies равны source. Синтетические
  malformed reader checks отделены от 748 native live cases.
- Это C02 runner baseline на Rust SHA `c76113e`; более поздний Rust gzip edge fix
  `4f6c41c` и native synthetic edge matrix принадлежат C03. Следующий шаг —
  координатор выполняет C03 на итоговом Rust snapshot и принимает изменения.

## WORLD-LOAD-001-C01 — native metadata oracle и snapshot

- Работа выполнена только в отдельном worktree runner, ветка `agents/region-runner`, после fast-forward `main` до `bf18c34f7cd119558f2c84ad8be64801032543ea`. Код Java и corpus зафиксирован snapshot-коммитом `02b03a27bded82dfffc8be28ba40d80a88d3c014`; reviewfixes/status/handoff — отдельным последующим коммитом.
- Добавлены `tools/oracle/src/WorldOracle.java`, `tools/oracle/world_corpus.py`, пять-file manifest и markers, 749-row coordinates-only fixture, provenance/native audit/generated audit и четыре Python tests. Oracle вызывает `ij.b(DataInput)`, `ei(nu)`, `qj.a(int,int)` оригинального JAR. Он не создаёт fd/World, lm/Chunk, entity или tile-entity objects; audit фиксирует `native_chunk_loads=0`.
- `python3 tools/oracle/world_corpus.py --write` → PASS: 1 level + 748 chunk cases; 704 occupied и 44 absent. Точные live calls: qj=748, ij=705 (level + 704 present), ei=1, native_chunk_loads=0. Два полного запуска имеют одинаковый output/audit SHA `f2e08c13997bb6fab03170dfbcec1fb3bd619e9079b792cf324fd853bd12fcc2`.
- Отдельный generated self-test: 10 labeled cases, native calls ij=11/ei=7/qj=0. Проверены defaults, отрицательные/ненулевые boolean bytes, пустой/отсутствующий Player, границы i32/i64, wrong native types, unknown Unicode/empty names, signed raw arrays, unknown entity/tile list payloads без factories, empty byte-type entity list и strict missing/type/length/coordinate/list errors.
- JAR SHA-256 `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`; class hashes `ij=9db862e2ee83e95a2db7a8495594d2c0e0bbe904d96c3f09f167f2b47b72b8d0`, `nu=a1a5701ec2180d01ddc604cc8c2e5aff1176b10faf83bdb64d2f1cc8c806ac4a`, `ei=5d01d1dda7be346c57fd00846d2425615186c3ef1c65c06aa0bd0f45aa9f7fef`, `qj=6c75a123b63d555caa98e1a2619aa2483e405566de9e356585b1dc682a7da4a6`. Source and copied hashes for level.dat and all four regions matched before/after. Latest run artifacts: `artifacts/oracle-world/run-lzwoftww/`; raw NBT and class outputs remain ignored.
- Checks: `python3 -m unittest discover -s tests/python -p test_world_corpus.py -v` → 4 PASS; `python3 tools/check.py` → 64 Python + 58 Rust all-targets, fmt/check/clippy/doc PASS; `git diff --check` PASS. A no-write corpus run left tracked audit/generated fixtures unchanged.
- C01 establishes native extraction/default evidence only; Rust parity and report runner belong to later cards. Next: coordinator review/integration; C02/A01 may proceed from the contract and this snapshot.

## WORLD-LOAD-001-C02 — implementation and C03 acceptance

- Назначенная модель: `gpt-6-luna`, reasoning `low`; без делегирования. Работа велась в отдельном worktree `world-runner`, ветка `agents/world-runner`.
- Начальный C01 snapshot `374f6e6` получен fast-forward. Разрешённый C01 post-review `88b510d0fc37304d4b26ef50b2af5ea8b6f567f1` не был потомком merge HEAD, поэтому включён отдельным `--no-ff` merge; добавлены лимиты входных файлов Oracle.
- Проверенные изменения в этом checkpoint: `tools/parity/run.py`, `tools/parity/world_run.py`, `tools/parity/verify_report.py`, `tools/parity/world_review.py`, новый `tests/python/test_world_runner.py`, status/handoff. Runner содержит dispatch, строгий координатный TSV parser/comparator, две независимые hash-verified копии, Java native audit и retained-byte binding, Rust build/run путь и saved-report/protocol/mutation verifier.
- Review нашёл и исправил дефект: `-0` разрешён контрактом, но одновременно попадал в список protocol-invalid inputs; теперь он проверяется отдельным положительным случаем, включая канонический вывод `0,0` и точную ширину typed row. Дополнительно `_check_copy` теперь отклоняет лишние файлы, директории и symlinks, проверяя ровно ожидаемые входы и два маркера. Saved verifier выдаёт явные ошибки при отсутствии process logs или pinned JAR.
- C02 implementation commit `dad835e` was reviewed and merged; C03 found and fixed three concrete runner/reviewer defects (snapshot evidence key, absent-vs-empty native audit manifest, missing mutation exception import). These fixes are in the final accepted revision below.
- Final `python3 tools/check.py` → PASS: 75 Python tests; cargo fmt/check/clippy; 40 library + 26 example Rust tests; doc tests. Focused world runner/corpus tests: 15 PASS, plus retained-manifest regression PASS.
- On the final accepted revision all five earlier parity suites and WORLD passed. WORLD report `artifacts/parity/world/run-eb0qhh2b/report.json`: 749/749 exact rows, `rust_dirty=false`. Saved report/protocol review passed 22 Java/Rust executions; generated edge review passed 42 cases / 84 reader executions. Full commands and report paths are in `docs/tasks/status/codex.json`.

## Final package state (2026-11-08)

- Rust snapshot: `a7aa6856f81da7f3e008b4c1d7ad9ac89cffac18`; accepted clean combined revision: `8292470d5cdcba84ad42527496226e2c1ca33f8d`.
- WORLD-LOAD-001 is accepted and fast-forwarded into `main`; all five stage cards are done. Overall game completion estimate is approximately **9%**.
- Scope is read-only typed world data loading. The project still lacks game simulation, save writing, graphics and the play loop. Next, prepare the VIEW-001 contract.

## VIEW-001-C01 — native view oracle and fixed scene

- Pinned the original client JAR (`af1fa04b…`) and extracted `terrain.png` only to
  the configured ignored local resource path. Added exact workspace pins/lock for
  `wgpu`, `winit`, `image` (PNG-only), `bytemuck`, `glam`, and `pollster`.
- Inspected `uu`/`cv` bytecode and call sites. The actual face mapping is virtual
  `uu.a(world,x,y,z,face)`; grass ID 2 overrides it based on the block above.
  The fixed copied 5×5 scene therefore records `(ID, metadata, above ID)` mapping
  contexts, native render/opaque/full-cube properties, six tiles and alpha masks,
  plus native neighbor-visibility pairs.
- `python3 tools/oracle/view_corpus.py --write` ran the original classes twice on
  hash-verified copies: 25 slots, 46 unique states, 50 mapping cases (4 contextual
  overrides), 1,617 neighbor pairs; native qj=25, ij=26, ei=1, tile=300 and
  visibility=1,917. Source and copies stayed unchanged. 20 mappings meet the
  initial supported opaque-cube filter; 30 remain unsupported.
- Native calls do not include `cv` vertex emission or a native GPU frame, so the
  fixture proves mapping/material behavior only, not vertex or pixel parity.
  `uu.d` bytecode was inspected but its live brightness output was not captured;
  no exact brightness parity is claimed by C01/C02.
  Synthetic metadata, above-block, transparent, partial-model, liquid and boundary
  inputs are separate from live observations.
- `python3 tools/oracle/view_corpus.py` → PASS repeatable fixture/audit;
  `python3 -m unittest tests.python.test_view_corpus -v` → 6 PASS;
  `python3 tools/check.py` → PASS for Python, fmt, check, clippy, workspace and docs;
  `git diff --check` → PASS.
- Next dependency-ready work: C02 runner/protocol checks and A01 CPU scene/mesher.
  C02 remains Codex-owned; A01 is agy-owned. Viewer/window work follows A01 and C02.

## VIEW-001-C02 — runner and saved-report verification complete

- Added `view` to `python3 tools/parity/run.py`; the native `ViewOracle` runs twice
  on the marker-guarded WORLD/REGION copied scene. The report binds its 25-slot
  coordinate input, native output, expected C01 fixture, JAR/atlas/classes, source
  hashes, copied-world hashes and native call audit. Comparison validates the TSV
  wire format and then compares every byte in order.
- `python3 tools/parity/run.py view --strict` → expected
  `not_implemented`, exit 1, because `crates/beta-viewer/examples/view_probe.rs`
  is not present on this snapshot. The original Java adapter ran twice and yielded
  50 mapping cases, 1,617 neighbor pairs and stable output SHA
  `ef343aab6bd490b27d52ac1e2aef9c272a00c6a67ca93212167dd52949f88453`.
  Saved report: `artifacts/parity/view/run-wgv2ocai/report.json`.
- `python3 tools/parity/verify_report.py artifacts/parity/view/run-wgv2ocai/report.json --protocol`
  → PASS: saved source/JAR/atlas/class/copy/output hashes and native audit;
  repeated outputs match. Four malformed/duplicate protocol forms, malformed
  arguments, missing/wrong JAR, wrong atlas, generic subprocess timeout and a
  one-byte mapping mutation are rejected in separately labelled synthetic checks.
- `python3 -m unittest tests.python.test_view_corpus -v` → 7 PASS.
  `python3 tools/check.py` → 82 Python tests; cargo fmt/check/clippy, 40 library
  and 26 example Rust tests, and docs PASS. `git diff --check` PASS.
- A02 probe discovery target is fixed at
  `crates/beta-viewer/examples/view_probe.rs` and built with
  `cargo build -p beta-viewer --example view_probe`. C02 compares mapping TSV only;
  native renderer vertex execution and live brightness/pixel parity remain outside
  evidence. The game completion estimate remains approximately 9%.

## VIEW-001-C03 — integration and acceptance

- Reviewed and accepted clean agy snapshot `973de0be1bc36f20ce5d136475cd7226ff884ebb`;
  fast-forwarded it into `main`. The accepted viewer loads only the pinned 5×5
  read-only world copy, renders supported full opaque cubes with the original
  terrain atlas, and reports unsupported states. Fixed-scene GPU smoke was
  inspected at `/tmp/beta-viewer-smoke.png`; the Xvfb Mesa backend emitted only
  its software DRI3 fallback warning. Camera and resize event paths were reviewed;
  interactive-control automation was not part of this smoke.
- `python3 tools/check.py`, all-target/doc tests, fmt/check/clippy `-D warnings`
  and `git diff --check` passed. Seven strict parity groups passed on the accepted
  clean snapshot: rng 96, rng-extended 13227, chunk 4182, nbt 5250, region 748,
  world 749, and view 50 mappings plus 1617 neighbor tuples.
- View report `artifacts/parity/view/run-r_35l3w0/report.json` has
  `rust_dirty=false`; `verify_report.py --protocol` and `view_review.py` passed.
  Pinned JAR, atlas, scene and source/copy hashes matched; original save source
  files were unchanged. Evidence covers CPU mapping and neighbor visibility plus
  a visual smoke frame, not native vertex execution, packed brightness/AO, or
  pixel parity. Only the confirmed opaque-cube subset is rendered.
- Updated overall estimate to approximately **10%**: the fixed-scene renderer is
  the first accepted graphics capability, while gameplay, simulation, general
  world loading, save writing and most rendering features remain unimplemented.
- Follow-up review of user-supplied screenshot led to agy commit `c98e227`:
  four tile-isolated mip levels and linear mip-level blending reduce texture
  minification aliasing without neighbor-tile bleed. Regression test covers all
  mip sizes and tile color isolation. `tools/check.py`, viewer tests/clippy and
  clean view strict/protocol/review all pass (`run-bfyl2hef`). The window started
  under Xvfb, but a fresh screenshot could not be captured because the sandbox's
  Xvfb socket directory currently has incorrect ownership; the prior visual frame
  remains available, and no pixel-parity claim is added.

## MATH-001-C01 — original MathHelper oracle and corpus

- Pinned client class `in`, JAR SHA
  `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`, class SHA
  `8693f3179fb0ff1df382e1d3388d23ab688ad3e26415454655da88f10b054f63`. `MathOracle.java` calls its ten native methods by
  reflection; it does not reproduce their formulas.
- Generated a committed behavior corpus of 131,408 operations. `sin` and `cos`
  each address every one of the original 65,536 LUT entries. Additional cases
  cover float/double edge bits, floor/sqrt/abs, signed integer division, native
  arithmetic errors, and null/empty/Unicode strings.
- `python3 tools/oracle/math_corpus.py --write` → PASS; repeated oracle run → PASS,
  identical output SHA `feddfb35ebc8b36f1403d16ffe6f7d3190036af721d0bad840bbabe6f9ec9f3f`.
  Input SHA `18ee45882081ede2178578572c3e16a5467091552870c1a004fc01fcca5c5dbf`.
  Fixture provenance is in `tests/fixtures/math/audit.json`.
- `python3 -m unittest discover -s tests/python -v` → PASS, 85 tests.
  `python3 tools/check.py` → PASS: 85 Python tests, fmt/check/clippy, all Rust
  targets and doc tests. `javac` MathOracle and `git diff --check` → PASS.
- During workspace checks, the portable world audit fixture exposed a hard-coded
  local-path assertion; validator now checks the pinned JAR basename. A 50 ms
  subprocess timeout test was scheduler-flaky, so its startup allowance is now
  1 second while the child remains a 2 second sleeper.
- C01 oracle/corpus does not claim Rust parity; C02 runner and A01/A02 remain.
  Whole-game estimate stays approximately 10%; this data-only package adds no
  accepted gameplay capability yet.

## MATH-001-C02 — parity runner and evidence review

- Added `math` to `tools/parity/run.py`: checks pinned client JAR and original
  `in` class hashes, runs the Java adapter twice, compares exact ordered TSV
  values and raw float bits, builds/runs `math_probe`, and records source,
  toolchain, process, and output hashes in a saved report. Strict mode returns
  failure until a Rust probe exists.
- Added saved-report verification and separately labelled synthetic checks for
  output bit/type/order/row-count and source-hash mutations. Optional protocol
  review sends malformed operations, float bits, integer ranges, UTF-8 and
  duplicate IDs to both implementations.
- Corrected the shared `floor_div_i32` contract after direct bytecode/fixture
  inspection: `i32::MIN / -1` returns `2147483646` through the original
  intermediate overflow; only division by zero is reported as an error.
- `python3 tools/oracle/math_corpus.py` → PASS; 131,408 original outputs remain
  byte-identical to the committed reference. `python3 -m unittest
  tests.python.test_math_runner -v` → PASS (4). Python compilation and
  `git diff --check` → PASS. Live strict run is recorded below after completion.
- C02 adds the repeatable differential harness, not gameplay behavior; overall
  estimate remains approximately 10% until MathHelper and its probe are accepted.

## MATH-001-C03 — integration and acceptance

- Reviewed and merged exact agy snapshot `aaebed54d22a9149894792e305ed264d9a2b1a36`.
  `beta_core::math` and `math_probe` reproduce the original `in` class across
  131,408 operations using exact raw float/double bits; all 65,536 sine and
  cosine LUT entries are covered. Java/Rust output SHA is
  `feddfb35ebc8b36f1403d16ffe6f7d3190036af721d0bad840bbabe6f9ec9f3f`.
- Final `python3 tools/parity/run.py math --strict` passed cleanly with
  `rust_dirty=false`; report `artifacts/parity/math/run-lgbtghq2/report.json`.
  `verify_report.py --protocol` verified pinned JAR/class, all report/source/
  binary/output hashes, detected five synthetic mutations and observed ten
  malformed-protocol rejections. The contract now records original
  `floor_div_i32(i32::MIN, -1) = 2147483646`; division by zero remains an error.
- All prior suites passed: rng 96, rng-extended 13227, chunk 4182, nbt 5250,
  region 748, world 749 and view 50 mappings plus 1617 neighbor tuples. World
  verification exposed absolute class-source paths; the validator now accepts
  the pinned JAR basename independent of checkout location, with regression
  coverage. World saved review passed 22 protocol executions and retained-byte
  checks; view saved review passed for all 50 mappings.
- Final `python3 tools/check.py` → PASS: 89 Python tests; fmt, workspace
  check/clippy, all-target tests and doc tests. MATH-001 is accepted in `main`.
  Updated full-game estimate from approximately 10% to **11%** for the first
  accepted implementation of game math; world simulation, movement, generation,
  interaction and most presentation remain unimplemented.

## JAVA-001-C01 — direct JVM oracle and behavior corpus

- Added `JavaCompatOracle.java` with direct JVM primitive casts and direct
  `String.hashCode()` on strict UTF-8-decoded strings. Its protocol and oracle
  source are independent of the Rust implementation and game client JAR.
- Generated 18,493 committed cases: 4,109 raw-bit inputs for each f32/f64 to
  i32/i64 conversion, including NaN, infinities, signed zero, subnormals and
  integer-range boundaries; plus 2,057 deterministic Unicode strings with
  supplementary scalars, combining marks and long hash-overflow cases.
- Two independent JVM runs matched exactly. Input SHA
  `ef9ee6fabab78ee9957abd7b9785ac0708ad9b736c9cc25671095b826bdb55ee`, output
  SHA `397c7c50db72608ccb863d2f00c331da8982dee4cb31cc702ddb158003ab99e8`, oracle
  SHA `c97fed3d941b130d69e454c190d2d1d4f22a3dfee357666035ed0320495da58f`.
- `python3 -m unittest tests.python.test_java_compat_corpus -v` → PASS (2);
  fixture regeneration, `py_compile` and `git diff --check` → PASS. A01 and C02
  can now proceed independently.

## JAVA-001-C02 — Java compatibility parity runner

- Added `java` group with raw-bit/UTF-8 parser, typed decimal comparator,
  repeatable JVM runs, source/runtime/class hashes, Rust build/run, timeout and
  subprocess diagnostics in saved reports. `verify_report.py` now routes to a
  dedicated evidence reviewer that checks the corpus/oracle audit, exact output,
  source snapshot and synthetic output/type/order/count/source-hash mutations.
- Protocol review sends unknown operations, bad bits, malformed UTF-8, wrong
  arity and duplicate IDs to both sides; the Rust probe must reject atomically.
- `python3 -m unittest tests.python.test_java_runner -v` → PASS (4);
  Python compilation, runner listing and `git diff --check` → PASS. The strict
  live run and timeout evidence are recorded with C02 status after execution.
# JAVA-001-C02 handoff

Parity runner and saved evidence reviewer are committed (`05cda3f`); follow-up
fix `6884665` corrects working-tree status capture. Four runner tests pass. The
Java oracle repeated all 18,493 rows identically (reference SHA
`397c7c50db72608ccb863d2f00c331da8982dee4cb31cc702ddb158003ab99e8`). Strict
run correctly reports `not_implemented` until A02 lands; timeout capture was
also exercised. C03 must run saved protocol review against the successful live
report after integrating the Rust probe.
# JAVA-001-C03 acceptance

Integrated exact agy snapshot `73dac4598dd2d596c1158b640a2bf9f630950b3d`.
`tools/check.py` passed with 95 Python tests and all Rust workspace checks.
`java --strict` passed 18,493/18,493 ordered rows; reference and Rust output SHA
are both `397c7c50db72608ccb863d2f00c331da8982dee4cb31cc702ddb158003ab99e8`,
and the report records `rust_dirty=false` at revision `7a2ca977`.
`verify_report.py --protocol` passed five synthetic mutations and ten malformed
Java/Rust protocol executions. Previous strict suites passed: RNG 96,
RNG-extended 13,227, chunk 4,182, NBT 5,250, region 748, world 749, and view
50 mappings plus 1,617 neighbor tuples. JAVA-001 is accepted; rounded full-game
estimate remains about 11% because this is a narrow compatibility foundation.

## GAME-ORACLE-001-C01

Integrated exact oracle snapshot 21198eed5ad7566d431fb5a1679d585fddfe8967, produced by gpt-6-luna low. Coordinator repeated corpus generation/verification and two Python tests: PASS. 6278 native cases, 22 operations, all six ray faces; repeat output SHA 84702c522f875d995182cec388ee3334c262770d26f5d57a0ef5d3d505689b73. Rust A01 snapshot ab1422f4cdbd878a88634980511c7473cecd6e63 integrated; actual live parity awaits A02 probe. No game JAR/resources/decompiled code committed.

## GAME-ORACLE-001-C02

Coordinator implemented strict geometry CLI and retained-report review. Seven unit tests pass; direct native runs repeated all 6278 cases but Rust probe is not yet present (strict not_implemented). Timeout captured. Custom inputs use the same native oracle and raw-bit comparator with scenario=custom; pinned fixtures are never rewritten. C03 will exercise retained evidence mutations/protocol after A02 arrives.

## GAME-ORACLE-001-C03 / EVIDENCE-001 acceptance

Clean checked revision 9347ae5. Native geometry 6284 exact records; report artifacts/parity/geometry/run-g4ziucta/report.json, rust_dirty=false. Custom repro changed native face5 vs Rustface4 before 011a59bf, now both face5; saved custom report run-o67darfn passes 14 malformed protocol executions. Native corpus extended independently by 6 cases in aa433b70. Coordinator made only Rust test count/format updates (71b3c85/9347ae5), no engine algorithm changes. Full check: 114 Python cases with 6 opt-in skips; all Rust fmt/check/clippy/all-target/doc checks passed. Explicit six opt-in integration tests all passed, including saved geometry and math/java mutations/protocol. Math run-yfts9lyj and Java run-vp2y_3bm each clean. All seven earlier strict suites passed again. EVIDENCE commits a8d7454/ebf49649 by gpt-6-luna low integrated, plus coordinator build-input snapshot fix becea99. Accepted game capability estimate approx12%; no GitHub push yet (batch baseline11%, requested interval5–10pp).

## MOVE-001 C01/C02

Integrated native oracle a0069de88d2545bd6a9ab9c893a13c99bf05c50e and pure Rust528161a +0bc6bcaa. Coordinator reviewed bytecode and requested fixes for missing step descent, step dy reference and grounded-air friction; native parity awaits A02. Runner independently executed two original sn/ls traces:643scenarios/8213states, all outputs/audits pinned and repeatable. NativeWorld constructors0; methods inherited from original JAR, dry environment/callback seam explicitly recorded. 11 normal Python corpus/parser tests PASS, strict correctly not_implemented; controlledtimeout handled. C04 215ad6e integrated for further independent review/GUI smoke; not yet accepted.

## MOVE-001 C04 local acceptance

Viewer snapshots `215ad6e` and `4862262` integrated; author gpt-6-luna low.
Coordinator reviewed controller/input/bounds and ran `cargo build --locked --offline -p beta-viewer` plus Xvfb screenshot smoke. Visible scene at `/tmp/beta-viewer-smoke.png`; no crash, Mesa DRI3 software-rendering warnings only. Agent full workspace checks passed before integration. Native core/probe and all-suite final acceptance remain C03. C02 evidence hardening snapshot `12e00cc` fixes path substitution and pins registry capture; live mutation evidence remains C03.

## MOVE-001 final acceptance

Clean tested revision `82f39308b06625dc509cdc3ae491d5b7daab70d8`. Integrated
agy `3e10761` and viewer `215ad6e`/`4862262`, all authored gpt-6-luna low.
Root authored runner/review and integrated/reviewed; no direct engine Rust edits.
Native report `artifacts/parity/movement/run-56tm9oak/report.json`: 643 scenarios,
8213 states, zero mismatches, clean revision, SHA output
`80e3faef85ac842d756c0ac85731208db583d009ecd884627ff33e94dbc91bbd`.
Both original JVM runs agree; actual primary calls move1045/travel7168/jump192;
zero native World constructors, explicit synthetic-static-dry environment.

`python3 tools/check.py` PASS: 128 Python cases (7 opt-in skips), workspace
fmt/check/clippy, all-target tests and docs. Explicit movement live test PASS
(including artifact/process/source/command/path substitution/registry mutation
rejections, 4 output mutations, 12 malformed executions). Explicit geometry +
math/java live tests: 6 PASS, protocols14/10/10 and saved mutations.

All previous strict suites passed on merged revision:
rng/run-_919kh4c; rng-extended/run-xptshku7; chunk/run-1__98i_e;
nbt/run-ltnp_uuo; region/run-t632w0m2; world/run-ihd1rvn9; view/run-mqupd9ac;
math/run-64cedc6f; java/run-uzpi97fx; geometry/run-500seiok.

GUI smoke: `env -u WAYLAND_DISPLAY xvfb-run -a sh -c <viewer/screenshot>`
PASS, screenshot `/tmp/beta-viewer-smoke.png` inspected: scene from grounded
camera. First capture while rebuild delayed startup was black; repeat after
completed build showed terrain. Mesa DRI3 software-render warnings only.
Read-only resources/save retained. No full graphical/gameplay parity claim.

Estimate about16%, +5pp from JAVA-001 baseline11; milestone eligible for user
requested GitHub push. Next work: VIEW-002 scope/contract, research recorded
separately. Remaining unsupported leaves/shapes/fluids/AO are explicit.

## UV-001 native C01

User reported rotated bark on adjacent faces. Native capture snapshot a936ffe
(gpt-6-luna low) invokes unchanged cv.a..f direct cube face methods; reads actual
original nw.g packed floats, six triangle vertices/four unique corners each.
Root independently compiled exact committed source and ran twice against pinned
client + local LWJGL2, both byte-identical to native fixture.
`artifacts/uv-review-boa1vsfk/report.json` retains commands/source/class/input/
output hashes/logs. 84 cases, 504 submissions, 336 corners, class/source bindings.
Actual differences observed on faces1/3; prior synthetic UV test repeated the
implementation rather than comparing native buffers. A01 now writes independent
fixture regression before fixing UV. No whole renderer/GL pixel parity claimed.

## UV-001 final acceptance

Integrated `6312bcf` (agy gpt-6-luna low), checked clean merged `a3ca583`.
Root reviewed small production change: only faces1/3 UV arrays. Independent
fixture regression by xyz, native triangles duplicate UV verified. Baseline
112 differences (56 each face1/3), afterwards336/336 corners raw-bit exact.
Pre-fix log retained artifacts/uv-review-boa1vsfk/rust-before-fix.log. Native
fixture and JAR unchanged. Oracle/tests authored separate gpt-6-luna low agent.

`python3 tools/check.py` PASS:133 Python cases10 opt-in skips, fmt/check/clippy,
workspace all-target tests and docs; beta-view8 tests includes native fixture.
`BETA_LIVE_UV=1 python3 -m unittest tests.python.test_uv_native -v`:5 PASS.
`python3 tools/parity/run.py view --strict`:PASS report
artifacts/parity/view/run-_z9dvlg7/report.json.
`cargo build --locked --offline -p beta-viewer`:PASS; Xvfb screenshot
/tmp/beta-viewer-smoke.png inspected, terrain visible/no crash.

Local main acceptance; user requested push only each5–10pp so no new push for
this bugfix. Estimate stays about16%. Next VIEW-002 tint/leaves; do not claim
grey grass or absent canopy fixed by UV correction. No direct engine Rust
changes by coordinator.

## COLOR-001 accepted; LEAF-001 C01 ready

Integrated Rust ea8bcfc (gpt-6-luna low); native C01 6eb78c6/3719b0e by separate
gpt-6-luna low; independent C02 review same model, coordinator wrote runner.
Root direct engineRust changes:none. Clean validated c332ebb.
147 Python tests15 optin skips +allworkspaceRustchecks PASS; 11 explicit native
color/corpus/runner tests PASS, saved evidence mutations and24 malformed
subprocesses. Latest pinned strict report artifacts/parity/color/run-6l18gnu9,
saved protocol PASS; customsingleton run-ib05q2ko/protocol PASS.

Concern: five baseline seed extrema include equal JavaRandom low48bit states,
so independently tested64 diverse seeds×5 points320additionalcases, exactnative
match run-n5ra0fx2; saved review4 mutations PASS. Reproduce seeds by
s=(i*0x9e3779b97f4a7c15+0x0123456789abcdef) mod2^64, reinterpret signed, i0..63;
points (0,0),(-17,31),(1234567,-7654321),(30000000,-30000000),(-30000000,30000000).
Retained input/source/assets/classes/executable/commands/logs in each reportdir.
Previous strict RNG96,extended13227,math131408,java18493 PASS at
run-kmg8v2xt/run-dj8u2naf/run-30q7ykig/run-u4s_v3yy.

LEAF C01 01dfacd native actual calls confirmed constructor-default leaves
opaque=true tile52; fancy settertrue=>opaque false52/132, leafleaf faces shown;
fastfalse=>opaque true53/133, leafleaf faces hidden. Grass nativeflat top tinted,
sidebasewhite plus tinted tile38 overlays; collision originaluu.f fullcube.
Earlier read-only agent guessedwrongfrombytecode; root directliveexperiment
GraphicsBehaviorProbe overturned guesses before freezing LEAF contract.
Fresh root two-JVM retained capture artifacts/leaf-review-dgf6nlww/report.json
PASS; native GL_LEQUAL515 and alphaGREATER516,.1 verified viajavap.
Renderer C04 in separate /173st/leaf-viewer worktree stillinprogress.
COLOR estimate stays16% pending visibleintegration. No push until +5–10pp.


## COLOR/LEAF/LIGHT — combined acceptance, 2026-11-08

- Clean tested source755d6dd. Native LIGHT c610b1a/8c8c6d1; Rust core/probe
  a39fda2; native leaf01dfacd; renderer8a12289/ec3759c/ff84905/2072705;
  vertex capture08c44c3; runner6bc6eaf hardened755d6dd.
- All delegated implementers gpt-6-luna low. Coordinator reviewed/merged exact
  snapshots and requested parser/artifact/type/revision/color fixes; direct Rust
  engine contribution by coordinator: none. Speculative leaf bytecode notes were
  corrected by actual native calls before implementation; packed nw integers in
  supplemental fixture are little-endian ABGR (visible RGB low-byte-first).
- Final tools/check.py PASS161 Python cases17 explicit opt-in skips;148 Rust,
  fmt/check/clippy/doc tests. Log /tmp/173st-light-accepted-check.log.
- LIGHT strict72928 exact and saved --protocol45 executions:
  artifacts/parity/light/run-mncwexld/report.json. Live runner suite10/10 PASS.
  Root independently reviewed this final saved report. Additional2048 custom
  signed-i64/randompartial rows run-cdjcng_7 strict +saved review PASS;
  emission-only singleton run-50vjjc8e strict+saved PASS (zero synthetic worlds).
- COLOR fresh1600 exact run-8sshr63k, saved4mutations/24malformed PASS;
  VIEW fresh50mappings/1617neighbors run-ibpvecb1 strict+savedprotocol PASS;
  MATH131408 run-o17qdyaa and JAVA18493 run-8og1c7c0 strict PASS.
  Native leaf3 live tests PASS, native UV336-corner regression PASS.
- Supplemental native vertex capture repeated independently by root:
  artifacts/vertex-color-review-bx9ziix0/report.json,504 vertices12calls,
  both JVM outputs equal committed fixture. Synthetic proxy supplies brightness;
  exact native saved-light behavior is tested separately by LIGHT oracle.
- Final cargo build beta-viewer and Xvfb screenshot /tmp/beta-viewer-smoke.png
  inspected: green terrain/canopies; no startup error, Mesa software DRI3 warnings.
  GPU evidence is smoke-only, no original pixel parity claim.
- Static saved-time clear weather only; no AO/light propagation/weather/fluid or
  special stair/slab lighting. Current pinned copy is Overworld; supporting
  Nether scalar tables does not claim DIM-1 scene loading. Source JAR/save/assets
  remain read-only and ignored. No new dependencies.
- Full-game estimate≈20% (graphics+3pp, biome climate/palettes+1pp), not test-count
  progress. Local main accepts this package; push interval still anchored16%.
  No GitHub push until cumulative requested5–10pp gain. Next capability priorities:
  biome/terrain generation and broader block models/interaction under new contracts.


## FLUID-001-C01 — native water geometry, 2026-11-08

- Queue was complete after LIGHT/LEAF acceptance; created a narrow next contract for
  read-only water rendering (IDs 8/9), leaving lava and fluid simulation unsupported.
- `tools/oracle/src/FluidOracle.java` directly calls original `cv.b(uu,int,int,int)`
  on synthetic `xp` scenes and reads the words the pinned native renderer wrote to
  `nw.g`. No Java render formula, native World/Chunk, GL draw or GL flush is used.
- `BETA_LIVE_FLUID=1 python3 tools/oracle/fluid_corpus.py --live --write` → PASS:
  two separate JVM captures byte-identical, 44 cases / 1,242 vertices; output SHA-256
  `92625f8caffbe29bf19325937725ddab85590207217b6cde7b4bea53703146c1`.
- `BETA_LIVE_FLUID=1 python3 -m unittest tests.python.test_fluid_corpus.LiveFluidTests -v`
  → PASS; ordinary corpus tests → 2 PASS + 1 opt-in skip. Source and class hashes are
  stored in fixture/audit; JAR SHA remains pinned. The renderer-specific blend state
  and full-scene visual behavior remain for C02/C03 review.
- C01 is complete; next independent Codex card is C02 runner, while agy A01 is ready
  after the shared queue contract is integrated into both role snapshots.
- Estimate remains about20%; a geometry reference alone adds no game capability and
  does not advance the 5–10pp GitHub push interval.

## FLUID-001-C02 — native water geometry runner

Added `tools/parity/fluid_run.py`, the `fluid` dispatch in `tools/parity/run.py`, and `fluid_review.py` for retained evidence validation. The strict runner captures the original renderer twice, requires byte-identical output against the committed 44-scene / 1,242-vertex C01 fixture, and when A02 adds `fluid_probe`, compares ordered case rows and all eight raw words per vertex. Reports bind JAR, oracle/classes, inputs, sources, processes, logs and Rust binary/output hashes. Before the probe exists, `fluid --strict` correctly returns `not_implemented` after native verification.

Verification: 8 relevant Python tests pass (1 opt-in live test skipped); `python3 tools/check.py` passes all Python and workspace checks; strict native run produced and retained a 44-case report and `fluid_review.py` verified its evidence; `git diff --check` passes. Commit: `7934634`.

## FLUID-001-C03 — water integration acceptance

Accepted the exact agy snapshot `0cd7321caeb35db67cace8b1d4826c684ae2b41d`
after independent review. `python3 tools/check.py` passed; strict parity for
RNG, RNG-extended, chunk, NBT, McRegion, world, view, and fluid all passed. The
retained fluid report `artifacts/parity/fluid/run-ywu68m6b/report.json` passed
`python3 tools/parity/fluid_review.py`: 44 cases, 1,242 vertices, eight verified
processes, clean revision binding and native SHA-256
`92625f8caffbe29bf19325937725ddab85590207217b6cde7b4bea53703146c1`.

The separate viewer pass smoke rendered 25 chunks, 334,776 opaque vertices and
9,768 water vertices; the 640×480 capture is `artifacts/fluid-viewer.png`
(SHA-256 `67f69d0afed0bbb3f44be16e0e074a7461c808199946340c21aaaffdb17e1009`).
The five pinned source-save hashes were unchanged before/after smoke. Water is
render-only: lava, liquid ticks/interactions, transparent overlap sorting, pixel
parity and FPS improvement remain unsupported. FLUID-001 is integrated locally;
GitHub push is deferred until the requested 30% readiness target. Estimated full
game readiness is now about 21%, with roughly one point credited to visible water.

## BLOCK-001-C02 — strict ray-only parity runner, 2026-11-08

Added `tools/parity/block_run.py` and `block_review.py`, dispatched `block` in
`tools/parity/run.py`, and added runner tests. To give the future Rust probe
actual scene/ray inputs rather than answer selectors, extended
`BlockOracle.java` with `--inputs`; this emits the pinned `input.tsv` directly
from the same case table used for native traces. Each row includes endpoints,
liquid flag, scene block ID/metadata/coordinates, and shape. The runner invokes
and compares those Java-generated bytes on both native repetitions before
feeding the pinned input to `block_probe`. Input SHA-256:
`730213021f0dae2684ad37fe483a756ef4826acaa2bdb577978f4c2988e356ec`.

The report and probe schema are version 1 and explicitly `ray-only`;
`action_status=not_implemented` and `actions_compared=false`. Probe output must
contain exact ordered hit type, voxel coordinates, side, decimal hit point and
raw binary64 words. Unexpected fields (including neighboring IDs/metadata or
stack counts) fail the parser; C01 has no native action snapshots to compare.
Live native rows and synthetic malformed/mutation tests are classified
separately. The expected discovery target is
`crates/beta-core/examples/block_probe.rs`, built with
`cargo build --locked --offline -p beta-core --example block_probe`; reports retain the Java commands/process output hashes and are
rechecked with `python3 tools/parity/block_review.py REPORT.json`.

Checks: `python3 -m unittest tests.python.test_block_corpus
tests.python.test_block_runner -v` → 10 PASS; py_compile and diff-check PASS.
`python3 tools/check.py` also passes the full Python suite and locked offline
workspace checks.
`python3 tools/parity/run.py block --strict` exits 1 with
`not_implemented` because the Rust `block_probe` does not exist; it still
captures native ray and input output twice. Saved report
`artifacts/parity/block/run-vxiftqw0/report.json` passes the review command and
verifies six completed processes, pinned class/JAR hashes, both fixtures and
repeatability. This is not action parity. No C03 or Rust probe work was started.

## BLOCK-001-C01 — native ray corpus, 2026-11-08

On `codex/verification`, added `tools/oracle/src/BlockOracle.java`, its
repeat-capture driver `tools/oracle/block_corpus.py`, fixture/audit/provenance
under `tests/fixtures/block/`, and protocol tests in
`tests/python/test_block_corpus.py`. Original client JAR SHA-256 is
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`. Audit
pins the JAR-loaded class hashes for fd/vf/bt/uu/ob/pj/iz/gm and records native
raytrace and per-block shape-dispatch methods. Bytecode confirms ob.b() = 5.0f
and pj extends ob.

The two original JVM captures matched byte-for-byte: 17 cases cover six axis
faces, miss, negative voxel coordinates, corner tie, endpoint boundaries, a
slab, 5.0 reach boundary, and paired native stop-on-liquid values. Each point
coordinate includes its raw IEEE-754 word. Fixture SHA-256:
`f7684fde5588475ff109e122c7dd3e8a7139097cf3eadb7cb817d5cd6c5f1bc7`.
The harness uses native ray/shape methods on private in-memory scenes, checks
class code sources and hashes, and does not access source saves/JAR/assets for
writing. `python3 -m unittest tests.python.test_block_corpus -v` → 3 PASS;
`python3 tools/oracle/block_corpus.py` → repeat exact; py_compile and
`git diff --check` → PASS.

The normal fresh-world constructor invokes spawn setup and cannot initialize
through this isolated headless provider. The oracle instead uses the native
copy constructor with an in-memory provider and explicitly allocated chunks.
Creative break/place are **not verified**: no action output, neighbor snapshot,
or stack change is claimed because a native Player/controller action adapter
was not completed. Do not treat C01 as action parity. Next assigned work is
BLOCK-001-C02; it was not started in this commit. Model: gpt-6-luna, low.

## ITEMS-001-C01 — native in-memory ItemStack/inventory evidence, 2026-11-09

Added `ItemsOracle.java`, `items_corpus.py`, six scenario inputs, the ordered
native output/audit/provenance, and two fixture/protocol tests. The pinned client
JAR SHA-256 is `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
Native mapping: `iz` ItemStack (`iz(int,int,int)`, `iz.a(int)` split, `iz.c()`
limit, `iz.h()` damage); `ix` inventory (`ix(gs)` with null owner, `ix.a(int,iz)`
set, `ix.a(int,int)` split/remove, `ix.c(int)` remove one by item ID,
`ix.a(iz)` insert/merge, `ix.b(int)` hotbar cycle, `ix.b()` selected lookup,
`ix.a(int,boolean)` select by item ID). Exact class hashes and runtime code source
are pinned in `tests/fixtures/items/audit.json`.

The native registry needs `Class.forName("uu")` before first stack item lookup;
otherwise the JAR enters a circular static initialization path. This ordinary
registry bootstrap completes without a World/player/window/save; its native
stdout is captured and hashed separately. `ix(null)` supports all tested slot
operations. No Prism save or mutable world/resource was opened.

Two independent original-JAR captures match byte-for-byte across 21 ordered
states. Every row contains the selected stack and all 36 main slots. Cases show
stone limit 64, snowball limit 16, iron sword limit 1, split of 10 into 4,
native item-ID decrement, slot replacement, merge 50+20 into stacks 64+6,
small-limit insertion 15+3 into 16+2, full-inventory incompatible overflow
preserving its input, and selected-slot behavior at -1/9 plus cycling and
selection by item ID. Native output SHA-256:
`b4c6befa3deaa740f90eaef2e79c8a9a53f5b48eb76fe4cdbc21565604af4916`.

Checks: `python3 tools/oracle/items_corpus.py --write` regenerated only after two
equal native runs; sequential `python3 tools/oracle/items_corpus.py` verified the
committed audit and output hashes; `python3 -m unittest
tests.python.test_items_corpus -v` → 2 PASS; `git diff --check` → PASS. Rust,
runner C02, general item catalog, tagged stacks, GUI and save behavior remain out
of this card. Next step: C02 strict runner, after this C01 commit is reviewed.

## FLIGHT-SMOOTH-001-C03 — принято

- Принята ревизия agy `7ee4df1691586314657340e23e50c3036af8dbe9`.
- `cargo test --locked --offline -p beta-viewer` — PASS (15 library + 6 binary).
- `python3 tools/check.py` — PASS: 182 Python tests (18 opt-in skips), Rust fmt/check/clippy/all-target/doc tests.
- Xvfb smoke запускался, но viewer правильно остановил чтение мира из-за несовпадения pinned `level.dat`. Локальное сохранение не изменялось. Визуальная проверка сглаживания на этой копии мира не подтверждена.
- Изменение ограничено отображением debug-flight: fixed 20 Hz state и controls сохранены, translation интерполируется; native flight parity не заявляется.
- Готовность остаётся около 23%: это качество представления уже имеющегося режима, не новая игровая подсистема.

## ITEMS-001-C02 — strict inventory runner, 2026-11-09

Added `tools/parity/items_run.py`, `items_review.py`, `items` dispatch in
`tools/parity/run.py`, and strict comparator/report mutation tests. Probe protocol
for A02: 21 LF-terminated state rows with six TSV columns:
`scenario, step, result, selected, selected_stack, slots`. Result is `none` or
ordered typed `key=value` fields. Stack tokens are
`id,count,damage,verified_limit`; slots contains exactly 36 tokens separated by
`|`. The C01 native display name alone is normalized away because the bounded
Rust model deliberately has no catalog/name lookup. Every remaining stack value,
selection, result, scenario/step order and row count compares exactly. Native
`index_minus_one` and `index_nine` observations are retained as selected values
-1/9 with empty lookup and unchanged slot state; probe output mirrors these
observations without widening internal selection bounds.

`python3 -m unittest tests.python.test_items_runner tests.python.test_items_corpus -v`
→ 15 PASS. `python3 tools/check.py` → PASS for the full Python suite and locked
offline Rust fmt/check/clippy/all-target/doc checks. The native strict runner
captured the original JAR twice and retained
`artifacts/parity/items/run-gjfal230/report.json`; strict status is correctly
`not_implemented` (strict exit 1) until A02 supplies `items_probe`.
`python3 tools/parity/items_review.py artifacts/parity/items/run-gjfal230/report.json`
→ PASS, six scenarios / 21 rows / four Java processes, output SHA-256
`b4c6befa3deaa740f90eaef2e79c8a9a53f5b48eb76fe4cdbc21565604af4916`.
Tests cover output mutations, order/count/type/limit errors, malformed framing,
slot and selection invariants, missing/wrong JAR, process failure/timeout, and
retained artifact/hash/path tampering. No Rust, fixture, oracle, or user save was
changed. Next: A01/A02 implementation; C03 not started. Model: gpt-6-luna low;
no delegated agent or direct Rust engine edits.

## ITEMS-001-C03 — принято

Точный Agy snapshot `a0b12bbd688baeb18dac1232c786c7a779d89135` fast-forward-объединён
в `codex/verification`; `beta_core::items` и `items_probe` просмотрены. Модель
покрывает 36 main slots/hotbar, native-supported stack limits, split/remove/
replacement/merge/spill и selection; только item IDs 1/3/332/267 с damage 0.
Видимый GUI, armor, crafting, pickup/drop, survival и сохранение не входят.

`python3 tools/check.py` — PASS: 195 Python tests (18 opt-in skips), полный
workspace fmt/check/clippy/all-target/doc checks. На clean merged revision
`a0b12bb` `python3 tools/parity/run.py items --strict` совпал со всеми шестью
сценариями и 21 упорядоченным native состоянием; сохранённый отчёт
`artifacts/parity/items/run-jtwas56h/report.json` прошёл `items_review.py`
(8 процессов, C01 output SHA-256
`b4c6befa3deaa740f90eaef2e79c8a9a53f5b48eb76fe4cdbc21565604af4916`).

На этой ревизии также прошли 12/15 прежних strict групп: rng, rng-extended,
chunk, nbt, math, java, geometry, movement, color, light, fluid, block.
`region --strict` остановился на несовпадении размера `r.-1.-1.mcr`; `world` и
`view` остановились на SHA `level.dat`. Guards отказали до копирования файлов в
артефакты или передачи native adapter/viewer; проверка размера/hash прочитала
только данные, нужные для integrity guard. Файлы не менялись, hashes не обновлялись.
Эти три suites остаются
непроверенными на текущем пользовательском snapshot, хотя ранее принятые
pinned-fixture результаты сохраняются. Inventory oracle/probe сам Prism save
не использует и не записывает.

FLIGHT-SMOOTH-001 также принят: debug flight state/mapping остаются fixed-step,
translation рендерится интерполированно; unit tests и `tools/check.py` прошли.
Xvfb viewer остановил pinned-world guard, поэтому новый визуальный кадр не заявлен.
Процент готовности: около24% (ITEMS-001 добавил примерно1 пункт предметного data
слоя; flight interpolation сама по себе оценку не увеличивает). Push в GitHub
отложен до30%, как указал пользователь.

## HUD-001-C01 — native HUD resource/layout audit

- Исходная ревизия: `49b90ab0ffe2495353b2f1d4deea2cae3231a970`.
  Реализация выполнена только в `codex/verification`, без Rust/viewer изменений.
- Модель исполнителя: gpt-6-luna, low. Координатор независимо указал obfuscated
  class `uq`; далее проверены pinned bytecode, resources и native draw calls.
- `tests/fixtures/hud/audit.json` фиксирует client JAR SHA-256
  `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`, size
  1,465,375; `uq.class` SHA-256
  `b4b61c370315e1ed95b42a24a01112afd7c2d44e387e48adc5e801edd4578f12`.
  Класс obfuscated (`uq extends ub`), его native render method — `a(FZII)V`;
  `javap -p -s -c` подтверждает scaled-resolution construction, draw helper,
  оба asset path и сохранённые offsets/UV operands. В архиве нет usable Java
  source; `GuiIngame.renderGameOverlay` — MCP name, а не декомпилированное имя.
- `gui/gui.png`: 14,210 bytes, SHA-256
  `e223bea0b5004a3b5497607c6d529daf840396526a46eff75c299364cf9dc795`, 256×256.
  `gui/icons.png`: 2,423 bytes, SHA-256
  `8b5a8cfd44b933559c36aecaa722165ad048f1e942f1eeae616554124d6dac43`, 256×256.
- Поддержанные layout values из native `uq.a(FZII)V`: hotbar frame
  `(w/2−91,h−22)`, gui UV `(0,0,182,22)`; selection
  `(w/2−92+20s,h−23)`, UV `(0,22,24,22)`; crosshair
  `(w/2−7,h/2−7)`, icons UV `(0,0,16,16)`. При health 20 рисуется 10
  полных сердец: first `(w/2−91,h−32)`, pitch 8, background UV `(16,0,9,9)`,
  full-heart UV `(52,0,9,9)` при неактивном damage blink. Это layout audit, не
  pixel parity и не симуляция здоровья.
- Файлы: `tools/oracle/hud_audit.py`, `tools/oracle/README.md`,
  `tests/fixtures/hud/audit.json`, `tests/python/test_hud_audit.py`, свой status
  и handoff. Assets, JAR и disassembly остались локальными/в памяти.
- `python3 tools/oracle/hud_audit.py` → PASS; точный JAR/resource/class hash,
  PNG dimensions, method descriptor и bytecode excerpts проверены. Пять тестов
  включают валидный pinned audit, wrong JAR, missing resource, changed layout
  evidence и malformed fixture. `python3 tools/check.py` → PASS: full Python
  suite и locked offline Rust fmt/check/clippy/all-target/doc checks.
  `git diff --check` → PASS.
- Следующий шаг: A01 может реализовать только render-only HUD по исходному
  контракту; C02 проверит код, порядок рисования и возможный visual smoke после
  получения Agy snapshot. Новые Rust изменения этой карточкой не принимались.

## HUD-001-C02 — integration and acceptance

- Принят Agy A01 commit `a84a4c6cfc0ec4b49b94cdf91951566734fec449` и handoff
  `87968d1122029547cd0a5318e1b8afd047b0a69d`; integrated main snapshot
  `53e765b51e5ff017da457a62a7faee8f41a37464`.
- Review подтвердил screen-space HUD без depth, нарисованный после opaque и fluid
  pass; native logical dimensions/ceil division, integer anchors, 182×22 frame,
  24×22 selection, crosshair, ten background/full heart pairs. Никакие item,
  inventory interaction, survival health или damage не заявлены.
- Rust loader проверяет полный JAR SHA-256, извлекает только точные atlas entries
  ограниченным ZIP reader на `flate2`, затем проверяет resource hashes, PNG
  format/dimensions. Текстуры остаются в памяти, новые PNG не добавлены.
- Проверки интеграции: `python3 tools/check.py` → PASS (200 Python tests, 18
  opt-in skips; fmt, workspace check, clippy `-D warnings`, all-target/doc tests);
  `python3 tools/oracle/hud_audit.py` → PASS; `cargo test --locked --offline
  -p beta-viewer --all-targets` → PASS (15 library + 15 binary tests and viewer
  examples); `git diff --check` → PASS.
- Xvfb smoke reaches resource extraction and verifies atlases, then exits at the
  pre-existing integrity guard: `pinned world source hash mismatch: level.dat`.
  No changed Prism world was copied/opened or retagged; no new screenshot or
  pixel-parity claim is made.
- HUD adds roughly one point to graphics/UI: total estimate ≈25%. Push remains
  deferred until the user-requested 30% threshold.

## HOTBAR-INPUT-001-C01 — native input audit, 2026-11-09

Pinned client JAR SHA-256 remains `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c` (1,465,375 bytes). The audit pins class hashes for `net.minecraft.client.Minecraft` (`db99df969e5f30d01ad4424a4d63d3b1f4a11832055dd291e65d476cfa4c38b3`), `ix` (`03eb35aef64c71087942c5dc8b2849e9b47e9482c97be825895cfc2149b8e009`), `kv` (`fd62ebcc36e2f2a1428cb41b2f1f8ebd86fee9855ec1b7f7055ba307d6577102`), and `qb` (`4c1524aeb93b82f91517c5ed5a3dcce02c1b110a9d65b3db7089cfc8a37cd7aa`). MCP mappings are descriptive only; native identifiers/descriptors are authoritative: `Minecraft.k()V` event loop, `ix.b(I)V` inventory scroll, `kv.<init>(Minecraft,File)` default key setup.

`Minecraft.k()V` routes each key-down event through a loop that compares LWJGL event keycodes 2..10 and assigns `ix.c` to 0..8. This selection is hardcoded, independent of configurable GameSettings bindings. The wheel path reads `Mouse.getEventDWheel()`, ignores zero, and invokes `ix.b(delta)` once per nonzero mouse event. `ix.b(I)V` clamps the delta to its sign, subtracts it from the selected index, and wraps to 0..8: positive means previous slot, negative means next. Thus a single +240 event is still one previous-slot step; two separate nonzero events make two steps. Native input is integer event based and provides no line/pixel/fractional wheel semantics; those conversions remain a viewer adapter policy.

Also recorded bytecode defaults from `kv`: W/A/S/D, Space, E, Q, T, F, Left Shift. The JAR and class hashes, native method descriptors, behavior examples, and disassembly excerpts are in `tests/fixtures/hotbar/audit.json`. Repeatable verifier: `tools/oracle/hotbar_audit.py`; mutation tests: `tests/python/test_hotbar_audit.py`. No viewer Rust, native JAR, decompiled sources, or user save changed.

Checks: `python3 tools/oracle/hotbar_audit.py` → PASS; `python3 -m unittest tests.python.test_hotbar_audit -v` → 4 PASS (wrong JAR, mutated behavior/provenance, malformed fixture). Next is HOTBAR-INPUT-001-A01; no C02 work started. Model: gpt-6-luna, low; no delegated subagents or direct Rust implementation.

## INVENTORY-UI-001-C01 — native inventory screen audit, 2026-11-09

Pinned client JAR SHA-256 is `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c` (1,465,375 bytes). `gui/inventory.png` is 2,761 bytes, 256×256, SHA-256 `53dd5bde88588215b1700aee4a7dd70a3049e7e53f914982774ebdc26d6862b0`; `gui/items.png` is 21,381 bytes, 256×256, SHA-256 `545699e0c5f655c1e1876bb9f2fa5d93c4fd514dae3802d1cea43ae0003343f3`. Neither texture is committed.

Native routing is `net.minecraft.client.Minecraft.k()V`: on key-down with no active screen, configured binding `kv.r` (default keycode 18/E) creates `ue(gs)` (`GuiInventory`). While a screen is active, key events are routed to `da.g()`/the screen's key handler; E does not close the inventory, and Escape/keycode 1 closes it through `da.a(CI)V` and invokes mouse recapture. There is no E-repeat toggle path while the GUI is active. This corrects the earlier A01 card wording that said E or Escape closes; the pinned native behavior is Escape-only close.

`id(dw)` (`GuiContainer`) sets the panel dimensions to 176×166. `ue.a(F)V` draws `/gui/inventory.png` at UV (0,0), with origin `((scaled_width-176)/2, (scaled_height-166)/2)` using integer division. Player container `aa.<init>(Lix;Z)V` creates the 27 inventory slots at x offsets 8+18c, y offsets 84+18r (c=0..8, r=0..2), followed by 9 hotbar slots at y=142 and x=8+18c. Slots are 16×16. The full ordered 36 positions and descriptor evidence are frozen in `tests/fixtures/inventory_ui/audit.json`.

The bounded Java oracle initializes only the original static registry (`Class.forName("uu")`) and calls original `iz(int,int,int)`, `iz.a()`, and reflected `gm.bh`; it creates no world, player, window, or save. It reports IDs 1→class `ck` tile 1 `(16,0)`, 3→`ck` tile 2 `(32,0)`, 332→`dm` tile 14 `(224,0)`, and 267→`qd` tile 66 `(32,64)`, each 16×16 in `/gui/items.png`. JAR and all directly relevant obfuscated class hashes, native descriptors, method snippets, and resource hashes are pinned in the fixture. The verifier checks complete expected class/evidence key sets and a fixture evidence digest so changed or omitted claims fail.

Files: `tools/oracle/inventory_ui_audit.py`, `tools/oracle/src/InventoryUiOracle.java`, `tests/fixtures/inventory_ui/audit.json`, `tests/python/test_inventory_ui_audit.py`, this handoff, and own codex task status. No Rust/viewer, shared contract, agy files/status, proprietary asset, or save changed.

Checks: `python3 tools/oracle/inventory_ui_audit.py` → PASS; `python3 -m unittest tests.python.test_inventory_ui_audit -v` → 4 PASS (live native extraction, wrong JAR, altered behavior/provenance/bytecode/slot/icon evidence, malformed fixture); `python3 -m py_compile tools/oracle/inventory_ui_audit.py tests/python/test_inventory_ui_audit.py` → PASS; `git diff --check` → PASS. Full `tools/check.py` was not run for this audit-only card. Next: C02 integration review after A01. Model: gpt-6.1-sol, no delegated agents or borrowed implementation.

## VIEW-LOCAL-001 — refreshed local preview snapshot, 2026-11-09

The previous tracked VIEW/WORLD fixture describes the earlier untouched snapshot;
the user's later gameplay changed `level.dat` and all four region files. At the
user's request, the viewer now reads the ignored, machine-local
`artifacts/user-world-postplay.sha256` baseline. Hash values are deliberately not
recorded in Git. The viewer copies exactly five regular, non-symlink inputs into
a unique ignored artifact run, verifies the source manifest and copied hashes,
then loads only the copy. The strict historical `view_probe` still requires its
old fixture hashes and remains independent.

For fresh verification, `ViewOracle` ran twice against the copied current scene.
The outputs/audits were byte-identical. The native audit retained pinned client,
terrain and `uu`/`cv` class hashes; calls were `ij=26`, `ei=1`, `qj=25`, 25 region
slots, `native_world_or_chunk_loads=0`, and `native_renderer_vertex_calls=0`.
It reported 47 mapping cases and 1,513 neighbor pairs. Relative to the historic
capture, two current pairs were new: dirt face 2 against water ID 9 metadata 6
is visible; water ID 9 metadata 6 face 3 against dirt is hidden. Both were added
to the native registry; no save hash, coordinates or world content was committed.
The separate `view_preview_probe` matched the native output byte-for-byte; output
SHA-256 is retained only in ignored review artifacts.

The graphical viewer loaded the copied 25-chunk scene and built 334,760 opaque
and 9,762 water vertices. GUI initialization failed because Xvfb's `:99` server
was not reachable (`xdpyinfo` failed), so there is no screenshot/pixel claim.
The five original files matched the ignored local baseline after all runs. Full
`python3 tools/check.py` passed: 208 Python tests (18 opt-in skips), fmt, workspace
check, clippy `-D warnings`, all-target and doc tests. `python3 tools/parity/run.py
view --strict` correctly refuses this user save because it remains pinned to the
older historical fixture; the new two-JVM preview comparison is the current-save
evidence.

The coordinator made direct Rust changes in `loader.rs`, the viewer entry point,
and a separate probe example because implementation agents were unavailable in
this turn. This contribution is called out rather than attributed to agy. No
game save, original JAR, atlas, or committed historical native fixture changed.

## Follow-up: shared preview pin and saved inventory, 2026-11-09

The first user launch in `codex/` showed `No such file`: the local pin lived under
`main/artifacts`, while the compiled viewer looked relative to the selected
worktree. The pin is now copied to the ignored shared sibling directory
`/home/andrew/173st/artifacts/user-world-postplay.sha256`; `local_world_manifest_path`
canonicalizes the worktree root before selecting that shared location. The exact
hash contents remain outside Git. A regression test covers sibling-worktree
resolution. Running from `main` now verifies the copy and builds all 25 chunks
(334,760 opaque / 9,762 water vertices); this session then stops at missing
Wayland compositor, rather than the file error. No window/pixel claim is made.

PLAYER-SAVE-001 reads only the copied `WorldMetadata.player` tag. It recognizes
Beta-style `Inventory` compound entries with byte `Slot`/`Count`, short `id` and
`Damage`, and delegates supported ID/count/damage validation to `ItemStack`.
Only main slots 0–35 are shown by the existing E-screen renderer; unsupported or
invalid stacks are skipped and counted. It does not load health, selected slot,
armor, unsupported item types, stack labels, or any mutation/persistence.
Regression tests cover supported stacks, unsupported IDs, missing/wrong list,
and armor slot exclusion. Viewer tests: 32 main tests plus 17 library tests; two
saved-player cases included. Clippy and fmt pass. Full `tools/check.py` passed
before the latest viewer launch.

## HUD render-pass depth compatibility, 2026-11-09

User reported a wgpu validation panic on the first rendered frame: the HUD
pipeline had no depth state, while the shared world render pass binds a
`Depth32Float` attachment. The HUD pipeline now declares a matching
`Depth32Float` state with depth writes disabled and `CompareFunction::Always`,
so screen-space elements overlay the world without depth occlusion. Viewer tests,
fmt and clippy pass after this change. A fresh Xvfb screenshot was unavailable:
the local X server again could not be opened, so GPU frame execution should be
confirmed from the user's compositor. No save or source hash changed.

## WATER-VISUAL-001 — alpha quad ordering, 2026-11-09

The user's shoreline image shows broad, unevenly dark overlapping water regions
and a nearly black rectangular opening. The geometry remains pinned to the
existing native fixture; no water vertices, UVs, colors or face visibility were
changed. Confirmed renderer cause for the overlapping layers: blended quads were
submitted in scene traversal order while depth writes were disabled. The viewer
now sorts each six-vertex water quad far-to-near from the current camera on every
frame, writes the ordered buffer with `COPY_DST`, and preserves the native two-
triangle order within each quad. A unit regression verifies distance ordering
and vertex order. `cargo fmt`, 33 beta-viewer main tests, 17 library tests,
clippy `-D warnings`, and `python3 tools/parity/run.py fluid --strict` pass.

Xvfb could not open its X server, so the screenshot was not visually repeated.
The black rectangular opening remains unclassified; it may be an actual exposed,
unlit part of the saved terrain under transparent water. Do not add a bottom face
without a native mismatch. This candidate is synchronized to codex/agy locally;
hold the visual claim and GitHub push until it is checked in a compositor.

## HOTBAR-ITEMS-001-C01 — native hotbar item renderer audit

- Codex branch synchronized through shared contract `3f44a6f`. The original JAR is pinned at SHA-256 `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`; inspected classes `uq`, `bb`, `sj`, `fp`, `iz`, `gm`, and supported item classes `ck`, `dm`, `qd` have hashes in `tests/fixtures/hotbar_items/audit.json`.
- `uq.a(FZII)V` draws the hotbar slot content through `uq.a(IIIF)V`; that method skips null slots and calls `bb.a(sj,ji,iz,int,int)` for icon then `bb.b(sj,ji,iz,int,int)` for count. In the parent HUD pass this occurs after crosshair and health/armor. The audited slot origin is `width/2 - 90 + slot*20 + 2`, `height - 19` (16x16 icon), for slots 0–8.
- `bb.a(sj,ji,int,int,int,int,int)` branches on item ID `<256`: terrain atlas; otherwise gui/items. Both use the native 16x16 tile UV from stack icon index. Native registry results: ID 1→ck tile 1, ID 3→ck tile 2, ID 332→dm tile 14, ID 267→qd tile 66. Resource and class hashes are checked by the verifier.
- Counts render only for stack size >1. Original `sj` font scan is captured from `font/default.png` plus `font.txt`; native digit widths are 8,8,8,8,9,9,8,9,8,8. Text anchor is `x+17-width`, `y+9`; original draw color is white. Thus count labels are in scope for A01 using the pinned font atlas. This is geometry/resource evidence, not a GL pixel capture.
- `python3 tools/oracle/hotbar_items_audit.py ../.local/minecraft/client-b1.7.3.jar` verifies JAR/resource/class hashes, bytecode excerpts, and two exact JVM oracle outputs. `python3 -m unittest tests.python.test_hotbar_items_audit -v` → 3 PASS. No Rust, Agy status, world, or save files changed.
- Native corpus/audit files: `tools/oracle/hotbar_items_audit.py`, `tools/oracle/src/HotbarItemsOracle.java`, `tests/fixtures/hotbar_items/audit.json`, and `tests/python/test_hotbar_items_audit.py`. A01 can implement item icons and counts using the per-ID atlas distinction and digit width evidence. No claim of rendered pixel parity is made.

## INVENTORY-INTERACTION-001-C01 — native inventory click corpus

The pinned Beta 1.7.3 client is exercised through `dw`/`aa` with an isolated
native `ix` inventory and a synthetic player. No window, world, or save is
created. `InventoryInteractionOracle` captures ordered operations and the full
cursor plus 36-slot state after each operation. The oracle repeats the exact
sequence twice and verifies the raw output against
`tests/fixtures/inventory_interaction/native.tsv`. The 23 ordered states cover
left pickup/place, right rounded split and one-item place, compatible merge
`60+4→64`, incompatible swap, close return through a native player sink, empty
click, and a full target that retains the cursor. Only IDs 1/3/332/267 are used.
Shift-click and world outside-drop remain outside the contract.

Class/JAR and oracle-source hashes are pinned. The evidence also records the
18×18 hitbox and native renderer methods: cursor icon is 16×16 at screen mouse
coordinates adjusted by the GUI origin and `(-8,-8)`; IDs below 256 use
`terrain.png`, others use `gui/items.png`; cursor draw order is icon, count,
tooltip after slot contents, and count appears only above one. No pixel-level GL
claim is made.

`python3 tools/oracle/inventory_interaction_audit.py
../.local/minecraft/client-b1.7.3.jar` → PASS, two native outputs byte-identical
to the committed reference. `python3 -m unittest
tests.python.test_inventory_interaction_audit tests.python.test_inventory_ui_audit
-v` → 7 PASS. No Rust, user world, or save data changed.

## INVENTORY-INTERACTION-001-C02 — acceptance

Accepted on the merged snapshot after reviewing the expanded 23-row native
fixture and Agy cursor interaction implementation. The viewer tests cover
pickup, rounded right-click split, placement, compatible merge, incompatible
swap, native 18×18 hitboxes at scale, closed/focus input gating, full-target
cursor preservation, cursor atlas dispatch and count pass ordering. The native
close operation is a world-drop through a synthetic player sink; the viewer
uses the documented safe in-memory reinsertion adaptation and keeps the screen
open when no capacity exists.

Checks: `python3 tools/oracle/inventory_interaction_audit.py
../.local/minecraft/client-b1.7.3.jar` PASS; focused Python audit/UI tests 7
PASS; `cargo test --locked --offline -p beta-viewer --all-targets` PASS (17
library and 40 viewer tests plus examples); fmt, beta-viewer clippy with
`-D warnings`, and `git diff --check` PASS. An additional fixture-driven Rust
test compares cursor and all 36 slots after each of the 14 native click states;
the native close/world-drop state is excluded because viewer intentionally uses
the safe reinsertion adaptation. `python3 tools/check.py` PASS: 214 Python tests
(18 opt-in skips), workspace fmt/check/clippy, all-target Rust tests, and docs.
No persistence, outside-drop, shift transfer, crafting, armor, item use or
survival behavior is claimed. The viewer was not opened against the user's
modified world; no world files were read or changed for this acceptance.

## HEALTH-001-C02 — acceptance

Accepted Agy `0d9fe94395de68f07e7c8d448f737df75e683ed1` after native audit
`bcabdd6c05a9c7321eae121d93de894f7a51aa88`. Saved `Player.Health` is a signed
Short; the pinned renderer uses background/full/half UV `(16,0)`, `(52,0)`,
`(61,0)` for 10 cells. Viewer displays saved values, clamps to 0..20 and uses 20
as its missing/malformed-data fallback; the native NBT reader uses 10 when the
tag is absent. This fallback difference is explicit.

Native audit and five HUD audit tests PASS. `cargo test --locked --offline
-p beta-viewer --all-targets` PASS (17 library + 44 viewer tests and examples);
viewer clippy `-D warnings` PASS. Full `python3 tools/check.py` PASS: 214 Python
tests (18 opt-in skips), workspace fmt/check/clippy, all-target Rust and doc
tests. No health simulation, save writes or screenshot/pixel-parity claim.

## FLUID-RENDER-001-C02 — acceptance

Accepted Agy implementation `5df8eb6` and overlap-test correction `ceb4e23`.
The viewer keeps packed fluid vertices static between scene rebuilds, sorts
six-index quad groups back-to-front with stable source order on ties, and
preserves the six source vertices within each quad. Candidate scene/fluid caches
and the replacement fluid GPU buffer pair are prepared before publication;
opaque-chunk GPU transactional behavior is outside this card.

Independent review confirmed per-frame fluid vertex upload is zero and each
quad uploads six `u32` indices (24 bytes). The fixture includes intersecting
projected quads with overlapping depth ranges `[4,6]` and `[4.3,5.3]`, distinct
center distances, and source-order assertions. Checked byte/count conversions
and buffer bounds are covered. `cargo test --locked --offline -p beta-viewer`
passed on merged main (17 library + 49 viewer + doc tests); Agy recorded passing
clippy, fmt, `git diff --check`, and full `python3 tools/check.py` (214 Python
tests, 18 opt-in skips, workspace Rust checks).

For the previously observed 9,768-vertex scene, the calculated dynamic payload
is 39,072 index bytes/frame versus 312,576 prior repacked vertex bytes (8x by
payload count only). No FPS or percentile claim is made. Xvfb smoke was
inconclusive: timeout after launch, black capture, and a separate X server
listener/ownership failure. No successful visual check is claimed. Overall
game estimate moves from about 33% to **34%**; GitHub push remains deferred
until the next 5–10 percentage-point batch.

## LIGHT-UPDATE-001-C01 — native block edit light oracle (accepted)

The synthetic-only oracle now drives the original `fd.a(x,y,z,id,meta)` edit,
its bounded `fd.a(eb,bounds)` queue, and `fd.j()` drain. Five scenarios record
complete bounded initial/final blocks, metadata, opacity/emission inputs,
height-map exceptions and raw block/sky nibbles using explicit domain defaults
plus sorted exception rows. They include stone removal/placement, an opaque roof
at the sky boundary, and the loaded x=31/unloaded x=32 edge. Unloaded cells are
sentinels, never interpreted as air. Native `lm.c()` initialization's sky
profile (`0` at y=0, `15` at y=1..127) is encoded in each domain. Query ordering
and combined light are captured; no callback order is inferred.

The manifest pins the original client JAR, `fd/lm/wi/uu/eb/st` classes, Java
source, inputs and output. `python3 tools/oracle/light_update_audit.py --jar
../.local/minecraft/client-b1.7.3.jar` passes two fresh native captures (2,322
rows; output SHA-256 `effa63e2c14cb76962b1023a07c60f4e9fcf6e55f4db62695f78b7013e5d02cb`).
The six focused Python tests pass, including wrong-JAR and malformed/mutated
evidence rejection. An independent review reconstructed each full domain and
matched all five initial digests.

Frozen A01 API (concrete Rust signature):

```rust
pub fn relight<F>(
    &mut self,
    changed: (i32, i32, i32),
    max_visited_cells: usize,
    material: F,
) -> Result<RelightResult, RelightError>
where
    F: Fn(u8, u8) -> (u8, u8), // (native opacity, native emission)
```

`MAX_RELIGHT_CELLS` is exactly `3 * 3 * 128 * 256 = 294_912`, the maximum
number of cells in C01's nine loaded chunks. `max_visited_cells` must be in
`1..=MAX_RELIGHT_CELLS`; it counts distinct loaded cells dequeued by the
deterministic update work queue. A smaller caller limit that is exhausted
returns `RelightError::QueueLimitExceeded` and leaves `self` byte-for-byte
unchanged; limits are never silently clamped. Invalid limits, an unloaded edit
cell, invalid material properties, or an internal chunk access failure also
return an error without publishing partial light changes. Material opacity uses
the native unsigned-byte domain `0..=255` (native stone ID 1 has opacity 255);
emission is restricted to `0..=15`. Native opacity zero is treated as
attenuation one by the update routine, while opacity `>=15` blocks propagated
light unless an emission source overrides it.

`RelightResult` contains sorted `changed_cells` entries with world `(x,y,z)` and
the final block/sky nibbles, a sorted/deduplicated `dirty_chunks` list, and a
sorted/deduplicated `unloaded_neighbors` list of world cell coordinates. Queue
expansion uses only the six cardinal cells; unloaded cells are reported and
never read as air or mutated. Viewer invokes this API on its disposable edited
candidate with `MAX_RELIGHT_CELLS`, then publishes block state, duplicate chunk
data, light, meshes and fluid caches in one transaction. Emission transitions,
fluid ticks, diagonal propagation shortcuts, chunk loading, persistence and
general world simulation remain excluded.

## WORLD-SELECT-001-C02 — review implementation notes (pending A02 integration)

Native menu audit and Python audit test pass. Review found that the A02 snapshot
on `agy/development` still exposed rows only in the titlebar, so this verification
worktree now carries a direct Codex implementation of the visible two-screen
flow: main menu with Singleplayer, a rendered list with disabled-row reasons,
mouse hit-testing aligned with rendered rows/buttons, keyboard navigation,
cancel-to-parent, and explicit activation without preselecting a world. Dynamic
selected-world loading remains guarded by the existing disposable snapshot API.

`cargo test --locked --offline -p beta-viewer --all-targets` passes (24 library,
51 viewer, and example tests); `cargo test --locked --offline -p beta-core --test
light_update_fixture` passes four tests; `python3 tools/oracle/world_select_audit.py`
and its unittest pass; full `python3 tools/check.py` passes (222 Python tests,
18 opt-in skips, workspace fmt/check/clippy/all-target/doc tests). The local
user-world manifest matches all six source files by size and SHA-256. An Xvfb
launch could not be visually confirmed because the final wrapper failed to open
its X server. No world was selected, copied, or modified.

This is a verification-branch review fix and is not yet integrated into
`agy/development`; that branch's A02 status remains `in_progress` at `05e8aec`.
Therefore C02 remains pending, the completion estimate stays at about 35%, and
no main/GitHub integration is claimed.

## User follow-up 2026-11-09 — shadow report and final native fidelity

Documentation-only request: retained the user's unmodified Rust-viewer PNG at
`docs/bug-reports/light-update-002-after-break.png` (SHA-256
`64fff96adb875c64df1198c94b8fe541b9da6084ed4129d93ff6661e183ceaa9`).
Added planned `LIGHT-UPDATE-002` with a native/Rust reproduction, light-state
capture and visual regression boundary; cause and fix remain unconfirmed.
Added planned `MENU-PARITY-001` for the temporary menu's font/resources/layout/
scaling/input deviations. Updated requirements, known issues, roadmap and
progress; corrected the obsolete claim that no edit relighting exists.
Both follow-ups remain backlog entries, with no active queue/status changes.
Validation: `git diff --check`, new document links and PNG hash checked.
No engine changes or gameplay tests; estimate remains about 35%.

## Parallel follow-up — WORLD-SELECT accepted; material light corrections

The user's new permission allows three independent workers. They ran on
`gpt-6-luna`/low after disclosure that the requested 5.6 model was unavailable.
World-selection implementation used `agy/development`; shadow/menu native
audits used their own assigned worktrees. No nested delegation was used.

Reviewed Agy `744898d` and safety amendment `6876aa5` (integrated as `5e0251b`):
catalog sorting, no auto-selection, strict pin/folder names, symlink directories,
changed-source hashes, fail-closed reads, disposable snapshots and dynamic
spawn-centered5x5 loading. Coordinator `tools/viewer_smoke.py` verified actual
Xvfb main/list/cancel/quit and selected-world-to-game paths; screenshots were
inspected and original six source files retained their manifest hashes. Xvfb's
socket needed the approved sandbox escalation; earlier failed attempts are not
counted as successful captures. WORLD-SELECT-C02 is accepted functionally;
original menu appearance and exact client input behavior remain a separate task.

Native material evidence from worker commits `a2941b3`, `7c4fa58`, `0a93e87`
pins all256 original registry entries and8159 rows of complete synthetic
pre-edit state/height exceptions plus samples. Coordinator's direct engine
contribution `1c83d2c` adds native opacity lookup, uses it in the viewer, and
requires the current cell to have opacity0 before assigning direct skylight.
The native water-place regression failed with15 vs12 before that second fix;
all19 sampled after-light states in three scenes now match, alongside the old
four light-update tests. The exact reported user scene, general native queue,
height-map propagation, and leaf metadata8 notifications are not accepted.
LIGHT-UPDATE-002 therefore remains open; no percentage is added for these fixes.

MENU-PARITY native numeric/resource evidence was integrated from `8c547cf`;
native frame capture is tracked separately in its handoff. No font live-call
claim is made for the derived metrics. The task board also rejects duplicate
JSON keys after a merged duplicate had silently reverted WORLD-SELECT-C01 to
pending, and now permits up to three active cards per role. Nine queue tests
pass, including the real duplicate-status regression and parallel activity.

Final combined workspace check:234 Python tests,18 opt-in skips, fmt/check/clippy,
Rust all-target and doc tests PASS; native material/menu/world-selector audits
also pass in their stated scopes. Read-only final viewer smoke and build pass.
WORLD-SELECT adds about1 interface point, giving **≈36%**; GitHub push remains
deferred to the next5–10 point batch (around40%).

## Full-domain light diagnostics

Coordinator added `crates/beta-core/examples/light_snapshot_probe.rs`, a bounded
stdin diagnostic over the independent V2 corpus from worker `f0dcc3e`
(integrated `8d010b7`). The native fixture contains 44806 rows /2769192 bytes;
SHA-256 `d12c5ba682813944647dbe3ac74e7d61235684d122cf40e8d9bc5440b78d8222`.
Each step starts from the preceding native snapshot and compares all294912
cells, separately counting light, ID and metadata differences. It does not
compare Rust height maps, GPU frames or accumulated Rust state across steps.

Command: `cargo run --quiet --locked --offline -p beta-core --example
light_snapshot_probe < tests/fixtures/light_material/native_edits_v2.tsv`.
The actual run exited1, with reports in
`artifacts/light-full-state-comparison.tsv` and `.log`:

| Case/phase | Light mismatches | ID | Metadata |
| --- | ---: | ---: | ---: |
| canopy place | 0 | 0 | 0 |
| canopy remove | 0 | 0 | 12 |
| water place/remove (each) | 0 | 0 | 0 |
| leaf place | 2107 | 0 | 0 |
| leaf remove | 0 | 0 | 0 |
| glowstone edge place | 6195 | 0 | 0 |
| glowstone edge remove | 3852 | 0 | 0 |

This confirms remaining propagation defects beyond the accepted probes;
it is explicitly a failing diagnostic, not a passed parity suite. Some native
glowstone differences also affect y0 beyond the submitted local scan. The
reported user camera/edit sequence is still unavailable, so no exact screenshot
fix is claimed. No readiness point is added for this investigation.
