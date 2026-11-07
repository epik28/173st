# Отчёт Codex

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
