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

check_not_exists() {
  local path="$1"

  if [[ -e "$path" ]]; then
    echo "FAIL: stale path exists: $path"
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

check_not_exists 'templates'
check_not_exists 'skill-drafts'
check_not_exists 'docs/workflow-spec.md'
check_not_exists 'skills/workflow-kit-specify/template/SCOPE-TEMPLATE.md'

check_exists 'docs/workflow.md'
check_exists 'skills/workflow-kit/SKILL.md'
check_exists 'skills/workflow-kit-list/SKILL.md'
check_exists 'skills/workflow-kit-list/workflow_list_specs.py'
check_exists 'skills/workflow-kit-specify/SKILL.md'
check_exists 'skills/workflow-kit-specify/template/SPEC-TEMPLATE.md'
check_exists 'skills/workflow-kit-plan/SKILL.md'
check_exists 'skills/workflow-kit-plan/template/PLAN-TEMPLATE.md'
check_exists 'skills/workflow-kit-plan/template/RESEARCH-TEMPLATE.md'
check_exists 'skills/workflow-kit-plan/template/DATA-MODEL-TEMPLATE.md'
check_exists 'skills/workflow-kit-plan/template/SCOPE-TEMPLATE.md'
check_exists 'skills/workflow-kit-tasks/SKILL.md'
check_exists 'skills/workflow-kit-tasks/template/TASKS-TEMPLATE.json'
check_exists 'skills/workflow-kit-tasks/template/VERIFICATION-TEMPLATE.md'
check_exists 'skills/workflow-kit-implement/SKILL.md'
check_exists 'skills/workflow-kit-verify/SKILL.md'

if [[ -f 'skills/workflow-kit-specify/specify-skill.md' ]]; then
  echo 'FAIL: stale duplicate skill file exists: skills/workflow-kit-specify/specify-skill.md'
  fail=1
fi

if ! python3 -m json.tool skills/workflow-kit-tasks/template/TASKS-TEMPLATE.json >/dev/null; then
  echo 'FAIL: skills/workflow-kit-tasks/template/TASKS-TEMPLATE.json is not valid JSON'
  fail=1
fi

# Verify tasks.json template has required top-level and per-task fields.
required_top='"version"  "change"  "spec"  "plan"  "scope"  "verification"  "statusValues"  "tasks"  "execution"  "coverage"  "summary"'
required_task='"id"  "title"  "details"  "status"  "priority"  "refs"  "dependsOn"  "parallel"  "confirmationRequired"  "files"  "checks"  "verification"'

tmpl='skills/workflow-kit-tasks/template/TASKS-TEMPLATE.json'
for key in $required_top; do
  if ! grep -q "$key" "$tmpl"; then
    echo "FAIL: $tmpl missing top-level field $key"
    fail=1
  fi
done
for key in $required_task; do
  if ! grep -q "$key" "$tmpl"; then
    echo "FAIL: $tmpl missing task field $key"
    fail=1
  fi
done

if (( fail )); then
  exit 1
fi

echo 'OK: Workflow Kit docs/templates are consistent.'
