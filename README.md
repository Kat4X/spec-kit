# Workflow Kit

> Skills-first workflow для AI-агентов: фиксируем намерение, план, границы редактирования, машинно-читаемые задачи и проверку в файлах, без тяжёлого CLI на старте.

## Статус

Черновик для валидации. Не установлен в `.agents/skills/` и не является активным workflow.

## Зачем

Обычная работа с AI часто разваливается из-за потери контекста: идея осталась в чате, агент сделал кусок, потом непонятно, что уже решено и почему.

Workflow Kit переносит состояние работы в файлы:

```text
idea → spec.md → research.md → plan.md → scope.md → tasks.json → code → verification.md
                         └→ data-model.md (optional)
```

Skill объясняет процесс, файлы хранят состояние, человек контролирует результат.

## Принципы

- **Skills-first, CLI-later** — сначала проверяем процесс как инструкции и Markdown-артефакты.
- **Scope-first** — явный текущий scope, P0/P1/P2 где это полезно, жёсткий out-of-scope, никаких «заодно перепишем половину проекта».
- **Small verified steps** — задачи маленькие, проверяемые, с явными файлами.
- **Machine-readable task queue** — `tasks.json` хранит статусы, зависимости, checks, scope files и verification evidence без парсинга Markdown-чекбоксов.
- **Managed edit scope** — агент заранее знает, что можно менять свободно, что требует подтверждения, а что запрещено.
- **Spec bug ≠ implementation bug** — если намерение неверное, меняем spec; если spec верна, чиним реализацию отдельной задачей/заметкой.
- **Tests as feedback loop** — завершение доказывается проверками, а не фразой «готово».
- **Batch only by consent** — по умолчанию агент делает одну задачу; batch/YOLO разрешён только явным запросом и останавливается на блокерах.

## Структура change workspace

```text
ai/specs/YYYYMMDD_HHMM_change-name/
├── spec.md             # что и зачем
├── research.md         # опционально: что выяснили перед планом
├── plan.md             # как делаем
├── data-model.md       # опционально: данные/схемы/форматы
├── scope.md            # managed files / границы редактирования
├── tasks.json          # машинно-читаемая очередь задач
├── implementation-fix.md # опционально: баг реализации при корректной spec
└── verification.md     # чем доказали, что работает
```

## Основной цикл

```text
specify → plan → tasks → implement → verify
   │        │       │          │         │
   ▼        ▼       ▼          ▼         ▼
spec.md  plan.md  tasks.json  code      verification.md
            │        │          │
            ▼        ▼          ▼
        scope.md  file paths  task statuses
```

## Что улучшено после первой проверки в Yutori

Изначальный черновик использовал `tasks.md` как Markdown-чеклист. После реального прогона workflow усилен так:

- `tasks.md` заменён на валидный `tasks.json` с явными `status`, `dependsOn`, `parallel`, `confirmationRequired`, `files`, `checks` и task-level `verification`.
- Добавлены режимы исполнения: normal mode выполняет одну следующую задачу, batch/YOLO mode выполняет dependency-ready очередь только по явному запросу пользователя.
- Добавлена batch artifact policy: обновлять `tasks.json`/`verification.md` на checkpoint-ах, не дублировать проверки и не ставить `done` без required checks или честного объяснения.
- Добавлена consolidated check policy: широкая финальная команда может покрывать несколько task-level checks и фиксируется как `covered`, а не как пачка повторных запусков.
- Добавлен `workflow-kit-list` — read-only scanner workspaces по `ai/specs/*`, который показывает `READY`, `BLOCKED`, `DONE`, `NEEDS_PLAN`, `NEEDS_TASKS`, `LEGACY`, `BROKEN` и первую runnable task.
- Уточнены stop conditions: Forbidden/Requires confirmation, schema/config/shared-surface risk, failing required checks и продуктовые вопросы всегда останавливают implementation.
- Усилен verify: `Ready` запрещён при `pending`/`in_progress`/`blocked` задачах, нарушении scope или непроверенном P0.

## Что взято из исследованных подходов

| Источник | Что берём | Что не тащим |
|---|---|---|
| OpenSpec | change как папка, artifact flow, delta-мышление, archive/sync как будущая идея | тяжёлый CLI/schema engine на старте |
| CodeSpeak | managed files, mixed-mode границы, spec bug vs implementation bug, test feedback loop | specs как полный replacement кода |
| WORKFLOW-SKILL | простой цикл specify/plan/tasks/implement, P0/P1/P2, stop conditions | жёсткую привязку к Yutori |

## Что валидировать в этом черновике

1. Остаётся ли обязательный минимальный `scope.md` достаточно лёгким для маленьких задач?
2. Не слишком ли тяжёлый `spec.md` для маленьких задач?
3. Достаточна ли JSON-схема `tasks.json` для автоматизации без CLI?
4. Работают ли skill-drafts без CLI и без установки?
5. Какие поля надо сделать обязательными для всех проектов, а какие оставить project rules?

## Содержимое

```text
docs/
  workflow-spec.md          # спецификация самого Workflow Kit
  artifact-contracts.md     # контракт файлов
  validation-checklist.md   # как проверять черновик

templates/change/
  spec.md
  research.md
  plan.md
  data-model.md
  scope.md
  tasks.json
  implementation-fix.md
  verification.md

scripts/
  check-consistency.sh     # sanity-check терминологии и синхронизации шаблонов

skill-drafts/
  workflow-kit/
  workflow-kit-list/       # read-only scanner workspaces и runnable tasks
  workflow-kit-specify/
  workflow-kit-plan/
  workflow-kit-tasks/
  workflow-kit-implement/
  workflow-kit-verify/
```

## Быстрый список workspaces

Из проекта, где лежат `ai/specs/*`, можно запустить:

```bash
python3 skill-drafts/workflow-kit-list/workflow_list_specs.py --root .
```

Для машинного вывода:

```bash
python3 skill-drafts/workflow-kit-list/workflow_list_specs.py --root . --json
```

## Следующий шаг

Проверить обновлённый workflow на нескольких изменениях разного размера: маленький локальный fix, feature с несколькими задачами и batch/YOLO-прогон до первого блокера.
