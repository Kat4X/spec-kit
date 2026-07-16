# Verification: {Change Name}

<!--
Verification ведётся во время spec-kit implement и spec-kit verify.
Tasks создаёт заготовку; implement/verify записывают фактические результаты.
Перед финалом удали HTML comments и placeholders.
-->

Spec: [spec.md](spec.md)
Plan: [plan.md](plan.md)
Tasks: [tasks.json](tasks.json)
Scope: [scope.md](scope.md)

## Summary

- Status: not started
- Change: {краткое описание проверяемого результата}

## Automated checks

| Проверка | Команда / сценарий | Покрывает | Когда запускать |
|---|---|---|---|
| {проверка} | `{command}` | {US/Case/R/NFR} | {после Txxx / финально} |

| Дата/время | Задача | Проверка | Результат | Заметки |
|---|---|---|---|---|
| {YYYY-MM-DD HH:MM} | `[Txxx]` | `{command}` | passed / failed / not run | {заметки} |

## Manual checks

- [ ] {сценарий и expected result}

## Coverage / gaps

{Какие требования покрыты, что не проверено и почему.}

## Scope check

{Что проверено по `scope.md`: были ли выходы за Freely editable, Requires confirmation или Forbidden; какие подтверждения получены, если были нужны.}

## Known risks

<!-- Если нет — оставь "- Нет." -->

- [ ] {проблема, ссылка на задачу/проверку}

## Decision

- Решение: {Ready / Ready with warnings / Blocked / Needs user decision / not evaluated}
- Блокеры: {нет / список}
- Следующий шаг: {что делать дальше}
