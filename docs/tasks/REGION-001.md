# REGION-001 — read-only McRegion Beta 1.7.3

Статус: согласованный контракт после приёмки NBT-001 (`2930029`). Область —
чтение McRegion `.mcr` и получение decompressed NBT payload отдельного чанка.
NBT дерево уже читает модуль NBT-001. Пакет не пишет/исправляет мир, не читает
`level.dat`, не генерирует чанки, не строит рендер и не запускает сам Minecraft.

## Входной мир и сохранность данных

Локальная конфигурация `prism_instance` указывает на каталог Prism instance.
Read-only исследовательский мир находится в
`<prism_instance>/minecraft/saves/New World/region/`. При подготовке контракта
обнаружены четыре настоящих региона с 704 непустыми слотами, все с compression
id 2; первичная проверка header ranges не выявила повреждённых записей или
пересечений. Это характеристика конкретных входов, а не универсальное свойство
формата.

| Файл | Размер | SHA-256 |
| --- | ---: | --- |
| `r.-1.-1.mcr` | 761856 | `b41220e2e389bf303984fa7211192debc7e1db3e49b89c168101a3ed748c435f` |
| `r.-1.0.mcr` | 843776 | `e189d75077763fd00813f5929a14a9de85704b55ef8042afb8fd2fb2f5801004` |
| `r.0.-1.mcr` | 1019904 | `59ce84edc6afcec1f962e0d88f519e359137c7245b688a58ab340414b12eb0ba` |
| `r.0.0.mcr` | 782336 | `67f39f4c77538f178fa7acb155030908f84c079b70f27d44d85f4a20fe965b7a` |

Ни исходные файлы, ни `level.dat` не изменять. Для каждого файла helper сверяет
закреплённый SHA, хеширует источник до копирования, после копирования и после
запуска обоих процессов; любая разница завершает сценарий ошибкой. Каждый запуск
копирует входные регионы в собственный `artifacts/parity/region-*/input/`.
Копии и игровые
данные остаются вне Git. Сохранение можно проверять только из копии.

## Формат и область совместимости

Файл состоит из 4096-байтовых секторов. Первые два сектора заняты таблицей
locations и timestamps: 1024 big-endian location entries (24-bit offset + 8-bit
sector count), затем 1024 big-endian 32-bit timestamps. Для local chunk `(x,z)`:
`slot = x + z*32`, x/z ∈ 0..31. Нулевой offset означает отсутствующий чанк.

Запись по offset содержит big-endian u32 length (включает следующий byte
compression id) и один byte id, за которыми идут `length-1` compressed bytes.
Sector count должен вмещать заголовок и все заявленные данные; offset/count должны
лежать внутри файла и не использовать зарезервированные сектора 0 и 1. Переполнения,
пересечение записей, truncation, length<1 и неверное число секторов дают ошибку.
Номер региона вычисляется floor(chunk_coord/32), локальная координата — остаток
0..31 и для отрицательных мировых координат. Использовать `div_euclid/rem_euclid`
или эквивалентную безошибочную арифметику.

Java 1.7.3 native reader остаётся эталоном для поддержанных им случаев.
Codex C01 устанавливает по фактическому вызову оригинального метода, какие
compression ids он понимает. Ожидаемый id 2 — zlib; id 1 — gzip, только если
оригинал его читает; id 3/raw и другие IDs не считать поддержанными заранее.
Unknown/unsupported ids Rust отвергает с отдельной ошибкой. Синтетические записи
для каждой кодировки маркируются отдельно от live-данных сохранения.

Для bounded pure-Rust распаковки используется `flate2` с backend `rust_backend`
(miniz_oxide); интегратор фиксирует точную разрешённую версию и `Cargo.lock` в C01
до начала A01. Этот backend не требует C-компилятора и использует safe Rust;
[документация flate2](https://docs.rs/flate2/latest/flate2/) описывает gzip/zlib и
выбор backend. Записывать сжатые файлы в этом пакете не нужно, поэтому parity
не зависит от уровня компрессии и байтового оформления encoder.

Зафиксировано C01: `flate2 = "=1.1.9"`, `default-features=false`,
`features=["rust_backend"]`; backend `miniz_oxide 0.8.9`. Для SHA-256 TSV и
сохранённых payload probe добавлен только dev-dependency `sha2 = "=0.10.9"`
(`default-features=false`, `features=["std"]`); движку SHA-256 не требуется.
`Cargo.lock` содержит точные transitive versions/checksums. Все crates уже
доступны в локальном Cargo cache; `cargo check --workspace --all-targets --locked
--offline` проходит. На новой машине provisioning: `cargo fetch --locked`, затем
обычные `--locked --offline` проверки. Лицензии — provenance пакета.

Native evidence C01: настоящий класс `qj`, constructor `qj(File)`, reader
`a(int,int): DataInputStream`, close `b()`. Поддержаны **1=gzip, 2=zlib**;
3/raw и остальные ids отвергнуты. Проверены trailing compressed bytes: мусор после
полного zlib/gzip stream игнорируется; gzip с несколькими корректными members
возвращает конкатенацию payloads. Лимит 64 MiB применяется к суммарному выводу.
Повреждённые checksum и незавершённые поддержанные streams — ошибка.
`MultiGzDecoder` сам по себе может отвергать trailing garbage: реализация должна
сохранить фактически измеренную политику, не ослабляя проверку завершения members.
Sector padding не входит в compressed record и игнорируется.

Уточнение C03 по дополнительным вызовам оригинального qj: после завершённого
gzip member неполный/невалидный заголовок следующего member считается trailing
garbage (в том числе хвост `1f8b`). После полностью прочитанного следующего
заголовка усечение body/trailer и неверная checksum остаются ошибкой. Начальный
gzip header обязан быть корректным. Это поведение используемой Java JVM;
runner фиксирует её версию вместе с оригинальным JAR.

Оригинал допускает overlap и length, превышающий sector capacity на 1–4 bytes;
Rust обязан отвергать это по safe API выше. Эти два случая — отдельно помеченные
native differences, не основания считать повреждённый region корректным.

Лимиты reader: input region ≤256 MiB; compressed record не больше
`255*4096-5` байт; decompressed record ≤64 MiB. Проверять лимиты до allocation,
decode с output cap, переполнения арифметики и неполное/ошибочное завершение потока.
Trailing bytes внутри `length-1` сравнивать с поведением Java и документировать;
не путать их с sector padding после record.

## Rust API

Модуль `beta_core::region`:

```rust,ignore
pub struct RegionFile<'a> { /* borrowed input; parsed metadata */ }
pub struct RegionLimits { /* defaults above */ }
pub struct ChunkRecord { pub timestamp: u32, pub compression: u8, pub nbt: Vec<u8> }
pub enum RegionError { InvalidHeader, InvalidLocation, OverlappingSectors,
    InvalidLength, UnsupportedCompression(u8), Decompression, LimitExceeded }
impl<'a> RegionFile<'a> {
    pub fn open(bytes: &'a [u8], limits: RegionLimits) -> Result<Self, RegionError>;
    pub fn read_local(&self, x: u8, z: u8) -> Result<Option<ChunkRecord>, RegionError>;
}
pub fn region_coord(chunk_coord: i32) -> i32;
pub fn local_coord(chunk_coord: i32) -> u8;
```

Open validates header size, file alignment, every nonempty location, record length,
sector bounds, timestamps table and overlap before exposing the file. Absent slot
returns `Ok(None)`. Invalid local coordinate (x/z≥32) is an error. Read decompresses
to raw NBT bytes; parsing the NBT root remains a separate call to NBT-001. Do not
allocate a copy of the complete region per read, mutate input bytes, infer chunk
coordinates from array order, or silently return absent on corruption.

## Oracle, corpus, and probe

Codex C01 discovers and calls the actual original client class/method using the
original client JAR SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
Adapter records source file hash, region/chunk coordinate, native return state,
timestamp, compression id, decompressed payload length and SHA-256. If the native
method/class cannot be proven, mark live reference blocked; do not replace it with a
handwritten Java implementation of the Rust parser.

Probe `region_probe` reads copied `.mcr` files and the exact same coordinate list.
CLI: `region_probe INPUT_COPY_DIR PAYLOAD_DIR`; сохранять каждый present payload
как `PAYLOAD_DIR/<case_id>.nbt` (пустые слоты файла не создают). Каталоги разделены,
файлы payload не перезаписывать. Snapshot helper создаёт в input marker
`.region-input-copy` с текстом `REGION-001 copied inputs\n`; probe требует этот
marker, чтобы не читать исходное сохранение по ошибке. Java вызывается как
`RegionOracle JAR INPUT_COPY_DIR PAYLOAD_DIR AUDIT_JSON`.
Use TSV `case_id<TAB>region_file<TAB>chunk_x<TAB>chunk_z`; `region_file` is a basename
matching the region coordinates derived from chunk_x/chunk_z. Resolve it only within
the per-run copied input directory; reject traversal/absolute paths. Output per case:
`case_id<TAB>present|absent<TAB>timestamp<TAB>compression<TAB>nbt_len<TAB>nbt_sha256`.
For absent records numeric fields are `-`; corrupted file/error is nonzero/stderr. Preserve input
order and LF. Raw NBT payloads are saved in the report artifact so a hash mismatch
can be inspected without putting world bytes in Git.

Fixture corpus includes all 704 nonempty slots and a sample of empty slots from all
four real files, region edges, negative world coordinates, and cross-checks of
`region_coord/local_coord` near -33,-32,-31,-1,0,31,32,33 and i32 boundaries.
Record counts and fixture coordinate list derive from the input world snapshot;
the Java and Rust sides receive identical files and coordinates. Additional
synthetic `.mcr` inputs cover compressed record length boundaries, sector padding,
unknown compression, truncation, invalid offsets/counts, overlaps, trailing compressed
bytes, and output bombs. Synthetic results are not included in the 704 real live
case count.

## Роли и приёмка

Codex C01 — pin flate2/lockfile, prove original adapter, snapshot helper and corpus.
Codex C02 — runner group `region`, path validation, preserved file copies, report and
mutation/protocol tests. agy A01 — safe Rust reader + bounded decompression and unit
tests. agy A02 — probe. Codex C03 integrates exact agy SHA and accepts.

On merged revision run `python3 tools/check.py`, `cargo test --all-targets --locked
--offline`, previous `rng`, `rng-extended`, `chunk`, `nbt`, and
`python3 tools/parity/run.py region --strict`. All 704 nonempty chunks in the four
provided real regions and agreed empty samples must match the native Java reader
exactly in presence, timestamp, compression id, uncompressed bytes (reported by
SHA-256 and length). The four source hashes before/after run must remain unchanged.
Synthetic malformed protocol/decompression tests must reject within limits. Missing
save/JAR/original method, wrong hash, timeout or build error is not `pass`.
Acceptance means region-file read parity for these files; it does not mean a complete
world is loaded or saved.
