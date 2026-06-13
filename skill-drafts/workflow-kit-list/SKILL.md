---
name: workflow-kit-list
description: "Показ доступных к реализации Workflow Kit specs/workspaces по ai/specs/*: READY/BLOCKED/DONE/LEGACY/BROKEN, выбор следующей pending task из tasks.json. Используй когда спрашивают что можно реализовать, какие спеки доступны, что следующее в workflow-kit."
metadata:
  title: "Список доступных specs"
---

# Workflow Kit: list

Покажи список Workflow Kit workspaces и задач, доступных к реализации.

Источник истины — только файлы workspace в `ai/specs/*`:

- `spec.md`
- `plan.md`
- `scope.md`
- `tasks.json`
- `verification.md`

Не составляй ручной реестр. Он протухает и начинает врать.

## Быстрый запуск

Из корня репозитория:

```bash
python3 .agents/skills/workflow-kit-list/workflow_list_specs.py
```

Полный список, включая завершённые:

```bash
python3 .agents/skills/workflow-kit-list/workflow_list_specs.py --all
```

Машинно-читаемый вывод:

```bash
python3 .agents/skills/workflow-kit-list/workflow_list_specs.py --json
```

Только конкретные статусы:

```bash
python3 .agents/skills/workflow-kit-list/workflow_list_specs.py --status READY --status BLOCKED
```

## Статусы workspace

- `READY` — есть хотя бы одна `pending` task, у которой все `dependsOn` имеют статус `done`/`skipped`, и `confirmationRequired: false`.
- `BLOCKED` — незавершённые задачи есть, но runnable pending tasks нет: зависимости не закрыты, требуется подтверждение, задача `blocked`/`in_progress` или статус нераспознан.
- `DONE` — все задачи имеют статус `done` или `skipped`.
- `NEEDS_PLAN` — есть `spec.md`, но не хватает `plan.md` и/или `scope.md` до этапа tasks.
- `NEEDS_TASKS` — есть `spec.md`, `plan.md`, `scope.md`, но нет `tasks.json`.
- `LEGACY` — найден старый `tasks.md`, но нет `tasks.json`; такой workspace нельзя честно отдавать в `workflow-kit-implement` без миграции.
- `BROKEN` — битый `tasks.json`, отсутствуют обязательные файлы при наличии `tasks.json`, нет `spec.md`, или структура задач невалидна.

## Как отвечать пользователю

1. Запусти scanner.
2. Покажи `READY` первым.
3. Для каждого `READY` назови progress, первые runnable tasks и команду запуска.
4. `BLOCKED` показывай кратко: причина блокера и что закрыть.
5. `DONE` не расписывай без запроса; scanner по умолчанию скрывает done-workspaces и показывает счётчик.
6. Если пользователь просит начать реализацию, передай точный workspace/task в `workflow-kit-implement`.

Пример:

```text
READY
- 20260608_1745_ui-review-fixes — 0/8 done
  next:
    T001 P0 — Стабилизировать UI smoke-тесты
    T002 P0 — Синхронизировать денежные поля
  command: workflow-kit-implement 20260608_1745_ui-review-fixes T001
```

## Правила

- Не выбирай workspace автоматически, если READY несколько — попроси выбор.
- Не считай `tasks.md` готовым к реализации. Это legacy-формат.
- Не ставь задачу runnable, если есть незакрытая зависимость или `confirmationRequired: true`.
- Не меняй workflow artifacts во время list. Это read-only этап.
