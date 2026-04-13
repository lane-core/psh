# Foundations

## What this document is

The specification of psh's type theory, execution model, and design
rationale. psh descends from rc (Duff 1990), not from the Bourne
shell. The analysis begins there.

This document is the output of a systematic design process:
interrogation of rc's design philosophy, ksh93's implicit type
theory (discovered via the sfio-analysis and ksh93-analysis.md
sequent calculus mapping), the duploid semantics of Mangel/Melliès/
Munch-Maccagnoni, Curien-Herbelin's λμμ̃-calculus, and the
profunctor optics framework of Clarke et al. Every design
decision references its lineage.


## Design position

psh is an excellent standalone shell first. It must be usable as
a login shell on Linux, macOS, FreeBSD, and other Unix-likes
without any external infrastructure. The theoretical foundations
— sequent calculus structure, duploid polarity, profunctor
redirections, typed values — serve the standalone shell. They
make pipelines compose correctly, catch errors at binding time,
and give the interactive experience richer context for completion
and highlighting. The theory earns its keep by making psh a
better shell, not by enabling a specific platform.


## Foundational commitment: every variable is a list

Every psh variable holds a list. There is no separate "scalar"
type distinct from "list of length 1." This is the uniform
abstraction inherited from rc and reinforced by the virtual
double category framework (see `docs/vdc-framework.md`), in
which sequences are the primitive structure on cell boundaries.

Concrete consequences:

- `let count : Int = 0` is shorthand for `let count : Int = (0)`.
  Both denote a list of one int. `$#count` is 1.
- Type annotations refer to **element types**. `: Int` means
  "list whose elements are Int." Length is runtime data, not
  part of the type.
- Substitution always splices a list. A "scalar" binding splices
  one element; a list binding splices its elements. rc's
  structural substitution discipline, unchanged.
- Tuples, sums, and structs are distinct types at the **element**
  level — they can appear inside the list. `let pos : Tuple = (10,
  20)` holds a list of one tuple. `$#pos` is 1. `$pos[0]` is
  `10`.
- No scalar/list distinction means no quoting ceremony is ever
  needed for correctness. Variables always splice structurally.
  Double quotes are for string interpolation (`"hello $name"`),
  not for protecting against word splitting.

This is Duff's principle extended across the type system: the
list structure is carried as data, never destroyed and
reconstructed.


## rc's execution model as sequent calculus

Duff's rc [Duf90] departed from the Bourne shell for structural
reasons, not aesthetic ones. The critical moves:

**List-valued variables.** Bourne conflated "list of strings"
with "string containing separators" — every expansion re-scanned
through IFS. rc made lists first-class: `path=(. /bin)` is two
strings, never rescanned. Duff: "Input is never scanned more
than once" [Duf90, §Design Principles]. This was the foundational
move. Everything else follows from treating the shell's data
type honestly.

**Syntax matching the semantics.** Bourne's syntax was
accidental — decades of accretion on the Mashey shell. rc
started fresh with consistent rules: `{` for grouping, `'` for
quoting (not three incompatible mechanisms), `()` for lists. The
syntax made the semantics visible.

**Plan 9 informed rc through the namespace.** `/env` was a
per-process-group directory where variables lived as files.
`fn name` stored the function body in `/env/fn#name`. This
meant `rfork e` gave you a new environment by kernel semantics,
not shell magic. The shell was a client of the namespace, not
its own micro-OS.

rc has the three-sorted structure of the λμμ̃-calculus [CH00],
unnamed and unenforced:

| rc construct | Sort | Evidence |
|---|---|---|
| Words: literals, `$x`, `` `{cmd} ``, `a^b` | Producers | Eager evaluation. "Input is never scanned more than once" [Duf90, §Design]. |
| Pipe readers, redirect targets, continuations | Consumers | Implicit — waiting to receive a value. |
| Simple commands, `if`, `for`, `match` | Consumers (coterms) | `echo`: consumes args, writes stdout. `if`: consumes status. |
| Pipelines, redirections, fork/exec | Cuts ⟨t \| e⟩ | `echo hello`: producer `hello` meets consumer `echo`. |

The shifts exist in rc but are unnamed:

| rc mechanism | Shift type | Direction |
|---|---|---|
| `` `{cmd} `` command substitution | Force then return (↓→↑), oblique map | computation → value |
| `<{cmd}` process substitution | Downshift ↓ (thunk into namespace) | computation → name |
| `x=val; rest` | μ̃-binding (let) | bind value, continue |

psh adds one statement-to-producer move that rc did not have:

| psh mechanism | Logical shape | Effect |
|---|---|---|
| `$((...))` arithmetic | `μα.⊙(e₁,e₂;α)` — μ-binding around a binop statement | In-process, no subprocess, pure central map in `P_t` |

`$((...))` is **distinct from command substitution**, not a
copy. Command substitution is a genuine oblique map in the
duploid — it packages the body as a thunk, forces it by
forking, runs a full shell statement whose effects include
subprocess creation and I/O, and captures the byte-valued
return; the inner computation straddles CBV/CBN because the
forked pipeline is itself co-Kleisli. `$((...))` has neither
polarity straddle nor subprocess. Per [BTMO23, §2.1] "Arithmetic
Expressions," arithmetic binop in λμμ̃ is a **statement**
shaped `⊙(p₁, p₂; c)` taking two producers and a consumer;
the surface form `e₁ + e₂` translates as `μα.⊙(⟦e₁⟧, ⟦e₂⟧; α)`
— a μ-binding wrapping a statement to produce a positive. Any
"shift" here is type-theoretic only: the shell does fire a
polarity frame around `$((...))` to match the uniform mechanism
described in §Polarity frames, but since the inner computation
is effect-free the frame's save and restore steps simplify to
no-ops. Operationally trivial; categorically a pure central
map. ksh93/POSIX heritage for the syntax; the categorical
reading is psh's own.

psh makes two shifts explicit that rc left implicit:

1. **Command substitution without IFS.** psh splits on newlines,
   not on an arbitrary `$ifs`. The return operation (bytes → list)
   is fixed. Duff kept `$ifs` only because "indispensable" [1,
   §Design Principles]; psh removes it, closing the last re-scanning hole.

2. **Process substitution as downshift into namespace.** rc's
   `<{cmd}` returned an fd path while the child ran concurrently.
   Categorically this is a **downshift `↓`**: the negative CBN
   pipeline is thunked behind a name (a `/dev/fd/N` string) so
   it can be passed to a CBV caller. The name is positive (CBV —
   a string); the computation behind the name is negative (CBN,
   demand-driven, reads through the fd trigger it). The downshift
   itself is synchronous (the bind is immediate), but the
   computation is only scheduled; it runs when the fd is opened.
   This is not a `↓→↑` shift — there is no upshift back, because
   the caller receives the name, not the computation's eventual
   value. This matches Plan 9's mount model: `mount` returns
   immediately with a name, the server behind the mount point is
   concurrent. Nobody considers `mount` a violation of sequential
   execution. The concurrency is behind the name, accessed only
   when something reads the fd.


## The sfio insight

ksh93's sfio library `refs/ksh93/sfio-analysis/` was the shell's implicit type
theory. The sfio-analysis suite [SFIO-1 through SFIO-12]
revealed:

**Buffer polarity.** sfio's five-pointer buffer system `refs/ksh93/sfio-analysis/03-buffer-model.md`
encodes polarity: `_endr` active = read mode (negative,
consuming), `_endw` active = write mode (positive, producing).
Mode switching (`_sfmode()`) is a polarity shift with
reconciliation cost — seek-back for read→write, flush for
write→read. This is a shift operator with a cost, not a free
operation.

**Discipline stacks as morphism chains.** Each `Sfdisc_t` in
the stack `refs/ksh93/sfio-analysis/07-disciplines.md` composes like an endomorphism between the
buffer (value) and the kernel (computation). The stack as a
whole mediates the value/computation boundary.

**Dccache as non-associativity witness.** When a discipline
is pushed onto a stream with buffered data `refs/ksh93/sfio-analysis/07-disciplines.md`, the two
possible bracketings yield different results because data
already in value mode (buffered) cannot be re-processed through
a new computation discipline. This is structurally analogous to
the duploid's failed fourth equation (Mangel/Melliès/
Munch-Maccagnoni [MMM], the non-associative composition of
call-by-value and call-by-name). The pattern matches; the full
duploid composition laws have not been formally verified for
sfio's discipline stack.

**The lesson for psh:** ksh93's authors built correct polarity
discipline in sfio and then failed to propagate it to the shell
proper. The `sh.prefix` bugs (SPEC.md `refs/ksh93/ksh93-analysis.md` bugs 001–003b)
are exactly the same non-associativity that Dccache handles
correctly — a computation (DEBUG trap) intruding into a value
context (compound assignment) with no mediator. sfio had the
mediator; the shell didn't.

psh makes polarity explicit:

- **Typed fd roles** (`Pipe`, `File`, `Tty`, `Coproc`,
  `Session`) — not sfio's universal `Sfio_t` with runtime mode
  bits. Explicit types over runtime flags.
- **Wrapped redirections** that make evaluation order structural
  — the AST nesting determines the only legal evaluation order.
  No Dccache problem possible because the profunctor composition
  prevents non-associative bracketing by construction.
- **Save/restore as lens roundtrips** — PutGet (restore after
  redirect gives saved state), GetPut (save without redirect is
  no-op), PutPut (consecutive redirects, only last matters).
  This is ksh93's `filemap[]` / `sh.topfd` pattern [SPEC,
  sfio-analysis/10-ksh-integration.md] translated to typed Rust.


