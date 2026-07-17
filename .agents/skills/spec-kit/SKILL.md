---
name: spec-kit
description: "Используй для любого действия Spec Kit: создать или уточнить spec, построить plan/scope, создать tasks.json, реализовать следующую задачу, проверить change, показать workspaces или продолжить полный цикл. Skill определяет текущую фазу по артефактам workspace и выполняет только допустимый переход state machine."
compatibility: "Requires the workflow-kit CLI on PATH and Git."
---

# Spec Kit

Работай через артефакты workspace, а не через память чата. Это единственная публичная точка входа; фазовые инструкции загружай из `references/` только после определения состояния и действия.

## State machine

```text
NO_WORKSPACE → SPECIFY → PLAN → TASKS → IMPLEMENT → VERIFY → COMPLETE
                                      ↕
                                   BLOCKED

Любая фаза → BROKEN / NEEDS_USER_DECISION
```

Состояние не хранится отдельно. Выводи его из файлов workspace и их содержимого:

| Наблюдаемое состояние | Фаза | Допустимое следующее действие |
|---|---|---|
| Workspace не выбран или не существует | `NO_WORKSPACE` | `specify` |
| Есть `spec.md`, но guard спецификации не пройден | `SPECIFY` | закончить spec |
| CLI: `NEEDS_PLAN`, guard спецификации пройден | `PLAN` | создать/закончить `plan.md` и `scope.md` |
| CLI: `NEEDS_TASKS` | `TASKS` | создать `tasks.json` и `verification.md` |
| CLI: `READY` | `IMPLEMENT` | выполнить одну execution unit |
| CLI: `BLOCKED` | `BLOCKED` | закрыть confirmation/dependency/error; не перескакивать блокер |
| CLI: `DONE`, но Decision в `verification.md` не Ready | `VERIFY` | финальная проверка |
| CLI: `DONE` и Decision равен Ready / Ready with warnings | `COMPLETE` | только отчёт или новый change |
| CLI: `LEGACY` / `BROKEN` | `BROKEN` | миграция или исправление контракта |

Для существующего workspace запусти:

```bash
workflow-kit status <workspace> --root . --json
```

Для списка или выбора workspace используй ту же команду без пути:

```bash
workflow-kit status --root . --json
```

`status` даёт структурный статус, но переход всё равно обязан проверить фазовый guard ниже.

## Guards переходов

- `SPECIFY → PLAN`: spec самодостаточна, описывает WHAT/WHY, не содержит блокирующих `ТРЕБУЕТ УТОЧНЕНИЯ`.
- `PLAN → TASKS`: существуют готовые `plan.md` и `scope.md`; продуктовые вопросы закрыты.
- `TASKS → IMPLEMENT`: `tasks.json` schema-valid, обязательные требования покрыты, `verification.md` существует.
- `IMPLEMENT → VERIFY`: все обязательные tasks имеют `done` или обоснованный `skipped`; required checks не падают.
- `VERIFY → COMPLETE`: P0 выполнено, scope чист, итоговое решение записано в `verification.md`.
- Любой рискованный продуктовый вопрос, неподтверждённый shared/public surface, Forbidden file или failing required check переводит работу в `BLOCKED`/`NEEDS_USER_DECISION`.

Не создавай отдельный state-файл: он неизбежно разойдётся с артефактами.

## Маршрутизация

Определи намерение пользователя и загрузи ровно одну основную инструкцию:

| Намерение / действие | Инструкция |
|---|---|
| показать workspaces, статус, следующую task | `references/list.md` |
| создать/уточнить spec | `references/specify.md` |
| создать plan/scope/research/data model | `references/plan.md` |
| создать tasks.json/verification.md | `references/tasks.md` |
| реализовать task, продолжить работу | `references/implement.md` |
| финально проверить/закрыть change | `references/verify.md` |

Если пользователь говорит «продолжай», выбери инструкцию по текущему состоянию. Если явно просит недопустимую фазу, остановись и назови отсутствующий guard/артефакт.

Полный цикл выполняй только по явному запросу. После каждой фазы заново вычисляй состояние; не считай успешное действие доказательством допустимости следующего перехода.

## Общие правила

- Цикл: `specify → plan → tasks → implement → verify`.
- `spec.md` отвечает на WHAT/WHY; `plan.md` — на HOW; `tasks.json` — очередь исполнения.
- `scope.md` обязателен перед tasks/implement.
- Normal mode реализует одну execution unit; batch/YOLO — только по явному запросу и `references/batch-mode.md`.
- Не меняй файлы вне `scope.md` без подтверждения.
- Не ставь task `done`, если required check не прошёл или невозможность проверки не записана.
- Ошибка требований возвращает в specify; ошибка подхода — в plan; ошибка декомпозиции — в tasks; ошибка кода — implementation fix.
- Артефакты workspace в `ai/specs/**` остаются локальными: не добавляй их в git и не используй `git add -f`.
- Commit создавай только в IMPLEMENT после успешно проверенной execution unit; в batch/yolo — один commit на успешно проверенный batch. В commit включай только implementation-файлы, изменённые по текущей spec, и не захватывай чужие или ранее существовавшие изменения.
- Push не выполняй автоматически. После `COMPLETE` спроси разрешение на push текущей ветки и выполняй его только после подтверждения.
