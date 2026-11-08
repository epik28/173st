# JAVA-001 — Java primitive conversion and String hash parity

Статус: открыт после MATH-001-C03. Цель — дать следующим игровым пакетам
малый, проверяемый слой тех преобразований Java, которые Rust обычно выражает
иначе по умолчанию. Контракт намеренно ограничен преобразованием floating-point
в целое и `String.hashCode`; arithmetic для floor и Java RNG принадлежат своим
уже принятым модулям.

## Независимый эталон

Эталон — текущая JVM: `JavaOracle` вычисляет прямые Java-приведения
`(int) Float.intBitsToFloat(bits)`, `(int) Double.longBitsToDouble(bits)`,
`(long) Float.intBitsToFloat(bits)`, `(long) Double.longBitsToDouble(bits)` и
`String.hashCode()` для UTF-8 строки, декодированной в Java `String`. Не
использовать формулу из Rust-реализации для ожидаемых значений. В отчёте
закрепляются версия JVM, хеш oracle и вход/выход.

## Rust API и probe

`beta_core::java_compat` предоставляет:

- `f32_to_i32(f32) -> i32`, `f64_to_i32(f64) -> i32`;
- `f32_to_i64(f32) -> i64`, `f64_to_i64(f64) -> i64`;
- `string_hash_code(&str) -> i32`.

Преобразование Java float-to-integer даёт 0 для NaN, ограничивает результат
минимумом или
максимуму целевого signed-типа для значений за диапазоном/бесконечностей, а
конечные дробные значения усекает к нулю. Это именно Java cast, не floor.
`string_hash_code` повторяет `String.hashCode()` по UTF-16 code units с
32-bit wrapping arithmetic; вход — валидный Rust UTF-8, supplementary scalar
представляется суррогатной парой при кодировании в UTF-16.

Probe `crates/beta-core/examples/java_compat_probe.rs` принимает TSV с сырыми
IEEE-битами и UTF-8 в lowercase hex; результат — точное decimal значение.
Невалидные биты, UTF-8, диапазоны, операция, дубли ID и лимиты отвергаются без
частичного stdout.

## Корпус

C01 фиксирует детерминированные входы для всех пяти операций, включая:

- signed zero, субнормальные значения, нормальные значения, ±1, соседние
  representable значения вокруг `i32`/`i64` границ, NaN payloads, ±infinity,
  крайние конечные значения и большие magnitudes;
- не менее 4096 воспроизводимых raw-bit значений на каждую conversion operation;
- строки пустой/ASCII/Unicode, BMP/non-BMP, combining sequences и длины,
  достаточные для переполнения 32-bit hash.

Все ожидания получаются двумя независимыми запусками Java oracle. Сравнение
строгое по ID, порядку и decimal values. Fixture не содержит исходников/JAR.

## Приёмка

- Java oracle вызвал перечисленные Java casts и `String.hashCode()` непосредственно;
- Rust совпал по всем fixture rows, raw float input cases и UTF-16 hash cases;
- probe атомарно отвергает malformed protocol, выводит ошибки только в stderr;
- runner сохраняет хеши и логи, проверяет повторяемость, мутации, timeout/error;
- `python3 tools/check.py`, предыдущие strict suites и новая группа `java --strict`
  проходят; saved protocol review проходит;
- итоговый снимок принят в `main`, handoff/status/progress/оценка обновлены.
