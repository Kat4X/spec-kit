# Verification: {Change Name}

<!--
Verification ведётся во время workflow-kit-implement и workflow-kit-verify.
Tasks создаёт заготовку; implement/verify записывают фактические результаты.
Перед финалом удали HTML comments и placeholders.
-->

Spec: [spec.md](spec.md)
Plan: [plan.md](plan.md)
Tasks: [tasks.json](tasks.json)
Scope: [scope.md](scope.md)

## План проверок

| Проверка | Команда / сценарий | Покрывает | Когда запускать |
|---|---|---|---|
| {проверка} | `{command}` | {US/Case/R/NFR} | {после Txxx / финально} |

## Результаты выполнения

<!-- Заполняется во время implement/verify. -->

| Дата/время | Задача | Проверка | Результат | Заметки |
|---|---|---|---|---|
| {YYYY-MM-DD HH:MM} | `[Txxx]` | `{command}` | passed / failed / not run | {заметки} |

## Ручные проверки

- [ ] {сценарий и expected result}

## Проверка границ scope.md

{Что проверено по `scope.md`: были ли выходы за Freely editable, Requires confirmation или Forbidden; какие подтверждения получены, если были нужны.}

## Нерешённые проблемы

<!-- Если нет — оставь "- Нет." -->

- [ ] {проблема, ссылка на задачу/проверку}

## Итоговая готовность

- Статус: {not started / in progress / passed / failed}
- Готово к `workflow-kit-verify`: {да / нет}
- Блокеры: {нет / список}
