# Discipline functions

## Discipline functions

A variable may be equipped with **discipline cells** — `def`-
registered bodies that mediate observation, refresh, and mutation.
A variable so equipped is **codata** in the sense of the sequent
calculus: its behavior is defined by destructors, and the
discipline cells *are* the variable's semantics. Three disciplines
are recognized:

- **`.get`** — the **pure observer**. A body of type `W(S, A)`
  that reads the stored slot and returns a value. No effects
  allowed.
- **`.refresh`** — the **effectful updater**. A body in `Kl(Ψ)`
  that may invoke arbitrary shell machinery (subshells, coprocess
  queries, filesystem reads) and writes the updated value into
  the stored slot. Invoked as an imperative command at a step
  boundary.
- **`.set`** — the **mutator**. A body that receives an incoming
  value, mediates the assignment, and writes the slot.

The split between observation and refresh is a return to rc's
observation philosophy ([Duf90] §Environment): observation is a read,
mutation is an imperative step, and the shell's reference model
never hides work behind a variable reference. Plan 9 realized
this via `/env` as a kernel filesystem; psh realizes the same
philosophy on contemporary unix-likes using whatever filesystem
or IPC mechanism the user chooses — the spirit is portable even
though Plan 9's specific mechanism is not.

ksh93 collapsed observation and refresh by allowing its `get`
discipline to run arbitrary shell code on every reference [KSH93,
§Discipline Functions]. psh declines to import that design: it
hides work at the reference site, conflicts with Duff's "no
hidden work" principle, and interacts unsoundly with session-
typed coprocess channels when a signal unwinds a polarity frame
holding a `PendingReply` obligation.

### The codata model

In the sequent calculus, data types are defined by constructors
(how to build a value) and eliminated by pattern matching. Codata
types are defined by destructors (how to observe or transform a
value) and eliminated by copattern matching: the producer is a
cocase that says how to respond to each destructor invocation
[BTMO23, §6.3].

A disciplined variable is the cocase

    cocase{ get(α)     ⇒ ⟨.get-body | α⟩,
            refresh(α) ⇒ ⟨.refresh-body | α⟩,
            set(v; α)  ⇒ ⟨.set-body[v] | α⟩ }

where `.get-body` is a pure producer in `W`, `.refresh-body` is
a statement in `Kl(Ψ)`, and `.set-body` is a statement taking one
producer argument (the incoming value) and mediating the slot
write. All three are **destructors** of the codata type; the
cocase is the sole constructor (the variable *is* its cocase).
Per [BTMO23, §6.3], a codata constructor is the whole cocase; `.set`
is a destructor with one producer argument, not a constructor
in its own right.

A variable without discipline cells is ordinary data: the stored
value is what you read, assignment replaces the stored value, and
there is no cocase.

### .get — the pure observer

A `.get` body is a pure computation `W(S, A)`: it reads the
stored slot and returns a value without invoking effects. No
subshells, no coprocess queries, no filesystem reads. Effects
belong in `.refresh`. By default every disciplined variable has
the trivial `.get` that reads its stored slot as a pure value;
user-defined `.get` bodies are permitted but must remain pure
(typically to compute a derived view of the slot).

The once-per-expression reuse property is a theorem, not an
operational convention: pure maps into positive values are
thunkable by construction in the symmetric monoidal duploid, and
thunkable maps are central [MMM, Prop 8550]. Central maps may be
reused at every consumption site inside an expression without
disturbing composition order. CBV argument expansion therefore
evaluates `.get` once and shares the result at every occurrence
of the variable in the same expression, as a consequence of
thunkability — not as an appeal to Downen-style static focusing
(which is a syntactic rewrite pass, not a runtime reuse
mechanism).

There is no polarity frame around `.get`. The input and output
both live in the positive subcategory `P_t`; there is no polarity
crossing, and nothing to reenter.

### .refresh — the effectful updater

A `.refresh` body is a statement in `Kl(Ψ)`: it may invoke any
shell machinery — subshells, coprocess queries, filesystem reads,
pipelines — and is responsible for writing the updated value into
the stored slot. It is invoked as an imperative command at a step
boundary, never implicitly by reference.

Canonical shape (portable across contemporary unix-likes; the
rc/Plan 9 "observation is a file read" philosophy [Duf90] §Environment
realized on unix without requiring `/env` or 9P services):

    let mut cursor = 0
    def cursor.refresh {
        cursor = `{ cat $XDG_STATE_HOME/psh/cursor }
    }

    cursor.refresh
    echo $cursor

`cursor.refresh` is a command-position invocation of the
discipline cell, parsed as a single NAME head and looked up
in Θ — syntactically the same shape as invoking a `def`-named
computation, and semantically the destructor `.refresh` of the
disciplined variable's cocase (§"The codata model"). It runs
at a step boundary, produces a status, and composes with
`try`/`catch` and `trap` the same way any other command does.
The parser's NAME-head dispatch plus the capitalization
convention (`def Type::method` with `::` for per-type methods;
`def varname.discipline` with `.` for per-variable disciplines)
is enough to disambiguate `cursor.refresh` from a
per-type method invocation. Users who want the ksh93 "live
variable" ergonomics wrap the pair in their own function — the
rc `fn cd` pattern [Duf90, §Functions] applied to discipline
invocation:

    def show_cursor { cursor.refresh; echo $cursor }

`.refresh` is the site of the ↓→↑ polarity shift. The body runs
in computation mode inside a polarity frame that saves the
surrounding expansion context, runs the computation, and restores
the context on exit (see §Polarity frames). Inside the frame,
`cursor = value` is the primitive slot write: it bypasses the
cocase (which would recurse into `.refresh`) and writes the slot
directly.

Failure propagation is rc-native: `.refresh` errors surface as
a nonzero `$status` at the invocation site, which `try`/`catch`
catches the same way it catches any command failure. Silencing
requires the user's explicit `try { cursor.refresh } catch (_) { }`.

**Race bound under frame unwind.** A `.refresh` body that
issues coprocess requests holds its `PendingReply` tag
obligations inside the polarity frame. If the frame unwinds
before the body completes — signal handler issues `return N`,
`try`/`catch` aborts — the outstanding tags enter the draining
state described in §"Shell-internal tracking" and any stale
Rresponse is discarded. The primitive slot write at the end of
the body is unreachable in this case, so the slot retains its
prior value. This bounds the drop-as-cancel race: the window
is the duration of an explicit `cursor.refresh` invocation,
not every variable reference, and the slot is always either
fully updated or fully untouched (never half-written). Users
who need transactional semantics across cancel should wrap the
refresh in `try { cursor.refresh } catch (_) { }` and test for
the prior-value case explicitly.

### .set — the mutator

A `.set` body receives the incoming value as `$1` and mediates
the assignment. Unlike `.get`, `.set` may have effects — the
assignment is already at a step boundary, and effects at that
point are user-visible and expected.

    def x.set {
        # $1 is the new value being assigned
        # the body may validate, transform, reject, or write
        # the slot via the primitive assignment x = v
    }

`.set` fires on every assignment to `x`. Typical patterns:

- **Validation.** Reject assignments that don't meet a constraint,
  by calling `return` with a nonzero status.
- **Transformation.** Normalize or clamp the value before storing
  (e.g., clamp a percentage to 0-100).
- **Propagation.** Write the value to an external resource
  (coprocess, filesystem) as a side effect of the assignment.
- **Notification.** Log the change, emit metrics, trigger
  dependent updates.

**Who writes the stored slot.** Under the cocase framing, the
`.set` body owns the write. Inside `.set`'s polarity frame,
`x = v` is the primitive slot write: it bypasses the cocase
(which would recurse into `.set`) and writes the stored slot
directly. A `.set` body that does not perform such an assignment
does not update the slot. The evaluator does not write the slot
after `.set` returns — every state transition goes through a
destructor body [BTMO23, §6.3]. This makes `.set` the sole legitimate
writer of a disciplined variable's slot from the assignment
side; `.refresh` is the legitimate writer from the observation
side.

### Reentrancy and the polarity frame

`.refresh` and `.set` bodies may reference the variable they are
mediating (a `.refresh` that reads `$x` to compute the next value,
a `.set` that reads the old value before deciding what to store).
Within such a body, a reference to `$x` fires the default pure
`.get` on the current slot — which returns whatever value is
currently stored. There is no reentrancy problem at `.get`: pure
observation has no frame to reenter.

`.refresh` and `.set` themselves are guarded by polarity frames
(see §Polarity frames). Each frame saves the expansion context,
raises a reentrancy flag on the variable, runs the body, restores
the context on exit, and clears the flag. Inside the frame,
`x = v` writes the slot directly (it bypasses the cocase).
Recursive invocation of `.refresh` within its own body — calling
`x.refresh` while the flag is raised — is caught by the flag and
reported as a runtime error; a discipline that needs to refresh
itself mid-refresh is ill-defined.

The narrowing relative to prior drafts: only `.refresh` and
`.set` need polarity frames. `.get` is pure and needs none,
which is the simplest possible reentrancy story — there is
nothing to reenter.

### Mixed-optic structure

A variable with `.get` and `.set` (with or without `.refresh`)
is a **mixed optic** in Clarke's sense — specifically a monadic
lens [Clarke, def:monadiclens]:

    MndLens_Ψ((A,B),(S,T)) = W(S, A) × W(S × B, ΨT)

The view `.get` lives in the pure base `W`, and the update
`.set` lives in `Kl(Ψ)` — two different categories of morphisms,
glued by the action `⋊ : W × Kl(Ψ) → Kl(Ψ)` [Clarke,
prop:monadiclens]. "Mixed optic" is Clarke's term for an optic
whose decomposition and reconstruction categories differ; it
is not a term for "impure optic." psh's disciplined variable is
monadic-lens-shaped precisely because its view is pure and its
update is effectful — the defining case of a mixed optic. Earlier
drafts of this spec claimed the psh construct was "a proper
monadic lens, not a mixed optic," which inverted Clarke's
terminology; `prop:monadiclens` explicitly establishes that
monadic lenses *are* mixed optics.

The monadic lens laws [Clarke, cited via AbouSaleh et al. 2016
§2.3], stated up to Kleisli equality on the update side:

- **PutGet.**  `set s b >>=_Ψ get  ≡  return b` — the view is
  pure on the right side of the equation; the left side runs one
  effect and discards it. Holds when `.set` stores the value
  faithfully.
- **GetPut.**  `get s >>=_Ψ (λa. set s a)  ≡_Ψ  return s` — the
  view produces a pure value, the update writes it back. Holds
  when `.set` is inverse to `.get` on the stored slot.
- **PutPut.**  `set s b₁ >>=_Ψ (λs'. set s' b₂)  ≡_Ψ  set s b₂`
  — consecutive writes collapse to the last. Holds when `.set`
  is idempotent under Kleisli composition.

For ordinary variables without discipline cells, the view is
identity in `W`, the update is trivial, and all three laws hold
unconditionally. Adding `.set` turns the laws into contracts the
user maintains by discipline: a `.set` that silently transforms
its input breaks PutGet; a non-idempotent `.set` breaks PutPut.
The spec does not mechanically verify the laws; they are
documented here as the expected discipline and the user's
obligation.

`.refresh` is orthogonal to the mixed-optic structure. It is an
imperative update to the view — a "write from outside" — that
the user invokes explicitly. The view remains pure between
refreshes; `.refresh` is not part of the lens, and its presence
or absence does not change the lens laws.


