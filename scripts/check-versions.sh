#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

crate_version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
lock_version=$(awk '
    /^name = "mithqal"$/ { found = 1; next }
    found && /^version = "/ { gsub(/[" ]/, "", $3); print $3; exit }
' Cargo.lock)
spec_version=$(sed -n 's/^> Spec version: \([^ ]*\).*$/\1/p' spec/README.md)
changelog_version=$(sed -n 's/^## \(.*\)$/\1/p' CHANGELOG.md | head -n 1)

status=0

if [ -z "$crate_version" ] || [ -z "$lock_version" ] || [ -z "$spec_version" ] || [ -z "$changelog_version" ]; then
    printf 'version check: missing version in Cargo.toml, Cargo.lock, spec/README.md, or CHANGELOG.md\n' >&2
    exit 1
fi

if [ "$crate_version" != "$lock_version" ] ||
   [ "$crate_version" != "$spec_version" ] ||
   [ "$crate_version" != "$changelog_version" ]; then
    printf 'version mismatch:\n  Cargo.toml: %s\n  Cargo.lock: %s\n  spec/README.md: %s\n  CHANGELOG.md: %s\n' \
        "$crate_version" "$lock_version" "$spec_version" "$changelog_version" >&2
    status=1
fi

for document in spec/*.md; do
    case "$document" in
        spec/grammar-v2.md) continue ;;
    esac
    header=$(sed -n 's/^> Spec version: \([^ ]*\).*$/\1/p' "$document" | head -n 1)
    if [ -z "$header" ]; then
        printf 'version check: %s lacks a spec version header\n' "$document" >&2
        status=1
    elif [ "$header" != "$crate_version" ]; then
        printf 'version check: %s has %s, expected %s\n' "$document" "$header" "$crate_version" >&2
        status=1
    fi
done

if [ "$status" -eq 0 ]; then
    printf 'version check: %s\n' "$crate_version"
fi

exit "$status"
