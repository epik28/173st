# FLUID-001 — native water surface geometry and viewer pass

## Цель и границы

Добавить отрисовку воды Beta 1.7.3 (block IDs 8 и 9) в текущий read-only viewer.
Сейчас эти состояния пропускаются как неподдержанные. Первая карточка ограничена
геометрией воды и её atlas UV/color; lava, распространение жидкости, тики,
редактирование мира и полная pixel parity не заявляются.

Эталон — pinned client JAR Beta 1.7.3 с SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
Вызывать настоящий `cv` renderer и читать native `nw.g` Tessellator submissions.
Адаптер может дать ограниченную synthetic `xp`-сцену, но не должен повторять
формулы рендера в Java, создавать/менять игровые классы или вызывать GL draw/flush.
Сохранять JAR/class/source hashes, exact input, raw float bits, native call audit и
два идентичных запуска. Оригинальные бинарники в Git не включать.

## Замороженный контракт

- Pure API `beta_view::fluid` принимает центр `(x,y,z)`, metadata и ограниченную
  локальную сетку блоков (3×3 X/Z, блок выше и ниже), выдаёт упорядоченные quad
  vertices с raw-compatible `f32` position/UV и native per-face color. API не
  загружает мир, текстуры, JVM или filesystem.
- Высоты жидкости, corner averaging, falling column, side/top/bottom visibility,
  tile selection и winding следуют только подтверждённым native вызовам/bytecode.
  Не подменять native результат геометрией полного куба.
- Scene mesher помещает воду в отдельный transparent mesh. Existing opaque mesh,
  leaves, tint, light, UV, collision and movement остаются совместимыми.
- Viewer рисует transparent mesh отдельным alpha-blended pipeline; depth test
  включён. Depth write, blend behavior и сортировку quad определить после native
  state inspection. Не менять cutout-листья и opaque materials.
- Поддержать source/still water и flowing water ID 8/9 с metadata 0..15. Не
  заявлять lava, water physics, fluid interactions или production optimization.
- Входы и vertex/index buffers ограничены. Неверные координаты, metadata,
  недостающая соседняя сетка и переполнение возвращают явную ошибку.

## Карточки

### FLUID-001-C01 — Native geometry oracle

Добавить synthetic harness, вызывающий оригинальный `cv` renderer для матрицы:
metadata 0..15 в isolated pool; cardinal neighbors с водой, воздухом и opaque
block; diagonal neighbors для corner averaging; water above/below; колонна воды.
Записать захваченные `nw.g` submissions как raw float32 bits
плюс native block/material/visibility calls. Проверить class/JAR code sources, input
hash, два независимых JVM runs, отсутствие GL draw/flush. Зафиксировать coverage;
не создавать Java формулу ожидаемых вершин.

### FLUID-001-C02 — Native comparison runner

Создать strict saved runner/verifier `fluid`, проверяющий native output против
committed fixture и будущего `fluid_probe`: точное число/порядок полей, raw float
bits, class/JAR/source/input/output hashes, повторы и отсутствие лишних строк.
Проверить missing JAR/probe, неверный protocol, duplicate cases, malformed и
late-invalid input, timeout, output mutation и synthetic mutation. До Rust probe
strict обязан вернуть `not_implemented`. Probe — Cargo example
`crates/beta-viewer/examples/fluid_probe.rs`; stdin — frozen one-line suite из
`tests/fixtures/fluid/input.tsv`; stdout — только упорядоченные `case` и `vertex`
TSV rows. Case row сохраняет имя, ID, metadata, rendered flag и vertex count;
vertex row хранит восемь native `nw.g` words как lowercase raw-u32 hex. Остальные
строки stdout запрещены.

### FLUID-001-A01 — Pure native-compatible water geometry

Реализовать только замороженный pure API в `crates/beta-view/src/` и unit tests.
Ожидаемые вершины брать только из committed C01. Не изменять Java oracle,
reference, runner, общий контракт и viewer pipeline. Проверить уровни, соседей,
направления, точный winding и лимиты; сохранить существующий API и регрессии.

### FLUID-001-A02 — Probe and transparent viewer integration

Добавить bounded atomic `fluid_probe` и подключить ID 8/9 к сцене. Создавать
отдельный transparent vertex/index buffer и alpha-blended draw pass, не меняя
opaque/cutout pass. `fluid --strict` должен совпадать с C01 полностью. Проверить
read-only invariants и сохранить screenshot fixed scene с водой. Записать число
вершин/чанков; не заявлять FPS improvement без benchmark.

### FLUID-001-C03 — Acceptance

Проверить точный agy commit, весь workspace `tools/check.py`, прежние strict parity
suites, полный `fluid --strict`, saved report/mutations, viewer Xvfb smoke и
screenshot. Повторно проверить source save/JAR hashes до/после. Принимать только
доказанную воду; lava и fluid simulation остаются unsupported. Обновить progress,
roadmap, estimate, handoff и задачу в main.

## Приёмка

- Два native captures побитово идентичны и подтверждают реальные вызовы client JAR.
- C01 inputs/vertices покрыты committed fixture; strict native ↔ Rust — точное
  равенство строк и raw float bits.
- Synthetic mutations и invalid protocol не проходят как успешное сравнение.
- Source save/JAR/resources не изменены; viewer показывает прозрачную воду с
  подтверждённой поверхностью и боковыми гранями в закреплённой сцене.
- Старые UV/color/leaf/light/movement/view behavior не регрессируют.
- Не утверждать поддержку lava, water ticks/flow, pixel parity или FPS improvement.
