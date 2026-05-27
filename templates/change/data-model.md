# Модель данных: {Change Name}

Spec: [spec.md](spec.md)
Plan: [plan.md](plan.md)

<!--
Создавай этот файл только если change меняет persistent data, schema, migration, file format, external contract shape или важную state model.
Перед финалом удали HTML comments и placeholders.
-->

## Область применения

{Какие данные/состояния меняются и зачем.}

## Сущности

### {EntityName}

| Поле | Тип | Ограничения | Описание |
|---|---|---|---|
| `id` | UUID | PK | {описание} |
| `{field}` | `{type}` | {constraints} | {описание} |

## Связи

{Описание связей между сущностями. Если не применимо — "Не требуется для этого change."}

## Схемы / миграции

```sql
-- Если применимо. Иначе удалить code block и оставить "Не требуется для этого change."
```

## Форматы / контракты

```json
{
  "example": "Если применимо. Иначе удалить code block и оставить 'Не требуется для этого change.'"
}
```

## Правила валидации

- {правило}

## Совместимость и миграция

- {backward/forward compatibility, migration notes, rollback considerations}

## Тестовые данные

- {какие фикстуры, factory, seed или examples нужны для тестов}

## Открытые вопросы

<!-- Если нет открытых вопросов, оставь "- Нет." -->

- [ ] {вопрос}
