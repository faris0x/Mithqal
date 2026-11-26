# Versioning

> Spec version: 0.0.3 (draft)

## Version format

The language version is independent of any crate or application version. It uses
a three part number.

A major version changes an observable semantic result.

A minor version adds syntax, primitives, profiles, or corpus cases that were
previously invalid or undefined.

A patch version clarifies prose or adds corpus cases that were already implied by
the normative text.

## The corpus and version changes

A corpus case may be added in a minor or patch version.

A corpus case may be changed or removed only in a major version, and the change
MUST be listed in the release notes with the reason.

A change that makes a previously conforming implementation nonconforming is a
major version.

## Frozen surface

The 0.0.x series is a draft. The frozen surface begins at version 0.1.0, matching
the crate stability policy. The following are frozen from version 0.1.0 onward.
A change to any of them requires a major version.

- abstract terms and term identity;
- canonical order and canonical printing;
- stable error codes;
- evaluation order and rule selection;
- scoping and hygiene;
- module privacy and export semantics;
- binder canonicalization and alpha equivalence;
- structure and instance semantics;
- the conformance corpus format.

## Evolvable surface

The following may change in a minor version when the change is additive.

- surface sugar and statement forms;
- notation declarations;
- the standard library API;
- diagnostic message text;
- performance and resource thresholds.

## Deprecation

A feature may be deprecated in a minor version. A deprecated feature MUST remain
available until the next major version. The release notes MUST name the
replacement.

## Conformance reports

A report names the spec version it was run against. A report against a different
major version is not comparable.
