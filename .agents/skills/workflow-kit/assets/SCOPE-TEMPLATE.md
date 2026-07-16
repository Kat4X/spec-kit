# Scope: {Change Name}

Spec: [spec.md](spec.md)
Plan: [plan.md](plan.md)

<!--
Scope задаёт границы редактирования для workflow-kit tasks/implement.
Перед финалом удали HTML comments и placeholders.
-->

## Glob semantics

- Паттерны — gitignore-style: `**` матчит произвольную глубину директорий, `*` — один уровень, без слешей.
- Более специфичный паттерн побеждает менее специфичный.
- При равной специфичности побеждает более строгий уровень: `Forbidden` > `Requires confirmation` > `Freely editable`.

## Policy

Если файл не указан явно ни в одной секции — агент считает его `Requires confirmation`, кроме очевидного чтения и локальных тестовых файлов, напрямую указанных в `tasks.json`.

Этот файл не отменяет `.gitignore`: ignored-файлы и ignored-директории нельзя добавлять в git без явного разрешения пользователя. Не используй `git add -f` для workflow artifacts.

## Freely editable

Файлы, которые агент может менять в рамках задач без подтверждения:

- `{path/or/glob}` — {почему безопасно}

## Requires confirmation

Перед изменением агент спрашивает пользователя:

- `{path/or/glob}` — {почему рискованно}
- `{config file}` — shared config
- `{migration/schema/public API}` — public/shared surface

## Forbidden

Нельзя менять в этом change:

- `{path/or/glob}` — {причина}

## Read-only reference

Файлы, которые агент может читать для контекста, но не должен менять в рамках этого change:

- `{path/or/glob}` — {почему reference-only}

## Shared / public surfaces

Поверхности, которые могут затронуть других пользователей, модули или интеграции:

| Surface | Type | Rule |
|---|---|---|
| `{path}` | config / API / schema / migration / data | requires confirmation |

## Generated / external files

Файлы, которые генерируются. Не редактируй руками; команда регенерации обязательна, если файл попадает в change.

- `{path}` — generated, регенерируется командой `{command}`
