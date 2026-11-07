# Начало работы: 173st

Открыть проект `/home/andrew/173st` в Codex и в Antigravity.
Рабочая папка одна; изменения выполняются по очереди в отдельных ветках.
Репозиторий: <https://github.com/epik28/173st>.

## Первая очередь Codex

> Прочитай AGENTS.md и указанные в нём документы. Займи сессию командой
> `python3 tools/agent_session.py start codex`. Реализуй Java-эталон и runner RNG-001
> по контракту. Rust-код делает Gemini. Выполни доступные проверки, оставь handoff,
> сделай коммит и заверши сессию командой `python3 tools/agent_session.py finish codex`.

## Первая очередь Gemini / agy

> Прочитай GEMINI.md и указанные в нём документы. Займи сессию командой
> `python3 tools/agent_session.py start agy`. Реализуй Rust JavaRandom и rng_probe
> по контракту RNG-001. Java-эталон и runner делает Codex. Не меняй эталоны.
> Выполни доступные проверки, оставь handoff, сделай коммит и заверши сессию
> командой `python3 tools/agent_session.py finish agy`.

Если занятие сессии отказало, папка уже используется: второй агент ожидает своей очереди.
Обмен заданиями между приложениями ведётся через пользователя, документы и коммиты;
автоматический запуск одного приложения другим не настраивался.

## Проверки

```bash
python3 tools/agent_session.py status
python3 tools/doctor.py
python3 tools/check.py
git status --short --branch
git branch -vv
```

Общие требования: [docs/project-requirements.md](docs/project-requirements.md).
Роли, ветки и интеграция: [docs/workflow.md](docs/workflow.md).
Состояние проекта: [docs/progress.md](docs/progress.md).

Клиентский JAR из Prism подготовлен в игнорируемом `.local/minecraft/`.
Игровые механики и runner сравнений ещё предстоит реализовать.

При подготовке чтение GitHub проверено. HTTPS-авторизация для отправки изменений
в существующем credential store не найдена. Локальная работа веток доступна;
для push нужно подключить GitHub через credential helper или SSH.
