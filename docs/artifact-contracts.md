# Контракты артефактов Spec Kit

## Change workspace

```text
ai/specs/YYYY.MM.DD_HH:MM_change-name/
```

Правила имени:

- timestamp: `YYYY.MM.DD_HH:MM`;
- change-name: 2–5 слов в Unicode kebab-case; кириллица сохраняется;
- одна папка = одна логическая единица работы.

## Обязательные файлы MVP

```text
spec.md
plan.md
scope.md
tasks.json
verification.md
```

`research.md` и `data-model.md` создаются по необходимости.

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

- **Normal mode** — режим по умолчанию: implement выбирает одну следующую `pending` задачу по `execution.order`, у которой закрыты `dependsOn` и подтверждение не требуется либо имеет status `approved`.
- **Batch / YOLO mode** — только по явному запросу пользователя: implement строит очередь dependency-ready задач и выполняет её до первого blocker.

Batch/YOLO не отменяет stop conditions: `Forbidden`, неподтверждённые `Requires confirmation`, failing required checks, schema/config/shared-surface risk и продуктовые вопросы останавливают выполнение.

Обязательные верхнеуровневые поля:

- `version` — версия схемы, сейчас `2`; scanner продолжает читать schema v1 для существующих workspaces.
- `change` — название change.
- `spec`, `plan`, `scope`, `verification` — ссылки на артефакты workspace.
- `research`, `dataModel` — строка с путём или `null`.
- `tasks` — массив задач.
- `execution` — порядок и первая задача: `order` (плоский массив всех task IDs), `firstTask` (ID первой задачи).
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
  "confirmation": {
    "required": false,
    "status": "not_required",
    "request": null,
    "evidence": "",
    "updatedAt": null
  },
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
- `done` ставится только после успешной обязательной проверки или явно записанного объяснения, почему проверка невозможна.
- `parallel: true` означает возможность параллельного выполнения после выполнения `dependsOn`.
- `confirmation.required: true` требует непустой `request`; до решения status равен `pending`.
- После явного решения implement сохраняет status `approved`/`rejected`, evidence и updatedAt в `tasks.json`. Подтверждение не должно жить только в памяти чата.
- `refs` связывает задачу с требованиями из spec: `US-1`, `Case-1`, `R-001`, `NFR-001`.
- `execution.order` — плоский массив всех task IDs в порядке выполнения; scanner и batch обязаны использовать именно его, а не физический порядок JSON-массива.
- `execution.firstTask` — ID первой задачи для старта.
- Задача ≤ 1 часа работы или должна быть разбита.
- Задача содержит явные `files` и `checks` или проверяемый manual scenario.
- Проверки ставятся рядом с кодом, который они проверяют.
- Финальная задача должна запускать проверки, обновлять `verification.md` и проверять `scope.md`.
- В batch/YOLO режиме статусы и verification evidence можно обновлять на checkpoint-ах, но `done` разрешён только для задач, чьи required checks прошли или покрыты broader check с явным evidence.

## `workflow-kit list` scanner

Назначение: read-only список workspaces и runnable tasks по файлам `ai/specs/*`.

Статусы workspace:

- `READY` — есть хотя бы одна `pending` task, у которой все `dependsOn` имеют статус `done`/`skipped`, а confirmation не требуется или имеет status `approved`.
- `BLOCKED` — незавершённые задачи есть, но runnable pending tasks нет.
- `DONE` — все задачи имеют статус `done` или `skipped`.
- `NEEDS_PLAN` — есть `spec.md`, но не хватает `plan.md` и/или `scope.md`.
- `NEEDS_TASKS` — есть `spec.md`, `plan.md`, `scope.md`, но нет `tasks.json`.
- `LEGACY` — найден старый `tasks.md`, но нет `tasks.json`.
- `BROKEN` — битый `tasks.json`, отсутствуют обязательные файлы при наличии `tasks.json`, нет `spec.md`, или структура задач невалидна.

Правила:

- Scanner не меняет workflow artifacts.
- Источник истины — только файлы workspace, не ручной реестр.
- `READY` определяется по schema-valid `tasks.json`, `execution.order`, `dependsOn`, `status` и confirmation lifecycle.
- `scripts/workflow-kit batch-queue <workspace>` симулирует успешное выполнение dependency-ready цепочки и возвращает `queue`/`blockers` без изменения артефактов.

## Rust CLI: execution unit

`scripts/workflow-kit packet <workspace> --json` вычисляет текущую execution unit по schema-valid `tasks.json`:

- явный `--task-id Txxx` возвращает только эту задачу, если она `pending`, все зависимости закрыты и confirmation не блокирует работу;
- если первая runnable задача имеет `parallel: false`, unit содержит ровно одну задачу;
- если первая runnable задача имеет `parallel: true`, unit содержит непрерывный ready prefix параллельных задач в `execution.order` до первой sequential, blocked или in-progress границы;
- malformed schema, unknown/cyclic/later dependency, неподтверждённый confirmation и отсутствие ready tasks не маскируются под пустой успешный packet.

`batch-queue` остаётся read-only симуляцией всей dependency-ready цепочки и не равен execution unit: packet ограничивает объём одной безопасной агентной итерации.

## `WorkPacketV1`

Packet — внешний versioned JSON contract. Обязательные поля:

```json
{
  "packetVersion": 1,
  "tasksSchemaVersion": 2,
  "change": "change name",
  "executionFirstTask": "T001",
  "workspace": {"name": "...", "path": "...", "status": "READY"},
  "mode": "single",
  "revision": "fnv1a64:0123456789abcdef",
  "tasks": [{"id": "T001", "status": "pending"}],
  "context": {
    "requirements": [{"source": "spec.md", "ref": "Case-1", "text": "..."}],
    "missingRefs": [],
    "plan": [{"source": "plan.md", "heading": "Краткое описание", "text": "..."}],
    "scope": {"source": "scope.md", "text": "..."},
    "files": {"read": [], "edit": [], "create": [], "delete": []},
    "checks": []
  },
  "blockers": []
}
```

Правила:

- `revision` — детерминированный FNV-1a token от точных bytes `tasks.json`; это stale-write guard, не security signature.
- `tasks` содержит полные objects только выбранной unit, без остальных задач очереди.
- `requirements` разрешает `refs` по canonical headings/list entries `spec.md`; неразрешённые refs перечисляются в `missingRefs`.
- `plan` содержит baseline и связанные refs/path sections, `scope` включается целиком как обязательная граница.
- CLI не читает содержимое product source files для формирования packet.
- Additive fields допустимы в version 1; breaking shape/semantics требуют новую `packetVersion`.

## Atomic claim

`scripts/workflow-kit claim <workspace> --revision <token> [--task-id Txxx] [--dry-run] --json`:

1. Берёт exclusive sidecar lock workspace.
2. Повторно читает и валидирует `tasks.json`.
3. Сравнивает revision и заново вычисляет exact ready unit.
4. Меняет только `status` выбранных задач на `in_progress`, сохраняя schema version и неизвестные поля.
5. Пишет временный файл, выполняет sync и atomic rename.

`--dry-run` выполняет шаги проверки и возвращает hypothetical packet, но не пишет файл. Stale revision, изменившаяся unit, blocked/claimed task или partial parallel selection дают conflict/no-work без частичного изменения.

## CLI output и exit codes

- Успешная machine-команда с `--json` пишет в stdout ровно один JSON document; diagnostics идут в stderr.
- `0` — success.
- `1` — валидный no-work/blocker/пустая runnable queue.
- `2` — ошибка CLI usage, сформированная argument parser.
- `3` — нарушение artifact/schema/path contract.
- `4` — stale revision или claim conflict.
- `5` — I/O/internal failure.
- Release binary самодостаточен: canonical templates встроены при компиляции. `scripts/workflow-kit` — только dev-launcher Rust binary/cargo; Python fallback отсутствует.

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
