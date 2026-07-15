---
name: workflow-kit
description: "Используй для оркестрации полного цикла изменения через Workflow Kit или объяснения порядка specify → plan → tasks → implement → verify. Для одного конкретного этапа используй соответствующий специализированный skill."
---

# Workflow Kit

Работай через артефакты, а не через память чата.

## Цикл

```text
specify → plan → tasks → implement → verify
```

## Change workspace

По умолчанию:

```text
ai/specs/YYYY.MM.DD_HH:MM_change-name/
```

Файлы:

```text
spec.md
research.md           # optional
plan.md
data-model.md         # optional
scope.md
tasks.json
verification.md
```

## Execution modes

- **Normal mode**: implement выполняет одну следующую задачу и сразу фиксирует её проверки.
- **Batch / YOLO mode**: только по явному запросу пользователя implement может выполнять серию dependency-ready задач до первого блокера, батчить обновление `tasks.json`/`verification.md` и использовать consolidated checks вместо дублирующих проверок.
- В любом режиме Forbidden, неподтверждённые Requires confirmation, failing required checks, schema/config/shared-surface risk и продуктовые вопросы остаются stop conditions.

## Правила

- Сначала понять WHAT/WHY, потом HOW, потом code.
- `spec.md` не должен содержать детали реализации: файлы, классы, функции, архитектурные решения, библиотеки и пошаговый HOW.
- `scope.md` обязателен до tasks/implement; для маленьких atomic changes он может быть коротким, но границы редактирования должны быть явными.
- Неясности помечай `ТРЕБУЕТ УТОЧНЕНИЯ`.
- Не меняй файлы вне `scope.md` без подтверждения.
- Не ставь задаче `status: "done"` в `tasks.json`, если проверка не прошла или не объяснено, почему её нельзя запустить.
- Если spec неверна — останови текущую фазу и верни change в `workflow-kit-specify`; не переписывай spec из plan/tasks/implement/verify.
- Если spec верна, но код ошибся — создай implementation fix task/note.

## Skill quality

- Проверяй frontmatter локальных skills: `scripts/workflow-kit validate-skills`.
- Запускай поведенческие проверки CLI/contracts: `scripts/workflow-kit test`.
- Trigger evals лежат в `references/trigger-evals.json`; используй их при изменении `description`.

## Связанные skills

- `workflow-kit-list`
- `workflow-kit-specify`
- `workflow-kit-plan`
- `workflow-kit-tasks`
- `workflow-kit-implement`
- `workflow-kit-verify`
