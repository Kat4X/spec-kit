#!/usr/bin/env bash
set -euo pipefail

fail=0

check_absent() {
  local pattern="$1"
  local description="$2"

  if rg -n "$pattern" README.md docs skills scripts >/tmp/workflow-kit-check.$$; then
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
if rg -n "$action_paths_pattern" README.md docs skills >/tmp/workflow-kit-check.$$; then
  echo 'FAIL: stale structure/reference found after skill-owned templates migration'
  cat /tmp/workflow-kit-check.$$
  fail=1
fi
rm -f /tmp/workflow-kit-check.$$

for path in templates skill-drafts docs/workflow-spec.md skills/workflow-kit-specify/template/SCOPE-TEMPLATE.md; do
  if [[ -e "$path" ]]; then
    echo "FAIL: stale path exists: $path"
    fail=1
  fi
done

for path in \
  skills/workflow-kit/SKILL.md \
  skills/workflow-kit-list/SKILL.md \
  skills/workflow-kit-list/workflow_list_specs.py \
  skills/workflow-kit-specify/SKILL.md \
  skills/workflow-kit-specify/create_workspace.py \
  skills/workflow-kit-specify/template/SPEC-TEMPLATE.md \
  skills/workflow-kit-plan/SKILL.md \
  skills/workflow-kit-plan/template/PLAN-TEMPLATE.md \
  skills/workflow-kit-plan/template/RESEARCH-TEMPLATE.md \
  skills/workflow-kit-plan/template/DATA-MODEL-TEMPLATE.md \
  skills/workflow-kit-plan/template/SCOPE-TEMPLATE.md \
  skills/workflow-kit-tasks/SKILL.md \
  skills/workflow-kit-tasks/template/TASKS-TEMPLATE.json \
  skills/workflow-kit-tasks/template/VERIFICATION-TEMPLATE.md \
  skills/workflow-kit-implement/SKILL.md \
  skills/workflow-kit-verify/SKILL.md; do
  check_exists "$path"
done

if [[ -f 'skills/workflow-kit-specify/specify-skill.md' ]]; then
  echo 'FAIL: stale duplicate skill file exists: skills/workflow-kit-specify/specify-skill.md'
  fail=1
fi

if ! python3 -m json.tool skills/workflow-kit-tasks/template/TASKS-TEMPLATE.json >/dev/null; then
  echo 'FAIL: skills/workflow-kit-tasks/template/TASKS-TEMPLATE.json is not valid JSON'
  fail=1
fi

if (( fail )); then
  exit 1
fi

echo 'OK: Workflow Kit docs/templates are consistent.'
