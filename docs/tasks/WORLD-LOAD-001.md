# WORLD-LOAD-001 — read-only headless world data loading

Статус: принят в `main` после REGION-001 (`37d64c9`), revision
`8292470d5cdcba84ad42527496226e2c1ca33f8d`. Реализована загрузка `level.dat`
и семантическое чтение сохранённых чанков через уже принятые gzip/NBT/McRegion.
Результат — пригодные для клиента параметры мира, `ChunkData`, height map и
сохранённые NBT-записи игрока/сущностей/tile entities. Окно, тики, генерация,
создание игровых объектов, исправление повреждённых данных и запись мира вне пакета.

## Сохранность и происхождение

Исходный мир: `<prism_instance>/minecraft/saves/New World/`. Четыре региона и
JAR закреплены REGION-001. Дополнительно закрепляем `level.dat`: 411 bytes,
SHA-256 `2711d16242afd13909575955d4a34502da01f6f666ee79341fe1c1ba96c8a960`.
Сохранение читается только для хеширования/копирования. Любые Java вызовы и Rust
loader запускаются на независимых копиях в per-run `artifacts/`.

Helper проверяет пять source hashes до/после копирования и в finally после
любого завершения процессов; обе копии также неизменны. В input copy лежат
`level.dat` и четыре basename `r.*.*.mcr` рядом, независимо от расположения
регионов в оригинале. Marker `.world-input-copy` содержит
`WORLD-LOAD-001 copied inputs\n`; дополнительно `.region-input-copy` сохраняет
текст REGION-001. Никаких symlinks, traversal, перезаписи snapshots/payloads.
Бинарники, сохранения, raw NBT и javap/decompiled outputs остаются вне Git.
Missing source/JAR/class/hash, ошибки сборки и timeout не считаются pass.

## Native reference и область доказательства

Оригинальный JAR SHA-256:
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
Metadata эталон — настоящий `ei(nu)`, созданный из native compound `Data`;
геттеры/поля и SHA класса фиксирует C01. NBT разбирает оригинальный `ij`, регионы
читает настоящий `qj.a(int,int)`. Chunk summary извлекается native `nu` accessors
по полям оригинального storage format; это **NBT data extraction**, а не вызов
полного `to.a(fd,nu)`/создание `fd` World. Entity/tile factories не запускаются.
Отчёт обязан различать эти виды native вызовов: `native_chunk_loads=0`.
Привязку реального класса/JAR и количество вызовов сохранять в audit.

Игровой chunk loader может восстанавливать массивы или перемещать несовпадающий
чанк, вызывая World. Safe loader этого пакета выдаёт ошибку. Такие generated cases
проверяются отдельно как safe API, не объявляются равной игровой parity.
Не подменять native reference ручной реализацией Rust-алгоритма на Java.

Исследование до C01 подтвердило `as.a(InputStream)` для compressed NBT, constructor
`ei(nu)` и getters: seed b, spawn c/d/e, time f, size g, Player h, dimension i,
name j, version k, lastPlayed l, thundering m, thunderTime n, raining o, rainTime p.
Для raw retained bytes adapter может bounded-decompress Java gzip, затем передать
точные bytes native `ij.b(DataInput)`; доказать этот native call в audit.
`ei.a()` не вызывать: serializer заменяет LastPlayed текущим временем.
`fm` save handler не создавать: constructor пишет session.lock. `nu.e/f/i/m/j`
дают соответственно int/long/string/bool/byte array; missing scalars имеют
0/empty defaults, wrong types бросают ClassCastException, numeric coercion нет.
Nonzero TAG_Byte (включая 0x80) означает true. Player entry count — число
именованных tags в compound Player, **не** размер Inventory; native map можно
инспектировать reflection без собственных игровых объектов.

## Metadata, chunk fields и ошибки

Level root — compound с compound `Data`. Параметры:
`RandomSeed:Long`, `SpawnX/Y/Z:Int`, `Time:Long`, `LastPlayed:Long`,
`SizeOnDisk:Long`, `LevelName:String`, `version:Int`, `rainTime:Int`,
`raining:Byte`, `thunderTime:Int`, `thundering:Byte`.
Отсутствующие scalars используют измеренные defaults native `ei`/`nu` (ожидаемо
0/false/empty string); неверные типы не приводить молча. Boolean — byte != 0.
`Player` отсутствует либо compound; сохраняется целиком, включая inventory,
позицию и неизвестные поля. Dimension берётся из `Player.Dimension:Int`, по
native policy отсутствующего Player/Dimension. Никакой симуляции игрока.

Chunk root содержит compound `Level`; обязательны `xPos:Int`, `zPos:Int`,
`Blocks:ByteArray[32768]`, `Data`, `BlockLight`, `SkyLight` по 16384 bytes,
`HeightMap:ByteArray[256]`. Координаты обязаны совпадать с запрошенным слотом.
`LastUpdate:Long` и `TerrainPopulated:Byte` допускают native missing default.
`Entities`/`TileEntities` отсутствуют либо lists; непустые списки содержат compounds,
пустые допускают любой поддержанный element type по NBT-001. Сохранять их как NBT,
включая неизвестные IDs/поля; не создавать игровые сущности. Все прочие root/Data/
Level tags сохраняются в `root` и исходном `raw_nbt`; перечислить неизвестные
поля Data/Level в summary. Неизвестный tag **name** допустим; неизвестный wire tag
type остаётся ошибкой существующего NBT reader.

Missing/wrong type обязательного chunk field, неверные array lengths/coordinates,
malformed NBT/gzip и limits дают явные ошибки без частично загруженного чанка.
Absence slot остаётся `None`; corruption не превращается в absence. Нельзя
пересчитывать свет/height map или заменять массивы нулями для прохождения.

## Rust API и лимиты

Новый pure module `beta_core::world` использует существующие NBT/ChunkData/Region.
Без новых dependencies и без файловой системы в core:

```rust,ignore
pub struct WorldLimits {
    pub nbt: nbt::Limits,
    pub max_level_compressed_bytes: usize, // 16 MiB
    pub max_level_decompressed_bytes: usize, // 64 MiB
}
pub struct WorldMetadata {
    pub seed: i64, pub spawn: [i32; 3], pub time: i64,
    pub last_played: i64, pub size_on_disk: i64, pub name: String,
    pub version: i32, pub dimension: i32,
    pub rain_time: i32, pub raining: bool,
    pub thunder_time: i32, pub thundering: bool,
    pub player: Option<nbt::Tag>,
    pub root: nbt::NamedTag, pub raw_nbt: Vec<u8>,
}
pub struct LoadedChunk {
    pub x: i32, pub z: i32, pub last_update: i64,
    pub terrain_populated: bool, pub data: chunk::ChunkData,
    pub height_map: [u8; 256],
    pub entities: Vec<nbt::Tag>, pub tile_entities: Vec<nbt::Tag>,
    pub root: nbt::NamedTag, pub raw_nbt: Vec<u8>,
}
pub enum WorldError { /* Nbt/Compression, MissingField, InvalidFieldType,
    InvalidArrayLength, CoordinateMismatch, LimitExceeded; Display/Error */ }
pub fn load_level_gzip(bytes: &[u8], limits: WorldLimits)
    -> Result<WorldMetadata, WorldError>;
pub fn load_chunk_nbt(bytes: Vec<u8>, expected: (i32, i32), limits: nbt::Limits)
    -> Result<LoadedChunk, WorldError>;
```

`Default` WorldLimits указан выше. Для gzip разрешено открыть существующий
region decompressor как `pub(crate)` и переиспользовать tested bounded path,
сохраняя прежний публичный API/ошибки. Лимиты проверяются до allocation; NBT
полностью потребляет decompressed payload. Более строгие limits/gzip guards
Rust не приписывать native constructor. Полный root и исходные raw bytes не
модифицировать при извлечении полей. Отдельные getters unknown names допустимы.

## Probe protocol

CLI Rust: `world_probe INPUT_COPY_DIR RETAINED_DIR`.
Java: `WorldOracle JAR INPUT_COPY_DIR RETAINED_DIR AUDIT_JSON`.
Reader обрабатывает chunks последовательно, region bytes допускается cache;
не накапливать все LoadedChunk до конца потока. Snapshot directories разделены
с output, payload создаётся исключительно новым файлом `<case_id>.nbt` для каждого
level/present chunk. Это точные **decompressed original bytes**, не NBT re-encoding.
Absent payload не создаётся. Raw retained bytes позволяют проверить сохранность
всех полей, включая неизвестные, без потерь Java writer normalization.

stdin — пять TSV fields:

```text
case_id\tlevel\tlevel.dat\t-\t-
case_id\tchunk\tr.<rx>.<rz>.mcr\tchunk_x\tchunk_z
```

IDs уникальны, `[A-Za-z0-9_-]+`, лимит 100000 cases; пустые строки и `#` comments
пропускаются, whitespace-only/лишние fields/неверные coordinates отвергаются.
Coordinate syntax `-?(0|[1-9][0-9]*)`, i32, `-0` допустим. Region basename точно
соответствует floor division, без absolute/traversal/symlink paths. Для level
допустим только `level.dat` и два `-`. Поддержка `level.dat_old` fallback не входит
в текущий read-only adapter. Ошибки — nonzero/stderr, результаты — LF stdout.

Level output: **21 fields**, именно в порядке:
`id, level, seed, spawn_x, spawn_y, spawn_z, time, last_played, size_on_disk,
dimension, name_utf8_hex, version, rain_time, raining, thunder_time, thundering,
player_present, player_entry_count, unknown_data_names, raw_nbt_len, raw_nbt_sha256`.

Chunk output: **17 fields**, именно в порядке:
`id, chunk, present|absent, x, z, last_update, terrain_populated, blocks_sha256,
data_sha256, block_light_sha256, sky_light_sha256, height_map_sha256, entities_count,
tile_entities_count, unknown_level_names, raw_nbt_len, raw_nbt_sha256`.
Absent: x/z остаются координатами запроса, остальные 12 fields — `-`.

Все hashes lowercase 64 hex. Integers decimal без plus/leading zeros; bools 0/1.
Empty name UTF8 hex — `-`. Unknown name set кодирует каждое UTF8 имя lowercase hex,
сортирует hex strings и соединяет запятой; пустой набор `-`, пустое имя `~`.
Known Data names — перечисленные metadata fields плюс Player. Known Level names —
перечисленные chunk fields (включая Entities/TileEntities); например TileTicks,
если присутствует, сохраняется и отражается как unknown, не исполняется.

## Corpus, очередь и приёмка

C01 фиксирует native API/defaults/code source/class hashes, five-file snapshot
helper/manifest и coordinates-only fixture: один level + прежние 748 slots,
итого **749 live rows**. До начала A01 подтвердить детали native defaults/type
errors, при необходимости уточнить этот контракт общим коммитом.
Generated metadata/chunks, unknown fields/list contents, malformed types/arrays,
coordinate mismatch, missing Data/Level, gzip limits/truncation и path errors
идут отдельно. Хеши/вывод Rust не используются для создания Java reference.

Пять карточек: C01 oracle/snapshot/corpus → C02 runner/review;
A01 core loader после C01 → A02 probe; C03 зависит от всех четырёх.
Перед началом зависимых карточек контракт уже закоммичен и объединён исполнителями.
Для новой очереди сохраняются все прежние 25 done statuses.

C03 проверяет конкретный agy SHA и merged revision, `python3 tools/check.py`
(включает all-targets), прежние rng/rng-extended/chunk/nbt/region strict suites,
новый `python3 tools/parity/run.py world --strict`, saved report/protocol review,
original source/copy hashes, sensitivity mutations и generated edge cases.
749 live rows должны совпасть точно. Отчёт фиксирует чтение raw region + native
NBT extraction и metadata constructor, не заявляет вызов полноценного World.
При успехе пакет принимается в main; обновляется `docs/completion-estimate.md`
и финальный отчёт пользователю завершается приблизительным общим процентом.
