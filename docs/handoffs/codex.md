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
