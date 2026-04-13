# Callables

## Two kinds of callable

ksh93's compound variables [SPEC, §Compound variables] were its
struct system, never named as such. `typeset -C` created
name-value trees; `${x.field}` accessed them; disciplines
mediated access. psh's `def`/lambda distinction is informed by
this: ksh93 needed both effectful procedures (functions) and
inert data accessors (compound variable fields), but conflated
them in the `Namval_t` machinery.

| | `def` | `let` + lambda (`|x|`) |
|---|---|---|
| Sort | Command (consumer) | Value (term) |
| Arguments | Variadic, positional ($1, $2, $*) | Fixed arity, named |
| First-class | No — named computation in Θ | Yes — value in Γ, storable |
| Scope | Dynamic (reads current scope) | Captures at definition |
| Effects | May have effects (oblique map) | Purity inferred (thunkable when pure) |
| CBPV type | `F(Status)` | `U(A → B)` or `U(A → F(B))` |
| rc analog | `fn name { body }` [Duf90, §Functions] | (no rc analog — extension) |
| Invocation | `name arg1 arg2` | `name arg1 arg2` (bare word forces the lambda) |

The `def` keyword replaces rc's `fn`. psh renames it because
psh draws a distinction between named computations and
first-class functions that rc did not make. `def` is neutral
— it defines a named computation without claiming its role
in a cut, which only happens at the invocation site.

**`return` typing.** `return` is a μ-binding: `return v` =
`μα.⟨v | α⟩` where α is the `def`'s outer continuation. In
value-returning defs (`def name : ReturnType`), `return expr`
checks `expr` against the declared return type (check-mode).
In status-returning defs (no declared return type), `return N`
checks N against `Int`. Implicit return from the final
expression in a body also checks against the declared return
type. Every `return` in a body must agree on type — multiple
return paths are checked against the same declared type.

**`for`/`while` typing.** Both are consumers. `for(x in
list) { body }`: the list expression is a producer in synth-mode;
the loop variable `x` is a μ̃-binder scoped to the body, typed as
the list's element type; the body is a command sequence. The
result status is the last iteration's status (0 for empty
iteration — rc convention). `while(cond) { body }`: the
condition pipeline is a command producing a status that drives
⊕ coproduct elimination (continue on zero, stop on nonzero);
the body is a command sequence. Both are standard consumer forms —
no polarity frame, no shift.


