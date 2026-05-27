# Workflow Kit

> Skills-first workflow для AI-агентов: фиксируем намерение, план, границы редактирования, задачи и проверку в Markdown-файлах, без тяжёлого CLI на старте.

## Статус

Черновик для валидации. Не установлен в `.agents/skills/` и не является активным workflow.

## Зачем

Обычная работа с AI часто разваливается из-за потери контекста: идея осталась в чате, агент сделал кусок, потом непонятно, что уже решено и почему.

Workflow Kit переносит состояние работы в файлы:

```text
idea → spec.md → research.md → plan.md → scope.md → tasks.md → code → verification.md
                         └→ data-model.md (optional)
```

Skill объясняет процесс, файлы хранят состояние, человек контролирует результат.

## Принципы

- **Skills-first, CLI-later** — сначала проверяем процесс как инструкции и Markdown-артефакты.
- **Scope-first** — явный текущий scope, P0/P1/P2 где это полезно, жёсткий out-of-scope, никаких «заодно перепишем половину проекта».
- **Small verified steps** — задачи маленькие, проверяемые, с явными файлами.
- **Managed edit scope** — агент заранее знает, что можно менять свободно, что требует подтверждения, а что запрещено.
- **Spec bug ≠ implementation bug** — если намерение неверное, меняем spec; если spec верна, чиним реализацию отдельной задачей/заметкой.
- **Tests as feedback loop** — завершение доказывается проверками, а не фразой «готово».

## Структура change workspace

```text
ai/specs/YYYYMMDD_HHMM_change-name/
├── spec.md             # что и зачем
├── research.md         # опционально: что выяснили перед планом
├── plan.md             # как делаем
├── data-model.md       # опционально: данные/схемы/форматы
├── scope.md            # managed files / границы редактирования
├── tasks.md            # маленький исполняемый чеклист
├── implementation-fix.md # опционально: баг реализации при корректной spec
└── verification.md     # чем доказали, что работает
```

## Основной цикл

```text
specify → plan → tasks → implement → verify
   │        │       │          │         │
   ▼        ▼       ▼          ▼         ▼
spec.md  plan.md  tasks.md  code      verification.md
            │        │          │
            ▼        ▼          ▼
        scope.md  file paths  task checkboxes
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
3. Нужен ли отдельный `implementation-fix.md` или хватит задач в `tasks.md`?
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
  tasks.md
  implementation-fix.md
  verification.md

scripts/
  check-consistency.sh     # sanity-check терминологии и синхронизации шаблонов

skill-drafts/
  workflow-kit/
  workflow-kit-specify/
  workflow-kit-plan/
  workflow-kit-tasks/
  workflow-kit-implement/
  workflow-kit-verify/
```

## Следующий шаг

Прогнать Workflow Kit на одном небольшом реальном изменении и проверить: агенту хватает инструкций без ручного объяснения процесса или нет.
