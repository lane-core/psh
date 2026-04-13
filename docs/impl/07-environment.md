# Environment — Γ/Θ/Δ realized

The environment is the runtime realization of the typing
contexts. Γ (value bindings) is a scope chain of variable
slots. Θ (computation bindings) is a registry of `def` bodies
and closures. Δ (continuations) is implicit in the evaluator's
call stack — trap handlers and catch blocks are Δ entries
managed by `run_cmd`.

Spec correspondence: 11-namespace.md defines the three tiers.
03-polarity.md §Linear resources defines the zone tracking.
14-invocation.md §Environment inheritance defines startup
population.

### Scope chain (Γ)

Variables form a scope chain — inner scopes shadow outer ones.
Each scope is a frame holding variable slots.

```rust
/// A single scope frame.
struct Frame {
    slots: HashMap<Name, Slot>,
}

/// The scope chain. Index 0 is the innermost (current) scope.
struct ScopeChain {
    frames: Vec<Frame>,
}

impl ScopeChain {
    fn push_scope(&mut self) {
        self.frames.push(Frame::new());
    }

    fn pop_scope(&mut self) -> Frame {
        self.frames.pop().expect("scope underflow")
    }

    /// Lookup: walk from innermost to outermost.
    fn get(&self, name: Name) -> Option<&Slot> {
        for frame in self.frames.iter().rev() {
            if let Some(slot) = frame.slots.get(&name) {
                return Some(slot);
            }
        }
        None
    }

    /// Set: write to the nearest frame that contains the name,
    /// or the current (innermost) frame if unbound.
    fn set(&mut self, name: Name, val: Val) {
        for frame in self.frames.iter_mut().rev() {
            if frame.slots.contains_key(&name) {
                frame.slots.get_mut(&name).unwrap()
                    .elements = vec![val];
                return;
            }
        }
        // New binding in current scope
        self.frames.last_mut().unwrap()
            .slots.insert(name, Slot::new(val));
    }

    /// Bind a new name in the current (innermost) scope.
    /// Used by `let`, `for`, pattern match bindings.
    fn bind(&mut self, name: Name, val: Val) {
        self.frames.last_mut().unwrap()
            .slots.insert(name, Slot::new(val));
    }
}
```

Frame push/pop is the scope mechanism for `for` bodies,
`match` arms, function calls, and subshells. SmallVec-backed
frames would optimize the common case (most scopes have ≤ 8
bindings), but HashMap is the correct starting point — profile
before optimizing.

### Def registry (Θ)

Named computations (`def` bodies) and closures (`let` lambdas
with captured environments).

```rust
/// A registered def — either a parsed body or a closure.
enum DefEntry {
    /// def name(params) { body } — parsed, not yet closed over
    Parsed {
        params: Vec<Name>,
        body: Program,
        return_type: Option<TypeRef>,
    },
    /// let f = |params| => body — closure with captured env
    Closure {
        params: Vec<Name>,
        body: LambdaBody,
        captured: Frame,                    // snapshot of Γ at definition site
        return_type: Option<TypeRef>,
    },
}

/// The def/closure registry.
struct DefRegistry {
    defs: HashMap<Name, DefEntry>,
    /// Per-variable discipline functions: (variable, discipline) → body
    disciplines: HashMap<(Name, Name), DefEntry>,
    /// Per-type methods: (TypeId, Name) → body
    /// (Mirrors sigma.methods, but holds the actual body for runtime dispatch)
    type_methods: HashMap<(TypeId, Name), DefEntry>,
}
```

`def` registration happens during `run_cmd` when the evaluator
encounters a `Command::Def` node. The body is stored without
evaluation — it's a computation, not a value. Invocation
happens later when the def is called.

Closures capture a snapshot of the current scope frame at the
point where the lambda is evaluated. The captured `Frame` is
immutable after capture — closures in psh do not close over
mutable state. (If the captured variable is reassigned after
capture, the closure sees the old value. This is standard
lexical scoping, not reference-capturing.)

### Discipline dispatch table

Variable disciplines (`def x.get`, `def x.set`, `def x.refresh`)
are stored in the `disciplines` map keyed by `(variable_name,
discipline_name)`. The evaluator checks this table on every
variable access and assignment:

```rust
impl DefRegistry {
    /// Look up a discipline function for a variable.
    fn get_discipline(&self, var: Name, disc: Name)
        -> Option<&DefEntry>
    {
        self.disciplines.get(&(var, disc))
    }
}
```

The reentrancy guard (08-discipline.md) is a per-variable flag
on `Shell`, not on the `DefEntry`. Inside a `.set` body, the
guard is raised, and `x = val` bypasses the discipline (direct
slot write).

### Export projection (Tier 1 → Tier 2)

On every `exec`, exported variables are materialized into the
child's environment. This is mark-for-projection, not snapshot
— the child sees the value at exec-time.

```rust
impl ScopeChain {
    /// Collect all exported variables for child env.
    fn export_env(&self) -> Vec<(SmolStr, SmolStr)> {
        let mut env = Vec::new();
        // Walk outermost to innermost; inner shadows outer
        let mut seen = HashSet::new();
        for frame in self.frames.iter().rev() {
            for (name, slot) in &frame.slots {
                if slot.exported && seen.insert(*name) {
                    // Stringify: multi-element lists join with spaces
                    let val_str = slot.elements.iter()
                        .map(|v| v.to_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    env.push((name.resolve(), val_str));
                }
            }
        }
        env
    }
}
```

### Per-command locals

`VAR=value cmd` scopes the assignment to a single command.
Implemented by pushing a temporary frame before the command
and popping it after:

```rust
impl Shell {
    fn run_with_locals(&mut self, locals: &[(Name, Val)], cmd: &Expr)
        -> Status
    {
        self.env.scopes.push_scope();
        for (name, val) in locals {
            self.env.scopes.bind(*name, val.clone());
        }
        let result = self.run_expr(cmd);
        self.env.scopes.pop_scope();
        result
    }
}
```

### Trap table (Δ, partially)

Signal handlers are Δ entries — continuations named by signal.
Lexical traps have a body scope; global traps persist until
overwritten or deleted.

```rust
enum TrapEntry {
    /// trap SIGNAL { handler } { body } — scoped, lexical
    Lexical { handler: Program },
    /// trap SIGNAL { handler } — global
    Global { handler: Program },
}

struct TrapTable {
    handlers: HashMap<Name, Vec<TrapEntry>>,
    // Stack: inner lexical traps shadow outer
}
```

The evaluator's `run_cmd` for `Command::Trap` pushes a
`TrapEntry` onto the handler stack for the named signal,
runs the body (if lexical), then pops on exit. Signal delivery
(via the self-pipe from `signal.rs`) walks the stack from top
to find the innermost active handler.

### The Environment struct

```rust
struct Environment {
    scopes: ScopeChain,
    defs: DefRegistry,
    positional: Vec<Val>,                   // $1, $2, ... $*
}
```

`positional` holds the positional parameters (`$*`, `$1`,
etc.). `set -- a b c` replaces them. They are not in the
scope chain — they're accessed by special variable syntax,
not by name lookup.

### Startup population

On startup (14-invocation.md §Environment inheritance):

1. **environ(7) scan.** Each `NAME=VALUE` pair creates a
   `Slot { elements: [Val::Str(value)], type_id: STR_ID,
   zone: Classical, exported: true }` in the outermost frame.

2. **Special variables.** `$pid`, `$status`, `$apid`,
   `$HOME`, `$PWD`, `$COLUMNS`, `$LINES`, etc. are
   initialized in the outermost frame.

3. **Prelude types.** Σ is populated with ground types, then
   prelude declarations (Unit, Either, Option, Result).

4. **Profile sourcing.** If login shell, source
   `$HOME/.config/psh/profile` via the `.` builtin. User
   definitions extend Γ, Θ, and potentially Σ (if the profile
   declares types).


# References

All citation keys resolve to `docs/spec/references.md`.

- `[Duf90]` — Duff, "Rc — The Plan 9 Shell." 1990.
- ksh26 Theoretical Foundation: `refs/ksh93/ksh93-analysis.md`
