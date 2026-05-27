---
name: workflow-kit
description: Лёгкий skills-first workflow для AI-разработки через Markdown-артефакты. Используй когда нужно провести изменение от идеи до проверенной реализации.
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
ai/specs/YYYYMMDD_HHMM_change-name/
```

Файлы:

```text
spec.md
research.md           # optional
plan.md
data-model.md         # optional
scope.md
tasks.md
implementation-fix.md # optional
verification.md
```

## Правила

- Сначала понять WHAT/WHY, потом HOW, потом code.
- `spec.md` не должен содержать детали реализации: файлы, классы, функции, архитектурные решения, библиотеки и пошаговый HOW.
- `scope.md` обязателен до tasks/implement; для маленьких atomic changes он может быть коротким, но границы редактирования должны быть явными.
- Неясности помечай `ТРЕБУЕТ УТОЧНЕНИЯ`.
- Не меняй файлы вне `scope.md` без подтверждения.
- Не отмечай задачу `[x]`, если проверка не прошла или не объяснено, почему её нельзя запустить.
- Если spec неверна — меняй `spec.md`.
- Если spec верна, но код ошибся — создай implementation fix task/note.

## Связанные draft skills

- `workflow-kit-specify`
- `workflow-kit-plan`
- `workflow-kit-tasks`
- `workflow-kit-implement`
- `workflow-kit-verify`
