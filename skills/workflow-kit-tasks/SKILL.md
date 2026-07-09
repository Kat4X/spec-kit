---
name: workflow-kit-tasks
description: Создание self-contained tasks.json по готовым spec.md, plan.md и scope.md. Используй после workflow-kit-plan, когда нужна машинно-читаемая очередь задач для workflow-kit-implement.
metadata:
  title: "Задачи реализации"
---

# Workflow Kit: tasks

Создай машинно-читаемые задачи реализации для изменения: `$ARGUMENTS`.

Цель: создать валидный `tasks.json` для следующего AI-этапа `workflow-kit-implement`. `tasks.json` — source of truth для очереди исполнения: маленькие задачи, зависимости, scope files, проверки и статус. Не пиши код на этапе tasks.

## Порядок работы

### 1. Найди workspace

Найди workspace, созданный `workflow-kit-specify` и заполненный `workflow-kit-plan`:

- если `$ARGUMENTS` — путь к workspace, используй его;
- если `$ARGUMENTS` — `change-name`, найди `ai/specs/*_{change-name}/`;
- если совпадений несколько — выбери последний по timestamp или уточни;
- если workspace не найден — остановись и попроси путь или точный `change-name`.

В workspace должны быть:

- `spec.md`;
- `plan.md`;
- `scope.md`.

Если любого из них нет — остановись: `tasks.json` нельзя честно создать без spec, plan и scope.

### 2. Проверь шаблоны

Перед созданием файлов проверь доступность обязательных шаблонов:

- `template/TASKS-TEMPLATE.json` — для `tasks.json`;
- `template/VERIFICATION-TEMPLATE.md` — для `verification.md`.

Если нужный шаблон недоступен — остановись и сообщи, какой шаблон отсутствует. Не создавай артефакты из памяти или резервной структуры.

### 3. Прочитай контекст

Прочитай:

- `spec.md`;
- `plan.md`;
- `scope.md`;
- `research.md`, если есть;
- `data-model.md`, если есть;
- project instructions, если они не были в контексте и влияют на задачи.

Не используй старые `ai/specs/*`, `plan.md` или `tasks.json` из других workspaces как источник требований для этого change, если пользователь явно не указал связанный workspace.

### 4. Проверь готовность plan

Перед декомпозицией проверь:

- `plan.md` помечен как готовый к `workflow-kit-tasks` или не содержит явных блокеров;
- блокирующих `ТРЕБУЕТ УТОЧНЕНИЯ` из `spec.md`/`plan.md` не осталось; продуктовые/рискованные вопросы нужно вернуть пользователю до создания `tasks.json`;
- `scope.md` существует и соответствует файлам/поверхностям из `plan.md`;
- задачи не требуют файлов из `Forbidden`.

Если есть блокер — остановись и скажи, что нужно закрыть до `tasks.json`.

### 5. Разбей plan на JSON-задачи

Задачи пишутся для AI-исполнителя и автоматизации. Каждая задача должна быть однозначной без доступа к этому диалогу.

Правила:

- Каждая задача занимает примерно 15–60 минут работы.
- Каждая задача имеет уникальный монотонный `id`: `T001`, `T002`, ... без пропусков.
- Каждая задача имеет `title`, `details`, `status`, `priority`, `refs`, `dependsOn`, `parallel`, `confirmationRequired`, `files`, `checks`, `verification`.
- `details` содержит достаточно контекста для реализации, но не превращает задачу в длинную инструкцию уровня “открыть файл”.
- `status` при создании обычно `pending`. Допустимые статусы: `pending`, `in_progress`, `done`, `blocked`, `skipped`.
- Обязательные/P0 требования идут раньше optional/P1/P2.
- Тестовые задачи ставь рядом с кодом, который они проверяют, а не только в конце.
- Не создавай задачи на файлы из `Forbidden`.
- Если задача требует `Requires confirmation`, выставь `confirmationRequired: true` и заполни `confirmation` строкой с тем, что нужно подтвердить.
- Если задачу можно делать параллельно после выполнения зависимостей, выставь `parallel: true`.
- Не добавляй задачи, которых нет в `plan.md`, если это не явно нужная проверка или подготовка.
- Финальная задача обязательно включает запуск проверок, запись результата в `verification.md` и проверку выхода за границы из `scope.md`.

Минимальная форма задачи:

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

### 6. Создай `tasks.json`

Используй `template/TASKS-TEMPLATE.json` как единственный источник структуры `tasks.json`.

Правила заполнения:

- Итоговый файл должен быть валидным JSON: без comments, trailing commas и Markdown.
- Сохраняй верхнеуровневые ключи шаблона, если нет сильной причины убрать поле.
- Удали placeholders или замени их реальными значениями.
- `research` и `dataModel` ставь `null`, если файлов нет.
- `execution.order` должен отражать последовательность задач.
- `coverage` должен показывать покрытие P0/обязательных требований задачами и проверками.
- `summary` должен совпадать с фактическим количеством задач.
- Если точный файл неизвестен, задача должна ссылаться на область поиска из `plan.md`, а не придумывать путь.

### 7. Создай `verification.md`

Если `verification.md` отсутствует — создай его по `template/VERIFICATION-TEMPLATE.md`.

Если `verification.md` уже есть — не затирай существующие результаты; обнови только ссылки/заготовки, если это безопасно.

### 8. Проверь качество

Перед финалом проверь:

- [ ] `tasks.json` валиден как JSON.
- [ ] `tasks.json` покрывает обязательные требования из `spec.md` и технический подход из `plan.md`.
- [ ] Все planned files из `plan.md` покрыты задачами или явно не нужны.
- [ ] Все P0/обязательные acceptance criteria покрыты задачами и проверками; если блокер остался, `tasks.json` не создан.
- [ ] Нет задач на файлы из `Forbidden`.
- [ ] Tasks, требующие `Requires confirmation`, имеют `confirmationRequired: true`.
- [ ] Есть финальная верификация и запись результата в `verification.md`.
- [ ] Задачи достаточно маленькие для AI-исполнителя, но не превращены в шумные микрошаги.
- [ ] `tasks.json` самодостаточен для `workflow-kit-implement`.

### 9. Git и `.gitignore`

Не выполняй `git add`, commit или push без явного запроса. Если пользователь попросил commit/stage — сначала проверь `git status --short --ignored` или `git check-ignore -v -- <path>` и не добавляй ignored-файлы без явного разрешения.

### 10. Результат

Если `tasks.json` создан, в финале покажи:

- путь к workspace;
- путь к `tasks.json`;
- путь к `verification.md`;
- всего задач;
- сколько P0/обязательных задач;
- сколько `parallel: true` задач;
- есть ли `confirmationRequired: true` задачи и какие;
- рекомендуемую первую задачу;
- готовность к `workflow-kit-implement`: да/нет;
- если не готово к `workflow-kit-implement` — что нужно закрыть;
- статус git/commit или причину, почему артефакты не добавлены.

Если остановилась до создания `tasks.json`, в финале покажи:

- почему tasks нельзя честно создать сейчас;
- блокирующие вопросы или отсутствующие шаблоны;
- что будет следующим шагом после ответа пользователя;
- явно укажи: `tasks.json` не создан.
