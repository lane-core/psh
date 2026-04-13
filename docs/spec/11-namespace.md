# Namespace

## Namespace (three tiers)

| Tier | Resolution | Structural rules |
|---|---|---|
| Shell variables | `$x` — scope chain lookup | Weakening, contraction, exchange (classical) |
| Process environment | `env.PATH` — flat key-value | Weakening, contraction, exchange (classical) |
| Filesystem namespace | `/srv/window/cursor` — read from filesystem | Weakening, exchange. **No contraction** — each read is a fresh operation. |

The first two tiers admit all three structural rules (classical
contexts). The filesystem tier restricts contraction — reading
a file twice may yield different results if the underlying state
changed. This is honest: the shell does not guarantee coherence
for filesystem reads.

`get`/`set` builtins resolve against all three tiers uniformly.
The namespace grows; the language does not. This is Plan 9's
principle: `/env` was a filesystem [Duf90, §Environment]; psh
extends the scope chain into the filesystem honestly.

**Per-command local variables** (rc heritage, rc.ms lines
1045-1066): `VAR=value cmd` scopes the assignment to the
duration of a single command. The variable reverts after the
command completes. This is the terse per-command form for
environment setup — `PATH='/custom/bin' make install` — and
is distinct from `let` block scoping. Both compose: block
scoping covers compound blocks, per-command scoping covers
the common single-command case.


