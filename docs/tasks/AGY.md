# Карточки роли agy

## WORLD-LOAD-001-A01 — pure read-only world metadata/chunk API

После C01 и получения committed native evidence прочитать
[WORLD-LOAD-001](WORLD-LOAD-001.md). Реализовать beta_core::world по API/типам/
лимитам/политике ошибок контракта. Использовать NBT/ChunkData/Region без новых
dependencies, JVM или файловой системы в core. Сохранять root и original raw NBT,
включая неизвестные Player/entity/tile fields; не запускать симуляцию и не
восстанавливать повреждённые массивы. Допустимо pub(crate) существующего bounded
gzip decoder. Файлы src/ и обычные Rust tests, свой status/handoff. Проверить
native defaults, malformed tags/lengths/coords, bytes/signs, preservation и limits;
обычные проверки, затем commit A01 и продолжить A02.

## WORLD-LOAD-001-A02 — headless world probe

После A01 создать examples/world_probe.rs: thin filesystem adapter только marked
copies, пять-field stdin, exact 21/17-field stdout и original retained NBT bytes
по контракту. Чанки обрабатывать последовательно, без накопления мира в памяти;
исключить paths/symlinks/overlap/overwrite. Файлы examples/, необходимый src/,
свой status/handoff. Проверить реальные subprocess stdout/stderr/exit, synthetic
valid/invalid preservation/limits, обычные проверки; commit A02 и передать root.
Oracle, runner, Cargo dependencies и shared contract не менять. Исправления
по concrete C03 feedback входят в эту же роль без нового разрешения пользователя.

Выбирать задачу через `python3 tools/tasks.py next agy` по [TASKS.md](../../TASKS.md).
Архивный протокол: [RNG-001](RNG-001.md). Новые: [RNG-002](RNG-002.md),
[CHUNK-001](CHUNK-001.md), [NBT-001](NBT-001.md), [REGION-001](REGION-001.md).
В каждой карточке изменять только свой
`docs/tasks/status/agy.json` и `docs/handoffs/agy.md` вместе с разрешённым Rust-кодом.

## RNG-001-A01 — Rust RNG

Зависимостей нет. Разрешены `crates/beta-core/src/` и собственные status/handoff.

Реализовать `beta_core::java_random::JavaRandom::new(seed: i64)` и
`next_int(&mut self) -> i32` по поведению Java. Подключить модуль в `lib.rs`.
Сохранять начальное преобразование seed, ширину состояния и определённое переполнение.
Другие методы RNG, новые зависимости и графику в этой задаче не добавлять.

Приёмка:

- Реальная вычисляемая последовательность, без списков ответов и вызова Java из Rust.
- Корректно обрабатываются seed `0`, `1`, `-1`, `42`, минимальный и максимальный `i64`.
- Выполнены подходящие обычные проверки и `python3 tools/check.py`.
- Полное соответствие Java пока не объявляется: независимый runner создаёт Codex.
- Результаты записаны в своём статусе и handoff; сделан коммит.

После коммита самостоятельно перейти к A02.

## RNG-001-A02 — Rust probe

Зависит от A01. Разрешены `crates/beta-core/examples/rng_probe.rs`, необходимые
обычные тесты своего probe внутри crate, собственные status/handoff.
Java-эталон, runner, fixture и общий контракт не менять.

Реализовать stdin/stdout TSV-интерфейс RNG-001. Probe использует библиотечный
JavaRandom, проверяет seed/count/ID и ошибки, пишет диагностику только в stderr.

Приёмка:

- `cargo run --locked --offline -p beta-core --example rng_probe` принимает fixture
  и выдаёт 96 строк в заданном формате с LF.
- Неверный ввод даёт ненулевой код; повтор ID и недопустимый count отвергаются.
- Два одинаковых запуска воспроизводимы; минимальный/максимальный seed принимаются.
- `python3 tools/check.py` проходит; выполненные команды и результаты записаны.
- Сделан коммит со статусом A02 `done`. Codex обнаружит его из Git автоматически.

После RNG-001-A02 своей доступной работы в исходном пакете больше нет. Завершить локальную
сессию и сообщить коммит. Codex самостоятельно увидит готовность и выполнит C03.
Сравнение через runner выполнять только на ревизии, в которой реально есть обе
части; отсутствие runner в своей ветке не обходить чтением незакоммиченного кода соседа.


## RNG-002-A01 — Rust API

После RNG-001-C03 и обновления своей ветки до принятого main. Прочитать
[RNG-002](RNG-002.md) целиком. Разрешены `crates/beta-core/src/` и обычные тесты
внутри crate, свой status/handoff. Реализовать точно указанный API и инварианты,
без новых зависимостей, Java-вызовов из Rust и зашитых таблиц ответов.
Прежние API и RNG-001 сохраняются. Для RNG-002 проверить общее состояние,
rejection sampling, signed long, reseed и bound-ошибку без изменения состояния.
Для CHUNK-001 проверить nibble-соседей, длины, координаты и отсутствие частичной
записи при ошибке. Выполнить `python3 tools/check.py` и
`cargo test --all-targets --locked --offline`, записать фактические результаты.
Сделать коммит с A01 done и без нового запроса перейти к A02.

## RNG-002-A02 — Rust probe

После RNG-002-A01. Разрешены соответствующий новый example в
`crates/beta-core/examples/`, необходимые обычные тесты probe, свой status/handoff.
Реализовать протокол [RNG-002](RNG-002.md), используя библиотечный API;
Java-адаптеры, fixture, runner, зависимости и контракты не изменять.
Проверить валидные цепочки, границы, неверный ввод, лимиты и LF/stdout/stderr.
Выполнить обычные проверки; live-сравнение только если обе стороны реально есть
в своей ветке. При необходимости получить закоммиченный C01/C02 Codex через
обычный merge его фиксированного SHA в свою ветку при чистом worktree.
Сделать коммит с A02 done; следить за приёмкой Codex по TASKS.md.
Исправления внутри контракта выполнять через переоткрытие своей карточки, не ждать,
пока пользователь вручную скопирует запрос. После приёмки выбирать следующий пакет;
если интегратор ещё работает, действовать по ограниченному ожиданию TASKS.md.


## CHUNK-001-A01 — Rust API

После RNG-002-C03 и обновления своей ветки до принятого main. Прочитать
[CHUNK-001](CHUNK-001.md) целиком. Разрешены `crates/beta-core/src/` и обычные тесты
внутри crate, свой status/handoff. Реализовать точно указанный API и инварианты,
без новых зависимостей, Java-вызовов из Rust и зашитых таблиц ответов.
Прежние API и RNG-001 сохраняются. Для RNG-002 проверить общее состояние,
rejection sampling, signed long, reseed и bound-ошибку без изменения состояния.
Для CHUNK-001 проверить nibble-соседей, длины, координаты и отсутствие частичной
записи при ошибке. Выполнить `python3 tools/check.py` и
`cargo test --all-targets --locked --offline`, записать фактические результаты.
Сделать коммит с A01 done и без нового запроса перейти к A02.

## CHUNK-001-A02 — Rust probe

После CHUNK-001-A01. Разрешены соответствующий новый example в
`crates/beta-core/examples/`, необходимые обычные тесты probe, свой status/handoff.
Реализовать протокол [CHUNK-001](CHUNK-001.md), используя библиотечный API;
Java-адаптеры, fixture, runner, зависимости и контракты не изменять.
Проверить валидные цепочки, границы, неверный ввод, лимиты и LF/stdout/stderr.
Выполнить обычные проверки; live-сравнение только если обе стороны реально есть
в своей ветке. При необходимости получить закоммиченный C01/C02 Codex через
обычный merge его фиксированного SHA в свою ветку при чистом worktree.
Сделать коммит с A02 done; следить за приёмкой Codex по TASKS.md.
Исправления внутри контракта выполнять через переоткрытие своей карточки, не ждать,
пока пользователь вручную скопирует запрос. После приёмки выбирать следующий пакет;
если интегратор ещё работает, действовать по ограниченному ожиданию TASKS.md.
## NBT-001-A01 — Rust NBT reader/writer

После CHUNK-001-C03 прочитать [NBT-001](NBT-001.md) целиком. Разрешены
`crates/beta-core/src/` и обычные Rust-тесты. Реализовать safe representation,
big-endian tags 0–11, modified UTF-8, limits и атомарные ошибки без Java-вызовов,
зашитых ответов или новых crates. Проверить все типы, empty lists/compounds,
NaN bits, malformed input, depth/size limits и deterministic writer. Выполнить
`python3 tools/check.py` и `cargo test --all-targets --locked --offline`, затем
закоммитить A01 и перейти к A02.

## NBT-001-A02 — Rust NBT probe

После A01 реализовать `nbt_probe` по контракту NBT-001 в новом example, используя
только библиотечный API. Java oracle, fixture, runner и общий контракт не менять.
Диагностика только stderr, LF/stdout строго по протоколу; проверить invalid cases,
лимиты и round-trip. Сделать commit A02 done и ждать/читать C03 через Git. Ошибки
внутри этого контракта исправлять самостоятельно по циклу TASKS.md.

## REGION-001-A01 — read-only McRegion API

После REGION-001-C01 прочитать [REGION-001](REGION-001.md) целиком; начать только
после того, как контракт зафиксировал точный `flate2`/backend в `Cargo.lock`.
Разрешены `crates/beta-core/src/`, `Cargo.toml`, `Cargo.lock` по согласованному
решению и тесты crate. Реализовать borrowed `RegionFile`, проверку заголовка,
location/timestamp, отрицательные region coordinates, чтение и bounded decompression
через выбранный API. Запись региона и любые правки исходного мира запрещены.
Проверить malformed headers, overlapping sectors, unsupported compression,
decompression errors/bombs и NBT payload. Запустить `tools/check.py` и
`cargo test --all-targets --locked --offline`; если dependency отсутствует offline,
зафиксировать это как blocked dependency provisioning, не подменять проверку.
Закоммитить A01 и перейти к A02.

## REGION-001-A02 — region probe

После A01 создать `region_probe` и TSV-протокол контракта. Файлы открывать только
из per-run copied input directory; не принимать абсолютные пути или `..`.
Сопоставить полный список слотов, включая absent; сохранять timestamp/compression,
length/SHA decompressed NBT. Игровой мир, runner и original Java adapter не менять.
Проверить stdin/stdout/stderr/exit status и повторяемость. Сделать коммит A02 done;
читать Codex C03/handoff по Git и самостоятельно выполнять исправления внутри
контракта по циклу TASKS.md.
