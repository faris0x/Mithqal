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
check_absent \
    'stale evaluator module description' \
    'are added in the next layer' \
    src
check_absent \
    'stale algebra module description' \
    'univariate rational roots, and a difference of squares pattern' \
    src
check_absent \
    'stale private arena wording' \
    'The arena, hash table, matcher, virtual machine, and garbage collector remain private' \
    docs/api.md
check_absent \
    'stale collection roots' \
    'definitions, the last result, or the supplied roots' \
    src/api.rs
check_absent \
    'stale fixture extension claim' \
    'Fixtures use the .expr extension' \
    docs

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
for document in docs/*.md; do
    case "$document" in
        docs/maturity.md|docs/roadmap.md) continue ;;
    esac
    if ! grep -q '^> Status:' "$document"; then
        printf 'stale documentation: %s lacks a status banner\n' "$document" >&2
        status=1
    fi
done

exit "$status"
