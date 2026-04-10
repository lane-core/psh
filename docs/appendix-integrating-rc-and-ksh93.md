# Appendix: Integrating Rc and Ksh93 in the Virtual Double Category Framework

## A.0 Purpose

This appendix bridges two documents: the *Sequences, Not Strings* report, which develops the virtual double category (VDC) interpretation of rc's shell semantics, and the *ksh26 Theoretical Foundation* (SPEC.md), which maps sequent calculus and duploid semantics onto the ksh93 interpreter. The goal is to show an implementer how features from each shell can be understood, evaluated, and integrated using the shared formal framework.

The two shells are complementary. Rc has the right data model (list-valued variables, no rescanning, structural substitution) but a minimal feature set (no compound variables, no coprocesses, no discipline functions, no accessor notation). Ksh93 has a rich feature set but the wrong data model (string-valued variables, IFS splitting, rescanning-dependent semantics) and an implementation where structural invariants are maintained by convention rather than by construction.

The framework — virtual double categories for the algebraic structure, sequent calculus for the type theory, duploid semantics for the composition laws — lets us extract ksh93's features and re-seat them in rc's data model, preserving Duff's principle at every step.


---


## A.1 Common Ground

Both shells already exhibit the three-sorted structure of the λμμ̃-calculus, though they realize it differently.

### The three sorts

| Sort | λμμ̃ | Rc | Ksh93 |
|------|------|-----|-------|
| Producer (term) | t : A | Word: literal, `$var`, `'{cmd}` | Word: literal, `$var`, `$(cmd)`, `$((expr))` |
| Consumer (coterm) | e : A⊥ | Context: pipe reader, redirect target, `for` body | Context: pipe reader, redirect, trap handler, discipline function |
| Statement (command) | ⟨t \| e⟩ | Command: `echo $path` | Command: `echo $path` |

In rc, the three sorts are distinguished by syntax and evaluation convention. In ksh93, they are additionally distinguished in the AST (`Shnode_t` tagged by `tretyp & COMMSK`) and in the interpreter dispatch (`sh_exec()` switching on node type). The SPEC classifies every AST node by polarity — Value, Computation, or Mixed — which is a refinement of the three-sort distinction that rc's simpler structure doesn't require.

### Cut as execution

In both shells, command execution is a cut: a producer (arguments, expanded words) meets a consumer (the command body, the execution context). The pipe operator is a cut on a stream channel; variable assignment is a μ̃-binding; redirection is a μ-binding. This shared structure is why the framework applies to both.

### Where they diverge

The divergence is in the **horizontal arrows** — the channels, the data that flows between producers and consumers.

In rc, horizontal arrows are **lists of strings**. The type system is trivial: every channel carries the same type. But the *structure* is right: lists preserve boundaries, substitution is structural, and the no-rescan invariant holds.

In ksh93, horizontal arrows are **strings with richer types** (integers, floats, compound variables, associative arrays, name references) but the *structure* is wrong: values are flattened to strings at substitution boundaries, IFS splitting reintroduces parsing, and the rescanning problem persists for word splitting and globbing.

The integration task is: take ksh93's richer type repertoire and seat it in rc's structural data model, using the VDC framework to ensure that the richer types preserve boundaries and the sequent calculus to ensure that the richer type operations respect polarity.


---


## A.2 Rc Features That Ksh93 Should Adopt

These are features where rc's design is structurally superior and ksh93's approach is a known source of bugs. The VDC framework explains *why* they are superior.

### A.2.1 List-valued variables

**Rc:**

```
path=(. /bin)
echo $#path          # prints 2
echo $path(2)        # prints /bin
```

A variable holds a list. The list has a definite length. Elements are individually addressable. Substitution splices the list without rescanning.

**Ksh93:**

```
PATH=".:/bin"
# No native way to count path components
# No native way to index into the path
# Splitting on : requires parameter expansion tricks
IFS=:; set -- $PATH; echo $#   # prints 2, but mutates $@ and IFS
```

A variable holds a string. To recover structure, the string must be split — but splitting depends on IFS, which is global mutable state. The split is a forced Segal condition: the string is the composite, and IFS-splitting tries to recover the sequence from the composite. This recovery is lossy (elements containing the delimiter are mis-split) and fragile (IFS is mutable).

**VDC interpretation:** In the virtual double category **Shell**, a list-valued variable is a horizontal arrow whose *type* is a sequence type — the arrow carries a definite number of elements with preserved boundaries. A string-valued variable is a horizontal arrow whose type is a single string, and splitting is the (destructive, lossy) attempt to compute a composite-to-sequence factorization.

**Integration principle:** All variables in the unified shell hold lists, as in rc. Types are refined (see §A.3) but the list structure is universal. A variable of type integer holds a list of one integer; a variable of type compound holds a list of one compound value. The empty list `()` is the unset state. This eliminates IFS-dependent word splitting entirely.

### A.2.2 Structural substitution

**Rc:**

```
files=(foo bar 'file with spaces')
wc $files
# Equivalent to: wc foo bar 'file with spaces'
# Three arguments, regardless of content
```

**Ksh93:**

```
files="foo bar file with spaces"
wc $files
# Equivalent to: wc foo bar file with spaces
# Five arguments — the spaces in "file with spaces" cause mis-splitting
# Fix: wc "$files" — but this gives ONE argument
# Neither behavior is correct
```

Ksh93 arrays partially fix this (`files=(foo bar 'file with spaces'); wc "${files[@]}"`), but the fix requires double-quoting `"${array[@]}"` — a syntactic incantation that exists solely to suppress the rescanning that should never have happened. Forgetting the quotes is the single most common shell bug.

**VDC interpretation:** Substitution is operadic composition of cells. The cell `wc` has a top boundary expecting a sequence of filename channels. The variable `$files` holds a sequence of three horizontal arrows. Substitution plugs the sequence into the top boundary. The length of the sequence is a property of the data, not of the delimiter content.

**Integration principle:** Substitution always splices lists structurally. There is no word splitting on substitution. Globbing applies to literal patterns in source code, not to the results of variable expansion. This is Duff's principle, and it is non-negotiable.

### A.2.3 Single-pass parsing

**Rc:**

```
size='{wc -l '{ls -t|sed 1q}}
```

Nested command substitutions are parsed in a single pass. The `'{...}` syntax is a unary operator taking a braced command. Nesting is handled by brace matching, not by escape counting.

**Ksh93:**

```
size=$(wc -l $(ls -t|sed 1q))
```

Ksh93's `$(...)` syntax also handles nesting correctly — this is one place where ksh93 improved on Bourne. The `$(...)` form is structurally equivalent to rc's `'{...}`. Both use delimiter matching rather than escape counting.

**Integration principle:** Adopt the `$(...)` or `'{...}` syntax (or both as aliases). The key property — single-pass parsability — is already shared. The difference is cosmetic.


---


## A.3 Ksh93 Features That Rc Should Adopt

These are features where ksh93's expressiveness is genuinely useful and rc has no equivalent. The VDC framework shows how to integrate them without violating Duff's principle.

### A.3.1 Typed variables (positive types)

**Ksh93:**

```
typeset -i count=0
typeset -F temperature=98.6
typeset -a colors=(red green blue)
typeset -A capitals=([France]=Paris [Germany]=Berlin)
```

Ksh93 variables carry type attributes: integer (`-i`), float (`-F`), indexed array (`-a`), associative array (`-A`). These types affect how assignment and arithmetic work.

**Rc has none of this.** Every value is a list of strings. Arithmetic requires calling external `expr` or relying on `test`.

**VDC interpretation:** Typed variables are horizontal arrows with refined types. In the virtual double category, the type of a horizontal arrow constrains what data it carries and what operations (cells) can act on it. An integer channel carries integers; an associative-array channel carries key-value pairs. The type is part of the arrow, not a runtime tag on the data.

These are all **positive types** — they are defined by their constructors (how you build a value). An integer is built by providing a numeral. An associative array is built by providing a set of key-value pairs. The consumer (the command that receives the value) must handle the type appropriately.

**How to integrate without violating Duff's principle:**

The critical constraint is: typed variables must still be lists. The list structure is orthogonal to the element type. A typed variable is a list of typed elements:

```
# Proposed syntax (rc-flavored, with type annotations)
count : int = (0)
temperatures : float = (98.6 99.1 97.8)
capitals : map[string, string] = ((France Paris) (Germany Berlin))
```

The list structure is preserved: `$#count` is `1`, `$#temperatures` is `3`. Substitution still splices lists. The type annotation constrains what values can be assigned and what operations are valid, but does not change the substitution semantics.

In the sequent calculus: these are positive formulas in the antecedent context (Γ). The type annotation is the formula; the variable name is the hypothesis label. Assignment is μ̃-binding; the type is checked at the binding site.

### A.3.2 Compound variables (positive product types)

**Ksh93:**

```
typeset -C point
point.x=3
point.y=4
echo ${point.x}   # prints 3
print -v point     # prints ( x=3 y=4 )
```

Compound variables are records — named fields accessible by dot notation. They can nest:

```
typeset -C rect
rect.origin.x=0
rect.origin.y=0
rect.size.width=10
rect.size.height=20
```

**Rc has nothing like this.** The only structure is the flat list.

**VDC interpretation:** A compound variable is a horizontal arrow carrying a **positive product type** (tensor, ⊗). In the sequent calculus, a product is introduced by pairing:

    Γ ⊢ t₁ : A      Γ ⊢ t₂ : B
    ─────────────────────────────
       Γ ⊢ (t₁, t₂) : A ⊗ B

And eliminated by pattern-matching:

    Γ, x : A, y : B ⊢ c
    ─────────────────────────────
    Γ ⊢ μ̃(x, y).c  :  (A ⊗ B)⊥

In the shell, introduction is construction (assigning fields), and elimination is destructuring (accessing fields). The dot notation `${point.x}` is a **projection** — it eliminates the product by selecting a component.

**How to integrate:**

Compound values become a kind of list element. A compound value is a single element that itself has internal structure:

```
point = ((x 3) (y 4))
echo $point.x           # prints 3 — accessor notation as destructor
```

The critical design decision: is `$point` a list of two pairs, or a single compound value? In the VDC framework, it depends on the type of the horizontal arrow. If the type is `List(Pair(string, int))`, then `$point` is a two-element list of pairs. If the type is `Record(x: int, y: int)`, then `$point` is a single compound value with named fields.

Both are valid. The list-of-pairs interpretation preserves rc's flat list model and makes `$#point` equal `2`. The record interpretation treats the compound as a single element and makes `$#point` equal `1`. The second is closer to ksh93's semantics and more useful for nesting.

**Proposed resolution:** Records are a new kind of scalar — a single list element with internal structure. `$#point` is `1`. Field access (`$point.x`) is destructor invocation on the record type. A list of records is a list of records:

```
points = ((x 3 y 4) (x 7 y 1))
echo $#points            # prints 2
echo $points(1).x        # prints 3
```

This preserves Duff's principle: the list has two elements, substitution splices two elements, and no rescanning occurs. The internal structure of each element is accessed by destructor (dot notation), not by reparsing.

### A.3.3 Discipline functions (codata)

**Ksh93:**

```
typeset -i count=0
function count.get {
    # Fires on every read of $count
    .sh.value=$(( .sh.value + 1 ))
}
echo $count   # prints 1
echo $count   # prints 2
echo $count   # prints 3
```

A discipline function defines a variable by its **behavior under observation** — how it responds to `get`, `set`, and `unset`. This is the textbook definition of codata: defined by destructors, not constructors.

**Rc has no equivalent.** Variables are inert data; reading a variable returns its value with no interposition.

**VDC interpretation:** A discipline variable is a horizontal arrow of **negative type** (codata). In the sequent calculus, a codata type is introduced by copattern matching — the producer provides a response for each destructor:

    Γ, α : A⊥ ⊢ s_get       Γ, α : A⊥ ⊢ s_set       Γ, α : A⊥ ⊢ s_unset
    ──────────────────────────────────────────────────────────────────────────
                  Γ ⊢ cocase { get(α) ⇒ s_get, set(α) ⇒ s_set, unset(α) ⇒ s_unset } : Var(A)

And eliminated by destructor invocation — the consumer chooses which destructor to call:

    Γ ⊢ get(e) : Var(A)⊥

In the shell, `$count` is a destructor invocation: it invokes `get` on the discipline variable. Assignment `count=5` invokes `set`. The variable's behavior is defined by its discipline functions, not by a stored value.

**The polarity boundary:** This is the critical point. Accessing a discipline variable is a **polarity shift**: the accessor appears in value position (positive — the `$count` is in an argument list), but the discipline function runs in computation mode (negative — it executes a function body). This is the ↓→↑ shift from the SPEC:

1. The expansion engine encounters `$count` (positive context, value mode)
2. The `get` discipline fires (shift to negative context, computation mode)
3. The discipline function runs and produces a value (shift back to positive)

The SPEC's polarity frame API (`sh_polarity_enter`/`sh_polarity_leave`) is exactly the mechanism needed to implement this shift safely. The discipline function runs inside a polarity frame; the frame saves the expansion context, runs the handler, and restores the context. This prevents the class of bugs documented in the SPEC (001, 002) where computation-mode operations corrupt value-mode state.

**How to integrate:**

```
fn count.get {
    # $value is the current stored value
    # the function's stdout (or a designated result variable) becomes the accessed value
    value = `{expr $value + 1}
}
```

The discipline function is an rc function whose name follows the `variable.destructor` pattern. The dot is the accessor — a destructor invocation in the codata type. The function is a cell whose top boundary includes the current stored value (a horizontal arrow of positive type) and whose bottom boundary is the value seen by the accessor (another horizontal arrow of positive type). The cell itself is the discipline — the computation that mediates between stored value and observed value.

Duff's principle is preserved: the discipline function is a pre-parsed cell (a function), not a string to be re-evaluated. The polarity shift is explicit (the function call boundary), not implicit (a hidden re-scan). The value produced by the discipline function is a list (positive, structural), and it is spliced into the command without rescanning.

### A.3.4 Accessor notation (destructor chaining)

**Ksh93:**

```
typeset -C config
config.db.host=localhost
config.db.port=5432
config.server.workers=4
echo ${config.db.host}       # prints localhost
```

Dot notation chains destructors: `config.db.host` is three levels of field access.

**VDC interpretation:** Each dot is a destructor invocation. In the sequent calculus (following the Grokking paper's analysis of direct vs indirect consumers), the chain

```
config.db.host
```

translates to a nested destructor:

```
μα. ⟨config | db(host(α)) ⟩
```

The destructors `db` and `host` compose directly — no intermediate bindings, no CPS indirection. This is the "direct consumer" pattern: the consumer is a chain of destructors applied directly to the producer. In the VDC, the chain is a sequence of horizontal arrows in the top boundary of a single cell, where each arrow carries a destructor name.

**How to integrate:**

```
config = (db (host localhost port 5432) server (workers 4))
echo $config.db.host    # prints localhost
```

The dot notation is syntactic sugar for destructor chaining. Each segment between dots is a destructor name. The chain resolves left to right: `config` is the producer, `.db` selects the `db` field, `.host` selects the `host` field within that.

The critical question is whether this is a value-level operation (pure projection, no computation) or a computation-level operation (discipline function, polarity shift). In the VDC framework:

- **If the variable has no discipline functions**, accessor notation is a pure projection — a value-level operation that does not cross a polarity boundary. It is a vertical arrow acting on the horizontal arrow (restricting/projecting the type). No polarity frame is needed.

- **If the variable has discipline functions**, accessor notation invokes a destructor — a computation-level operation that crosses a polarity boundary. The discipline function fires, a polarity frame is pushed, and the shift discipline from the SPEC applies.

The type system (the sequent calculus) distinguishes these cases: a record type (positive, data) has projections that are value-level; a disciplined type (negative, codata) has destructors that are computation-level. The implementer can inspect the variable's type to determine which path to take.

### A.3.5 Coprocesses (session-typed channels)

**Ksh93:**

```
cmd |&                         # start coprocess
print -p "input line"          # write to coprocess stdin
read -p result                 # read from coprocess stdout
echo $result
```

A coprocess is a background process with bidirectional communication: the shell can write to its stdin and read from its stdout. The coprocess runs concurrently.

**Rc has only unidirectional pipes:**

```
cmd1 | cmd2
```

There is no way to write to `cmd2`'s stdin from the shell while also reading from `cmd2`'s stdout. Rc's process substitution (`<{cmd}` and `>{cmd}`) allows non-linear pipeline topologies, but the communication is still unidirectional on each channel.

**VDC interpretation:** A coprocess is a **bidirectional horizontal arrow** — or more precisely, a pair of horizontal arrows in opposite directions, bundled into a single session. In the sequent calculus, this is a **session type**: a protocol describing the sequence of interactions between two processes.

A simple coprocess session type might be:

    CoprocSession = !String.?String.End

This reads: "send a string, then receive a string, then done." The `!` prefix means "output" (the shell writes); `?` means "input" (the shell reads). The session type constrains the order of operations: you must write before you read.

In the VDC, a coprocess is a cell with two horizontal arrows in its interface — one for each direction. The cell runs concurrently, and the session type governs the interaction protocol. The cut connects the shell's write-channel (a positive operation: constructing a value and sending it) with the coprocess's read-channel (a negative operation: waiting for input), and vice versa.

**How to integrate:**

Rc already has the right syntax for non-linear pipe topologies:

```
cmp <{old} <{new}
```

Extending this to coprocesses means allowing bidirectional connections:

```
cmd |&
echo input >[p]        # write to coprocess (fd p, write end)
line = '{read <[p]}    # read from coprocess (fd p, read end)
```

Or, using rc's redirection syntax more naturally:

```
cmd |[1=0]&            # coprocess: shell's fd 1 → cmd's fd 0, cmd's fd 1 → shell's fd p
```

The key design constraint, from the VDC framework: the coprocess's bidirectional channel is a horizontal arrow with a session type. Each read or write operation is a destructor/constructor invocation on the session type, and the type system ensures that operations occur in the correct order. A write-then-read protocol cannot be violated by reading before writing, because the type of the channel after construction (`!String.?String.End`) only permits output first.

**The polarity story:** Writing to a coprocess is a positive operation (constructing and sending a value). Reading from a coprocess is a negative operation (waiting for a computation to produce a value). The boundary between them is a polarity shift — specifically, the ↑ (return) shift that converts a computation's result into a value. The polarity frame from the SPEC mediates this boundary: the shell saves its expansion context, performs the read (blocking on the coprocess), receives the result, and restores the expansion context.

### A.3.6 Name references (indirection without rescanning)

**Ksh93:**

```
typeset -n ref=target
target=hello
echo $ref          # prints hello
ref=world
echo $target       # prints world
```

Name references (`typeset -n`) provide indirection: `$ref` transparently accesses `$target`. This is useful for functions that need to modify a caller's variable.

**Rc has no equivalent.** Indirection in rc requires `eval`:

```
target=hello
refname=target
eval echo '$'^$refname     # prints hello — but this is eval, which rescans
```

This violates Duff's principle. The `eval` forces the Segal condition, rescanning the concatenated string.

**VDC interpretation:** A name reference is a **vertical arrow** — an interface transformation that maps one variable name to another. It does not create a new horizontal arrow; it redirects access from one existing arrow to another. In the VDC, this is a restriction: the horizontal arrow `ref : X ⇸ Y` is the restriction of `target : X ⇸ Y` along a vertical arrow (the name mapping).

**How to integrate without eval:**

```
ref = *target              # hypothetical syntax: ref is a reference to target
echo $ref                  # follows the reference — prints the value of target
```

The critical property: following a reference is **not rescanning**. The reference is a typed indirection — a vertical arrow in the VDC — and resolving it is a structural operation (follow the arrow), not a textual operation (reparse the name as a string). The implementer can resolve references at substitution time without invoking the parser, because the reference carries typed metadata (a pointer to the target variable), not a string to be re-evaluated.

This eliminates the need for `eval` in the most common case of indirection. The `eval` command remains as the explicit "force the Segal condition" escape hatch, but idiomatic code should never need it for variable indirection.


---


## A.4 Features That Require Both Sides

Some features cannot be cleanly implemented with rc's data model alone or ksh93's execution model alone. They require the VDC framework's ability to coordinate the algebraic structure (from rc) with the operational discipline (from ksh93).

### A.4.1 Compound assignment with discipline interaction

**The problem (ksh93 bug 002 from the SPEC):**

```
function setup {
    typeset -C config=(
        host=localhost
        port=5432
    )
}
trap 'echo debug' DEBUG
setup
```

The compound assignment (`typeset -C config=(...)`) operates in value mode (positive — constructing a compound value). The DEBUG trap fires in computation mode (negative — executing a handler). When the trap fires mid-assignment, the value-mode context (`sh.prefix`) is exposed to the computation-mode handler, causing corruption.

This is the critical pair from the SPEC: a μ̃-binding (the compound assignment) cut against a μ-binding (the trap handler), with two possible reduction orders.

**How the VDC framework resolves it:**

In the VDC, the compound assignment is a cell with:

- **Top boundary:** A sequence of horizontal arrows carrying the field values (`host=localhost`, `port=5432`). These are positive (data, eagerly evaluated).
- **Bottom boundary:** A single horizontal arrow carrying the completed compound value.
- **The cell itself:** The assignment operation — a cut that binds the compound value to the variable name.

The trap handler is a **separate cell** that runs at polarity boundaries. It should not interleave with the assignment cell's internal operations. The VDC framework makes this explicit: the assignment cell is atomic — its internal structure (the sequence of field assignments) is the top boundary of a single cell, not a sequence of separate cells that can be interleaved with trap handlers.

**Implementation principle:** Compound assignments are single cells. Trap handlers fire between cells, not during them. The polarity frame from the SPEC enforces this: the assignment cell runs inside a polarity frame that suppresses trap delivery until the frame exits. This is the focused evaluation strategy from the Grokking paper — subexpressions are focused (evaluated to values) before the enclosing cut fires.

In rc's syntax:

```
config = (host localhost port 5432)    # atomic cell — no trap interposition
```

The assignment is a single cell. The list on the right is evaluated as a sequence of horizontal arrows (field name-value pairs), and the assignment binds the sequence to the variable. Traps fire after the assignment completes, not during evaluation of the right-hand side.

### A.4.2 Process substitution with argument lists

**Rc's process substitution:**

```
cmp <{old} <{new}
```

This runs `old` and `new` as subprocesses, connects their stdouts to file descriptors, and passes the fd paths as arguments to `cmp`. The process substitution is a shift from computation (negative — running a subprocess) to value (positive — a filename string that `cmp` can open).

**Ksh93's process substitution is essentially the same:**

```
cmp <(old) <(new)
```

Both shells handle this correctly. The VDC interpretation is: each `<{cmd}` / `<(cmd)` is a cell whose bottom boundary is a positive horizontal arrow (a filename), and the enclosing command (`cmp`) receives a sequence of these positive arrows as its top boundary.

**Where it gets interesting — feeding list-valued results:**

Rc's command substitution produces a list:

```
files='{ls}     # files is a list of filenames
```

Ksh93's command substitution produces a string (split by IFS into words when unquoted):

```
files=$(ls)     # files is a string; must be "${files[@]}" for safe use
```

In the unified shell, process substitution should integrate with list-valued variables:

```
diff <{sort $files(1)} <{sort $files(2)}
```

Here `$files(1)` is a single element of a list (a positive value), passed as an argument to `sort` inside a process substitution. The process substitution runs `sort` (a cell) on the single filename (a horizontal arrow from the list), and the resulting fd path is a positive horizontal arrow in `cmp`'s top boundary.

The VDC ensures this is well-typed: `$files(1)` is a subscript operation on a list-valued variable, producing a single horizontal arrow. The process substitution `<{sort ...}` wraps a cell around it, producing a new horizontal arrow (the fd path). No rescanning occurs at any stage.

### A.4.3 Pipelines with typed channels

**Rc:**

```
who | wc
```

The pipe carries an untyped byte stream. `who` produces text; `wc` consumes text. The compatibility is checked by nothing.

**In the unified shell with session types:**

```
who |: Stream(Line) :| wc -l
```

The `|: Type :|` annotation (hypothetical syntax) declares the type of the channel. The type `Stream(Line)` says: the pipe carries a stream of newline-delimited text lines. This is a positive type (data — a sequence of values).

The type annotation serves two purposes:

1. **Static checking:** The shell can verify that `who`'s declared output type is compatible with the pipe type, and that `wc`'s declared input type is compatible. If there's a mismatch, the shell reports a type error before execution.

2. **Protocol documentation:** The annotation makes the pipe's protocol visible in the source code, serving as documentation for the implementer and the user.

**VDC interpretation:** The pipe is a horizontal arrow with type `Stream(Line)`. The cells `who` and `wc` declare their interface types — `who` promises to produce `Stream(Line)` on stdout, and `wc` expects to consume `Stream(Line)` on stdin. The cut rule checks the types:

    Γ ⊢ who : Stream(Line)       Γ, x : Stream(Line) ⊢ wc(x)
    ─────────────────────────────────────────────────────────
                     Γ ⊢ ⟨who | wc⟩

**The Segal condition and pipeline fusion:**

When the types match, the Segal condition may hold: the two-cell pipeline may be fusible into a single cell. `cat file | grep pattern` is fusible to `grep pattern file` because the intermediate channel type (`Stream(Line)`) is the same on both sides and `grep` can accept a filename argument directly. The VDC framework makes fusion an optimization — applicable when the opcartesian cell exists — rather than a requirement.

### A.4.4 Error handling: unifying ⊕ and ⅋

The SPEC identifies the ⊕/⅋ duality in ksh93's error handling:

- **⊕ (exit status):** The command returns a status code; the caller inspects it.
- **⅋ (trap):** The command invokes a trap handler; the caller doesn't need to check.

Rc uses the ⊕ convention exclusively:

```
vc junk.c
if(~ $status 0) echo success
if not echo failure
```

Ksh93 uses both:

```
# ⊕ convention
if command; then echo success; else echo failure; fi

# ⅋ convention
trap 'echo error' ERR
command    # ERR trap fires automatically on failure
```

**VDC interpretation:**

⊕ (exit status) is a **positive type** — a data type with constructors (success, failure + code). The caller must pattern-match on the result:

```
# ⊕ in rc
if(~ $status 0) echo ok
if not echo 'failed with' $status
```

This is a case split on the data type — a cell that branches on the constructor of the exit status.

⅋ (trap) is a **negative type** — a codata type with destructors (the trap handlers). The command chooses which continuation to invoke:

```
# ⅋ — the command drives control flow
fn sigexit { cleanup }
fn ERR { echo 'command failed' >[1=2] }
```

The trap handler is a copattern: the system responds to the `ERR` destructor by running the handler.

**In the unified shell, both conventions are available and their relationship is explicit:**

```
# ⊕ — explicit status checking
result = '{might-fail}
switch($status){
case 0
    process $result
case *
    echo 'failed:' $status >[1=2]
}

# ⅋ — trap-based error handling
fn ERR { echo 'failed' >[1=2]; exit 1 }
might-fail
process $result

# Bridge: set -e converts ⊕ to ⅋
flag e +     # errexit — convert exit-status failures to ERR trap invocations
```

The SPEC's observation that `set -e` is a ⊕→⅋ converter is given a type-theoretic reading: `set -e` installs a default case-split on every command's exit status that invokes the ERR continuation on nonzero status. It is a global transformation from data-driven (positive) error handling to continuation-driven (negative) error handling.


---


## A.5 The Duploid Composition Laws in Practice

The SPEC identifies three composition patterns in ksh93's `sh_exec()`, corresponding to three duploid equations. These patterns carry over to the unified shell and guide the implementer in deciding how features compose.

### A.5.1 Pipeline composition (•, Kleisli/monadic)

**The pattern:** Two cells composed through a positive intermediary (a pipe carrying data).

```
ls | sort | uniq -c
```

Each `|` is a positive-intermediary composition: the left cell produces data, the right cell consumes it. Associativity holds: `(ls | sort) | uniq -c` and `ls | (sort | uniq -c)` produce the same result, because the intermediary is a value (data on a pipe) and value composition is associative.

**Implementation:** Pipeline composition creates actual pipes (`pipe(2)`), forks subprocesses, and connects file descriptors. The monadic state is the pipe fd. The Kleisli bind is: create pipe, fork, redirect, execute.

### A.5.2 Sequential composition (○, co-Kleisli/comonadic)

**The pattern:** Two cells composed through a negative intermediary (the execution context).

```
cd /sys/man || { echo 'No manual!' >[1=2]; exit 1 }
```

The `||` is a negative-intermediary composition: the left cell runs in the current execution context, produces an exit status, and the right cell runs in the same context only if the status is nonzero. The intermediary is the execution context (negative, comonadic), not a data value.

Associativity holds: nested `||` and `&&` compose correctly because they all operate within the same execution context. The comonadic extract (observe exit status) and extend (set up conditional) compose associatively.

### A.5.3 Cut (⟨t|e⟩, fundamental interaction)

**The pattern:** A producer meets a consumer directly — no intermediary.

```
for(i in $list) {
    process $i
}
```

The `for` loop is a cut: the list `$list` (producer) is cut against the loop body (consumer). Each iteration binds one element of the list to `$i` and runs the body. There is no intermediary — the producer's elements are consumed directly.

### A.5.4 The non-associativity failure

**The pattern that breaks:** Mixing (•) and (○) — a positive intermediary inside a negative context.

The SPEC's bug 002 is the concrete example:

```
f: parameter expansion         (positive — produces value)
g: compound assignment body    (positive → negative — enters computation)
h: DEBUG trap dispatch         (negative — computation intrusion)

(h ○ g) • f  ≠  h ○ (g • f)
```

The left bracketing contains the positive state within the computation frame. The right bracketing exposes it.

**For the implementer:** Whenever a new feature mixes value-mode and computation-mode operations, check the associativity law. If the feature involves:

1. A value-level operation (expansion, assignment, projection) — this is (•), monadic.
2. A computation-level operation (command execution, trap dispatch, subprocess) — this is (○), comonadic.
3. Both — this is a polarity boundary. Use a polarity frame. The frame enforces the left bracketing `(h ○ g) • f`, which contains the positive state.

**Concrete decision procedure for the implementer:**

- Is the new feature **purely value-level**? (e.g., new expansion syntax, new string operation, new type annotation) → Monadic. Thread through expansion context. No polarity frame needed.

- Is the new feature **purely computation-level**? (e.g., new control flow construct, new job control feature) → Comonadic. Save/restore execution context. Use existing polarity frame discipline.

- Does the new feature **cross the boundary**? (e.g., discipline functions, command substitution in assignment, process substitution) → Polarity shift. Push a polarity frame at the boundary. The shift is explicit in the code (a frame enter/leave pair) and in the type theory (a shift connective ↓ or ↑).


---


## A.6 Engineering Principles

### A.6.1 Duff's Principle Generalized

The original principle: **input is never scanned more than once.**

Generalized for the unified shell: **structure is never destroyed and reconstructed.** This includes:

- List boundaries (Duff's original case)
- Type information (a typed value is never flattened to a string and re-parsed)
- Polarity (a value-mode context is never silently entered from computation mode)
- Session protocols (a channel's protocol state is never lost and re-inferred)

### A.6.2 The Horizontal Arrow Discipline

Every channel (pipe, fd, variable, argument) is a horizontal arrow with a type. The type is assigned at creation time and does not change. Operations on the channel must be compatible with the type. The type is carried in the data structure, not recovered from the content.

In C implementation terms: every `Namval_t` (variable node) carries a type descriptor alongside its value. Every pipe fd carries a session type descriptor. The descriptors are set at creation and checked at use.

### A.6.3 The Polarity Frame Discipline

Every boundary crossing between value mode and computation mode goes through a polarity frame. The frame saves positive-mode state, clears it, runs the computation, and restores the state. No exceptions. No "this case is simple enough to skip the frame."

The SPEC's polarity frame API (`sh_polarity_enter`/`sh_polarity_leave`) is the C implementation of the shift connective. The VDC framework says: this is the mechanism that preserves horizontal arrow types across mode boundaries. Without the frame, a computation-mode operation can corrupt value-mode state, which means a horizontal arrow's type can be silently changed — violating the horizontal arrow discipline.

### A.6.4 The Segal Condition as Optimization Guide

Pipeline fusion (replacing a multi-cell pipeline with a single cell) is an optimization, not a requirement. The VDC framework makes this precise: fusion is possible when the Segal condition holds (an opcartesian cell exists for the sequence).

The implementer should:

- **Not** assume fusion is always possible.
- **Check** whether a proposed fusion preserves the types of the intermediate channels.
- **Document** which pipeline patterns are fusible and which are not.

Example: `cat file | grep pattern` is fusible because `grep` can accept a file argument directly. The fusion replaces two cells with one cell, and the types match. `sort | uniq` is not fusible because `sort` buffers its entire input before producing output, and the buffering boundary is part of the computation's semantics.

### A.6.5 Named Cells over Eval

Whenever a construct would traditionally require `eval` (indirection, computed variable names, dynamic command construction), the implementer should ask: can this be expressed as a named cell, a name reference, or a discipline function instead?

- **Variable indirection?** → Name reference (a vertical arrow), not `eval`.
- **Dynamic command dispatch?** → A function table (a collection of named cells), not `eval`.
- **Computed field access?** → Accessor notation with a variable key, not `eval`.

`eval` is the "force the Segal condition" escape hatch. It should be available but never necessary for idiomatic code. Every use of `eval` in existing ksh93/rc scripts is a design smell: it indicates a case where the shell's type system should have provided a structural solution but didn't.


---


## A.7 Summary: What Comes From Where

| Feature | Source | VDC Role | Sequent Calculus Role |
|---------|--------|----------|----------------------|
| List-valued variables | Rc | Sequence of horizontal arrows | Multi-formula context |
| No rescanning / structural substitution | Rc | Operadic cell composition | Cut on multi-source |
| Single-pass parsing | Rc + ksh93 | — | — |
| Typed variables (int, float, array) | Ksh93 | Typed horizontal arrows | Positive formulas |
| Compound variables / records | Ksh93 | Product types (⊗) | Tensor introduction/elimination |
| Discipline functions | Ksh93 | Negative (codata) types | Copattern matching |
| Accessor / dot notation | Ksh93 | Destructor chaining | Direct consumers |
| Coprocesses | Ksh93 | Bidirectional horizontal arrows | Session types |
| Name references | Ksh93 | Vertical arrows (restrictions) | Variable indirection |
| ⊕ error handling (exit status) | Both | Positive type (data) | Sum type / case split |
| ⅋ error handling (traps) | Both | Negative type (codata) | Cosum type / copattern |
| Polarity frames / shift discipline | SPEC (ksh93 internal) | Preservation of arrow types across mode boundaries | Shift connectives (↓, ↑) |
| Duploid composition laws | SPEC (ksh93 internal) | Cell composition rules | Cut reduction strategies |
| Pipeline fusion | VDC framework | Segal condition / opcartesian cells | Cut elimination |
| The eval escape hatch | Both | Forced Segal condition | Re-parsing |
| Signal handlers as functions | Rc | Pre-parsed cells | Definitions, not deferred strings |


---


## A.8 Concluding Remark

The two shells represent two successful but partial discoveries of the same underlying structure. Rc found the algebraic side: list-valued variables, structural substitution, the no-rescan invariant. Ksh93 found the operational side: polarity-sensitive execution, the three-sorted AST, the save/restore-as-shift pattern, the ⊕/⅋ error duality, discipline functions as codata.

The virtual double category is the framework that holds both discoveries simultaneously. It says: shell programs are pasting diagrams of cells, where cells are commands, horizontal arrows are typed channels, vertical arrows are interface transformations, and the Segal condition tells you when composition is available. The sequent calculus is the type theory for the horizontal arrows, with polarity governing evaluation order. The duploid semantics gives the composition laws, with non-associativity marking the boundary between value mode and computation mode.

A shell designed from this framework inherits rc's structural cleanliness and ksh93's operational richness. The key engineering constraint is Duff's generalized principle: **structure is never destroyed and reconstructed.** Every feature — discipline functions, accessor notation, coprocesses, typed variables — is designed so that the type information, the list boundaries, the polarity markers, and the session protocols are created once and carried thereafter. The framework does not prevent the shell from being a shell. It prevents it from being a macro processor.
