# Error model

## Error model

### ⊕ and ⅋

Every command returns `Status = Result((), ExitCode)` — a
genuine ⊕ coproduct (see §ExitCode and Status for definitions).
rc heritage [Duf90, §Exit status]: "On Plan 9 status is a
character string describing an error condition. On normal
termination it is empty." psh adapts this to Unix: the numeric
code is the POSIX reality (`waitpid` 0-255); the descriptive
string lives in `ExitCode.message`, populated by builtins and
signal-death synthesis.

Linear logic gives two disjunction connectives — not two names
for the same thing, but two genuinely different kinds of error
handling [BTMO23, §"Linear Logic and the Duality of Exceptions"]:

- **⊕ (plus, positive / data):** a tagged return value.
  Constructors `Inl(t)` / `Inr(t)`; elimination by
  `case{Inl(x) ⇒ s₁, Inr(y) ⇒ s₂}`. The caller inspects the
  tag. Rust's `Result<T, E>` and Haskell's `Either` are this
  shape. **psh's Status is ⊕**: every command returns a
  tagged value `ok | err(ExitCode)`, and
  `try { body } catch (e) { handler }` is the coproduct
  elimination — `e : ExitCode` binds the error payload.

  **Pipeline status.** `$status : ExitCode` holds the exit
  code of the last command (or the last pipeline component).
  For full pipeline diagnostics, `$pipestatus : List(ExitCode)`
  holds the exit codes of all pipeline components in order.
  Two variables, two types — `$status` never changes type.
  For a simple command (not a pipeline), `$pipestatus` is a
  single-element list equal to `($status)`. This follows
  bash/zsh convention (`$PIPESTATUS` / `$pipestatus`) with
  psh's native list type.

- **⅋ (par, negative / codata):** a pair — more generally an
  N-tuple — of continuations, one per outcome. The callee
  decides which continuation to invoke. A cocase
  `cocase{Par(α,β) ⇒ s}` binds two covariables and the body
  `s` jumps to exactly one of them. **psh's `trap SIGNAL
  { handler } { body }` is ⅋**: the body runs under a cocase
  binding one continuation per installed handler, and signal
  delivery is the callee (the shell's step-boundary dispatcher)
  jumping to the chosen handler's α. `trap` is codata — the
  same codata framing psh uses for discipline cells (§"The
  codata model").

The two are De Morgan duals: `(σ ⊕ τ)⁻ = σ⁻ ⅋ τ⁻`. Both styles
are present in psh because both styles show up in shell
programs. `try`/`catch` is the data/caller-inspects form;
`trap` is the codata/callee-jumps form. They compose
orthogonally (§"Signal interaction with try blocks") because
they act on different sorts: `try` on command statuses in the
positive Γ, `trap` on signal continuations in the negative Δ.

### try/catch — scoped ErrorT (⊕ discipline)

`try { body } catch (e) { handler }` changes the sequencing
combinator within `body` from unconditional `;` to monadic `;ₜ`
that checks Status after each command. On nonzero status,
execution aborts to the handler. The handler binding
`e : ExitCode` is a μ̃-binder on the error case — the
coproduct elimination of `Status = Result((), ExitCode)`.
The handler can inspect `$e.code` and `$e.message`.

Equivalent to lexically-scoped `set -e` without POSIX `set -e`'s
composability defects. Boolean contexts (if/while conditions,
&&/|| LHS, `!` commands) are exempt.

### trap — unified signal handling (⅋ discipline)

Grammar: `trap SIGNAL (body body?)?`. Three forms distinguished
by block count:

**Lexical** (two blocks): `trap SIGNAL { handler } { body }`
— installs the handler for the duration of the body, the
μ-binder of Curien-Herbelin [CH00, §2.1]. The handler captures a
signal continuation scoped to the body. Inner lexical traps
shadow outer for the same signal. The handler may `return N`
to abort the body with status N.

**Global** (one block): `trap SIGNAL { handler }` — registers
the handler at the top-level object's signal interface.
Persists until overridden or removed. This is the vertical
arrow form: it modifies the ambient signal-channel interface.

**Deletion** (no block): `trap SIGNAL` — removes a
previously-installed global handler.

Precedence at signal delivery: innermost lexical > outer
lexical > global > OS default. Signal masking via empty
handler (`trap SIGNAL { }` in either scope).

### Signal delivery model

Signals fire at **interpreter step boundaries**, which include
(1) between-command points in a block and (2) wake-from-block
points during child waits. The shell's main loop uses `poll(2)`
on both the child-status fd and a self-pipe. When a signal
arrives during a child wait, the self-pipe wakes the poll loop;
the shell handles the signal before resuming the wait. For
SIGINT specifically, the shell forwards the signal to the
child's process group (`kill(-pgid, SIGINT)`) so that
long-running children can be interrupted.

**EINTR policy:** builtins retry on EINTR by default. External
commands handle EINTR themselves — if an external command exits
nonzero due to interruption, that status flows through `try`
normally. This avoids spurious `catch` triggers from transient
EINTR returns in builtins.

**EXIT handler:** `EXIT` is an artificial signal synthesized by
the shell when the process is about to exit (rc heritage —
Duff's `sigexit`). It fires on normal exit and on `exit` called
from within a signal handler. Each process (including
subshells created via `@{ }`) has its own EXIT handler
registration; inherited handlers are copies that evolve
independently in the child (Plan 9 `rfork` model).

### Signal interaction with try blocks

When a signal arrives during a `try { body } catch (e) { }`:

1. **Between commands, handler returns.** The signal fires at
   the next step boundary. If the handler calls `return N`,
   the try body terminates with status N. The catch clause
   fires if N is nonzero.

2. **Between commands, handler does not return.** The handler
   runs its side effects, execution resumes at the next
   command, and try's status check proceeds normally.

3. **Handler calls `exit`.** The shell (or subshell) exits.
   Neither the remaining try body nor the catch clause runs.
   EXIT handlers fire during shutdown.

4. **Builtin interrupted by signal (EINTR).** The builtin
   retries on EINTR. The signal flag is still set. The handler
   fires at the next step boundary after the builtin completes.

`trap` and `try` compose orthogonally because they operate on
different sorts: `trap` on signal continuations (⅋), `try` on
command status (⊕). A lexical `trap` inside a `try` body fires
first when a signal arrives; if the trap returns a status, try
inspects it through its normal status-check mechanism.


