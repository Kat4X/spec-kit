# Batch / YOLO mode

Use only when the user explicitly asks for `yolo mode`, `batch`, `продолжай в yolo`, `делай все готовые задачи`, or equivalent.

## Queue

Build the deterministic queue from `execution.order`. The command simulates completion of earlier queued tasks, so downstream tasks may enter the same batch without intermediate artifact writes:

```bash
scripts/workflow-kit batch-queue <workspace> --root .
```

Then load each task payload before editing:

```bash
scripts/workflow-kit next-task <workspace> --task-id Txxx --root .
```

A dependency is complete only when the dependency task is `status: "done"` or has justified `status: "skipped"`. Inside a batch, a dependency may count as complete after the earlier queued task is implemented and verified. Tasks awaiting/rejecting confirmation remain in `blockers`; approved confirmations may enter `queue`.

Do not jump past a blocker if that can distort the dependency chain or hide risk. Independent later tasks are allowed only when their `dependsOn` are clean and scope is obvious.

## Artifact policy

You may batch workflow artifact updates:

- keep a short verification ledger while working;
- update `tasks.json` and `verification.md` at checkpoints, not after every microstep;
- checkpoint after a related group, before a risky transition, and at the end;
- after a batched update, validate `tasks.json` once (`jq empty` or equivalent);
- avoid duplicate checks when a later/wider command strictly covers earlier required checks; say what it covers in evidence;
- never mark `done` until required checks passed or are explicitly covered by a covering-check.

Implementation fixes in MVP belong in `tasks.json` plus `verification.md` evidence, not in a separate file.

## Stop output

When stopping mid-batch, report:

- tasks completed;
- checks that cover their `done` status;
- the next pending task blocked;
- the blocker and next action.

## Result format

```text
Batch завершён: T002–T006 (`status: done`)

Проверки:
  ./gradlew :app:assembleDebug --quiet — passed (covers T003–T006)
  ./gradlew :app:testDevDebugUnitTest :app:testProdDebugUnitTest --quiet — passed (covers T002)

Workflow artifacts:
  tasks.json updated once after batch
  verification.md updated once after batch

Прогресс: 6/8 done
Следующая: [T007] ...
```
