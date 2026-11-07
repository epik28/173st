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
