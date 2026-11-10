# WATER-SIM-001 — симуляция воды Beta 1.7.3

## Цель

Текущий viewer рисует native-совместимые поверхности water ID 8/9, но не
обновляет их блоки. Добавить проверенные обновления воды в памяти выбранной
копии мира: распространение по горизонтали и вниз, metadata/flow decay,
источники, scheduled ticks и реакции с окружением в пределах подтверждённых
правил версии Beta 1.7.3. Исходное сохранение остаётся read-only; runtime
изменения живут только в disposable snapshot. Lava и сохранение изменений в
этой задаче не входят.

## C01 — оригинальный алгоритм и эталон (Codex)

По pinned Beta 1.7.3 JAR найти оригинальные liquid block/update/scheduler
методы и зафиксировать class/method/JAR hashes и порядок вызовов. Вызывать
оригинальные методы на синтетическом in-memory World/chunks, не переписывать
ожидаемые формулы в Java. Корпус должен содержать повторяемые пошаговые сцены:

- одиночный источник, горизонтальный бассейн и ограниченный берег;
- падение с уступа, вертикальная колонна и распространение вниз;
- несколько источников с/без соседней воды, создание и утрата источника;
- смена metadata/flow vector и обновления после постановки/удаления соседнего
  блока;
- очередь scheduled ticks, одинаковое время обновления, границы загруженного
  чанка и неизвестные соседи.

Для каждого шага записать все изменённые block ID/metadata, scheduled tick
добавления/удаления с delay и координатой, вызовы neighbor update и запросы
loaded chunks. Два независимых запуска обязаны совпасть побайтно. Случайный
порядок/seed фиксировать. Отдельно описать, какие механики Beta 1.7.3 реально
содержит; не выводить правила из современных Minecraft.

## A01 — чистый переход состояния (agy)

Реализовать в beta-core pure bounded transition по frozen C01: вход — локальные
соседи, metadata, tick kind/delay и явное состояние loaded/unloaded; выход —
упорядоченные block/meta edits и scheduled tick requests. Не читать файлы,
часы или глобальный мир из pure API. Кандидатное состояние при ошибке/лимите
не должно частично применяться. Не использовать угадываемую формулу, которой
нет в oracle fixture.

## A02 — игровой scheduler (agy)

Подключить принятый transition к fixed 20 Hz viewer simulation. Вода получает
ticks только по правилам native schedule; блоковые edits и новые/соседние
loaded chunks ставят нужные события. Очередь ограничена, порядок одинаков при
одинаковых входах, пауза окна не меняет последовательность simulation ticks.
Изменения обновляют только in-memory snapshot и dirty chunks/meshes; никакой
записи в исходное сохранение.

## C02 — сравнение и приёмка (Codex)

Strict runner сравнивает полный порядок native tick/state transcripts и raw
ID/metadata с Rust на каждом промежуточном тике. Проверить повторяемость,
просроченные и повторные события, loaded/unloaded границы, atomic limit failure,
edit→flow и world source hashes. В viewer отдельно подтвердить, что пересчитанная
вода изменяет форму, а mesh обновляется только в затронутой области. Не заявлять
lava, течения через unloaded chunks, fluids persistence, bucket interactions,
water movement physics или полную визуальную parity.

## Приёмка

Native capture и runner проходят с двумя идентичными эталонными запусками;
каждый тик и queue event совпадают точно. Rust transitions проходят все frozen
сцены; viewer использует фиксированный tick scheduler и disposable copy.
Оригинальный save/JAR и неподдержанные области явно остаются вне claim.

## C01 bounded capture evidence (2026-11-10)

The native probe in `tools/oracle/src/WaterTickOracle.java` invokes the pinned
Beta 1.7.3 water block classes on a synthetic `fd` copy backed only by nine
in-memory byte-array chunks. It captures still water, flowing water, decay,
falling, a blocked basin, adjacent source/flow input, two adjacent sources, and
one- and two-step horizontal flow. Each scene freezes its full setup, bounded
state (`x/z=-2..2`, `y=62..65`), random seed, ordered edits, scheduler queue and
neighbor callback transcript. The adapter runs separate byte-identical native
captures and verifies the JAR plus `uu`, `om`, `nx`, `rp`, `fd`, and `qy` hashes.

The synthetic loaded area is all-loaded, and the tick pump advances to each
captured native due time before calling `fd.a(false)`. This establishes the
captured block transitions and queue behavior for these controlled scenes. It
does not establish unloaded-chunk behavior, source creation rules, world-wide
random ticking, lava, neighbor edits beyond the fixtures, or save/runtime
integration. In the `two-adjacent-sources` input, the center remains flowing
water for this exact arrangement; do not generalize that row into a source
creation rule.
