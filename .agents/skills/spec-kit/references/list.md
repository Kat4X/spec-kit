# Spec Kit: list

Покажи список Spec Kit workspaces и задач, доступных к реализации.

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
workflow-kit list
```

Полный список, включая завершённые:

```bash
workflow-kit list --all
```

Машинно-читаемый вывод:

```bash
workflow-kit list --json
```

Только первый runnable task без вывода всего `tasks.json`:

```bash
workflow-kit next-task ai/specs/2026.06.08_17:45_ui-review-fixes
```

Только конкретная task без вывода всего `tasks.json`:

```bash
workflow-kit next-task ai/specs/2026.06.08_17:45_ui-review-fixes --task-id T003
```

Без аргумента берётся первый READY workspace:

```bash
workflow-kit next-task
```

Детерминированная очередь для batch/yolo с учётом `execution.order`, зависимостей и подтверждений:

```bash
workflow-kit batch-queue ai/specs/2026.06.08_17:45_ui-review-fixes
```

Только конкретные статусы:

```bash
workflow-kit list --status READY --status BLOCKED
```

## Статусы workspace

- `READY` — есть хотя бы одна `pending` task с закрытыми `dependsOn`, для которой подтверждение не требуется или `confirmation.status: "approved"`.
- `BLOCKED` — незавершённые задачи есть, но runnable pending tasks нет: зависимости не закрыты, требуется подтверждение или задача имеет `blocked`/`in_progress`.
- `DONE` — все задачи имеют статус `done` или `skipped`.
- `NEEDS_PLAN` — есть `spec.md`, но не хватает `plan.md` и/или `scope.md` до этапа tasks.
- `NEEDS_TASKS` — есть `spec.md`, `plan.md`, `scope.md`, но нет `tasks.json`.
- `LEGACY` — найден старый `tasks.md`, но нет `tasks.json`; такой workspace нельзя честно отдавать в `spec-kit implement` без миграции.
- `BROKEN` — битый `tasks.json`, отсутствуют обязательные файлы при наличии `tasks.json`, нет `spec.md`, или строгая schema v1/v2 задач невалидна.

## Как отвечать пользователю

1. Запусти scanner.
2. Покажи `READY` первым.
3. Для каждого `READY` назови progress, первые runnable tasks и команду запуска.
4. `BLOCKED` показывай кратко: причина блокера и что закрыть.
5. `DONE` не расписывай без запроса; scanner по умолчанию скрывает done-workspaces и показывает счётчик.
6. Если пользователь просит начать реализацию, передай точный workspace/task в `spec-kit implement`; для экономии контекста используй `workflow-kit next-task`.

Пример:

```text
READY
- 2026.06.08_17:45_ui-review-fixes — 0/8 done
  next:
    T001 P0 — Стабилизировать UI smoke-тесты
    T002 P0 — Синхронизировать денежные поля
  command: spec-kit implement 2026.06.08_17:45_ui-review-fixes T001
```

## Правила

- Не выбирай workspace автоматически, если READY несколько — попроси выбор.
- Не считай `tasks.md` готовым к реализации. Это legacy-формат.
- Не ставь задачу runnable, если есть незакрытая зависимость или обязательное подтверждение не имеет status `approved`.
- Не меняй workflow artifacts во время list. Это read-only этап.
