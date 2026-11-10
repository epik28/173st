# WATER-MOVE-001 — движение игрока в воде

`MOVE-001` пока реализует только сухое движение: water/lava исключены из
environment queries. Это отдельный контракт от распространения block ID 8/9
в [WATER-SIM-001](WATER-SIM-001.md).

## C01 — native движение в жидкости (Codex)

Вызвать оригинальные Beta 1.7.3 player/entity methods на synthetic World и
сравнить tick-by-tick состояние при входе в воду, погружении, плавании вверх/
вниз, диагональном движении, отпускании клавиш, выходе на берег, поверхности
воды и столкновении со стеной/дном. Зафиксировать точные native environment
queries, friction/acceleration/drag/gravity, water height/eyes-in-fluid и
состояния прыжка. Пинить классы, поля, invocation counts и raw float/double
state. Lava и drowning/health исключить, если отдельный native корпус их не
добавит.

### C01 evidence — bounded in-memory player travel

The pinned client adapter is `tools/oracle/src/WaterMovementOracle.java`; run
`python3 tools/oracle/water_movement_corpus.py` to regenerate both native JVM
captures, compare them byte-for-byte with the retained fixture, and verify the
audit. The input/output and audit are in `tests/fixtures/water_movement/`.
The committed corpus has seven scenarios and 72 ordered states: crossing into a
pool from the side, submerged diagonal travel and release, jump impulse and
coast, wet feet with eyes above the surface, leaving water onto a shore, a wall
and floor, and metadata-level water. Each state stores the complete base player
position, velocity, bounding box, movement flags, water-contact result, eyes-in-
water result, and native water-query counts. Position/velocity/bounds are raw
f64 words and native float fields are raw f32 words.

The adapter constructs the native `gs` player base over an Unsafe-allocated
synthetic `fd` world, so world/chunk constructors and saved data are not used.
The client’s actual `sn.U()V`, `sn.k_()Z`, `sn.a(Lln;)Z`, `fd.a(Leq;Lln;Lsn;)Z`,
`gs.a_(FF)V` (which delegates to `ls.a_(FF)V`), `ls.R()V`, `rp.d(I)F`, and water
block flow callback are executed. The native fluid-volume query itself runs
from the JAR; only block-ID, metadata, and loaded-region answers are supplied by
the sparse fixture grid. Sound/particle sinks suppress environmental effects.
The audit pins the whole JAR, all relevant original classes, raw Code-byte hashes
for the listed methods, code sources, exact invocation totals, both run hashes,
and input/output hashes. Python tests validate the fixture schema, edge-state
coverage, method/class provenance, and hashes.

Bytecode inspection of the pinned `ls.a_(FF)V` water branch records this
operation order: apply input acceleration with `0.02f`, resolve movement through
`sn.b(DDD)V`, multiply each velocity component by the widened float constant
`0.800000011920929d`, then subtract `0.02d` from vertical velocity. If the
horizontal-collision branch's future-box query succeeds, it sets vertical
velocity to `0.30000001192092896d`. `ls.R()V` assigns the jump impulse
`0.41999998688697815d`. These are observations of Beta 1.7.3 bytecode only; the
corpus does not generalize them to other liquid branches or client scheduling.

Scope is the player base movement wrapper, not the `dc` GUI-bound local-player
controller or keyboard event scheduler. Each recorded tick applies `sn.U()`, an
optional direct native `ls.R()` jump call, native `gs.a_(FF)` travel, then the
native eyes-in-water query. It does not claim the complete client update order,
keyboard jump gating, drowning/health, lava, fluid flow ticks, or multiplayer
movement. The synthetic world uses fixed water block IDs 8/9 and solid AABBs;
other world block behavior is outside this corpus. This is native movement
evidence, not yet a Rust comparison.

## A01 — Rust реализация (agy)

Расширить pure motion API отдельным водным режимом, сохраняя сухие native
регрессии. Принимать воду как ограниченный запрос окружающих блоков; не читать
мир внутри core physics. Копировать operation order и casts из подтверждённого
bytecode/native cases. Неподдержанные блоковые формы не считать водой.

## C02 — сравнение и интеграция

Сравнить каждый frame/state с оригиналом, затем подключить в viewer controller.
Проверить погружение, поверхность, берег, углы, focus pause и устойчивость при
20/30/60/144 Hz frame partition. Не считать визуальную волновую анимацию или
fluid block ticks доказательством движения игрока в воде.
