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
