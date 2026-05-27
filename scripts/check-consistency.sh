#!/usr/bin/env bash
set -euo pipefail

fail=0

check_absent() {
  local pattern="$1"
  local description="$2"

  if rg -n "$pattern" README.md docs skill-drafts templates >/tmp/workflow-kit-check.$$; then
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

check_same() {
  local left="$1"
  local right="$2"

  if ! cmp -s "$left" "$right"; then
    echo "FAIL: template drift: $left differs from $right"
    fail=1
  fi
}

legacy_scope_pattern='managed'"-"'files[.]md|MANAGED'"-"'FILES-TEMPLATE'
ambiguous_parallel_pattern='\\['"P"'\\]'
duplicated_scope_heading='Scope / '"Scope"

check_absent "$legacy_scope_pattern" 'stale legacy scope filename found; use scope.md'
check_absent "$duplicated_scope_heading" 'duplicated Scope heading found'
check_absent "$ambiguous_parallel_pattern" 'ambiguous short parallel marker found; use [PAR]'

check_exists 'templates/change/spec.md'
check_exists 'templates/change/research.md'
check_exists 'templates/change/plan.md'
check_exists 'templates/change/data-model.md'
check_exists 'templates/change/scope.md'
check_exists 'templates/change/tasks.md'
check_exists 'templates/change/verification.md'
check_exists 'templates/change/implementation-fix.md'
check_exists 'skill-drafts/workflow-kit-plan/template/SCOPE-TEMPLATE.md'

if [[ -f 'skill-drafts/workflow-kit-specify/specify-skill.md' ]]; then
  echo 'FAIL: stale duplicate skill file exists: skill-drafts/workflow-kit-specify/specify-skill.md'
  fail=1
fi

check_same 'templates/change/spec.md' 'skill-drafts/workflow-kit-specify/template/SPEC-TEMPLATE.md'
check_same 'templates/change/research.md' 'skill-drafts/workflow-kit-plan/template/RESEARCH-TEMPLATE.md'
check_same 'templates/change/plan.md' 'skill-drafts/workflow-kit-plan/template/PLAN-TEMPLATE.md'
check_same 'templates/change/data-model.md' 'skill-drafts/workflow-kit-plan/template/DATA-MODEL-TEMPLATE.md'
check_same 'templates/change/scope.md' 'skill-drafts/workflow-kit-plan/template/SCOPE-TEMPLATE.md'
check_same 'templates/change/scope.md' 'skill-drafts/workflow-kit-specify/template/SCOPE-TEMPLATE.md'
check_same 'templates/change/tasks.md' 'skill-drafts/workflow-kit-tasks/template/TASKS-TEMPLATE.md'
check_same 'templates/change/verification.md' 'skill-drafts/workflow-kit-tasks/template/VERIFICATION-TEMPLATE.md'

if (( fail )); then
  exit 1
fi

echo 'OK: Workflow Kit docs/templates are consistent.'
