# Отчёт Gemini / agy

## Метаданные
- **Model ID:** Gemini 3.8 Flash (High) via Antigravity
- **Заимствования:** Нет (алгоритм LCG по спецификации `java.util.Random`, probe по контракту RNG-001/RNG-002)
- **Ручная помощь:** Нет
- **Внешние зависимости:** Не добавлялись

---

## Задача: RNG-001-A01 — Rust JavaRandom.next_int
- **Статус:** Выполнена (`done`), принята в `main`.

---

## Задача: RNG-001-A02 — Rust probe по TSV-контракту
- **Статус:** Выполнена (`done`), принята в `main`.

---

## Задача: RNG-002-A01 — Rust API расширенного RNG

### Статус
Выполнена (`done`).

### Изменённые файлы
- `crates/beta-core/src/java_random.rs`:
  - Добавлен `RandomError::InvalidBound` с реализацией `Display` и `Error`.
  - Реализован `set_seed(&mut self, seed: i64)`.
  - Реализован `next_int_bound(&mut self, bound: i32) -> Result<i32, RandomError>`:
    - Проверка `bound <= 0` возвращает `Err(RandomError::InvalidBound)` без продвижения состояния генератора.
    - Ветка степени 2: `(bound * r) >> 31`.
    - Rejection sampling ветка: цикл перевыборки при `u - r + (bound - 1) < 0` (в 32-битном знаковом переполнении).
  - Реализован `next_long(&mut self) -> i64` с сохранением знаковых промежуточных преобразований `(hi << 32).wrapping_add(lo)`.
  - Реализован `next_boolean(&mut self) -> bool` (`next(1) != 0`).
  - Реализован `next_float(&mut self) -> f32` (`next(24) as f32 / 16777216.0f32`).
  - Реализован `next_double(&mut self) -> f64` (`((next(26) << 27) + next(27)) as f64 / 9007199254740992.0f64`).
  - Написаны модульные тесты:
    - `test_invalid_bound_does_not_mutate_state`
    - `test_reseed_matches_new`
    - `test_extended_methods_shared_state_sequence` (пошаговая цепочка методов `int`, `bound`, `long`, `boolean`, `float`, `double` сверена со стандартным выводом Java 17)
    - `test_rejection_sampling_bound_1073741825` (2048 последовательных вызовов)
- `docs/tasks/status/agy.json` — отметка `RNG-002-A01` в состоянии `done`.

### Проверки
- `cargo test --all-targets --locked --offline` → 15 тестов пройдены успешно (11 в `lib.rs`, 4 в `rng_probe.rs`).
- `python3 tools/check.py` → 27 Python-тестов, fmt, clippy, workspace checks passed.
- `python3 tools/parity/run.py rng --strict` → 96 записей совпадают с Java-эталоном.

---

## Задача: RNG-002-A02 — Rust probe расширенного RNG

### Статус
Выполнена (`done`).

### Изменённые файлы
- `crates/beta-core/examples/rng_extended_probe.rs`:
  - Реализован CLI probe по контракту `RNG-002`.
  - Читает TSV stdin (`case_id\tseed\toperations`), где `operations` — список токенов через `;` (`int`, `bound:N`, `long`, `boolean`, `float`, `double`, `seed:S`).
  - Пишет TSV stdout (`case_id\tindex\top_name\tvalue`) с LF-окончаниями:
    - `int`: десятичный signed i32;
    - `bound`: десятичный signed i32 или `invalid_bound` при `bound <= 0`;
    - `long`: десятичный signed i64;
    - `boolean`: `0` или `1`;
    - `float`: raw IEEE-754 биты (8 lowercase hex цифр без префикса);
    - `double`: raw IEEE-754 биты (16 lowercase hex цифр без префикса);
    - `seed`: `ok`.
  - Строгая валидация:
    - Строго пустые строки и комментарии (`#`) пропускаются;
    - Непустые whitespace-строки (`" \n"`, `"\t\n"`) отклоняются с ошибкой;
    - Запрещены пробелы внутри поля `operations`;
    - Пустые элементы между `;` (двойная точка с запятой, завершающая точка с запятой) отклоняются;
    - Проверка уникальности `case_id`;
    - Лимит не более 100 000 операций суммарно;
    - Некорректный синтаксис/аргументы/неизвестные токены завершаются с кодом 1 и ошибкой в stderr.
  - Написаны модульные тесты: `test_valid_operations_stream`, `test_empty_line_and_comment_skipped`, `test_whitespace_line_rejected`, `test_invalid_ops_syntax`, `test_duplicate_case_id_rejected`.
- `docs/tasks/status/agy.json` — отметка `RNG-002-A02` в состоянии `done`.

### Проверки
- `cargo test --example rng_extended_probe --locked --offline` → 5 тестов пройдены успешно (0 failed).
- `cargo test --all-targets --locked --offline` → 20 тестов пройдены (11 модульных `lib.rs`, 5 `rng_extended_probe`, 4 `rng_probe`).
- Тестирование через subprocess 12 сценариев ошибок (неверное число полей, невалидный ID, дубликаты ID, пробельные строки, табуляции, некорректный seed, завершающая `;`, пустая операция `;;`, пробелы внутри ops, нечисловой bound, нечисловой seed, неизвестная операция) → все возвращают код 1 с выводом в stderr.
- Валидный сценарий выдаёт 11 строк, побитово совпадающих с Java.
- `python3 tools/check.py` → 27 Python-тестов, cargo fmt/check/clippy/test все PASS.
- `python3 tools/parity/run.py rng --strict` → PASS (96 записей совпадают).

### Следующий шаг
- Задачи пакета RNG-002 для роли `agy` завершены. Codex выполнит C01, C02, интеграцию и приёмку в C03.
