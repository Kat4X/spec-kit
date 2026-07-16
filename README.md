# Spec Kit

> Skills-first workflow для AI-агентов: фиксируем намерение, план, границы редактирования, машинно-читаемые задачи и проверку в файлах, а компактный Rust CLI выдаёт агенту только следующую работу и связанный контекст.

## Статус

Черновик для валидации. Единый state-machine skill установлен в `.agents/skills/spec-kit/`, Rust CLI покрывает lifecycle workspace, task selection, context packet и безопасный claim.

## Зачем

Обычная работа с AI часто разваливается из-за потери контекста: идея осталась в чате, агент сделал кусок, потом непонятно, что уже решено и почему.

Spec Kit переносит состояние работы в файлы:

```text
idea → spec.md → research.md → plan.md → scope.md → tasks.json → code → verification.md
                         └→ data-model.md (optional)
```

Skill объясняет процесс, файлы хранят состояние, человек контролирует результат.

## Принципы

- **State-machine skill, CLI-assisted** — один skill маршрутизирует фазы по состоянию артефактов, CLI детерминированно выполняет шаблонные операции и выбор очереди.
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

## Содержимое

```text
docs/
  artifact-contracts.md    # контракт файлов

scripts/
  workflow-kit            # dev-launcher Rust CLI
  check                   # все проверки репозитория

tools/
  validate_skill.py       # dev-only проверка Agent Skills metadata/ссылок

src/                      # единственная реализация CLI

.agents/skills/
  spec-kit/
    SKILL.md              # state machine и единая точка входа
    references/           # инструкции фаз, загружаемые по текущему состоянию
    assets/               # canonical templates, embedded в Rust binary
```

## Быстрый список workspaces

Из проекта, где лежат `ai/specs/*`, можно запустить:

```bash
scripts/workflow-kit list --root .
```

Для машинного вывода:

```bash
scripts/workflow-kit list --root . --json
scripts/workflow-kit batch-queue <workspace> --root .
```

## Rust CLI для агента

Первый build требует stable Rust; после этого launcher использует актуальный локальный binary. Python fallback для core-команд отсутствует:

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"
rustup default stable
cargo build --release
```

Минимальный agent flow:

```bash
WORKSPACE="$(scripts/workflow-kit create 'export tasks' --root .)"

# Агент редактирует spec.md, затем запрашивает шаблоны следующей фазы.
scripts/workflow-kit scaffold "$WORKSPACE" --phase plan --root . --json
scripts/workflow-kit scaffold "$WORKSPACE" --phase tasks --root . --json

# JSON packet: одна sequential task или ready parallel prefix + связанный context.
scripts/workflow-kit packet "$WORKSPACE" --root . --json

# revision берётся из packet; dry-run ничего не записывает.
scripts/workflow-kit claim "$WORKSPACE" --revision 'fnv1a64:…' --dry-run --root . --json
scripts/workflow-kit claim "$WORKSPACE" --revision 'fnv1a64:…' --root . --json

scripts/workflow-kit validate "$WORKSPACE" --root . --json
```

`packet --task-id Txxx` выдаёт одну явно выбранную dependency-ready задачу. Без `--task-id` CLI возвращает одну непараллельную задачу либо непрерывный ready-набор `parallel: true` до первого последовательного/barrier состояния. Пакет содержит requirement/plan excerpts, scope, files, checks и `missingRefs`, но не весь `tasks.json` и не содержимое product source files.

`claim` под блокировкой повторно проверяет revision и готовность unit, затем атомарно переводит выбранные задачи в `in_progress`. Stale packet завершается conflict без частичной записи; `--dry-run` возвращает предполагаемый результат без изменения workspace.

Проверить весь репозиторий:

```bash
scripts/check
```

Установленный skill вызывает `workflow-kit` из `PATH`; путь `scripts/workflow-kit` существует только для разработки этого репозитория.

## Следующий шаг

Проверить обновлённый workflow на реальных изменениях разного размера и собрать примеры экономии agent context относительно чтения полных артефактов.
