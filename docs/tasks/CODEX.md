# Карточки роли codex

## JAVA-001-C01 — Java oracle и conversion/hash corpus

После MATH-001-C03 прочитать [JAVA-001](JAVA-001.md). Создать JavaOracle с
прямыми cast expressions и `String.hashCode()`, deterministic raw-bit/Unicode
корпус, независимый повтор двух oracle runs и audit JVM/source/input/output.
Зафиксировать только ожидаемые TSV fixtures, без JAR/source от игры. Добавить
тесты генератора и commit C01; после него открыть C02/A01.

## JAVA-001-C02 — parity runner и evidence review

Добавить `java` suite в parity runner: strict ordered decimal comparator,
SHA-bound saved report, повтор Java output, timeout/process failure, malformed
protocol и синтетические type/order/row/hash mutations. Runner сверяет pinned
fixture и source snapshot. Сохранить проверки и коммит C02.

## JAVA-001-C03 — интеграция и приёмка

После C01/C02/A01/A02 проверить точный agy SHA из очереди; выполнить полный
workspace check, `java --strict`, saved protocol review и все принятые strict
suites. Исправить concrete расхождения внутри контракта; при успехе принять в
main, обновить прогресс и estimate, синхронизировать свободные worktrees.

## MATH-001-C01 — original MathHelper oracle и корпус

После VIEW-001-C03 и clean main прочитать [MATH-001](MATH-001.md). Проверить
bytecode и хеш оригинального класса `in`; реализовать Java oracle, который
рефлексивно вызывает именно закреплённые методы клиентского JAR, не копирует их
формулы. Создать детерминированный corpus по всем операциям и edge cases контракта,
зафиксировать Java outputs/audit как fixture. Отдельно доказать покрытие LUT
индексов и повторяемость двух запусков. Добавить тесты генератора, не менять Rust.
Закоммитить C01 со статусом и handoff, затем продолжить C02.

## MATH-001-C02 — parity runner и evidence verifier

После C01 реализовать группу `math` в существующем runner: exact typed TSV,
raw IEEE bits, process errors/timeouts и строгий статус при отсутствии probe.
Добавить saved report/protocol review и synthetic mutations для типов, битов,
порядка, числа строк и источников. Выполнить проверки по MATH-001, сохранить
результаты и коммит; после этого ожидать A01/A02 через очередь для C03.

## MATH-001-C03 — интеграция и приёмка

После C01/C02/A01/A02 взять точный agy SHA из очереди, проверить diff и
интегрировать его. Выполнить `tools/check.py`, все предыдущие strict suites,
`math --strict` и saved protocol review. При расхождении дать конкретный repro,
не изменяя oracle ради Rust. При успехе обновить progress/roadmap/estimate,
принять пакет в main и синхронизировать idle worktrees.

## WORLD-LOAD-001-C01 — native metadata oracle и snapshot

После REGION-001-C03 прочитать [WORLD-LOAD-001](WORLD-LOAD-001.md). Зафиксировать
оригинальный ei(nu), nu/ij/qj, class/JAR hashes и фактические defaults/type errors.
Создать WorldOracle с реальными native metadata constructor/NBT/region calls;
полноценный World/chunk/entity factories не запускать. Подготовить пять-file
snapshot helper с pinned level.dat/region hashes и markers, 749-row live corpus
и отдельно synthetic valid/invalid cases. Сохранить raw retained payloads только
в artifacts, native audit и повторяемость. До A01 согласовать изменения контракта
с координатором. Файлы: tools/oracle/, tests/fixtures/world/, tests/python/,
свой status/handoff. Сделать коммит и передать координатору для открытия A01/C02.

## WORLD-LOAD-001-C02 — world runner и report review

После C01 добавить группу `world`, exact typed TSV/array hashes/raw retained bytes,
пять pinned sources и две независимые input copies. Native metadata constructor,
NBT field extraction и полноценный World load явно различать в audit/report.
Создать saved verifier/protocol checks, mutations для scalar/array/payload/audit,
missing process/JAR/probe/hash, timeout, paths и limits. Не менять Rust/контракт
или oracle ради pass. Файлы: tools/parity/, tests/python/, свой status/handoff.
Обычные проверки и live при наличии probe, commit с фактическими результатами.

## WORLD-LOAD-001-C03 — интеграция и приёмка

Координатор берёт точный agy SHA, reviews diff и объединяет с C01/C02. Проверяет
все критерии контракта: check.py/all-targets, прежние пять strict live suites,
world strict 749 rows, saved protocol/edge/mutation/error paths и неизменность
original sources/copies. При дефекте возвращает конкретный repro исполнителю,
не ослабляя эталон. При успехе обновляет progress/roadmap/контракт/handoff,
процент готовности всей игры, принимает пакет в main и синхронизирует idle ветки.
Очередь/world capabilities не расширять за пределы принятого контракта.

Выбирать задачу через `python3 tools/tasks.py next codex` по [машиночитаемой очереди](queue.json).
Архив: [RNG-001](RNG-001.md). Новые контракты: [RNG-002](RNG-002.md),
[CHUNK-001](CHUNK-001.md), [NBT-001](NBT-001.md), [REGION-001](REGION-001.md);
правила проверок — [parity-testing](../parity-testing.md).
В каждой карточке изменять свой status JSON и `docs/handoffs/codex.md`.
Отметка `done` появляется в коммите вместе с кодом и фактическими результатами.

## RNG-001-C01 — Java-эталон

Зависимостей нет. Разрешены `tools/oracle/`, собственный статус и handoff.

Сделать `tools/oracle/src/RandomOracle.java`, вызывающий настоящий `java.util.Random`.
Прочитать TSV из stdin, проверить вход по общему контракту, для каждого случая создать
новый RNG и выдать последовательность `nextInt()` в согласованном формате.
Не реализовывать формулу LCG вручную и не писать Rust-код.

Приёмка:

- `javac` успешно компилирует адаптер в свой `artifacts/`.
- Базовый fixture выдаёт 96 записей без лишнего stdout; два запуска с одними входами
  дают одинаковые выходы. Это проверка адаптера, не совпадения с Rust.
- Неверные seed/count/ID/повтор ID дают ненулевой код и stderr; границы `i64` принимаются.
- В статусе записаны точные выполненные команды и результаты; сделан коммит.

После коммита самостоятельно перейти к C02.

## RNG-001-C02 — Runner и компаратор

Зависит от C01. Разрешены `tools/parity/`, `tests/python/test_rng_runner.py`,
собственный статус и handoff. Исходный fixture и Rust-реализацию не менять.

Реализовать `python3 tools/parity/run.py rng --strict`: подготовку обоих процессов,
подачу одного сохранённого входа, тайм-ауты, точное сравнение, логи и JSON-отчёт.
Соблюдать метаданные и статусы из `docs/parity-testing.md`.

Приёмка:

- Тесты компаратора обнаруживают замену значения, пропущенную/лишнюю запись,
  нарушение порядка и отсутствие результата; синтетические данные тестов помечены.
- Ошибка компиляции/процесса и timeout не превращаются в `pass`.
- Неизвестная группа ошибочна; отсутствующий Rust-probe даёт явную ошибку или
  `not_implemented` с ненулевым strict-кодом. Не выдавать такой запуск за успех.
- Java-эталон действительно запускается. Полное Rust↔Java возможно после C03.
- `python3 tools/check.py` проходит. Результаты записаны и закоммичены.

После коммита перечитать очередь. Если agy ещё не готов, соблюдать ожидание из workflow.

## RNG-001-C03 — Интеграция и приёмка

Зависит от C01, C02, A01 и A02 в закоммиченных статусах.
Разрешены интеграционные merge в своём worktree, необходимые исправления инфраструктуры
проверки, свой status/handoff, `docs/progress.md`, статус общего RNG-001.
Ошибки Rust-реализации по умолчанию возвращать agy, не скрывать помощь Codex.

Порядок:

1. Из вывода `tasks.py next codex` сохранить точный `snapshots.agy.commit`.
2. Прочитать handoff и diff этого SHA через Git; проверить отсутствие изменений вне роли.
3. При чистом своём worktree интегрировать **этот SHA** в `codex/verification`.
4. Выполнить `python3 tools/check.py` и `python3 tools/parity/run.py rng --strict`.
5. При расхождении записать `blocked` с входом, SHA, ожидаемым/фактическим результатом.
   Создать конкретный запрос исправления для agy в своём handoff. Очередь и чужие
   статусы не менять незаметно; исправление оформляется отдельной согласованной карточкой.
6. При успехе обновить общий прогресс и RNG-001, записать статус C03, сделать коммит,
   интегрировать принятую ветку в `main` и вернуться в свою ветку.
   Если интеграция в main не удалась, явно вернуть C03 в `blocked` и зафиксировать причину.

Приёмка: все 96 реальных значений совпали, оба процесса успешны, отчёт и область
покрытия сохранены, защита компаратора от намеренной ошибки проверена.
В отчёте указаны реально проверенные ревизии; RNG-001 принят в `main`.
После RNG-001-C03 исходный пакет завершён. Новые пакеты перечислены ниже;
работу вне машиночитаемой очереди самостоятельно не начинать.


## RNG-002-C01 — независимый эталон и корпус

После RNG-001-C03 и обновления своей ветки до принятого main. Прочитать
[RNG-002](RNG-002.md) целиком. Разрешены `tools/oracle/`, соответствующая новая
папка/файл в `tests/fixtures/`, свой status/handoff; движок Rust не изменять.
Реализовать указанный Java-адаптер и воспроизводимый корпус нужного объёма.
Проверить независимость эталона, повторяемость, границы и ошибки адаптера.
Для CHUNK-001 доказать вызов реального nibble-класса и сохранить хеш/имена.
Для RNG-002 использовать настоящий java.util.Random и подтвердить покрытие rejection.
Зафиксировать результаты и ограничения; без Rust не объявлять parity. Сделать
коммит с C01 done и самостоятельно перейти к C02.

## RNG-002-C02 — runner и проверка протокола

После RNG-002-C01. Разрешены `tools/parity/`, `tests/python/`, свой status/handoff;
общий контракт/фикстуры не ослаблять. Добавить группу из контракта в существующий
runner, сохранив прежние группы, логи, хеши и статусы. Добавить отдельную команду
проверки сохранённого отчёта/протокола либо явно расширить существующую; записать
точный вызов в handoff. Проверить ошибки, лимиты, порядок/количество/типы результатов,
синтетическую порчу и отсутствие процесса/timeout. Не считать отсутствие нового
Rust probe успехом. В `tools/check.py` включить запуск тестов examples через
`cargo test --workspace --all-targets --locked --offline` и отдельный `--doc`,
чтобы регрессионные тесты probe входили в обычную проверку.
Выполнить применимые проверки, сделать коммит с C02 done; выбирать C03 через очередь.

## RNG-002-C03 — интеграция и приёмка

После C01/C02/A01/A02 этого пакета. Разрешены merge в своём worktree, инструменты
проверки, свой status/handoff, `docs/progress.md`, статус принятия контракта.
Взять точный `snapshots.agy.commit` из очереди, проверить diff и объединить этот SHA.
Выполнить все критерии [RNG-002](RNG-002.md) и прежние live-группы.
При дефекте передать воспроизводимый запрос через blocked C03 и свой handoff;
исправления agy получать по новым коммитам через очередь без посредничества пользователя.
Не менять чужие статусы и Rust-движок ради зелёного отчёта. Перед done проверить
реальную независимость Java-эталона и область покрытия.
При успехе записать результаты и ревизии, обновить прогресс/контракт, закоммитить,
интегрировать в main и вернуться в codex/verification. Если обновление main не удалось,
C03 не оставлять принятой. Следующий пакет выбирать самостоятельно через очередь.


## CHUNK-001-C01 — независимый эталон и корпус

После RNG-002-C03 и обновления своей ветки до принятого main. Прочитать
[CHUNK-001](CHUNK-001.md) целиком. Разрешены `tools/oracle/`, соответствующая новая
папка/файл в `tests/fixtures/`, свой status/handoff; движок Rust не изменять.
Реализовать указанный Java-адаптер и воспроизводимый корпус нужного объёма.
Проверить независимость эталона, повторяемость, границы и ошибки адаптера.
Для CHUNK-001 доказать вызов реального nibble-класса и сохранить хеш/имена.
Для RNG-002 использовать настоящий java.util.Random и подтвердить покрытие rejection.
Зафиксировать результаты и ограничения; без Rust не объявлять parity. Сделать
коммит с C01 done и самостоятельно перейти к C02.

## CHUNK-001-C02 — runner и проверка протокола

После CHUNK-001-C01. Разрешены `tools/parity/`, `tests/python/`, свой status/handoff;
общий контракт/фикстуры не ослаблять. Добавить группу из контракта в существующий
runner, сохранив прежние группы, логи, хеши и статусы. Добавить отдельную команду
проверки сохранённого отчёта/протокола либо явно расширить существующую; записать
точный вызов в handoff. Проверить ошибки, лимиты, порядок/количество/типы результатов,
синтетическую порчу и отсутствие процесса/timeout. Не считать отсутствие нового
Rust probe успехом. В `tools/check.py` включить запуск тестов examples через
`cargo test --workspace --all-targets --locked --offline` и отдельный `--doc`,
чтобы регрессионные тесты probe входили в обычную проверку.
Выполнить применимые проверки, сделать коммит с C02 done; выбирать C03 через очередь.

## CHUNK-001-C03 — интеграция и приёмка

После C01/C02/A01/A02 этого пакета. Разрешены merge в своём worktree, инструменты
проверки, свой status/handoff, `docs/progress.md`, статус принятия контракта.
Взять точный `snapshots.agy.commit` из очереди, проверить diff и объединить этот SHA.
Выполнить все критерии [CHUNK-001](CHUNK-001.md) и прежние live-группы.
При дефекте передать воспроизводимый запрос через blocked C03 и свой handoff;
исправления agy получать по новым коммитам через очередь без посредничества пользователя.
Не менять чужие статусы и Rust-движок ради зелёного отчёта. Перед done проверить
реальную независимость Java-эталона и область покрытия.
При успехе записать результаты и ревизии, обновить прогресс/контракт, закоммитить,
интегрировать в main и вернуться в codex/verification. Если обновление main не удалось,
C03 не оставлять принятой. Следующий пакет выбирать самостоятельно через очередь.
## NBT-001-C01 — независимый эталон и корпус

После CHUNK-001-C03 и обновления ветки до принятого `main`. Прочитать
[NBT-001](NBT-001.md). Разрешены `tools/oracle/`, `tests/fixtures/`, собственный
status/handoff. Найти настоящий NBT reader/writer оригинала или явно зафиксировать
его отсутствие; не выдавать ручную копию формата за игровой эталон. Подготовить
не менее 5000 valid/invalid случаев, доказать modified UTF-8, типы, endian,
лимиты, consumed bytes и происхождение JAR/API. Сделать коммит и перейти к C02.

## NBT-001-C02 — runner и canonical comparator

После C01 добавить группу `nbt` в runner, не меняя Rust и не ослабляя прежние
группы. Сравнивать canonical tree, raw bytes где writer детерминирован, consumed
length и ошибки; сохранить input/output/hash/version metadata. Проверить truncation,
length overflow, malformed UTF, unknown tag, trailing bytes, timeout и mutation.
Отсутствующий оригинальный адаптер должен дать `error`/`not_implemented`, не pass.
Выполнить Python/Codex проверки, записать evidence и закоммитить C02.

## NBT-001-C03 — интеграция и приёмка

После C01/C02/A01/A02 взять точный snapshot agy, проверить diff и интегрировать его.
Запустить `tools/check.py`, all-targets и группы `rng`, `rng-extended`, `chunk`,
`nbt --strict`. При расхождении использовать цикл blocked C03 из workflow; при
успехе сохранить отчёты, обновить progress, принять NBT-001 в `main` и выбрать
следующий контракт REGION-001. NBT parity не объявляет загрузку мира.

## REGION-001-C01 — original adapter, dependency и corpus

После NBT-001-C03 и обновления до принятого `main`. Прочитать
[REGION-001](REGION-001.md), проверить наличие Prism save и four source hashes,
оригинальный client JAR hash/classpath. Исследовать настоящий Java RegionFile
reader и вызвать его на неизменённой по содержимому копии файлов. Подготовить
snapshot helper: копировать четыре `.mcr` только в run-specific artifacts, хешировать
до/после и запрещать path escape. Реальные world files в Git не включать.
Зафиксировать live 704 non-empty/empty cases и synthetic malformed corpus отдельно.
В этой карточке как интегратор добавить `flate2` с pure Rust backend, проверить
лицензию, закрепить разрешённый диапазон версии и `Cargo.lock`; сохранить offline
installability или описать provisioning. Не копировать код из mc173 reference.
Если реальный native adapter невозможен, записать точную причину blocked до того,
как называть corpus оригинальным эталоном. Закоммитить C01 и перейти к C02.

## REGION-001-C02 — runner и protocol checks

После C01 добавить группу `region` в parity runner. Проверить разрешённые пути,
копирование/хеши, координаты и стабильно воспроизводимый JSON report. Сравнивать
presence, timestamp, compression, decompressed length и SHA-256 NBT bytes.
Native live cases и synthetic cases показывать раздельно. Добавить тесты на ошибку
копирования/hash change, traversal path, invalid header, sector overlap, unknown
compression, timeout, decompression limit и мутацию одного payload. Сначала runner
собирается с dependency из C01 без Rust probe, strict верно падает/not_implemented.
Запустить общие проверки и сделать коммит C02.

## REGION-001-C03 — integration and acceptance

Зависит от C01/C02/A01/A02. Взять точный agy SHA из task snapshot, проверить diff,
интегрировать SHA и выполнить все команды из контракта REGION-001, включая 704
native live chunks и прежние parity suites. При дефекте открыть blocked C03 и
передать конкретный input/expected/actual/SHA через handoff по общему циклу TASKS.
При успехе проверить неизменность файлов в Prism source, сохранить итоговый report,
обновить progress и REGION-001, принять в main. REGION-001 не означает загрузку
полного мира или право записывать region files.

## VIEW-001-C01 — native view oracle and fixed scene

Начать после принятого WORLD-LOAD-001-C03 и clean main. Прочитать
[VIEW-001](VIEW-001.md). Исследовать оригинальный клиентский JAR с закреплённым
SHA; доказать native вызовы для block render type/opacity и virtual face tile
mapping, исследовать renderer bytecode для UV/shading. Подготовить hash-verified
copies фиксированного 5×5 scene вокруг spawn, resource provenance и матрицу реально
присутствующих block states/neighbor pairs. Synthetic состояния хранить отдельно
от live corpus. Проверить исходный мир/JAR/resource до
и после; не включать их или декомпиляцию в Git. До выдачи A01 закрепить точные
версии графических crates и Cargo.lock, Linux features, лицензии/MSRV и offline
сборку. Зафиксировать protocol/fixtures и commit C01.

## VIEW-001-C02 — native mapping oracle and runner

После C01 реализовать Java oracle на реальных методах оригинального JAR, строгий
runner/comparator, hash-bound report verifier и synthetic protocol/error tests.
Сравнивать native-derived mapping TSV точно; face geometry/culling дополнительно
проверять synthetic tests по контракту VIEW-001. Float передавать raw bits. Проверить live/synthetic
разделение, missing/wrong JAR, timeout, malformed TSV, duplicate ID и mutation.
Без Rust probe strict должен вернуть `not_implemented`/ненулевой exit, не PASS.
Обычные проверки, evidence, commit C02. Не добавлять pixel-perfect claims.
Runner обнаруживает `crates/beta-viewer/examples/view_probe.rs` как target
`cargo build -p beta-viewer --example view_probe`; A02 должен использовать
этот путь, чтобы strict discovery увидел probe автоматически.

## VIEW-001-C03 — scene integration and acceptance

После C01/C02/A01/A02 получить чистый agy SHA, проверить diff и принять только
по контракту. Запустить все старые parity suites и `view --strict`, проверить
scene/resource/world hashes и read-only исходника. Запустить viewer на фиксированной
сцене, проверить окно/camera/resize и сохранить локальный screenshot/review record
без коммита приватных данных. Отдельно указать CPU parity coverage и smoke-only
GPU evidence. Обновить progress/estimate/status, затем принять VIEW-001 в main.
Не объявлять полную визуальную parity.
