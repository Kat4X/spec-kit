# Контракты артефактов Workflow Kit

## Change workspace

```text
ai/specs/YYYYMMDD_HHMM_change-name/
```

Правила имени:

- timestamp: `YYYYMMDD_HHMM`;
- change-name: 2–5 слов в kebab-case;
- одна папка = одна логическая единица работы.

## Обязательные файлы MVP

```text
spec.md
plan.md
scope.md
tasks.json
verification.md
```

`research.md`, `data-model.md` и `implementation-fix.md` создаются по необходимости.

## `spec.md`

Назначение: что и зачем делаем.

Формат: чистый Markdown. Не заворачивай весь spec в JSON/XML/YAML: spec должен оставаться удобным для человека. HTML comments допустимы как подсказки в шаблоне.

Обязательные секции:

- `## Контекст`
- `## Суть изменения`
- `## Требования / кейсы`
- `## Нефункциональные требования`
- `## Критерии успеха`
- `## Вне скоупа`
- `## Открытые вопросы`

Опциональные brownfield-секции:

```markdown
## Изменение поведения

### ADDED Behavior
### MODIFIED Behavior
### REMOVED Behavior
```

Правила:

- Spec — не набор обязательных user stories и не строгая форма кейсов; это понятное описание желаемого поведения/результата.
- Внутри `## Требования / кейсы` можно использовать правила, сценарии, таблицы, flows, примеры input/output, edge cases — что лучше описывает задачу.
- P0/P1/P2 используй там, где нужна приоритизация или спорный scope. Для маленького atomic change достаточно явно отделить обязательное от out-of-scope.
- Проверяемые пункты формулируй как `КОГДА ... ТО ...`, Given/When/Then, expected output, invariant или другой ясный формат.
- Неясности помечаются `ТРЕБУЕТ УТОЧНЕНИЯ`.
- Spec отвечает на WHAT/WHY, а не HOW.
- В `spec.md` не должны попадать implementation details: файлы, классы, функции, библиотеки, архитектурные решения, internal APIs и пошаговая реализация.
- Технические детали допустимы только как ограничения результата, риски, открытые вопросы или кандидаты для `research.md`.
- Машинные метаданные не нужны в MVP. Если позже понадобится CLI/валидатор, добавь отдельный `meta.yaml`, а не засоряй `spec.md`.
- HTML comments в шаблоне можно оставить как подсказки агенту или удалить после заполнения.

## `research.md`

Назначение: зафиксировать результаты исследования, чтобы не смешивать факты с догадками.

Создаётся, если:

- есть `ТРЕБУЕТ УТОЧНЕНИЯ`, которое можно решить через код/доки;
- нужно выбрать из нескольких технических вариантов;
- надо понять существующие паттерны проекта.

Правила:

- У каждой находки должен быть источник: файл, команда, документация, наблюдение.
- Нерешённые продуктовые вопросы не закрываются research — они возвращаются человеку.

## `plan.md`

Назначение: как реализовать изменение.

Обязательные секции:

- `## Краткое описание`
- `## Архитектура / поток данных`
- `## Файлы`
- `## Стратегия тестирования`
- `## Риски и митигации`
- `## Решения, влияющие на scope`

Правила:

- План должен ссылаться на `spec.md` и, если есть, `research.md`.
- Новые и изменяемые файлы перечисляются явно.
- Если план меняет P0/P1/P2 — это отмечается как scope decision.

## `data-model.md`

Назначение: зафиксировать модель данных, схему, миграцию, формат файла, внешний контракт или важную state model, если change их меняет.

Создаётся, если изменение затрагивает:

- persistent data model;
- database schema / migration;
- file format;
- external contract shape;
- state model, важную для реализации и тестов.

Если данные/состояние не меняются — отдельный `data-model.md` не нужен; это явно указывается в `plan.md`.

## `scope.md`

Назначение: managed files и границы редактирования. Обязателен до `tasks.json`/implementation; для маленьких локальных changes может быть коротким.

Обязательные секции:

- `## Freely editable`
- `## Requires confirmation`
- `## Forbidden`
- `## Shared/public surfaces`

Семантика:

- **Freely editable** — агент может менять в рамках задач.
- **Requires confirmation** — перед изменением нужно спросить человека.
- **Forbidden** — нельзя менять в этом change.
- **Shared/public surfaces** — API, configs, migrations, schemas, lockfiles, generated files и другие поверхности риска.

Правила:

- Если файл не указан нигде, агент должен считать его `requires confirmation`, если изменение не очевидно локальное.
- Конфиги, миграции, public API и файлы данных по умолчанию требуют подтверждения.

## `tasks.json`

Назначение: машинно-читаемая очередь задач для AI-исполнителя и будущей автоматизации. Это source of truth для выбора следующей задачи; человек не обязан читать файл целиком.

Формат: валидный JSON без comments и trailing commas.

Режимы исполнения:

- **Normal mode** — режим по умолчанию: implement выбирает одну следующую `pending` задачу, у которой закрыты `dependsOn` и нет `confirmationRequired`.
- **Batch / YOLO mode** — только по явному запросу пользователя: implement строит очередь dependency-ready задач и выполняет её до первого blocker.

Batch/YOLO не отменяет stop conditions: `Forbidden`, `Requires confirmation`, failing required checks, schema/config/shared-surface risk и продуктовые вопросы останавливают выполнение.

Обязательные верхнеуровневые поля:

- `version` — версия схемы, сейчас `1`.
- `change` — название change.
- `spec`, `plan`, `scope`, `verification` — ссылки на артефакты workspace.
- `research`, `dataModel` — строка с путём или `null`.
- `statusValues` — допустимые статусы.
- `tasks` — массив задач.
- `execution` — порядок, parallel groups и первая задача: `order` (плоский массив всех task IDs), `parallelGroups` (массив групп, например `[["T003", "T004"]]`), `firstTask` (ID первой задачи).
- `coverage` — соответствие требований задачам и проверкам.
- `summary` — агрегаты для быстрого чтения.

Минимальная задача:

```json
{
  "id": "T001",
  "title": "Короткое описание",
  "details": "Детали реализации для AI-исполнителя.",
  "status": "pending",
  "priority": "P0",
  "refs": ["US-1"],
  "dependsOn": [],
  "parallel": false,
  "confirmationRequired": false,
  "confirmation": null,
  "files": {
    "read": [],
    "edit": ["path/to/file"],
    "create": [],
    "delete": []
  },
  "checks": [
    {
      "type": "automated",
      "command": "npm test",
      "scenario": null,
      "required": true,
      "covers": ["US-1"]
    }
  ],
  "verification": {
    "result": "not_run",
    "evidence": "",
    "updatedAt": null
  }
}
```

Правила:

- ID уникальный и монотонный: `T001`, `T002`, ...
- Допустимые `status`: `pending`, `in_progress`, `done`, `blocked`, `skipped`.
- `statusValues` хранит допустимые статусы внутри файла для автоматизации и валидации без внешней схемы.
- `done` ставится только после успешной обязательной проверки или явно записанного объяснения, почему проверка невозможна.
- `parallel: true` означает возможность параллельного выполнения после выполнения `dependsOn`.
- `confirmationRequired: true` означает, что implement должен остановиться до подтверждения пользователя.
- `refs` связывает задачу с требованиями из spec: `US-1`, `Case-1`, `R-001`, `NFR-001`.
- `execution.order` — плоский массив всех task IDs в порядке выполнения.
- `execution.parallelGroups` — массив групп task IDs, которые можно выполнять параллельно: `[["T003", "T004"], ["T006", "T007"]]`.
- `execution.firstTask` — ID первой задачи для старта.
- Задача ≤ 1 часа работы или должна быть разбита.
- Задача содержит явные `files` и `checks` или проверяемый manual scenario.
- Проверки ставятся рядом с кодом, который они проверяют.
- Финальная задача должна запускать проверки, обновлять `verification.md` и проверять `scope.md`.
- В batch/YOLO режиме статусы и verification evidence можно обновлять на checkpoint-ах, но `done` разрешён только для задач, чьи required checks прошли или покрыты broader check с явным evidence.

## `implementation-fix.md`

Назначение: исправить реализацию без изменения spec, когда spec верна.

Создаётся, если:

- тесты/ручная проверка показывают баг;
- desired behavior в `spec.md` корректен;
- менять spec было бы самообманом.

Правила:

- Содержит описание расхождения: expected vs actual.
- Ссылается на requirement/user story.
- После фикса может быть превращён в задачи в `tasks.json`.

## `workflow-kit-list` scanner

Назначение: read-only список workspaces и runnable tasks по файлам `ai/specs/*`.

Статусы workspace:

- `READY` — есть хотя бы одна `pending` task, у которой все `dependsOn` имеют статус `done`/`skipped`, и `confirmationRequired: false`.
- `BLOCKED` — незавершённые задачи есть, но runnable pending tasks нет.
- `DONE` — все задачи имеют статус `done` или `skipped`.
- `NEEDS_PLAN` — есть `spec.md`, но не хватает `plan.md` и/или `scope.md`.
- `NEEDS_TASKS` — есть `spec.md`, `plan.md`, `scope.md`, но нет `tasks.json`.
- `LEGACY` — найден старый `tasks.md`, но нет `tasks.json`.
- `BROKEN` — битый `tasks.json`, отсутствуют обязательные файлы при наличии `tasks.json`, нет `spec.md`, или структура задач невалидна.

Правила:

- Scanner не меняет workflow artifacts.
- Источник истины — только файлы workspace, не ручной реестр.
- `READY` определяется по `tasks.json`, `dependsOn`, `status` и `confirmationRequired`.

## `verification.md`

Назначение: доказать результат.

Обязательные секции:

- `## Summary`
- `## Automated checks`
- `## Manual checks`
- `## Coverage / gaps`
- `## Scope check`
- `## Known risks`
- `## Decision`

Возможные решения:

- `Ready`
- `Ready with warnings`
- `Blocked`
- `Needs user decision`

Правила:

- Каждая команда фиксируется с результатом.
- Если более широкая команда покрывает несколько task-level checks, это фиксируется как `covered` с перечислением покрытых задач/checks.
- Если проверка невозможна, фиксируется причина и риск.
- Нельзя скрывать failing tests, даже если они pre-existing.
- `Ready` нельзя использовать, если есть незакрытый P0, `pending`/`in_progress`/`blocked` обязательная задача, forbidden change, падающая обязательная проверка или несанкционированное изменение shared/public surface.
