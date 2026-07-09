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
ai/specs/YYYY.MM.DD_HH:MM_change-name/
├── spec.md             # что и зачем
├── research.md         # опционально: что выяснили перед планом
├── plan.md             # как делаем
├── data-model.md       # опционально: данные/схемы/форматы
├── scope.md            # managed files / границы редактирования
├── tasks.json          # машинно-читаемая очередь задач
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
4. Работают ли skills без CLI и без установки?
5. Какие поля надо сделать обязательными для всех проектов, а какие оставить project rules?

## Содержимое

```text
docs/
  artifact-contracts.md    # контракт файлов

scripts/
  check-consistency.sh     # sanity-check структуры, терминологии и шаблонов

skills/
  workflow-kit/
  workflow-kit-list/
  workflow-kit-specify/
  workflow-kit-plan/
  workflow-kit-tasks/
  workflow-kit-implement/
  workflow-kit-verify/
```

## Быстрый список workspaces

Из проекта, где лежат `ai/specs/*`, можно запустить:

```bash
python3 skills/workflow-kit-list/workflow_list_specs.py --root .
```

Для машинного вывода:

```bash
python3 skills/workflow-kit-list/workflow_list_specs.py --root . --json
```

## Следующий шаг

Проверить обновлённый workflow на нескольких изменениях разного размера: маленький локальный fix, feature с несколькими задачами и batch/YOLO-прогон до первого блокера.
