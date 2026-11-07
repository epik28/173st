# Прогресс

## Текущее состояние

- Цель и границы: Minecraft Beta 1.7.3 на Rust, без модов; см.
  `project-requirements.md`, `workflow.md` и `parity-testing.md`.
- **RNG-001 принят**: C01, C02, A01, A02 и C03 выполнены. Коммит приёмки C03
  `fa875dd` интегрирован в `main` fast-forward; текущая очередь завершена. Новые этапы не начинались.
- C01 (`7d435e6`): Java-эталон вызывает настоящий `java.util.Random.nextInt()`.
- C02 (`185b828`): live-runner с точным сравнением, логами, тайм-аутами,
  метаданными/хешами и регрессионными тестами компаратора.
- A01/A02: Rust RNG и TSV-probe реализованы agy. Последний принятый snapshot
  `80dee8a5296d0769d29a938b0aea2795d4f0e556`, включая исправление whitespace
  и четыре регрессионных теста. Объединённая проверенная ревизия Codex:
  `544e9d2c0adb3412939ce203a278d56f97c904f9`.
- Прежняя блокировка C03 снята: строки из пробела или табуляции теперь
  отклоняются Rust с ненулевым кодом и stderr, как требует контракт.
- Сторонний код не переносился. Codex не правил Rust-движок; Codex подготовил
  независимый Java-эталон, runner и приёмку, agy реализовал RNG и исправил probe.
- Клиентский JAR Beta 1.7.3 локально найден и захеширован; серверный JAR
  и игровые адаптеры отсутствуют. Для Java API-сценария игровой JAR не нужен.

## Фактические проверки объединённого кода

- `python3 tools/doctor.py`: инструменты доступны.
- `python3 tools/check.py`: PASS — 27 Python-тестов (14 runner), Rust
  fmt/check/clippy с `-D warnings`, 7 Rust-тестов ядра.
- `cargo test --locked --offline -p beta-core --example rng_probe`: PASS — 4 теста
  обработки whitespace, пустых строк, комментариев и валидного TSV.
- `python3 tools/parity/run.py rng --strict`: PASS — шесть seed,
  включая границы i64, все 96 значений совпали точно; оба процесса exit 0.
  Ревизия `544e9d2c0adb3412939ce203a278d56f97c904f9`, dirty=false.
  Отчёт: `artifacts/parity/rng/run-oajcevsr/report.json`.
- `python3 tools/parity/verify_report.py artifacts/parity/rng/run-oajcevsr/report.json --protocol`:
  PASS, exit 0. Сохранённые хеши подтверждены; подмена одного значения
  в синтетической копии обнаружена. Все 36 live-запусков проверки TSV прошли.
  Результаты: `protocol-review.json` и `comparator-sensitivity.json` рядом с отчётом.
- История найденного и устранённого дефекта сохранена в `handoffs/codex.md`
  и исходном отчёте `artifacts/parity/rng/run-lwytrhsv/`.
- Область live-сравнения — только Java RNG API nextInt на фиксированных seed.
  Генерация мира, движение, игровой цикл и производительность игры не проверялись.

## Следующий шаг

Очередь RNG-001 завершена. Следующий этап требует отдельного контракта;
предлагаемое направление — другие методы RNG и смешанные последовательности.
Перед следующей задачей обе роли обновляют свои ветки до принятого `main`.
