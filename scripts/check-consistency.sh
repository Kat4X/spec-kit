#!/usr/bin/env bash
set -euo pipefail

fail=0

check_absent() {
  local pattern="$1"
  local description="$2"

  if rg -n "$pattern" README.md docs .agents/skills scripts >/tmp/workflow-kit-check.$$; then
    echo "FAIL: $description"
    cat /tmp/workflow-kit-check.$$
    fail=1
  fi
  rm -f /tmp/workflow-kit-check.$$
}

check_exists() {
  local path="$1"

  if [[ ! -f "$path" ]]; then
    echo "FAIL: missing required file: $path"
    fail=1
  fi
}

# Patterns are split by concatenation so the script doesn't match itself.
legacy_scope_pattern='managed'"-"'files[.]md|MANAGED'"-"'FILES-TEMPLATE'
ambiguous_parallel_pattern='\[P\]'
duplicated_scope_heading='Scope / '"Scope"

action_paths_pattern='templates/change|skill-drafts|workflow-spec[.]md|implementation-fix[.]md'

check_absent "$legacy_scope_pattern" 'stale legacy scope filename found; use scope.md'
check_absent 'TASKS-TEMPLATE[.]md' 'stale Markdown tasks template found; use tasks.json'
check_absent 'tasks[.]md[[:space:]]+#' 'stale Markdown tasks artifact in workspace tree found; use tasks.json'
check_absent "$duplicated_scope_heading" 'duplicated Scope heading found'
check_absent "$ambiguous_parallel_pattern" 'ambiguous short parallel marker found; use parallel: true in tasks.json'
if rg -n "$action_paths_pattern" README.md docs .agents/skills >/tmp/workflow-kit-check.$$; then
  echo 'FAIL: stale structure/reference found after skill-owned templates migration'
  cat /tmp/workflow-kit-check.$$
  fail=1
fi
rm -f /tmp/workflow-kit-check.$$

for path in \
  templates \
  skill-drafts \
  docs/workflow-spec.md \
  .agents/skills/workflow-kit-list \
  .agents/skills/workflow-kit-specify \
  .agents/skills/workflow-kit-plan \
  .agents/skills/workflow-kit-tasks \
  .agents/skills/workflow-kit-implement \
  .agents/skills/workflow-kit-verify; do
  if [[ -e "$path" ]]; then
    echo "FAIL: stale path exists: $path"
    fail=1
  fi
done

for path in \
  scripts/workflow-kit \
  .agents/skills/workflow-kit/SKILL.md \
  .agents/skills/workflow-kit/agents/openai.yaml \
  .agents/skills/workflow-kit/scripts/validate_skills.py \
  .agents/skills/workflow-kit/scripts/workflow_list_specs.py \
  .agents/skills/workflow-kit/scripts/create_workspace.py \
  .agents/skills/workflow-kit/references/list.md \
  .agents/skills/workflow-kit/references/specify.md \
  .agents/skills/workflow-kit/references/plan.md \
  .agents/skills/workflow-kit/references/tasks.md \
  .agents/skills/workflow-kit/references/implement.md \
  .agents/skills/workflow-kit/references/verify.md \
  .agents/skills/workflow-kit/references/batch-mode.md \
  .agents/skills/workflow-kit/references/trigger-evals.json \
  .agents/skills/workflow-kit/assets/SPEC-TEMPLATE.md \
  .agents/skills/workflow-kit/assets/PLAN-TEMPLATE.md \
  .agents/skills/workflow-kit/assets/RESEARCH-TEMPLATE.md \
  .agents/skills/workflow-kit/assets/DATA-MODEL-TEMPLATE.md \
  .agents/skills/workflow-kit/assets/SCOPE-TEMPLATE.md \
  .agents/skills/workflow-kit/assets/TASKS-TEMPLATE.json \
  .agents/skills/workflow-kit/assets/VERIFICATION-TEMPLATE.md \
  tests/test_workflow_kit.py; do
  check_exists "$path"
done

if ! python3 -m json.tool .agents/skills/workflow-kit/assets/TASKS-TEMPLATE.json >/dev/null; then
  echo 'FAIL: .agents/skills/workflow-kit/assets/TASKS-TEMPLATE.json is not valid JSON'
  fail=1
fi

if ! python3 -m json.tool .agents/skills/workflow-kit/references/trigger-evals.json >/dev/null; then
  echo 'FAIL: .agents/skills/workflow-kit/references/trigger-evals.json is not valid JSON'
  fail=1
fi

if ! scripts/workflow-kit validate-skills >/dev/null; then
  echo 'FAIL: Agent Skills frontmatter/reference validation failed'
  fail=1
fi

if ! scripts/workflow-kit test >/dev/null; then
  echo 'FAIL: Workflow Kit behavioral tests failed'
  fail=1
fi

if (( fail )); then
  exit 1
fi

echo 'OK: Workflow Kit docs/templates are consistent.'
