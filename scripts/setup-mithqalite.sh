#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fork="$root/vendor/mithqalite"

if [ ! -d "$fork/.git" ]; then
    printf 'vendor/mithqalite is missing; run git submodule update --init --recursive first\n' >&2
    exit 1
fi

if ! git -C "$fork" remote get-url upstream >/dev/null 2>&1; then
    git -C "$fork" remote add upstream https://github.com/mhogrefe/malachite
fi

git -C "$fork" fetch upstream master
printf 'Mithqalite upstream is ready at %s\n' "$fork"
