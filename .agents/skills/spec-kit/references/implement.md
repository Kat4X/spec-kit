# Spec Kit: implement

Имплементируй задачу для изменения: `$ARGUMENTS`.

Формат аргумента:

```text
{change-name} [task-id]
```

Если `task-id` не указан — в normal mode выбери первую незавершённую задачу из `tasks.json`, которую можно выполнить без нарушения зависимостей и подтверждений; в batch/yolo mode построй очередь таких задач.

Режимы исполнения:

- **Normal mode** — по умолчанию выполни одну маленькую задачу.
- **Batch / YOLO mode** — только по явному запросу; перед работой прочитай `references/batch-mode.md`.

Цель: сохранить соответствие `spec.md`/`plan.md`, обновить фактические статусы задач и записать проверки в `verification.md` без лишнего микрошума.

## Порядок работы

### 1. Найди workspace

Найди workspace:

- если `$ARGUMENTS` содержит путь к workspace, используй его;
- если `$ARGUMENTS` содержит `change-name`, найди `ai/specs/*_{change-name}/`;
- если совпадений несколько — выбери последний по timestamp или уточни;
- если workspace не найден — остановись и попроси путь или точный `change-name`.

В workspace должны быть:

- `tasks.json`;
- `plan.md`;
- `spec.md`;
- `scope.md`;
- `verification.md`.

Если любого из обязательных файлов нет — остановись и скажи, какой этап нужно выполнить перед implement.

### 2. Загрузи контекст до правок

Получай task payload через CLI:

```bash
workflow-kit next-task <workspace> --root .
workflow-kit next-task <workspace> --task-id Txxx --root .
```

Для batch/yolo используй очередь и правила из `references/batch-mode.md`.

Прочитай:

- task payload из команды выше;
- `verification.md`;
- `scope.md`;
- `plan.md`;
- `spec.md`;
- `research.md`, если есть;
- `data-model.md`, если есть;
- project instructions, если они не были в контексте и влияют на реализацию;
- релевантные файлы текущей реализации для выбранной задачи.

Не используй старые `ai/specs/*`, `plan.md` или `tasks.json` из других workspaces как источник требований для этого change, если пользователь явно не указал связанный workspace.

### 3. Выбери задачу или batch

- Если указан `Txxx` — используй `workflow-kit next-task <workspace> --task-id Txxx` и работай только с задачей с таким `id`.
- Если `task-id` не указан и batch/yolo не запрошен — используй `workflow-kit next-task <workspace>` и работай с возвращённой задачей.
- Если явно запрошен batch/yolo — работай по `references/batch-mode.md`.
- Проверь `dependsOn`, `confirmation`, `refs`, `files` и `checks` выбранной задачи или всех задач batch-очереди. Для schema v1 прочитай legacy-поля `confirmationRequired`/`confirmation`.

Остановись до правок, если:

- зависимость явно не выполнена и не будет выполнена ранее в той же batch-очереди;
- задача требует продуктового решения;
- `confirmation.required: true`, а `confirmation.status` не равен `approved`;
- задача требует файл из `Forbidden`;
- задача слишком крупная для одного прохода и требует декомпозиции.

В batch/yolo режиме не перескакивай через блокер; детали — в `references/batch-mode.md`.

Если пользователь явно подтвердил задачу schema v2, до реализации обнови её `confirmation.status` на `approved`, запиши краткое `evidence` и `updatedAt`. При отказе выставь `rejected`. Не считай память чата достаточным долговременным подтверждением. Для legacy schema v1 сначала мигрируй весь `tasks.json` на schema v2, сохранив смысл задач и подтверждений.

### 4. Проверь границы `scope.md`

Используй `scope.md` как границы редактирования:

- `Freely editable` — можно менять в рамках выбранной задачи;
- `Requires confirmation` — спроси перед изменением;
- `Forbidden` — не меняй;
- неуказанный файл — считай `Requires confirmation`, если изменение не очевидно локальное и напрямую не указано в `tasks.json`.

Останавливайся перед изменением public/shared surfaces: API, schema, migrations, config, generated files, shared data, если они не разрешены в `scope.md`.

Не делай «маленькую полезную правку» вне задачи. Это не инициатива, это scope creep.

### 5. Реализуй минимально

Для выбранной задачи или каждой задачи batch-очереди:

1. Уточни expected result из `tasks.json`, `plan.md`, `spec.md` и `verification.md`.
2. Прочитай существующие файлы перед изменением.
3. Внеси минимальные изменения только для текущей задачи.
4. Добавь/обнови тесты, если задача или plan это требует.
5. Запусти минимально достаточную проверку или более широкий covering-check, если он строго покрывает обязательные checks текущей задачи.
6. Запиши результат проверки в verification ledger / `verification.md`.
7. Обнови задачу в `tasks.json`: `status: "done"` только если обязательные проверки прошли или невозможность проверки явно объяснена в `verification.md`; иначе используй `blocked` или оставь `pending` с объяснением.

Не выполняй несколько независимых задач за один проход, если пользователь явно не попросил batch/yolo mode.

### 6. Обнови артефакты

После реализации обнови только нужное:

- `tasks.json` — обнови `status`, `verification.result`, `verification.evidence`, `verification.updatedAt` выбранной задачи или задач batch-очереди и не меняй смысл невыполненных задач без причины;
- `verification.md` — добавь фактический результат проверки;
- `tasks.json` — добавь fix task только если обнаружен баг реализации, который не был покрыт задачами, и это безопаснее, чем править сразу.

Batch artifact policy описана в `references/batch-mode.md`.

Implementation fix в MVP фиксируй как задачу в `tasks.json` и evidence в `verification.md`, а не отдельным файлом.

Не переписывай `spec.md` или `plan.md` молча. Если они неверны или неполны — остановись и предложи обновление через соответствующий этап.

### 7. Implementation bug vs spec/plan bug

Если поведение не сходится:

- Если `spec.md` неверна или неполна — остановись и предложи вернуться к `spec-kit specify`.
- Если `plan.md` неверен или неполон — остановись и предложи вернуться к `spec-kit plan`.
- Если `tasks.json` неверен или неполон — остановись и предложи обновить tasks.
- Если spec/plan/tasks верны, но код не соответствует — исправь в рамках текущей задачи или создай fix task в `tasks.json`.

Не искажай spec или plan, чтобы оправдать баг реализации.

### 8. Условия остановки

В normal и batch/yolo режимах одинаково остановись и сообщи пользователю, если:

- нужен файл из `Forbidden`;
- нужен файл из `Requires confirmation` без подтверждения;
- нужно изменить public API/schema/migration/config/generated/shared data без явного подтверждения;
- тесты падают и причина неочевидна;
- есть риск потери данных;
- нужна новая крупная зависимость;
- P1/P2 начинает пролезать в P0 без решения;
- задача слишком крупная и требует декомпозиции;
- требуется изменить `spec.md` или `plan.md`.

В batch/yolo режиме при остановке используй stop output из `references/batch-mode.md`.

### 9. Git checkpoint

Перед изменениями зафиксируй `git status --short --ignored`. После успешных required checks и перевода execution unit в `done` создай отдельный commit только с implementation-файлами этой unit. `tasks.json`, `verification.md` и остальные файлы `ai/specs/**` оставь локальными; не используй `git add -f`. Не захватывай посторонние или ранее существовавшие изменения.

Рекомендуемый commit message: `workflow(implement): <task-id> <краткое название>`. Для batch/yolo создай один commit с implementation-файлами после успешной проверки всего batch. При блокере, падающем required check или отсутствии implementation-изменений commit не создавай. Push на этом этапе не выполняй.

### 10. Результат после задачи или batch

Сообщай кратко:

```text
[T005] Завершена (`status: done`)

Файлы:
  + path/new-file
  ~ path/existing-file

Проверки:
  command — passed

Scope boundaries:
  все изменения в Freely editable / подтверждено: ...

Workflow artifacts:
  tasks.json updated
  verification.md updated

Прогресс: 5/12 done
Commit: <hash>
Следующая: [T006] ...
```

Для batch/yolo результата используй формат из `references/batch-mode.md`.

Если проверка не запускалась:

```text
Проверки:
  command — not run
  Причина: ...
  Риск: ...
```

Если остановилась до правок:

- назови выбранную/запрошенную задачу;
- объясни блокер;
- скажи, что нужно сделать дальше;
- явно укажи, что файлы не изменены.
