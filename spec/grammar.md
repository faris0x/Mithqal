# Surface Grammar

> Spec version: 0.0.3 (draft)

## Source encoding

A source file is Unicode. Line endings may be LF or CRLF. A source file is a
sequence of sentences. A sentence ends with a period.

The period is mandatory. A period inside a number, string, parenthesized
expression, list, or association does not end a sentence.

## Comments

`//` begins a line comment. `(*` begins a block comment terminated by `*)`.
Comments may appear between sentences.

## Identifiers

An identifier begins with a Unicode letter or `_` and continues with Unicode
letters, digits, and underscores. Built-in names use dromedaryCase.

A context-qualified name is a sequence of identifiers separated by `;`, as in
`ctx;name`. System names begin with `$`.

## Module grammar

```text
program       := "module" name "." module_item* "end module" "."
module_item   := use | export | sentence
use           := "use" (name | string) "."
export        := "export" name_list "."
```

Statements after `end module` are rejected.

## Sentences

```text
sentence      := clause_list_or_statement "."
clause        := "where" condition
               | "assuming" condition
               | "provided that" condition
```

Clauses may appear before or after a statement. All clauses in a sentence
attach to that sentence.

```text
statement     := definition
               | binding
               | print
               | assumption
               | conditional
               | match
               | structure
               | instance
               | theorem
```

## Definitions and bindings

```text
definition    := "we define" pattern "=" expression
binding       := "let" name "be" expression
print         := ("print" | "show") expression
assumption    := "assume" condition
```

A definition with no parameters defines a value. A definition whose left side
is a call defines rules.

## Conditionals and matches

```text
conditional   := "if" condition "then:" sentence*
                 ("else:" sentence*)? "end if" "."
match         := "match" expression "with:" case*
                 ("else:" sentence*)? "end match" "."
case          := "case" pattern ":" sentence*
```

## Structures and instances

```text
structure     := "structure" name "."
                 ("field" name "." | "law" name ":" expression ".")*
                 "end structure" "."
instance      := "instance" name ":" name ("assumed")? "."
                 (name "=" expression ".")*
                 "end instance" "."
```

## Theorems and proofs

```text
theorem       := "theorem" name "." sentence "proof:" proof_step* "end proof" "."
proof_step    := "assume" condition "."
               | "have" name ":" condition "."
               | "show" condition "."
               | "by" justification "."
```

## Patterns

```text
pattern       := name
               | name "..."
               | name "(" pattern_list? ")"
               | "[" pattern_list? "]"
               | literal
```

A bare name binds a value. A trailing `...` binds a sequence. A call pattern
uses its head as a constructor. A list pattern matches a list.

## Expressions

Expression forms follow the existing term grammar: literals, identifiers,
calls, lists, associations, indexing, anonymous functions, binders, and
parenthesized expressions. Infix operator precedence is:

1. calls and indexing
2. `^`
3. unary `-` and `!`
4. `*`, `/`, juxtaposition
5. `+`, `-`
6. comparisons
7. `&&`
8. `||`

## Derivative notation

`d(expression)/dx` and `d/dx (expression)` denote the derivative with respect to
`x`. A leading `*` selects the global notation binding when a local `d` shadows
it.

## Unicode normalization

Unicode and ASCII operator aliases denote the same tokens. The canonical printer
emits Unicode.
