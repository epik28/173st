# Прогресс

## Текущее состояние

HOTBAR-ITEMS-001 принят: hotbar показывает поддержанные предметы из сохранённого
инвентаря, цифры стека рисуются только при count > 1; atlas для block/item IDs
выбирается отдельно. Ошибочная привязка иконок блоков в E-screen также исправлена.
Native audit, `items --strict`, viewer tests и полный `tools/check.py` прошли.
GUI smoke остановлен hash guard изменённого локального `level.dat`, поэтому новый
кадр не проверен. Оценка полной игры — около **31%**. Следом открывается перенос
стеков мышью между 36 main slots; crafting, armor, drop и запись сохранения пока
остаются вне этой задачи.

WORLD-LOAD-001 принят: read-only metadata и typed chunk data из Beta 1.7.3 save.
VIEW-001 принят: реализован CPU mesher и локальный wgpu viewer фиксированной
5×5 сцены с оригинальной terrain-текстурой; окно показывает рельеф и поддержанные
opaque cube блоки. Это первый графический milestone, не полная визуальная parity.
MATH-001 и JAVA-001 приняты. JAVA-001 добавил Java-совместимые float/double
casts в int/long и UTF-16 `String.hashCode`, сверенные с прямым JVM oracle.
GAME-ORACLE-001 принят: оригинальные AABB/Vec3/ray методы доступны агентам через
CLI с пользовательскими входами, Rust geometry сверена по raw bits.
EVIDENCE-001 усилил сохранённые доказательства. MOVE-001 принят: native dry
movement и фиксированный игровой шаг подключены к viewer. UV-001 принят: исправлена ориентация UV на верхней грани и грани +Z по
оригинальному Tessellator buffer. COLOR-001 принят: scalar biome climate и палитры точно сверены с оригиналом.
LEAF-001 и LIGHT-001 приняты: листья, biome tint и нативное плоское освещение подключены к viewer.
FLUID-001 принят: вода ID 8/9 побитово совпадает с native геометрией в 44 сценах и
рисуется отдельным alpha-blended проходом. Lava и симуляция жидкости не поддержаны.

BLOCK-001 принят: exact native ray targeting на поддержанных формах, локальный
creative break/place тестового полного блока и пересборка видимой сцены после
изменения. После пользовательского сообщения, что первый локальный фикс не убрал
задержку, viewer переведён на отдельные GPU buffers по чанкам: CPU/GPU геометрия
обновляется только для dirty chunks.
Strict runner сверяет 17 native ray cases; действия не имеют native oracle и
подтверждены Rust tests, без заявления action parity. Источник save не изменяется.
Survival mining, ориентация блоков и light propagation остаются вне пакета;
три последовательных core edits покрыты тестом, но latency, reset после третьего
удара, тёмные грани и вид листвы ещё требуют GUI проверки и native сравнения.

FLIGHT-SMOOTH-001 принят: debug-flight сохраняет fixed 20 Hz движение, а камера
рендерит промежуточную позицию. Тесты покрывают 20/30/60/144 Hz, смену режима,
reset и потерю фокуса. Это сглаживание нашего debug-режима, не parity полёта
Minecraft; GUI-smoke остановил hash guard из-за изменённого локального `level.dat`.

ITEMS-001 принят как первый ограниченный слой инвентаря: 36 main slots,
9-позиционная hotbar, выбор слота, split/remove, merge и spill. Native `iz`/`ix`
эталон и Rust probe совпали по всем 21 состояниям, каждому из 36 слотов и
подтверждённым stack limits. Пока поддержаны только IDs 1/3/332/267 и damage 0;
нет GUI, armor, crafting, pickup/drop, survival, полного каталога предметов или
записи сохранения. `items --strict` и retained report review прошли. На объединённой
ревизии прошли 12 из 15 прежних strict parity групп; `region`, `world` и `view`
остановлены source-hash guard на изменённых файлах пользовательского Prism-мира
до копирования в артефакты/передачи native adapter. Эталоны не перепривязывались,
мир не изменялся.

HUD-001 принят: viewer рисует нативную рамку и выделение hotbar, 10 полных сердец
и прицел отдельным screen-space pass после мира. GUI-атласы извлекаются в память
из SHA-pinned клиентского JAR и проверяются по resource hashes; item sprites,
интерактивный инвентарь и настоящее здоровье не реализованы. `tools/check.py`
прошёл на объединённой ревизии. Xvfb smoke загрузил атласы, но остановился на
защите изменённого `level.dat`, поэтому новый скриншот не получен.

HOTBAR-INPUT-001 принят: клавиши 1–9 и колесо теперь выбирают слот в
`Inventory`, а рамка HUD следует выбранной позиции. Native bytecode подтверждает
hardcoded mapping клавиш и один sign-directed шаг на ненулевое целочисленное
wheel event с wrap по 0..8. Fractional line/pixel conversion — политика viewer,
не native parity. `items --strict`, pinned hotbar audit и полный `tools/check.py`
прошли. Xvfb startup проверил HUD atlas и остановился на защите изменённого
локального `level.dat`; мир не затронут.

Оценка готовности **полной игры — около 26%** после HOTBAR-INPUT-001.
Замечание о кроне остаётся открытым; визуальная проверка сглаживания полёта
отложена до доступной закреплённой сцены. См. [known-issues.md](known-issues.md).
Методика и веса: [completion-estimate.md](completion-estimate.md).
Это приблизительная оценка возможностей, не доля закрытых задач.

INVENTORY-UI-001 принят: клавиша E открывает отдельный экран, Escape закрывает
его и возвращает захват мыши. На экране отображаются нативная панель 176×166,
36 проверенных слотов и иконки четырёх поддержанных типов предметов из атласов,
которые извлекаются только в память из локального hash-pinned JAR. Текущий
Inventory пуст, поэтому иконки предметов пока не появляются без загрузки данных
игрока. Полный `tools/check.py`, inventory audit и `items --strict` прошли.
Визуальный запуск остановлен существующей защитой изменённого локального
`level.dat`; `view --strict` не копировал и не открывал мир. Оценка полной игры
теперь около **27%**.

VIEW-LOCAL-001 обновил только локальный preview baseline: приложение проверяет
игнорируемый workspace-level manifest пяти текущих source-файлов, копирует их
в отдельный artifact-каталог и загружает копию из main/codex/agy. Старые
WORLD/VIEW strict fixtures и их
ожидаемые хеши остались нетронутыми. На обновлённой копии два независимых native
запуска совпали с Rust preview probe побайтно: 47 mapping cases и 1 513 face
neighbors. Также устранен запускной путь с `..`: viewer подготовил 25 чанков,
334,760 opaque и 9,762 water vertices; текущая среда без Wayland compositor не
показывает окно. Исходные пять файлов до и после копирования имеют те же хеши.

PLAYER-SAVE-001 подключил к E-screen поддерживаемые предметы из `Player.Inventory`
этой проверенной копии. Читаются только слоты 0–35 и текущие четыре поддержанных
item ID; неизвестные/испорченные стеки считаются и пропускаются. Чтение остаётся
read-only; здоровья, переноса предметов и сохранения изменений нет. Unit tests,
viewer tests и clippy проходят. Общая экспертная оценка — около **30%** с
погрешностью в несколько пунктов.

BLOCK-001: `170033d` интегрирован в main. Проверены 17 native ray cases,
bounded viewer break/place и полная пересборка сцены; edits остаются в памяти.
См. подробности в [completion-estimate.md](completion-estimate.md).

FLIGHT-SMOOTH-001 и ITEMS-001 приняты в `a0b12bb`. Inventory пока data-only:
в viewer ещё нет item sprites, inventory screen/controls или survival health state.
Оценка остаётся приблизительной и не означает полноту игрового цикла.

- **RNG-001, RNG-002, CHUNK-001, NBT-001, REGION-001, WORLD-LOAD-001,
  VIEW-001, MATH-001, JAVA-001, GAME-ORACLE-001, MOVE-001, COLOR/LEAF/LIGHT,
  FLUID-001, BLOCK-001, FLIGHT-SMOOTH-001, ITEMS-001 и HUD-001 приняты**.
  Проверенная ревизия REGION — `1e076324e992412d396a15da72f808facb00632f`;
  коммит приёмки содержит запись C03 и переносится в `main` fast-forward.
- RNG-001: 96 результатов Java API nextInt.
- RNG-002: 13227 смешанных операций, общее состояние, reseed, rejection sampling,
  raw IEEE-биты float/double.
- CHUNK-001: 4182 операции с контейнером 16×128×16 и оригинальным nibble-классом.
- NBT-001: 5250 live round-trips оригинальных ij/nu, точные raw bytes,
  canonical tree и consumed length; 86 дополнительных protocol/wire/safety
  executions и пять синтетических мутаций проверены отдельно.
- REGION-001: 704 occupied + 44 absent slot точно совпали с оригинальным qj;
  37 synthetic cases/74 reader executions проверены отдельно от live parity.
- WORLD-LOAD-001: 749 live rows (level.dat + 748 slots) совпали с native Java;
  704 chunk payload present, 44 absent. Отдельно проверены 22 protocol executions
  и 42 generated edge cases/84 executions. Native audit: qj=748, ij=705, ei=1,
  full native chunk/world loads=0. Это read-only data extraction, без генерации,
  тиков, создания игровых объектов и записи.
- Rust принадлежит роли agy. Текущий REGION реализован внутренним агентом
  `gpt-6-luna low`, runner C02 — другим агентом той же модели. Координатор
  Codex выполнил ревью, запросил исправления Rust и принял объединённый результат;
  прямых правок Rust координатором нет. Предыдущие handoff сохраняют историю моделей.
- JAR/ресурсы/дизассемблированный код и пользовательский мир в Git не включены.
  Проверки читают копии; работа с `.mcr` остаётся read-only. C01 закрепляет
  `flate2 =1.1.9` (`rust_backend`) и probe-only `sha2 =0.10.9`; lockfile проверен offline.

## Последняя независимая приёмка

FLUID-001 принят на clean agy revision `0cd7321caeb35db67cace8b1d4826c684ae2b41d`.
Pure water mesh и bounded `fluid_probe` совпадают с original `cv.b` на 44 сценах /
1,242 вершинах по raw словам; retained report прошёл отдельную проверку источников,
процессов и выходов. Viewer вынес воду в отдельный alpha pass; Xvfb capture показывает
25 чанков, 334,776 opaque и 9,768 water vertices. `tools/check.py` и все предыдущие
strict parity suites прошли. Хеши пяти pinned save-файлов не изменились при smoke.
Это только визуальная вода: fluid simulation, lava, transparent sorting и pixel
parity не заявляются. Оценка полной игры — около 21%; push отложен до 30% по указанию
пользователя.

COLOR/LEAF/LIGHT совместно приняты на чистой ревизии `755d6dd`.
Rust LIGHT `a39fda2`, renderer `8a12289`/`ec3759c`/`ff84905`/`2072705`.
Все72928 native light records exact; дополнительно2048 custom time rows и
emission-only singleton совпали. Saved evidence review +45 live protocol
executions PASS, проверены actual sources/revision/JAR/classes/binary/audits.
Нативный цвет вершины подтверждён504 packed submissions (12 renderer calls),
два независимых JVM запуска совпали. Графический путь сохраняет исходный порядок
f32 и byte quantization. Native leaf/UV, fresh COLOR1600 и legacy VIEW50/1617,
previous MATH131408/JAVA18493, полный check161 Python cases (17 skips)/148 Rust
прошли. Xvfb после финальной8-float GPU layout показывает зелёную траву/кроны.

Ограничения: фиксированная5×5 сцена из проверенного Overworld save, static saved
time, flat light, без AO/погоды/распространения света/полного pixel parity.
Nether проверен численно; DIM-1 regions viewer пока не загружает. Оригинальные
ресурсы/мир только читаются и отсутствуют в Git. Прямых правок Rust координатором
в этом пакете нет; реализация делегирована `gpt-6-luna low`.
Оценка≈20%, прирост≈4pp от последнего push16%; новый push пока не выполняется.

### Предыдущая приёмка COLOR-001

COLOR-001 принят: Rust snapshot `ea8bcfc`, clean tested source `c332ebb`.
1280 climate +320 palette rows exact, включая raw f64 и полныйARGB. Дополнительно
320 custom rows across64 diverse seeds и singleton newseed точно совпали с JVM.
Проверены retained artifacts/processes/commands/sources/resources,4 output
mutations,24 malformed executions; full workspace check147 Python cases15 skips
и прежние RNG/math/java strict suites PASS. Core APIs Java-free, PNG palette
resources читаются локально с проверкойSHA. На момент этой отдельной приёмки видимый renderer ещё ожидал LEAF-001;
историческая оценка составляла около16%.


UV-001 принят: Rust snapshot `6312bcf`, clean merged test revision `a3ca583`.
Прямые original cv.a..f calls → native nw.g packed xyz/uv дали 84 случая,
504 triangle submissions и 336 unique corners. Новый fixture test до исправления
обнаружил112 отличий на faces1/3, после remap — ноль; позиции, winding, tile
selection и свет не менялись. `tools/check.py` PASS (133 Python cases,10 opt-in
skips, все Rust checks); отдельные5 native UV tests, `view --strict` и viewer
startup/screenshot PASS. Старый synthetic UV test повторял implementation;
новая native buffer regression закрыла этот пробел. Это scoped CPU UV parity,
не полная pixel parity. Оценка остаётся около16%; missing tint/leaves отдельно.


MOVE-001 принят на clean объединённой ревизии `82f3930`. Snapshot agy
`3e10761`, viewer `215ad6e` + `4862262` (gpt-6-luna low); root reviewed and
integrated, без прямых правок engine Rust в этом пакете. 643 native сценария /
8213 состояний совпали по всем raw-bit полям. Полный workspace check (128 Python
cases, 7 opt-in skips) и отдельные 7 live tests прошли, как и все 10 прежних
strict suites. Сохранённый movement report `run-56tm9oak` имеет rust_dirty=false;
12 malformed executions и output/artifact/path/registry mutations проверены.
Viewer запускается с ходьбой, прыжком и sneak, 20Hz accumulator сохраняет
backlog, rendering интерполирует; Xvfb screenshot показывает сцену с земли.
Границы: статичная 5×5 сцена, коллизии поддержанных opaque full cubes, dry
physics; fluids/ladder/full player tick/health/interactions не заявляются.
Оценка полной игры — около 16%.


GAME-ORACLE-001 принят на clean объединённой ревизии `9347ae5`. Все 6284
native geometry cases совпали точно. Custom differential input обнаружил
реальную ошибку выбора ray face на тонком box при близких расстояниях; Rust
исправлен и native corpus расширен для X/Y/Z в обоих направлениях. Saved review
проверяет процессы, команды, code source, compiled adapter, Rust binary, source
и input/output hashes; 14 malformed executions отклонены. Полный workspace check
и все предыдущие strict suites прошли. EVIDENCE-001 независимо усилил
math/java evidence и прошёл live mutations/protocol. Геометрия готова к
использованию в физике; игрового шага пока нет. Оценка полной игры — около 12%.

JAVA-001: snapshot agy `73dac4598dd2d596c1158b640a2bf9f630950b3d` принят.
Rust casts и `String.hashCode` побайтно совпали с direct JVM oracle на 18 493
строках; `python3 tools/check.py` прошёл с 95 Python-тестами и всеми Rust
workspace checks. Saved review обнаружил 5 синтетических изменений и отклонил
10 некорректных JVM/Rust protocol inputs. Семь предыдущих strict suites также
прошли. Это узкий набор базовой Java-совместимости, поэтому округлённая оценка
полной игры остаётся около 11%.

MATH-001: agy snapshot `aaebed54d22a9149894792e305ed264d9a2b1a36` объединён и
принят. Rust MathHelper совпал с исходным классом Beta 1.7.3 по 131 408
операциям и raw IEEE-754 bits; `sin` и `cos` покрывают каждый из 65 536 LUT
индексов. Saved report `artifacts/parity/math/run-lgbtghq2/report.json` имеет
`rust_dirty=false`; protocol review проверил 10 отказов некорректных входов и
синтетические изменения type/bit/order/row count/source. `python3 tools/check.py`
прошёл с 89 Python-тестами и всеми Rust workspace, clippy, fmt и doc checks.
Семь предыдущих strict suites также прошли. Набор функций не включает игровой
цикл, генерацию или физику; готовность полной игры увеличена примерно на один
пункт до 11%.

VIEW-001 принят с clean agy snapshot `973de0be1bc36f20ce5d136475cd7226ff884ebb`.
`python3 tools/check.py` и все Rust all-target/doc, fmt/check/clippy проверки прошли.
Все семь strict suites на принятом снимке прошли: RNG 96, RNG-extended 13227,
chunk 4182, NBT 5250, region 748, world 749, view 50 mappings + 1617 neighbor
tuples. Для view выполнены saved protocol/hash review и Xvfb GUI smoke; локальный
скриншот `/tmp/beta-viewer-smoke.png` просмотрен. Renderer пока ограничен
фиксированной сценой и подтверждёнными полными opaque cubes; native brightness,
AO и pixel parity остаются вне покрытия. По пользовательскому кадру добавлены
tile-isolated atlas mipmaps для уменьшения minification aliasing; тесты и повторная
view parity прошли. Xvfb sandbox после этого не позволил сохранить новый кадр.

## Последняя независимая приёмка

COLOR/LEAF/LIGHT совместно приняты на чистой ревизии `755d6dd`.
Rust LIGHT `a39fda2`, renderer `8a12289`/`ec3759c`/`ff84905`/`2072705`.
Все72928 native light records exact; дополнительно2048 custom time rows и
emission-only singleton совпали. Saved evidence review +45 live protocol
executions PASS, проверены actual sources/revision/JAR/classes/binary/audits.
Нативный цвет вершины подтверждён504 packed submissions (12 renderer calls),
два независимых JVM запуска совпали. Графический путь сохраняет исходный порядок
f32 и byte quantization. Native leaf/UV, fresh COLOR1600 и legacy VIEW50/1617,
previous MATH131408/JAVA18493, полный check161 Python cases (17 skips)/148 Rust
прошли. Xvfb после финальной8-float GPU layout показывает зелёную траву/кроны.

Ограничения: фиксированная5×5 сцена из проверенного Overworld save, static saved
time, flat light, без AO/погоды/распространения света/полного pixel parity.
Nether проверен численно; DIM-1 regions viewer пока не загружает. Оригинальные
ресурсы/мир только читаются и отсутствуют в Git. Прямых правок Rust координатором
в этом пакете нет; реализация делегирована `gpt-6-luna low`.
Оценка≈20%, прирост≈4pp от последнего push16%; новый push пока не выполняется.

### Предыдущая приёмка COLOR-001

WORLD-LOAD-001: clean Rust snapshot `a7aa6856f81da7f3e008b4c1d7ad9ac89cffac18`,
объединённая ревизия `8292470d5cdcba84ad42527496226e2c1ca33f8d`; Rust↔Java
749/749 exact. `python3 tools/check.py` — 75 Python + 66 Rust tests,
fmt/check/clippy/doc PASS. Все пять прежних strict parity suites и новый world
suite PASS; полный world protocol/saved review и generated edges PASS. Отчёты и
артефакты перечислены в `docs/tasks/status/codex.json` и `docs/handoffs/codex.md`.

REGION-001: snapshot agy `4f6c41c6a1edef399ac4bfa001f6be13d5089158`,
C02 `e388657`, объединённая ревизия `1e076324e992412d396a15da72f808facb00632f`.
60 Python + 58 Rust tests, fmt/check/clippy/doc PASS. Все пять live-групп
PASS на этой чистой ревизии: RNG 96, extended 13227, chunk 4182, NBT 5250,
region 748. Region report: `artifacts/parity/region/run-pq6ftpqm/report.json`.
16 TSV executions, шесть report mutations, missing-probe/wrong-JAR/timeout
paths и 37 synthetic cases проверены отдельно. Исходные четыре `.mcr`, JAR
и обе копии входов неизменны. Подробные пути — `docs/handoffs/codex.md`.

## Предыдущая приёмка NBT-001

Snapshot agy: `541d719e83e32ad13e9448e102c8cc1ed16bcf40`.
Чистая проверенная объединённая ревизия:
`93ed4fe8356d9c30cfa9013bca111d61e75f0ebf` (включает отдельное исправление
инструментария Codex). Финальные live-отчёты имеют `rust_dirty=false`.

- `python3 tools/check.py`: 47 Python + 43 Rust all-targets,
  fmt/check/clippy `-D warnings` и отдельный doc — PASS.
- `cargo test --all-targets --locked --offline`: 43 PASS.
- `python3 tools/parity/run.py rng --strict`: 96/96 PASS,
  `artifacts/parity/rng/run-cxelcb7r/report.json`.
- `python3 tools/parity/run.py rng-extended --strict`: 13227/13227 PASS,
  `artifacts/parity/rng-extended/run-w6ixr3u6/report.json`.
- `python3 tools/parity/run.py chunk --strict`: 4182/4182 PASS,
  `artifacts/parity/chunk/run-rohs2w09/report.json`.
- `python3 tools/parity/run.py nbt --strict`: 5250/5250 PASS,
  `artifacts/parity/nbt/run-a56fm2ot/report.json`.
- `python3 tools/parity/verify_report.py artifacts/parity/nbt/run-a56fm2ot/report.json --protocol`:
  hashes/audit/canonical/consumed PASS, пять синтетических подмен обнаружены,
  86 дополнительных executions PASS. Native differences не считаются parity.

Оригинал NBT/CHUNK: клиент Beta 1.7.3, JAR SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
NBT реально вызывает `ij.b(DataInput)` и `ij.a(ij,DataOutput)` из JAR.
Корпус усилен 50 граничными случаями без удаления прежних 5200. Подробности:
`tests/fixtures/nbt/provenance.md`, `docs/handoffs/codex.md`.

## Ограничения и следующий шаг

NBT parity относится к общей области оригинального API: типы 0–10,
представимые в Rust String строки. IntArray — отдельное wire extension.
Пустой список, отрицательные длины, End/unknown list types и одиночные surrogate
имеют явно измеренные отличия от оригинала; требования безопасного Rust API
сохранены. Java DataOutput нормализует NaN; входные биты остаются в evidence.
Root-End/trailing-byte/TSV guards принадлежат адаптеру, не оригиналу.

Игровая логика, симуляция мира, запись сохранения, большая часть графики,
производительности и игровой цикл ещё не реализованы. VIEW-001 показывает только
фиксированный участок и не загружает игровой World в полноценный renderer.
Следующий пакет — FLUID-001: добавить Beta water ID 8/9 по прямым capture
Tessellator оригинального `cv` renderer. Вода занимает14 из50 уникальных native
состояний закреплённой сцены и пока пропускается viewer; lava и simulation в пакет
не входят. C01 завершён: 44 native cases / 1,242 captured vertices; следующий шаг —
strict runner C02 и pure Rust mesh A01.
исследование оригинальных методов записано в docs/view-002-research.md.
