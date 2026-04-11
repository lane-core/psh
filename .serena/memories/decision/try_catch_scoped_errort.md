---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [try, catch, errort, monad-transformer, sequencing, oplus, status-check, scoped, set-e]
agents: [psh-sequent-calculus, psh-architect, vdc-theory]
related: [decision/unified_trap_three_forms, decision/catch_binding_parenthesized]
---

# Decision: `try { body } catch (e) { handler }` as scoped ErrorT

## Decision

`try { body } catch (e) { handler }` changes the sequencing combinator inside `body` from unconditional `;` to **monadic `;ₜ` that checks Status after each command**. On nonzero status, execution aborts to the handler; the handler binding `e` is a **μ̃-binder** on the error case. Boolean contexts (`if`/`while` conditions, `&&`/`||` LHS, `!` commands) are **exempt** from the check.

This is the **⊕ side** of the error duality — caller-inspects-tagged-value.

## Why

POSIX `set -e` has well-known composability defects: the check-after-each-command rule interacts badly with boolean contexts, pipelines, subshells, and function calls. `try`/`catch` fixes this by being **lexically scoped** — the modified sequencing only applies inside `body`, and boolean-context exemption is explicit in the semantics (not a subtle quirk).

As a category-theoretic construction, `try` applies a local **ErrorT monad transformer** over the command-sequencing monad. The `;` combinator outside is unconditional (runs the next command regardless); `;ₜ` inside inspects status and short-circuits on nonzero. When the body exits normally, the transformer unwraps and sequencing returns to unconditional.

Alternative considered: a global `set -e` mode or an explicit `?`-check operator. Rejected: scoped is more composable (no global mode leak), and `;ₜ` is structural rather than requiring a new operator on every statement.

## Consequences

- `try { cmd1; cmd2; cmd3 } catch (e) { handler }` runs cmd1, then cmd2 only if cmd1 succeeded, etc. On any failure, `handler` runs with `e` bound to the error status. `cmd3`'s status becomes the body's status if everything succeeded.
- Boolean contexts exempt: `try { if $check { cmd } }` does not treat `$check` failing as an error to catch — `if` is a boolean context.
- `try` and `trap` compose orthogonally (see `decision/unified_trap_three_forms`). Signals fire at step boundaries and are handled by trap; statuses flow through try's `;ₜ`.
- The `catch (e)` form is parenthesized for grammar consistency with `if_cmd`, `for_cmd`, `while_cmd`, `match_cmd`. See `decision/catch_binding_parenthesized`.

Spec: `docs/specification.md` §"Error model §try/catch — scoped ErrorT", §"Signal interaction with try blocks". Syntax: `docs/syntax.md` §"try / catch".
