---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: normal
keywords: [catch, try, parenthesized, binding, grammar, consistency, rc-parens]
agents: [psh-architect, psh-sequent-calculus, plan9-systems-engineer]
related: [decision/try_catch_scoped_errort, analysis/error_duality_oplus_par, analysis/cbpv_f_u_separation]
supersedes: [pre-2026-04-10 bare-NAME form of catch binding in syntax.md:255 and related locations]
verified_against: [docs/syntax.md@HEAD, docs/specification.md@HEAD, docs/deliberations.md@HEAD, PLAN.md@HEAD]
---

# Decision: `catch` binding form is parenthesized

## Decision

The `try_cmd` grammar production uses **parenthesized** catch binding:

```
try_cmd = 'try' body 'catch' '(' NAME ')' body
```

not the prior bare form:

```
try_cmd = 'try' body 'catch' NAME body
```

Concrete syntax: `try { body } catch (e) { handler }`.

## Why

The `try_cmd` production was the lone outlier among the rc-style control constructs. `if_cmd`, `for_cmd`, `while_cmd`, and `match_cmd` all parenthesize their bound name or condition with `'(' ... ')'`:

- `if_cmd = 'if' '(' pipeline ')' body ...`
- `for_cmd = 'for' '(' NAME 'in' value ')' body`
- `while_cmd = 'while' '(' pipeline ')' body`
- `match_cmd = 'match' '(' value ')' '{' ...`

`try_cmd` with bare `NAME` broke the pattern. Lane requested the change during the 2026-04-10 agent ecosystem setup session. Alternative: keep bare `NAME`, which was deemed inconsistent with the rest of the grammar.

## Consequences

**Locations updated** in the 2026-04-10 change:

- `docs/syntax.md:255` — grammar production tightened
- `docs/syntax.md:376` — example updated (`} catch (e) {`)
- `docs/syntax.md:384` — prose updated ("The `catch (e)` binding is a μ̃-binder")
- `docs/specification.md:283` — coterm table row
- `docs/specification.md:293` — `catch bindings` prose
- `docs/specification.md:959` — try/catch scoped ErrorT prose
- `docs/specification.md:1021` — signal interaction section prose
- `PLAN.md:81, :119` — roadmap references

`docs/deliberations.md` has §"`catch` binding form: parenthesized (APPLIED)" as the supersession record citing each updated location.

Spec: `docs/specification.md` §"Error model §try/catch". Syntax: `docs/syntax.md` §"try / catch". Ledger: `docs/deliberations.md` §"`catch` binding form: parenthesized (APPLIED)".
