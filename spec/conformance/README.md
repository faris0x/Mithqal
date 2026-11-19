# Conformance Corpus

> Spec version: 0.0.2 (draft)

The corpus is the executable form of the specification. A corpus case is
normative. An implementation conforms to a profile when it passes every case in
that profile and in the profiles the profile depends on.

## Files

Each profile has one tab separated file:

- `core.tsv`
- `exact.tsv`
- `numeric.tsv`
- `modules.tsv`
- `embedded.tsv`

A runner MAY load the files in any order. The order of cases within a file is
not significant.

## Columns

Each non-comment line has six tab separated fields.

1. `id`: a unique case identifier. The identifier begins with the profile name.
2. `profile`: one of `core`, `exact`, `numeric`, `modules`, `embedded`.
3. `kind`: one of the case kinds below.
4. `expected`: the expected result.
5. `source`: the Mithqal source for the case.
6. `rationale`: a short statement of the property the case checks.

A line that begins with `#` is a comment. A blank line is ignored.

## Escapes

The `expected` and `source` fields use backslash escapes. `\n` is a newline,
`\t` is a tab, and `\\` is a backslash. Any other backslash sequence is kept
literally.

## Case kinds

`value` evaluates the source in a fresh session and compares the canonical
printing of the final value with `expected`.

`unevaluated` is the same as `value` and records that the expected canonical
form is an unevaluated call.

`output` evaluates the source and compares the lines produced by `prt` with
`expected`, joined by newlines.

`parse` requires the source to fail with a parse diagnostic. `expected` is the
stable error code.

`diagnostic` requires the source to fail with a diagnostic of any kind.
`expected` is the stable error code.

`trace_line` requires the source to fail, and requires the diagnostic span or a
trace frame to carry the line number in `expected`.

`roundtrip` parses the source as a statement, prints it canonically, parses the
result again, and requires the two canonical forms to be equal. When `expected`
is not empty, the first canonical form MUST equal it.

## Running

A runner creates a fresh session for each case and evaluates the source with a
source name equal to the case identifier, so diagnostics carry a stable
location. A runner MAY filter by profile. The reference runner is
`tests/conformance.rs` and can be filtered with
`MITHQAL_CONFORMANCE_PROFILE`.

## Adding cases

A new case is added in a minor or patch version when the behavior it records is
already implied by the specification. A case is added in a major version when it
records a new intended behavior. A case is never deleted without a major
version.

A new case MUST have a unique identifier and a nonempty rationale.
