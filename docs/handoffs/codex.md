# Отчёт Codex

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
- Следующий шаг: перечитать очередь; C03 после закоммиченного A02, ожидание по TASKS.md.

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

В `docs/antigravity.md` записаны официальные настройки CLI accept-edits,
отдельные политики команд/артефактов, варианты App и ссылки. Настройки пользователя
не изменялись. Модель текущей сессии Codex; точный model ID/токены/стоимость
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
  ожидание по TASKS.md. Дополнительные агенты не запускались.

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

Локальный Prism мир `/home/andrew/.local/share/PrismLauncher/instances/b1.7.3/minecraft/saves/New World`
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

- Работа выполнена только в `/home/andrew/173st/runner`, ветка `agents/region-runner`, после fast-forward `main` до `bf18c34f7cd119558f2c84ad8be64801032543ea`. Код Java и corpus зафиксирован snapshot-коммитом `02b03a27bded82dfffc8be28ba40d80a88d3c014`; reviewfixes/status/handoff — отдельным последующим коммитом.
- Добавлены `tools/oracle/src/WorldOracle.java`, `tools/oracle/world_corpus.py`, пять-file manifest и markers, 749-row coordinates-only fixture, provenance/native audit/generated audit и четыре Python tests. Oracle вызывает `ij.b(DataInput)`, `ei(nu)`, `qj.a(int,int)` оригинального JAR. Он не создаёт fd/World, lm/Chunk, entity или tile-entity objects; audit фиксирует `native_chunk_loads=0`.
- `python3 tools/oracle/world_corpus.py --write` → PASS: 1 level + 748 chunk cases; 704 occupied и 44 absent. Точные live calls: qj=748, ij=705 (level + 704 present), ei=1, native_chunk_loads=0. Два полного запуска имеют одинаковый output/audit SHA `f2e08c13997bb6fab03170dfbcec1fb3bd619e9079b792cf324fd853bd12fcc2`.
- Отдельный generated self-test: 10 labeled cases, native calls ij=11/ei=7/qj=0. Проверены defaults, отрицательные/ненулевые boolean bytes, пустой/отсутствующий Player, границы i32/i64, wrong native types, unknown Unicode/empty names, signed raw arrays, unknown entity/tile list payloads без factories, empty byte-type entity list и strict missing/type/length/coordinate/list errors.
- JAR SHA-256 `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`; class hashes `ij=9db862e2ee83e95a2db7a8495594d2c0e0bbe904d96c3f09f167f2b47b72b8d0`, `nu=a1a5701ec2180d01ddc604cc8c2e5aff1176b10faf83bdb64d2f1cc8c806ac4a`, `ei=5d01d1dda7be346c57fd00846d2425615186c3ef1c65c06aa0bd0f45aa9f7fef`, `qj=6c75a123b63d555caa98e1a2619aa2483e405566de9e356585b1dc682a7da4a6`. Source and copied hashes for level.dat and all four regions matched before/after. Latest run artifacts: `artifacts/oracle-world/run-lzwoftww/`; raw NBT and class outputs remain ignored.
- Checks: `python3 -m unittest discover -s tests/python -p test_world_corpus.py -v` → 4 PASS; `python3 tools/check.py` → 64 Python + 58 Rust all-targets, fmt/check/clippy/doc PASS; `git diff --check` PASS. A no-write corpus run left tracked audit/generated fixtures unchanged.
- C01 establishes native extraction/default evidence only; Rust parity and report runner belong to later cards. Next: coordinator review/integration; C02/A01 may proceed from the contract and this snapshot.

## WORLD-LOAD-001-C02 — WIP checkpoint (not accepted)

- Назначенная модель: `gpt-6-luna`, reasoning `low`; без делегирования. Worktree `/home/andrew/173st/world-runner`, ветка `agents/world-runner`.
- Начальный C01 snapshot `374f6e6` получен fast-forward. Разрешённый C01 post-review `88b510d0fc37304d4b26ef50b2af5ea8b6f567f1` не был потомком merge HEAD, поэтому включён отдельным `--no-ff` merge; добавлены лимиты входных файлов Oracle.
- WIP файлы: `tools/parity/run.py`, `tools/parity/world_run.py`, `tools/parity/verify_report.py`, `tools/parity/world_review.py`, `docs/tasks/status/codex.json`. Раннер содержит dispatch, координатный TSV parser/comparator, hash-verified copy flow, native JAR/class/call audit, Java запуск и retained raw byte binding, Rust build/run путь и сохранённый verifier/protocol/mutation scaffolding.
- Проверки только быстрые: `git diff --check` → PASS; `python3 -m py_compile tools/parity/world_run.py tools/parity/world_review.py tools/parity/run.py tools/parity/verify_report.py` → PASS. Live Java/Rust run, Python unit suite, saved-report/protocol review, status queue check и общий check.py **не выполнялись**. C02 не принят и status оставлен `in_progress`.
- Следующий шаг: reviewer должен внимательно отладить Python API/путь subprocess и verifier, добавить `tests/python` coverage (включая `-0`, лишние строки, missing/error/timeout и actual retained-byte tampering), затем дождаться зафиксированного Rust SHA от координатора перед live parity. После unit checks собрать runner; при отсутствии probe только Java 749-row run со strict `not_implemented`, без false pass. Не запускать world edges: их готовит координатор.
