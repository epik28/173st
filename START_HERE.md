# Старт двух агентов: 173st

Открыть одновременно:

- Codex: `/home/andrew/173st/codex`, ветка `codex/verification`.
- Gemini / Antigravity: `/home/andrew/173st/agy`, ветка `agy/development`.

Это два worktree одного репозитория. Каждый работает в своей папке; ждать очередь
другого агента не требуется. Репозиторий: <https://github.com/epik28/173st>.

## Запрос Codex

> Прочитай AGENTS.md, общие требования, workflow, progress и задачу RNG-001.
> Выполни `python3 tools/agent_session.py start codex`.
> Реализуй настоящий Java-эталон, runner и проверки компаратора по контракту.
> Rust RNG и rng_probe одновременно реализует Gemini в другом worktree; их не пиши.
> Пока Rust-часть отсутствует в твоей ветке, явно сообщай, что сравнение не выполнено.
> Проверь свою часть, запиши handoff, сделай коммит, заверши локальную сессию.
> Сообщи commit SHA. Сравнение полного результата выполним после интеграции обеих частей.

## Запрос Gemini / agy

> Прочитай GEMINI.md, общие требования, workflow, progress и задачу RNG-001.
> Выполни `python3 tools/agent_session.py start agy`.
> Реализуй Rust JavaRandom и rng_probe по контракту RNG-001.
> Codex одновременно пишет Java-эталон и runner в другом worktree.
> Не меняй эталоны, runner, общий контракт и зависимости. Не используй незакоммиченные
> файлы соседнего worktree. Выполни `python3 tools/check.py`, запиши handoff,
> сделай коммит и заверши локальную сессию. Сообщи commit SHA.
> Если runner пока отсутствует в твоей ветке, явно укажи, что Rust↔Java ещё не проверено.

## После двух результатов — Codex

> Прочитай оба handoff по их коммитам. Зафиксируй SHA результата Gemini,
> просмотри diff и интегрируй его в своей папке. Запусти общие проверки и
> `python3 tools/parity/run.py rng --strict`. При успехе интегрируй в main
> и обнови прогресс. При ошибке передай Gemini входы и фактическое расхождение.

## Команды внутри каждого worktree

```bash
python3 tools/agent_session.py status
python3 tools/doctor.py
python3 tools/check.py
git status --short --branch
```

Документы: [требования](docs/project-requirements.md), [workflow](docs/workflow.md),
[прогресс](docs/progress.md). Локальные JAR и источники лежат в родительском
`../.local/`, результаты сборки и проверки — отдельно в каждой папке.

SSH GitHub проверен для epik28. Подготовленные коммиты пока локальные.
Реализация игровых механик и сравнительный runner ещё впереди.
