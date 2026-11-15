#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

source_tests=$(grep -rhoE '^[[:space:]]*#\[test\]' tests src 2>/dev/null | wc -l | tr -d ' ')
assertions=$(grep -rhoE 'assert(_eq|_ne)?!' tests src 2>/dev/null | wc -l | tr -d ' ')
runtime_tests=$(cargo test -- --list 2>/dev/null | grep -c ': test$')

report=$(printf 'source test functions: %s\nassertion sites: %s\nruntime test cases: %s' "$source_tests" "$assertions" "$runtime_tests")
printf '%s\n' "$report"

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        printf '### Test counts\n\n'
        printf '```text\n'
        printf '%s\n' "$report"
        printf '```\n'
    } >> "$GITHUB_STEP_SUMMARY"
fi
