#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

status=0

check_absent() {
    description=$1
    pattern=$2
    shift 2
    if matches=$(grep -RIn -- "$pattern" "$@" 2>/dev/null); then
        printf 'stale documentation: %s\n%s\n' "$description" "$matches" >&2
        status=1
    fi
}

check_absent \
    'approximate values described as future work' \
    'Approximate values arrive through an explicit evaluation boundary in a later version' \
    docs/semantics.md
check_absent \
    'numeric constants described as future work' \
    'numeric evaluators in a later version' \
    docs/semantics.md
check_absent \
    'automatic collection described as future work' \
    'automatic collection is enabled in a later version' \
    docs/api.md
check_absent \
    'assumptions described as future work' \
    'assumptions in a later version' \
    docs/semantics.md
check_absent \
    'README multivariable factorisation boundary' \
    'Full multivariate factorization' \
    README.md

if ! grep -q 'initial_arena_capacity' docs/api.md; then
    printf 'stale documentation: Config lacks initial_arena_capacity\n' >&2
    status=1
fi
if ! grep -q 'staged multivariate factorisation through eight variables' README.md; then
    printf 'stale documentation: README lacks the staged factorisation boundary\n' >&2
    status=1
fi
if ! grep -q 'Maturity' README.md; then
    printf 'stale documentation: README lacks the maturity link\n' >&2
    status=1
fi
if ! grep -q 'Process gates' docs/maturity.md; then
    printf 'stale documentation: maturity document lacks process gates\n' >&2
    status=1
fi

exit "$status"
