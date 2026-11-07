# CHUNK-001 — координаты и хранение чанка Beta

Статус: **accepted (C03)** после принятого RNG-002.
Snapshot agy `14b50482bc1fa9ef4850d7983780ceda876bb80a`; проверенная чистая
ревизия `6b0bdfc26a5c53934da9980c4a3ed5051b7910bc`. Все 4182 операции совпали
с оригинальным nibble-классом, прежние RNG-группы и обычные проверки прошли.
Отчёт `artifacts/parity/chunk/run-jvj35_be/report.json`; происхождение/имена/хеши
в `tests/fixtures/chunk/provenance.md` и handoff Codex. Синтетические подмены,
протокол, missing JAR/reflection error обнаружены. Коммит приёмки с этим статусом
включается в main. Покрытие — контейнер, не загрузка мира или рендер.
Пакет создаёт контейнер данных без NBT, файлов, света по тикам и генерации.
Структура исходного формата: 16×128×16, Blocks=32768 байт, Data/BlockLight/SkyLight
по 16384 байта. Это описание задачи; подлинность nibble-порядка Codex проверяет
вызовом оригинального класса из локального клиентского JAR.

## API Rust

Модуль `beta_core::chunk`, без новых зависимостей:

```rust,ignore
pub struct ChunkData { /* поля закрыты */ }
pub struct BlockData {
    pub id: u8, pub metadata: u8, pub block_light: u8, pub sky_light: u8,
}
pub enum ChunkError { InvalidCoordinate, InvalidNibble, InvalidLength }
pub fn split_block_coord(value: i32) -> (i32, u8);
pub fn block_index(x: u8, y: u8, z: u8) -> Result<usize, ChunkError>;
impl ChunkData {
    pub fn new() -> Self;
    pub fn from_raw(blocks: Vec<u8>, data: Vec<u8>, block_light: Vec<u8>,
                    sky_light: Vec<u8>) -> Result<Self, ChunkError>;
    pub fn get(&self, x: u8, y: u8, z: u8) -> Result<BlockData, ChunkError>;
    pub fn set(&mut self, x: u8, y: u8, z: u8, value: BlockData)
        -> Result<(), ChunkError>;
    pub fn raw_arrays(&self) -> (&[u8], &[u8], &[u8], &[u8]);
}
```

BlockData — Clone/Copy/Debug/PartialEq/Eq; ChunkError — Debug/PartialEq/Eq;
ChunkData реализует Default как new. new обнуляет все четыре массива, включая
sky light: это контейнер, а не уже освещённый сгенерированный чанк.
Индекс `y + z*128 + x*2048`; x,z<16, y<128. Чётный индекс — младшие четыре бита,
нечётный — старшие. Все nibble значения 0..15. set сначала проверяет все поля
и не меняет ни одного байта при ошибке. from_raw проверяет все длины.
ID пока сырой u8, без предположений о свойствах блока и без реестра.
`split_block_coord` возвращает floor(value/16) и остаток 0..15, включая i32::MIN;
не выполнять потенциально переполняющееся умножение в i32 при обратной проверке.

## Probe

Пример `chunk_probe`, Java-класс `ChunkOracle`, группа runner `chunk`.
Вход — TSV `case_id<TAB>operations`; ID/комментарии/пустые строки — как RNG-001.
На строку — новый нулевой чанк. Токены через `;`, 1..100000 операций и максимум
100000 операций на вход. Все числа десятичные; неизвестный токен/неверный диапазон
числа — stderr/nonzero. Поля координат probe разбирает в u8, ID в u8, nibble в u8.

| Токен | Выходное operation_name | Выходное value |
| --- | --- | --- |
| `coord:V` (V signed i32) | `coord` | `chunk,local` decimal |
| `index:X:Y:Z` | `index` | индекс или `invalid_coordinate` |
| `get:X:Y:Z` | `get` | `id,metadata,block_light,sky_light` или `invalid_coordinate` |
| `set:X:Y:Z:ID:M:B:S` | `set` | `ok`, `invalid_coordinate` или `invalid_nibble` |
| `byte:A:I` | `byte` | raw u8 decimal; A=`blocks`, `data`, `block_light`, `sky_light` |

Для byte I — целое 0..длина-1, нарушение — ошибка протокола stderr/nonzero.
Для set с одновременно неверными координатами и nibble первична InvalidCoordinate.
Каждая операция даёт `case_id<TAB>zero_based_index<TAB>operation_name<TAB>value`, LF.
Ошибки get/set/index — валидные операции и не завершают цепочку.
from_raw проверяется обычными Rust-тестами и не нуждается в отдельном токене.

## Независимость проверки

Codex C01 готовит corpus `tests/fixtures/chunk/layout.tsv`, Java-адаптер и запись
происхождения. Для coord использует арифметику Java (`>> 4`, `& 15`); для nibble
находит оригинальный класс в client JAR через javap/исследование и вызывает его
конструктор, get/set и raw-массив через reflection. Обёртка проверяет границы API,
поскольку оригинальный внутренний метод может не иметь этих защитных проверок.
В отчёте отдельно помечаются оригинальные вызовы и проверки нашей обёртки.
Публикуются адаптер/имена/hash, исходный JAR и декомпилированный код в Git не входят.
Если адаптер не получен, C01 blocked/error; вручную переписанная Java-формула
не выдаётся за вызов оригинального nibble-класса. Нужно разрешить runtime
зависимости оригинального класса локально; их отсутствие — явная ошибка.

Корпус: координаты -17,-16,-15,-1,0,1,15,16,17 и границы i32; крайние x/y/z;
оба nibble одного байта, порядок разных x/z, ID 0/127/128/255; каждый nibble 0..15;
повторные записи, проверка соседей, чтение raw-байтов, отказ без изменения данных.
Не менее 1000 операций, включая последовательность записей на границах колонок.
agy A01 реализует модуль и его инварианты, A02 — probe. Codex C02 расширяет runner,
C03 интегрирует; движок agy не пишет Java и не правит fixture/компаратор.

## Приёмка

`python3 tools/check.py`, `cargo test --all-targets --locked --offline` и прежние
группы `rng`/`rng-extended` проходят на объединённой ревизии.
`python3 tools/parity/run.py chunk --strict` действительно вызывает оригинальный
nibble-класс и Rust probe, все ≥1000 операций совпадают точно. Сохраняются входы,
выходы, хеш JAR, использованные имена классов/методов, версии и ревизии.
Синтетические изменения nibble-порядка/индекса или одного результата обнаруживаются.
Неверный протокол, отсутствующий JAR, ошибка reflection и timeout не дают pass.
Обычные тесты дополнительно проверяют длины from_raw и атомарность set при ошибке.
Принятие означает совместимость контейнера данных, а не загрузку мира или рендер.
