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

## C01 source-formation extension (2026-11-10)

A separate `tests/fixtures/water_tick_sources/` extension exercises the original
`om.a(fd,int,int,int,Random)` update without changing the nine frozen C01 native
rows (`native.tsv` SHA-256 remains
`cf829ee9aa63bdf2ace25104c36e9cc84eb9c6b1e92c1607573e8f18008f5276`). The
adapter has a `--sources` mode; it still verifies the same pinned client and
liquid/scheduler class hashes, uses only the nine synthetic byte-array chunks,
and performs two independent byte-identical captures. The extension fixture has
its own inputs, transcript, audit, and SHA-256.

In the bounded supported case, center `(0,64,0)` begins as flowing water ID 8,
metadata 1; west and east are still water ID 9, metadata 0; below and the north
and south neighbors are stone ID 1. After the first due tick, center and both
side water blocks are ID 8, metadata 0. The transcript queues all three for the
next due time. After the second due tick, center is ID 9, metadata 0; the exact
queue and edits are frozen in the extension. With one west source, the same
supported/enclosed scene leaves the center at ID 9, metadata 1 after one due
tick. With two side sources but air below, water flows down to `(0,63,0)` as ID
8, metadata 9, and center remains ID 8, metadata 1. With ID 9, metadata 0 below,
the center becomes ID 9, metadata 1. These are exact observed transitions for
these inputs; the ID 9/meta 1 controls are not evidence for a level-zero source.

The extension supports a bounded claim about the observed two-tick supported
transition and its recorded neighbor/scheduler effects. It does not establish a
universal source-formation threshold, arbitrary geometry, unloaded-neighbor
behavior, or world-wide random ticking. Reproduce and test with
`MINECRAFT_BETA_173_JAR=/path/to/pinned/client.jar python3 tools/oracle/water_tick_corpus.py --source-live --write`
to recapture, or run `python3 tools/oracle/water_tick_corpus.py --sources` to
audit the committed extension.

## C01 initial scheduling and neighbor callback extension (2026-11-10)

The separately versioned `water-tick-seeding-1` fixture records three bounded
native `fd.a`/`fd.i` edit paths. It uses a fresh synthetic world for each case,
nine loaded byte-array chunks, world time 0, and a full 36-cell snapshot over
`x/z=-1..1`, `y=62..65`. No world tick is dispatched. The adapter captures the
ordered `fd.c` schedule calls, the resulting `qy` queue, edit and neighbor
callbacks, and the post-edit state. Two fresh JVM captures are byte-identical.

Placing flowing water ID 8/meta 0 at `(0,64,0)` invokes a schedule request for
that coordinate and ID with delay 5, then records the block edit; the queue has
one event due at world time 5. In a separate case, placing stone at `(1,64,0)`
next to still water ID 9/meta 0 and explicitly calling native
`fd.i(1,64,0,1)` records the neighbor callback, two schedule requests for
`(0,64,0,8)` with delay 5, and the water edit 9→8; the native queue contains one
event. Removing stone below a still source at `(0,64,0)` and calling
`fd.i(0,63,0,0)` produces the same two schedule requests, flowing-water edit,
and single queued event. Exact ordering and resulting state/queue are frozen in
`tests/fixtures/water_tick_schedule/`.

These rows establish only the explicit native setter and neighbor-notification
calls above, including native duplicate-request deduplication in this empty
queue. They do not establish a complete player interaction path, chunk-load
seeding, behavior with pre-existing scheduled events, unloaded neighbors, or
world-wide random ticks. Reproduce with
`MINECRAFT_BETA_173_JAR=/path/to/pinned/client.jar python3 tools/oracle/water_schedule_corpus.py --live --write`
or audit the frozen rows with `python3 tools/oracle/water_schedule_corpus.py`.
