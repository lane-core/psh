# Sequences, Not Strings: Shell Semantics via Sequent Calculus and Virtual Double Categories

> **Status:** Theoretical framework report. Develops the categorical
> semantics (virtual double categories, sequent calculus reading,
> composition laws) that `specification.md` draws on but does not
> reproduce. The spec is the source of truth for psh syntax and
> resolved design decisions; this document is the source of truth for
> the underlying categorical argument. Where psh departs from rc, the
> departure is noted inline. Where this document and the spec disagree,
> the spec wins.

## 0. Overview

This report develops a precise interpretation of shell semantics grounded in the sequent calculus, using virtual double categories as the unifying categorical framework. The starting point is Tom Duff's rc shell for Plan 9 and the design principle that governs it: **the shell is not a macro processor; input is never scanned more than once.** We argue that this principle is, at bottom, a type-theoretic statement about the preservation of structure under substitution. The sequent calculus — specifically, the λμμ̃-calculus of Curien and Herbelin — provides the logical framework in which this statement becomes precise. Virtual double categories, as developed by Cruttwell and Shulman (building on Leinster and Burroni), supply the categorical framework in which the compositional structure of shell programs can be analyzed without presupposing that composition always exists.

The intended audience is someone comfortable with the basics of sequent calculus (cuts, identity, structural rules) but less familiar with virtual double categories. We proceed by explicit analogy with rc code throughout, using actual rc idioms rather than invented syntax, and pointing out the conceptual structure already present in Duff's design.

The arc is: §1 states Duff's principle precisely. §2 develops rc's data model as the motivating mathematical structure. §3 introduces the sequent calculus reading of shell commands. §4 presents virtual double categories as the framework that makes this reading compositional. §5 works out the detailed mapping between rc constructs and the categorical/logical apparatus. §6 assembles the framework and draws implications for shell design. §7 tabulates the correspondences. §8 presents the composition laws (three duploid patterns and their non-associativity failure) with a decision procedure for new features. §9 gives engineering principles derived from the framework. §10 concludes.


---


## 1. The Founding Principle

### 1.1 Duff's Observation

The Bourne shell's fundamental data type is the character string. When a variable is substituted into a command, the resulting string is rescanned: split on IFS, subjected to globbing, quote-stripped. This makes the Bourne shell a macro processor. The value of a variable is not a datum but a program fragment that must be re-parsed at each use.

Duff's rc eliminates rescanning by changing the fundamental data type from "string" to "list of strings." A variable's value is a sequence of arguments — an ordered, length-aware, boundary-preserving container:

```
path=(. /bin)
user=td
```

The parentheses in the first assignment indicate that `path` holds a list of two strings. Substitution splices this list into the command:

```
echo $path
```

is equivalent to

```
echo . /bin
```

The two arguments remain two arguments. No rescanning occurs. No IFS splitting, no globbing, no re-parsing. The boundaries between `.` and `/bin` were established at assignment time and preserved at substitution time.

### 1.2 The Principle Stated Precisely

Duff puts it plainly in his paper:

> The most important principle in rc's design is that it's not a macro processor. Input is never scanned more than once.

We can restate this as a structural property:

**Substitution is structure-preserving.** The operation of replacing a variable with its value does not change the number, boundaries, or identity of arguments. If a variable holds n arguments, substituting it into a command produces exactly n arguments in the corresponding positions. The argument list is an algebraic structure (a finite sequence), and substitution is a homomorphism of that structure.

This is exactly what fails in the Bourne shell. When `$*` is substituted and then rescanned, the resulting argument count depends on the *content* of the strings (whether they contain IFS characters), not on the *structure* of the value. The map from values to argument lists is not a homomorphism — it depends on a global parameter (IFS) and on the character-level content of the data.

### 1.3 What This Rules Out

The no-rescan invariant has direct consequences:

1. **The `$@` vs `$*` distinction is unnecessary.** In the Bourne shell, `$*` joins arguments into a single string, losing boundaries, while `$@` (inside double quotes) attempts to preserve them. This distinction exists only because the Bourne shell lost structure by flattening to a string and needs two different recovery strategies. In rc, `$*` is always a list; there is nothing to recover.

2. **Four kinds of quoting collapse to one.** Bourne's single quotes, double quotes, backticks, and backslash each address a different aspect of the rescanning problem. Rc has apostrophes and nothing else: you quote when you want a syntax character treated literally, and at no other time. (psh has two string forms — single quotes are literal, double quotes interpolate — but the structural point holds: quoting is syntax, not a rescanning workaround.)

3. **The IFS security hole is eliminated.** Because the Bourne shell's `system()` and `popen()` functions invoke `/bin/sh`, which rescans its input splitting on IFS, an attacker can set `IFS=/` and leave Trojan binaries in the current directory. Rc never rescans, so IFS manipulation cannot alter the parse of a command.

4. **Signal handlers are functions, not strings.** Bourne stores a signal handler as a string to be re-parsed when the signal fires, meaning you can get a syntax error at interrupt time. Rc parses the handler at definition time (it is a function), so errors appear when you write them.

Each of these is an instance of the same principle: structure that is present at parse time should not be destroyed and then reconstructed. The list-of-strings data type is the mechanism; the no-rescan invariant is the law.


---


## 2. Rc's Data Model as Algebraic Structure

### 2.1 Lists

A value in rc is a finite sequence of strings. We write these sequences in rc's syntax with parentheses:

```
list=(How now brown cow)
```

This assigns a list of four strings to `list`. The critical distinction is between the empty list and the list containing one empty string:

```
empty=()
null=''
```

These are different values. `$#empty` is `0`; `$#null` is `1`. The empty list is the unit for list concatenation (juxtaposition). The list containing one empty string is a non-trivial one-element sequence. Duff acknowledges this confuses novices but calls the distinction "arguably sensible," and it is: a null argument is not the same as no argument.

The set of all rc values forms a free monoid on the set of strings, where the monoid operation is list concatenation (juxtaposition in rc's syntax):

```
a=(x y)
b=(1 2 3)
all=($a $b)     # all=(x y 1 2 3)
```

The identity element is the empty list `()`.

### 2.2 Concatenation

Rc has a second operation: the caret `^`, which is string-level (not list-level) concatenation. It distributes over lists:

```
echo (a b c)^(1 2 3)    # prints a1 b2 c3
src=(main subr io)
cc $src^.c               # equivalent to: cc main.c subr.c io.c
```

The distribution rule is: if both operands are lists of the same nonzero length, they are concatenated pairwise. If one operand is a single string, it is concatenated with each element of the other. Any other combination is an error.

This gives rc's values the structure of a module over strings (under `^`) that is simultaneously a free monoid (under juxtaposition). The two operations interact but do not collapse: `^` is pointwise, juxtaposition is sequential.

### 2.3 Free Carets

Rc inserts implicit carets where Bourne would have relied on the no-whitespace-means-concatenation convention:

```
cc -$flags $stems.c
```

is equivalent to

```
cc -^$flags $stems^.c
```

This is a notational convenience, but it reveals something structural: the adjacency of tokens in rc source code is itself an operation (concatenation), distinct from whitespace-separation (list formation). Rc distinguishes two modes of combining things — "next to" (`^`, pointwise on elements) versus "alongside" (space, sequential on lists) — and the syntax reflects this.

> **psh departure:** psh reserves `.` for dot accessors (field/method
> access). `$stem.c` in psh is a dot accessor on `$stem`, not
> concatenation. The free-caret-before-dot rule does not apply. Use
> explicit `^` for file extension concatenation: `$stem^.c`. See
> `docs/syntax.md` §Free carets.

### 2.4 Command Substitution

Command substitution in rc is a unary operator:

```
files='{ls}
```

This runs `ls`, captures its stdout, splits the output on `$ifs` to produce a list, and assigns the resulting list to `files`. The split happens exactly once: the result is a list, and that list is never re-split.

In Bourne, command substitution uses backticks and the result is rescanned, meaning nested substitutions require exponential escaping:

```
size=`wc -l \`ls -t|sed 1q\``
```

Rc's syntax makes the backquote a unary prefix operator taking a braced command:

```
size='{wc -l '{ls -t|sed 1q}}
```

No escaping. The nesting is syntactic (matched braces), not textual (escaped backticks), because the parser handles it in a single pass.

### 2.5 The Data Model Summarized

The rc value domain is:

- **Objects:** Finite sequences (lists) of strings.
- **List formation:** Juxtaposition / parenthesized grouping. This is the free monoid structure.
- **Pointwise concatenation:** The caret operator `^`. This distributes over lists.
- **Substitution:** Structural — splices a list into a command without rescanning.
- **Command substitution:** Runs a command, splits output on `$ifs` once (rc) or on newlines (psh), produces a list. No further rescanning. (psh removes `$ifs` entirely — see `docs/specification.md` §rc-isms removed.)

The key invariant: the boundaries between elements of a list are part of the data, not recoverable from the content. Structure is carried, not reconstructed.


---


## 3. The Sequent Calculus Reading

We now interpret rc's computational model through the lens of the sequent calculus — specifically the λμμ̃-calculus (also called System L in Munch-Maccagnoni's treatment). The claim is that the structure already present in rc maps naturally onto the components of the sequent calculus, and that this mapping makes explicit what Duff's design leaves implicit.

### 3.1 Two Sorts: Values and Commands

The core of the λμμ̃-calculus has two syntactic sorts:

- **Terms** (also called producers in the call-by-value reading): things that compute to a value.
- **Commands** (also called statements): interactions between a producer and a consumer. A command has no "return type" — it is self-contained.

The typing judgments are:

    Γ ⊢ t : A          (term t has type A in context Γ)
    Γ ⊢ c              (command c is well-formed in context Γ)

The critical point is the second judgment: commands are typed by a context alone. They do not produce a result of any type. They just execute.

This maps directly onto rc's two modes of use:

- **Arguments** are values. A string, a variable holding a list, a command substitution that produces a list — these are the producers. They have a "type" in the sense that they carry data with a definite structure.
- **Commands** are interactions. When rc executes `echo $path`, the command `echo` interacts with its argument list. The execution itself does not "return" a value to the syntactic position where it appeared — it produces effects (output, exit status, file modifications) that exist outside the term language.

Consider a concrete example:

```
for(i in printf scanf putchar) look $i /usr/td/lib/dw.dat
```

Here `(printf scanf putchar)` is a value — a list of three strings. The body `look $i /usr/td/lib/dw.dat` is a command template that, when `$i` is instantiated, becomes a command. The `for` construct iterates the command over the elements of the list. The list is a producer; the command body is a consumer (it consumes one element of the list per iteration); the `for` is the cut that connects them.

### 3.2 Cut as Execution

In the sequent calculus, the cut rule connects a producer of type A with a consumer of type A:

    Γ ⊢ t : A       Γ, x : A ⊢ c
    ────────────────────────────────
           Γ ⊢ ⟨t | μ̃x.c⟩

The command ⟨t | μ̃x.c⟩ runs the producer t, binds its result to x, and continues with c. In the call-by-value reading, t is evaluated first; then x is bound; then c runs.

In rc, the simplest cut is variable substitution in a command:

```
file=/tmp/junk
wc $file
```

The first line is the producer: it creates the value `/tmp/junk`. The second line is the command in which the value is consumed. The substitution of `$file` into `wc $file` is the cut: it connects the producer (the value `/tmp/junk`) with the consumer (the command template `wc _`).

A more explicit example is piping:

```
who | wc
```

This is a cut connecting the producer `who` (which generates output on stdout) with the consumer `wc` (which reads from stdin). The pipe operator `|` is syntactic sugar for the cut. The left side produces; the right side consumes; execution proceeds by reducing the cut (i.e., passing data through the pipe).

### 3.3 μ and μ̃: Capturing Context and Capturing Value

The λμμ̃-calculus has two binding forms:

- **μα.s** ("mu"): captures the current *continuation* (evaluation context) and binds it to α. In rc, this corresponds to any construct that names or reifies what will happen to a command's output. The simplest case is redirection:

  ```
  who >user.names
  ```

  The redirection `>user.names` is a reification of the continuation: instead of sending `who`'s output to the default destination (the terminal), it names an alternative destination. The μ-binding captures "where the output goes" and redirects it.

- **μ̃x.s** ("mu-tilde"): captures the current *value* and binds it to x. In rc, this corresponds to variable binding:

  ```
  line='{awk '{print;exit}'}
  ```

  This captures the output of the `awk` command (a value — a list of strings produced by command substitution) and binds it to the variable `line`. The μ̃-binding captures "what was produced" and names it.

The duality between μ and μ̃ — between capturing the continuation and capturing the value — corresponds to the duality between redirection (naming where output goes) and variable binding (naming what was produced).

### 3.4 Critical Pairs and Evaluation Order

When a μ-binding meets a μ̃-binding in a cut, there is a *critical pair*:

    ⟨μα.s₁ | μ̃x.s₂⟩

This can reduce in two ways:

- **Call-by-value:** Evaluate the producer first, substitute its value for x in s₂. This gives s₂[μα.s₁/x].
- **Call-by-name:** Evaluate the consumer first, substitute it for α in s₁. This gives s₁[μ̃x.s₂/α].

In rc, the default evaluation order is call-by-value: arguments are fully expanded before the command sees them. The command

```
echo '{ls}
```

first evaluates `'{ls}` (the producer — command substitution), then passes the resulting list to `echo` (the consumer). This is the call-by-value reduction of the critical pair: produce the value, then consume it.

The sequent calculus makes this choice explicit rather than implicit. Both evaluation orders are expressible; the choice is a parameter of the system, not a hidden assumption.

### 3.5 Data and Codata

The sequent calculus reveals data types and codata types as perfectly dual:

- **Data types** are defined by their constructors (how to build a value). The consumer must pattern-match. Example: a list is either Nil or Cons(head, tail). The producer chooses which constructor to use; the consumer must handle both cases.

- **Codata types** are defined by their destructors (how to observe a value). The producer must handle all observations. Example: a stream has destructors `hd` (get the current element) and `tl` (advance to the next). The consumer chooses which destructor to invoke; the producer must respond to both.

In rc, this duality appears in the distinction between:

- **Arguments** (data): the command receives a list of strings. The list was constructed by the caller. The command must pattern-match on it (e.g., dispatching on argument count).

  ```
  # rc syntax (psh uses match/=> — see docs/syntax.md §match):
  switch($#*){
  case 1
      cat >>$1
  case 2
      cat >>$2 <$1
  case *
      echo 'Usage: append [from] to'
  }
  ```

  This is a case split on the data type — the argument list. The producer (the caller) chose how many arguments to provide. The consumer (the script) must handle each case.

- **File descriptors** (codata): a process exposes its output through numbered descriptors (fd 1, fd 2, etc.). The calling context chooses which descriptor to observe (redirect, pipe, close). The process must be prepared to write to any of them.

  ```
  vc junk.c >junk.out >[2=1]
  ```

  Here the caller (the shell) is choosing to observe fd 1 (redirect to `junk.out`) and fd 2 (duplicate to fd 1). These are destructor invocations on the codata type that is the process's output interface.

The sequent calculus makes this duality first-class: constructors and destructors, pattern matches and copattern matches, are symmetric. Rc does not formalize this symmetry, but it is already present in the design.


---


## 4. Virtual Double Categories

We now introduce the categorical framework that unifies the preceding observations. The goal is to provide a setting in which:

1. Sequences of arguments (rc lists) are represented as sequences of morphisms, with boundaries preserved.
2. Commands are represented as cells mediating between input and output sequences.
3. Composition (piping, command substitution) is available but not required — it is a *property* of the structure, not an axiom.
4. The type theory (sequent calculus) and the compositional structure (pasting of cells) are aspects of the same framework.

### 4.1 Motivation: Why Not Just Categories?

A category has objects, morphisms, and composition. Composition is total and associative: given f : A → B and g : B → C, the composite g ∘ f : A → C always exists.

This is too strong for shell semantics. Not every pair of commands can be meaningfully composed. Piping `who | wc` works because `who` produces text on stdout and `wc` reads text from stdin — their interfaces are compatible. But `who | gcc` does not compose meaningfully, even though it is syntactically legal. The composition exists as a Unix process, but it does not produce useful results; the interfaces are mismatched.

More fundamentally, the interesting structure of a pipeline is not the composite but the *sequence*. The pipeline `a | b | c` has three stages. Knowing only the composite (the overall input-output behavior) loses information about the intermediate structure. Optimizations like pipeline fusion, error localization, and resource management all depend on seeing the individual stages, not just the overall map.

### 4.2 Motivation: Why Not Multicategories?

A multicategory generalizes a category by allowing morphisms with multiple inputs:

    f : A₁, A₂, ..., Aₙ → B

This is closer to what we want: a command takes a sequence of arguments (multiple inputs) and produces a result. But a multicategory has a single output, and shell commands have multiple outputs (stdout, stderr, exit status, other fds). More importantly, a multicategory still requires composition to be total — the operadic substitution must always produce a result.

### 4.3 Virtual Double Categories

A virtual double category, introduced by Burroni (under the name "multicatégorie") and Leinster (under the name "fc-multicategory"), and given their current name by Cruttwell and Shulman, is a structure with four kinds of data:

1. **Objects.** Written as X, Y, Z, etc.

2. **Vertical arrows.** Ordinary morphisms f : X → Y between objects. These compose associatively as in a category. They are the "tight" or "strict" part of the structure.

3. **Horizontal arrows.** Written as p : X ⇸ Y (with a distinct notation to distinguish them from vertical arrows). These are the "loose" part — they do not necessarily compose. Given p : X ⇸ Y and q : Y ⇸ Z, the composite q ∘ p may or may not exist.

4. **Cells.** A cell has the following shape:

   ```
   X₀ ——p₁——⇸ X₁ ——p₂——⇸ X₂ ——p₃——⇸ ··· ——pₙ——⇸ Xₙ
   |                                                  |
   f                        α                         g
   |                        ⇓                         |
   Y₀ ————————————q————————————————————————————⇸ Y₁
   ```

   The top boundary is a *sequence* of horizontal arrows (p₁, p₂, ..., pₙ). The bottom boundary is a *single* horizontal arrow q. The sides are vertical arrows f : X₀ → Y₀ and g : Xₙ → Y₁. The cell α mediates between the sequence on top and the single arrow on the bottom.

   The top sequence can have length zero (a "nullary cell"), in which case X₀ = Xₙ and the cell has the shape of a triangle.

**Composition of cells** is defined as follows. Given a cell β with bottom arrow q, and cells α₁, ..., αₘ whose bottom arrows form the top sequence of β, the composite β(α₁, ..., αₘ) is a cell whose top sequence is the concatenation of all the top sequences of the αᵢ, and whose bottom arrow is the bottom arrow of β. This composition is associative and unital (identity cells exist for each horizontal arrow).

The key point: **horizontal arrows do not compose, but cells do.** The compositional structure lives at the level of cells (two-dimensional), not at the level of arrows (one-dimensional). A sequence of horizontal arrows along the top of a cell is just a sequence — it is not required to have a composite.

### 4.4 Composites and the Segal Condition

A virtual double category *may* have composites for some or all sequences of horizontal arrows. A **composite** of a sequence p₁, p₂, ..., pₙ is a horizontal arrow q together with a cell

```
X₀ ——p₁——⇸ X₁ ——p₂——⇸ ··· ——pₙ——⇸ Xₙ
|                                     |
id                   κ               id
|                    ⇓                |
X₀ ———————————q———————————————⇸ Xₙ
```

that is **opcartesian**: for any other cell with top boundary (p₁, ..., pₙ), there is a unique cell with top boundary (q) that factors through κ. In other words, q is the universal single arrow that represents the sequence (p₁, ..., pₙ).

When all composable sequences have composites, the virtual double category is essentially a pseudo double category — the horizontal arrows form a bicategory. The Segal condition (from the homotopy-theoretic perspective) says precisely this: composition fibers are contractible, meaning composites exist and are unique up to unique isomorphism.

But the *virtual* setting does not require this. Sequences of horizontal arrows exist as sequences, whether or not they can be composed. The framework is native to the uncomposed case.

### 4.5 The Analogy, Previewed

The analogy with multicategories is exact, and Cruttwell-Shulman state it explicitly:

> Intuitively, virtual double categories generalize pseudo double categories in the same way that multicategories generalize monoidal categories.

In a monoidal category, any two objects can be tensored. In a multicategory, morphisms can have multiple inputs, but those inputs are not required to be tensorable into a single input. In a pseudo double category, any composable pair of horizontal arrows has a composite. In a virtual double category, cells can have multiple horizontal arrows on top, but those arrows are not required to have a composite.

The passage from Bourne shell to rc is precisely the passage from monoidal category to multicategory — and the passage from "composition is required" to "sequences are primitive." Virtual double categories are the framework in which this passage is native.


---


## 5. The Mapping

We now lay out the explicit correspondence between rc shell concepts, sequent calculus constructs, and virtual double category structure. We proceed concept by concept, with rc code examples throughout.

### 5.1 Objects = Process Interfaces

**In the virtual double category:** Objects X, Y, Z are the vertices — the things that horizontal arrows connect.

**In the shell:** An object is the interface of a process at a point in time: its typed file descriptors, its environment, its expected argument structure. When we write

```
who | wc
```

there are three objects: the initial environment (before `who`), the interface between `who` and `wc` (a stream of text lines on a pipe), and the final environment (after `wc`).

### 5.2 Horizontal Arrows = Channels

**In the virtual double category:** A horizontal arrow p : X ⇸ Y is a "loose" morphism — a connection between two objects that does not necessarily compose with other horizontal arrows.

**In the shell:** A horizontal arrow is a channel — a typed communication path between processes. The most familiar channel is a pipe, but file descriptors, files, and argument lists are all channels. A pipe connecting `who`'s stdout to `wc`'s stdin is a horizontal arrow. An argument list `(printf scanf putchar)` passed to a `for` loop is a horizontal arrow (carrying structured data from the calling context to the loop body).

These do not compose freely. A pipe carrying text lines and a pipe carrying binary data are both horizontal arrows, but there is no general way to compose them into a single "text-then-binary" channel. Composition may exist in specific cases (two text streams can be concatenated), but it is not guaranteed.

### 5.3 Vertical Arrows = Interface Transformations

**In the virtual double category:** A vertical arrow f : X → Y is a "tight" morphism. These compose associatively. They transform one object (interface) into another.

**In the shell:** A vertical arrow is a transformation of a process interface — a type coercion, a protocol adaptation, a renaming. File descriptor manipulation is the clearest example:

```
vc junk.c >[2=1]
```

The redirection `>[2=1]` transforms the interface of `vc` by duplicating fd 1 onto fd 2. The process's external interface changes (stderr now goes where stdout goes), but the process itself is unchanged. This is a vertical arrow: it maps one interface to another, and such transformations compose (you can chain redirections):

```
vc junk.c >junk.out >[2=1]
```

This is the composite of two vertical arrows: first redirect fd 1 to a file, then duplicate fd 1 onto fd 2. The composite is a single interface transformation (both stdout and stderr go to the file). Composition is associative and order matters — reversing the redirections gives a different result, as Duff carefully explains in his paper.

### 5.4 Cells = Commands

**In the virtual double category:** A cell α has a sequence of horizontal arrows on top (the multi-source), a single horizontal arrow on the bottom (the target), and vertical arrows on the sides. It mediates between the input sequence and the output.

**In the shell:** A cell is a command. The top boundary is the command's input interface — its argument list, its stdin, its other input fds — laid out as a *sequence* of channels. The bottom boundary is its output interface. The vertical arrows on the sides express how the command's interface relates to the surrounding context.

Consider:

```
cat '{ls -tr|sed 10q}
```

This concatenates the ten oldest files in the current directory. The cell structure is:

- **Top boundary:** A sequence of horizontal arrows, one for each filename produced by `'{ls -tr|sed 10q}`. The command substitution produces a list — say (file1 file2 ... file10) — and each element is a horizontal arrow from the filesystem to `cat`'s argument parser. These are *not composed*. They remain as a sequence of ten separate inputs.

- **Bottom boundary:** A single horizontal arrow — cat's stdout, carrying the concatenated contents.

- **The cell itself:** `cat`, which mediates between the sequence of input files and the single output stream.

The top boundary is a sequence, not a composite. This is Duff's principle expressed categorically: the argument list is a sequence of horizontal arrows, and the cell (the command) receives them as a sequence.

### 5.5 Cell Composition = Command Substitution and Piping

**In the virtual double category:** Given a cell β whose top boundary is (q₁, ..., qₘ), and cells α₁, ..., αₘ whose bottom boundaries are q₁, ..., qₘ respectively, the composite cell β(α₁, ..., αₘ) has:

- Top boundary: the concatenation of all top boundaries of the αᵢ.
- Bottom boundary: the bottom boundary of β.

This is the operadic composition: plug outputs into input slots.

**In the shell:** This is command substitution. Consider:

```
wc '{ls}
```

Here `ls` is a cell α whose bottom boundary is a list of filenames (produced on stdout, split on `$ifs`). The command `wc` is a cell β that expects a sequence of filenames as its top boundary. The command substitution `'{...}` plugs α's output into β's input. The composite cell is `wc` applied to the files listed by `ls`.

Piping is the special case where the connection is on a single channel (stdin/stdout):

```
who | wc
```

The cell `who` has a bottom boundary including its stdout. The cell `wc` has a top boundary including its stdin. The pipe connects who's stdout to wc's stdin. The remaining inputs and outputs (who's arguments, wc's stdout, both processes' stderr) stay open.

### 5.6 The Segal Condition = Pipeline Fusibility

**In the virtual double category:** A sequence of horizontal arrows has a composite when there exists an opcartesian cell — a universal single arrow representing the sequence.

**In the shell:** A pipeline segment is fusible when the sequence of stages can be replaced by a single command with equivalent behavior. For example:

```
cat file | grep pattern
```

is fusible to

```
grep pattern file
```

The two-stage pipeline (cat, then grep) has a composite: a single command (grep with a file argument) that produces the same result. The opcartesian cell is the equivalence witness.

Not all pipelines are fusible. `sort | uniq` cannot generally be replaced by a single command, because `sort` needs to see all input before producing any output, while `uniq` processes line by line. The pipeline exists as a sequence — two cells — but the sequence does not have a composite in the virtual double category. The Segal condition fails for this pair, and that is fine: the virtual framework does not require it to hold.

This is the advantage of the virtual setting. We can reason about pipelines as sequences of cells — analyzing each stage, optimizing individual stages, localizing errors to specific stages — without needing to reduce the pipeline to a single monolithic command. Composition is available when it exists (pipeline fusion) but the sequence is the primary object.

### 5.7 The Empty List and Nullary Cells

**In the virtual double category:** A cell with an empty top boundary (no horizontal arrows on top) is a nullary cell. It has the shape of a triangle: two vertical arrows from a common source, and one horizontal arrow on the bottom.

**In the shell:** A nullary command — one that takes no arguments — is a nullary cell:

```
date
```

This command takes no arguments (empty top boundary) and produces output (bottom boundary). The empty list `()` in rc is the identity for list formation, and a command with no arguments corresponds to a cell with a nullary source.

The distinction between `()` and `''` is precisely the distinction between a nullary cell (no inputs) and a cell with one input that happens to carry no data. A command invoked with no arguments is different from a command invoked with one empty-string argument, and the virtual double category respects this distinction because the length of the top boundary is part of the cell's structure.

### 5.8 Concatenation = Functorial Action

**In the virtual double category:** Vertical arrows act on horizontal arrows by pre- and post-composition (when the virtual double category has restrictions, i.e., is a virtual equipment). Given a vertical arrow f : X → Y and a horizontal arrow p : Y ⇸ Z, the restriction p(f, id) : X ⇸ Z transforms the source of p.

**In the shell:** The caret operator `^` transforms the content of arguments without changing the sequence structure:

```
src=(main subr io)
cc $src^.c
```

is equivalent to

```
cc main.c subr.c io.c
```

The caret applies a transformation (append `.c`) to each element of the list, preserving the list structure. This is a vertical arrow acting pointwise on each horizontal arrow in the sequence: each filename-channel is transformed by the suffix operation, but the sequence of three channels remains a sequence of three channels.

The distribution law for `^` — pairwise when both operands have equal length, broadcast when one is a singleton — is the functorial action of a vertical arrow on a horizontal sequence: either act on each element by a corresponding transformation (pairwise) or act on each element by the same transformation (broadcast).

### 5.9 Functions = Named Cells

**In the virtual double category:** There is no primitive notion of "named cell" — but one can consider a virtual double category with a distinguished collection of cells, indexed by names.

**In the shell:** Rc functions are named cells:

```
# rc syntax (psh uses `def` — see docs/specification.md §def):
fn g {
    grep $1 *.[hcyl]
}
```

This defines a cell named `g`. Its top boundary is a sequence of one horizontal arrow (the pattern argument). Its body `grep $1 *.[hcyl]` is a command (a cell) that receives the argument and produces output. Invoking `g pattern` instantiates the cell: it fills in the top boundary and executes.

Function definitions are deleted by writing `fn name` with no body. This removes the named cell from the collection — it does not leave a "null function" behind. Again, the distinction between absence and nullity is maintained.

> **psh departure:** psh uses `def` instead of `fn`. rc's `fn` was a
> misnomer — it defines a cut template, not a first-class function.
> `def` names the sort honestly. See `docs/specification.md` §Two
> kinds of callable.

### 5.10 Signal Handlers = Cells on Exceptional Arrows

Signal handlers in rc are functions triggered by external events:

```
# rc syntax:
fn sigint {
    rm /tmp/junk
    exit
}
```

This defines a cell that runs when the SIGINT horizontal arrow arrives. Because it is a function (parsed at definition time), it is a well-formed cell — its internal structure was checked when it was defined. In the Bourne shell, signal handlers are strings, which means they are horizontal arrows that must be *re-parsed* (cut-eliminated at signal time). This is precisely the violation of Duff's principle: the Bourne shell stores a deferred computation as a flat string rather than as a pre-parsed cell.

> **psh departure:** psh replaces rc's `fn SIGNAL { body }` pattern
> with a unified `trap` grammar: `trap SIGNAL { handler }` (global),
> `trap SIGNAL { handler } { body }` (lexical scope), `trap SIGNAL`
> (deletion). The structural point is unchanged — handlers are
> pre-parsed cells, not deferred strings — but the syntax is
> different. See `docs/specification.md` §Unified trap.

### 5.11 Local Variables = Restricted Cells

Rc allows local variable bindings scoped to a single command:

```
a=global
a=local echo $a
echo $a
```

This prints `local` then `global`. The assignment `a=local` is in force only for the duration of the `echo` command.

In the virtual double category, this is a cell whose top boundary includes a restricted horizontal arrow — the variable binding is part of the input interface, scoped to the cell. The restriction does not propagate to subsequent cells.

### 5.12 The eval Command = Collapsing Structure

Duff describes `eval` as the one construct that deliberately violates the no-rescan principle:

```
s='*'
eval 'pages='$s/$i
```

The `eval` command concatenates its arguments into a single string and re-parses it. This is the only place in rc where structure (the list) is deliberately flattened to a string and then rescanned. Duff says: "Input is never scanned more than once by the lexical and syntactic analysis code (except, of course, by the eval command, whose raison d'être is to break the rule)."

In the virtual double category, `eval` is the composite map — it takes a sequence of horizontal arrows (the argument list), forces them into a single composite arrow (the concatenated string), and then re-parses the composite to recover structure. This is exactly the Segal condition applied destructively: force the composite to exist, even when it does not naturally. The result is a new cell whose top boundary has been recomputed from the flattened data, possibly with different boundaries than the original sequence.

The fact that `eval` must be explicitly invoked — that it is a command, not an implicit behavior — is the design expression of the virtual principle. Composition is not the default. You must ask for it. The default is the sequence.

### 5.13 Here Documents = Cells with Embedded Data

```
for(i) grep $i <<!
tor 2T-402 2912
kevin 2C-514 2842
bill 2C-562 7214
!
```

A here document is a cell whose top boundary includes a literal data channel — the text between the markers. The cell `grep $i` receives two inputs: the pattern (from the `for` loop) and the data (from the here document). These are two horizontal arrows in the top boundary of the cell, one variable and one fixed. Variable substitution occurs within the here document (unless the marker is quoted), which is the functorial action of the enclosing context's variable bindings on the literal data channel.

### 5.14 The Holmdel Example: A Complete Pasting Diagram

Duff's `holmdel` script is worth examining as a complete worked example. Here is a simplified excerpt in rc syntax:

> **psh syntax differs in three places**, annotated with `# psh:`
> comments below. The pasting diagram analysis applies to both.

```
t=/tmp/holmdel$pid

fn read {                        # psh: def read {
    $1='{awk '{print;exit}'}
}

fn sigexit sigint sigquit sighup {  # psh: separate trap statements
    rm -f $t                        #   trap SIGEXIT { rm -f $t; exit }
    exit                            #   trap SIGINT  { rm -f $t; exit }
}                                   #   etc.

while(){                         # psh: while(true) {
    lab='{fortune $t}
    echo $lab
    if(~ $lab Holmdel){
        echo You lose.
        exit
    }
    while(read lab; ! grep -i -s $lab $t) echo No such location.
    if(~ $lab [hH]olmdel){
        echo You win.
        exit
    }
}
```

The pasting diagram of this script is:

1. **Initialization cells:** `t=/tmp/holmdel$pid` is a cell that produces a value (the filename) and binds it. The `fn` definitions register named cells. The here document (omitted for space) populates the data file — a cell with embedded constant horizontal arrows.

2. **The outer loop** is an iterated cell with no termination condition (`while()`), meaning the cell repeats indefinitely.

3. **Inside the loop:** `'{fortune $t}` is command substitution — an inner cell whose output is plugged into the binding `lab=...`. This is operadic composition: the fortune cell's output becomes a horizontal arrow in the top boundary of the subsequent cells.

4. **Pattern matching:** `~ $lab Holmdel` is a cell that tests whether the value on one horizontal arrow (the contents of `$lab`) matches a pattern. Its bottom boundary is the exit status — a horizontal arrow carrying a boolean. The `if` construct is conditional cell composition: compose the body cell only if the status arrow carries "true."

5. **The inner loop** `while(read lab; ! grep -i -s $lab $t)` is a compound condition: two cells composed sequentially, with the status of the second determining repetition. The `read` function itself is a cell that captures stdin (a horizontal arrow from the terminal) and binds it to a variable (inserting a new horizontal arrow into the context).

6. **Signal handling:** The signal handler definitions are cells pre-registered on exceptional horizontal arrows. If SIGINT arrives (an exceptional horizontal arrow from the operating system), the registered cell fires: it removes the temp file and exits. Because the handler is a pre-parsed cell, there is no rescanning — the cell's structure was established at definition time. (In psh, each signal gets its own `trap` statement rather than rc's multi-signal `fn` form.)

The whole script is a pasting diagram in the virtual double category **Shell**: cells stacked and nested, with horizontal arrows carrying data between them, vertical arrows transforming interfaces (redirections, variable scoping), and the sequence structure of argument lists preserved at every level.


---


## 6. The Framework Assembled

### 6.1 The Virtual Double Category of Shell Programs

We can now describe the virtual double category **Shell** whose structure captures rc's semantics:

**Objects** are process interfaces: typed descriptions of a process's communication endpoints (file descriptors, argument expectations, environment variables, exit status protocol).

**Vertical arrows** f : X → Y are interface transformations: redirections, fd manipulations, environment modifications. These compose strictly — `>[2=1]` followed by `>file` gives a definite composite transformation.

**Horizontal arrows** p : X ⇸ Y are channels: typed communication paths between process interfaces. A pipe, a file, an argument list element, a signal — each is a horizontal arrow. These do not compose freely.

**Cells** are commands: each cell has a sequence of input channels (top boundary), an output channel configuration (bottom boundary), interface transformations on the sides, and an internal computation. The cell structure is:

```
In₁ ——arg₁——⇸ In₂ ——arg₂——⇸ ··· ——argₙ——⇸ Inₙ₊₁
|                                              |
redir_in                cmd                redir_out
|                        ⇓                     |
Out₁ ——————————result——————————————⇸ Out₂
```

**Cell composition** is command substitution and piping: plugging one cell's output into another cell's input slot. This is the operadic composition of the virtual double category.

**Composites** (the Segal condition) hold for pipeline segments that can be fused into a single command, and fail for segments that cannot. The framework does not require fusion; it works natively with sequences.

### 6.2 The Sequent Calculus as the Type Theory of Shell

The sequent calculus provides the *type theory* for the horizontal arrows in **Shell**. Each horizontal arrow carries a type — a description of the data protocol on that channel. The cut rule of the sequent calculus is the typing rule for cell composition (piping, command substitution). The identity rule is the typing rule for identity cells (pass-through channels). Structural rules (weakening, contraction) correspond to unused file descriptors and shared file descriptors.

The polarity discipline of the focused sequent calculus maps onto shell semantics:

**Positive types** (data, values) correspond to *argument-like* channels. Arguments are fully evaluated before the command sees them. A file on disk, a string literal, a fully expanded variable — these are positive. They are eager: their content is determined before the cut fires.

**Negative types** (codata, continuations) correspond to *stream-like* channels. A pipe is not a value — it is a computation that produces data on demand. A file descriptor connected to a running process is negative. It is lazy: its content is determined incrementally as the cut reduces.

**Focusing** is the discipline that determines evaluation order within a command. The static focusing of Binder et al. corresponds to the shell's argument expansion: before a command runs, all its arguments are focused (evaluated to values). This is why rc evaluates `'{ls}` before passing the result to the outer command — command substitution is a focusing step that reduces a negative type (the running `ls` process) to a positive type (the resulting list of filenames).

**The shift connectives** mediate between positive and negative types. In the shell, this is the boundary between "a file as data" (positive — a filename string you can pass as an argument) and "a file as a stream" (negative — an open fd you can read from). The operation of opening a file for reading is a shift from positive to negative; capturing a command's output into a variable is a shift from negative to positive.

### 6.3 Preserving Duff's Principle

The entire framework is designed to preserve and formalize Duff's core insight:

**Arguments are lists, not strings** becomes **the top boundary of a cell is a sequence of horizontal arrows, not a single composite arrow.** The virtual double category is the mathematical setting where sequences are primitive and composition is optional.

**Substitution is structure-preserving** becomes **cell composition preserves the multi-source structure.** When a cell's output is substituted into another cell's input slot, the resulting cell's top boundary is the concatenation of the input sequences — no flattening, no rescanning, no loss of boundaries.

**Input is never scanned more than once** becomes **horizontal arrows are parsed once (when created) and carried as typed data thereafter.** The type of a horizontal arrow is fixed at creation time. Substitution does not re-type the arrow; it merely places it in a new position within a cell's top boundary.

**eval is the exception that proves the rule** becomes **forcing the Segal condition (computing a composite) is an explicit operation, not a default behavior.** In the virtual double category, most sequences of horizontal arrows do not have composites. Composition must be constructed explicitly. This is the categorical expression of "input is never scanned more than once, except by eval."

### 6.4 From Rc to a Typed Shell

> **Note:** This section was written as a prospective sketch. psh's
> spec has since resolved all four items below. The spec is
> authoritative for the concrete design; this section preserves the
> categorical motivation.

Rc gets the data model right but does not have a type theory. All horizontal arrows in rc carry the same type (lists of strings). All channels are untyped byte streams. The sequent calculus provides the machinery to refine this:

**Typed arguments.** Instead of all arguments being strings, each position in a command's argument sequence can have a specific type (filename, pattern, integer, flag). The type is part of the horizontal arrow, and the cell (the command) specifies the types it expects in its top boundary.

**Typed channels.** Instead of all pipes carrying byte streams, each channel can carry a session type — a protocol describing the sequence of messages. The type of a pipe between `sort` and `uniq` could be "a sorted stream of lines," which differs from the type of a pipe between `cat` and `wc` ("an unsorted stream of lines"). The cut rule checks that the output type matches the input type.

**Static error detection.** Pipeline type errors become cut-typing failures — detectable before execution. If `cmd1` produces binary on stdout and `cmd2` expects newline-delimited text on stdin, the cut `cmd1 | cmd2` is ill-typed, and the shell can report this without running either command.

**Polarity-aware evaluation.** The shell can use the polarity discipline to determine evaluation order. Positive arguments (files, strings) are evaluated eagerly. Negative arguments (process substitutions, pipes) are evaluated lazily. The focusing discipline makes this explicit rather than relying on ad hoc conventions.

The virtual double category provides the scaffold for all of this: objects are typed interfaces, horizontal arrows are typed channels, vertical arrows are interface coercions, and cells are commands with typed multi-source and typed target. The Segal condition tells you when pipelines can be fused, and the operadic composition gives you the rule for command substitution. The sequent calculus is the internal logic of the virtual double category — the type theory that governs the horizontal arrows and cells.


---


## 7. Summary of Correspondences

| Shell concept               | Sequent calculus               | Virtual double category              |
|-----------------------------|--------------------------------|--------------------------------------|
| Process interface           | Context (Γ, Δ)                 | Object                               |
| Argument / file / fd        | Formula (A, B)                 | Horizontal arrow                     |
| Fd redirect / dup           | (structural transformation)    | Vertical arrow                       |
| Command execution           | Cut ⟨p \| c⟩                  | Cell                                 |
| Argument list               | Multi-formula context          | Sequence of horizontal arrows (multi-source) |
| Pipe `\|`                   | Cut on a single formula        | Cell composition (single slot)       |
| Command substitution `` `{} `` | Operadic substitution       | Cell composition (general)           |
| Variable binding            | μ̃x.s (value capture)          | Naming a horizontal arrow            |
| Redirection                 | μα.s (continuation capture)    | Vertical arrow / restriction         |
| `eval`                      | Forced re-parsing              | Forcing the Segal condition          |
| Pipeline fusion             | Cut elimination                | Opcartesian cell (composite exists)  |
| Unfusible pipeline          | Irreducible cut sequence       | Sequence without composite           |
| `()` (empty list)           | Empty context                  | Nullary source                       |
| `''` (one empty string)     | Singleton context with unit    | Unary source with trivial arrow      |
| Positive type (data)        | Constructor-defined type       | Horizontal arrow, positive polarity  |
| Negative type (codata)      | Destructor-defined type        | Horizontal arrow, negative polarity  |
| Focusing / arg expansion    | Static focusing                | Evaluation to canonical form         |
| `def name { body }`         | Named proof / definition       | Named cell                           |
| `trap SIGNAL { handler }`   | Cell on exceptional channel    | Cell on distinguished horizontal arrow |
| Here document               | Literal data in proof          | Cell with embedded constant arrows   |
| `for(i in list) cmd`        | Iterated cut over list         | Family of cells indexed by sequence  |
| `match` / `=>`             | Case split on data type        | Cell with branching on constructor   |
| `^` (caret)                 | (pointwise transformation)     | Vertical arrow acting on sequence    |
| Local variable              | Scoped binding                 | Restricted horizontal arrow          |
| `let x = M`                | μ̃x.s (monadic bind, CBPV)    | Value capture into Γ                 |
| `try { } catch { }`        | Scoped ErrorT                  | Cell with error channel              |
| `$((…))` arithmetic        | Pure producer (no cut)         | Cell with empty side arrows          |
| Tuple `(a, b)`             | Product type                   | Horizontal arrow with product structure |
| Struct `T { x=1; y=2 }`   | Named product                  | Horizontal arrow with labelled fields |
| Enum variant `ok(v)`       | Coproduct constructor          | Constructor injection into sum       |
| Coprocess channel           | Binary session type            | Typed horizontal arrow (per-tag session) |
| `.get` / `.set` discipline | Codata observer / constructor  | MonadicLens in Kl(Ψ)                |


---


## 8. Composition Laws

The ksh26 Theoretical Foundation (`refs/ksh93/ksh93-analysis.md`)
identifies three composition patterns in shell execution,
corresponding to three duploid equations. These patterns carry over to any shell built on
the VDC framework and serve as a decision procedure for new
features.

### 8.1 Pipeline composition (•, Kleisli/monadic)

Two cells composed through a positive intermediary — a pipe
carrying data.

```
ls | sort | uniq -c
```

Each `|` is a positive-intermediary composition: the left cell
produces data, the right cell consumes it. Associativity holds:
`(ls | sort) | uniq -c` and `ls | (sort | uniq -c)` produce the
same result, because the intermediary is a value (data on a pipe)
and value composition is associative.

In duploid terms, the *composition structure* is (•) — Kleisli
composition. The pipe fd is the positive intermediary; data flows
left to right. Note the distinction: the *execution strategy* is
demand-driven (operationally co-Kleisli-flavored — `yes | head -1`
does not evaluate `yes` to completion, because the pipe's blocking
read is the demand). The Kleisli label describes how the pipeline
*types* (data on the pipe is a positive intermediary), not how it
*runs* (demand flows right-to-left). See `docs/specification.md`
§Pipeline execution for the full disambiguation.

### 8.2 Sequential composition (○, co-Kleisli/comonadic)

Two cells composed through a negative intermediary — the execution
context.

```
cd /sys/man || { echo 'No manual!' >[1=2]; exit 1 }
```

The `||` is a negative-intermediary composition: the left cell runs
in the current execution context, produces an exit status, and the
right cell runs in the same context only if the status is nonzero.
The intermediary is the execution context (negative, comonadic), not
a data value.

Associativity holds: nested `||` and `&&` compose correctly because
they all operate within the same execution context. The comonadic
extract (observe exit status) and extend (set up conditional)
compose associatively.

In duploid terms, this is (○) — co-Kleisli composition. Exit status
is carried in the execution context.

### 8.3 Cut (⟨t|e⟩, fundamental interaction)

A producer meets a consumer directly — no intermediary.

```
for(i in $list) {
    process $i
}
```

The `for` loop is a cut: the list `$list` (producer) is cut against
the loop body (consumer). Each iteration binds one element of the
list to `$i` and runs the body. There is no intermediary — the
producer's elements are consumed directly.

### 8.4 The non-associativity failure

Mixing (•) and (○) — a positive intermediary inside a negative
context — is the one composition that fails to associate. In
duploid terms, the (+,−) equation does not hold:

```
(h ○ g) • f  ≠  h ○ (g • f)
```

The left bracketing evaluates f first (producing a value), then
composes g and h in computation mode — the positive state is
contained within the computation frame. The right bracketing
composes g and f through the positive intermediary first, then h
fires around the result — the positive state is exposed to h.

This is the structural condition underlying the sh.prefix bugs
documented in `refs/ksh93/ksh93-analysis.md`: a computation (a DEBUG trap) intruding into
a value context (compound assignment), with two possible reduction
orders yielding different results. The critical pair is not a
theoretical curiosity — it is the root cause of real interpreter
corruption bugs.

The VDC framework resolves this by making the boundary explicit.
Polarity frames (save/restore at the computation boundary) enforce
the left bracketing — the one that contains the positive state.
This is the operational analog of Curien and Munch-Maccagnoni's
focused calculus, which eliminates the critical pair syntactically.
The shell eliminates it operationally, by the same structural means.

> **Dialogue commitment.** psh commits to dialogue-duploid
> structure [MMM, Definition 9.4] — duploid + involutive
> negation via a strong monoidal duality functor. This does not
> restore the (+,−) equation: the non-associativity failure still
> holds for oblique maps. What dialogue provides is the
> Hasegawa-Thielecke theorem [MMM, §9.6]: **thunkable ⇔
> central**. The maps that restore full associativity are exactly
> the maps that commute with all others under tensor. This
> equivalence sharpens the boundary between the "safe"
> subcategory (pure, freely reorderable) and the "unsafe" one
> (oblique, requiring polarity frames). The ⊗/⅋ De Morgan
> duality (`¬(A ⊗ B) ≅ ¬A ⅋ ¬B`) is an immediate theorem of
> dialogue structure; the ⊕/⅋ duality noted in
> `docs/specification.md` §Error model follows via the
> L-calculus's additive/multiplicative relationship. See
> `docs/specification.md` §The semantics for the full commitment.

### 8.5 Decision procedure for new features

When adding a new feature to a shell built on this framework, the
implementer should classify the feature by its composition pattern:

**Purely value-level?** (e.g., new expansion syntax, new string
operation, new type annotation) → Monadic. Thread through the
expansion context. No polarity frame needed. The feature composes
with other value-level operations via (•). Under the dialogue
commitment, the purity test has a single criterion: a map is
purely value-level if and only if it is thunkable, which by the
Hasegawa-Thielecke theorem is equivalent to being central.

**Purely computation-level?** (e.g., new control flow construct,
new job control feature) → Comonadic. Save/restore the execution
context. Use the standard polarity frame discipline. The feature
composes with other computation-level operations via (○).

**Crosses the boundary?** (e.g., discipline functions that query
external state, command substitution within expansion, coprocess
queries inside a `.get` discipline) → Polarity shift. Push a
polarity frame at the boundary. The shift is explicit in the code
(a frame enter/leave pair) and in the type theory (a shift
connective ↓ or ↑). The frame enforces the left bracketing of the
(+,−) composition, preventing the non-associativity failure.

This three-way classification — monadic, comonadic, or
boundary-crossing — is the operational content of the duploid
structure. It replaces ad hoc reasoning about "when to save and
restore state" with a structural criterion derived from the
composition laws.


---


## 9. Engineering Principles

### 9.1 Duff's Principle Generalized

The original principle: **input is never scanned more than once.**

Generalized for a shell built on the VDC framework: **structure is
never destroyed and reconstructed.** This includes:

- **List boundaries** (Duff's original case). A variable holding
  three strings always splices three strings. No IFS, no rescanning.
- **Type information.** A typed value is never flattened to a string
  and re-parsed to recover its type. The type is carried with the
  value.
- **Polarity.** A value-mode context is never silently entered from
  computation mode. The shift is explicit (a polarity frame), not
  implicit (a hidden re-scan or ad hoc save/restore).
- **Session protocols.** A channel's protocol state is never lost
  and re-inferred. The session type is established at creation time
  and tracked throughout the channel's lifetime.
- **Frame boundaries.** A length-prefixed message's boundaries are
  carried in the framing (the length prefix), not recovered by
  scanning for delimiters. This is Duff's principle applied at the
  byte level.

### 9.2 The Horizontal Arrow Discipline

Every channel (pipe, fd, variable, argument) is a horizontal arrow
with a type. The type is assigned at creation time and does not
change. Operations on the channel must be compatible with the type.
The type is carried in the data structure, not recovered from the
content.

In implementation terms: every variable carries a type descriptor
alongside its value. Every coprocess channel carries a session type
descriptor. The descriptors are set at creation and checked at use.
This is the VDC framework's horizontal arrow discipline made
concrete.

### 9.3 The Polarity Frame Discipline

Every boundary crossing between value mode and computation mode goes
through a polarity frame. The frame saves positive-mode state, clears
it, runs the computation, and restores the state. No exceptions.

The polarity frame is the C/Rust implementation of the shift
connective from the focused sequent calculus. The VDC framework says:
this is the mechanism that preserves horizontal arrow types across
mode boundaries. Without the frame, a computation-mode operation can
corrupt value-mode state — which means a horizontal arrow's type can
be silently changed, violating the horizontal arrow discipline.

Discipline functions (`.get` bodies that query external state,
`.set` bodies that propagate assignments) are the most common site
for polarity frames in user-facing code. The frame wraps the
discipline body, protecting the surrounding expansion context from
the computation-mode intrusion. CBV focusing (the value is computed
once per expression and reused at each consumption site) prevents
re-invocation after the frame exits.

### 9.4 The Segal Condition as Optimization Guide

Pipeline fusion — replacing a multi-cell pipeline with a single
cell — is an optimization, not a requirement. The VDC framework
makes this precise: fusion is possible when the Segal condition
holds (an opcartesian cell exists for the sequence of horizontal
arrows).

The implementer should:

- **Not** assume fusion is always possible.
- **Check** whether a proposed fusion preserves the types of the
  intermediate channels.
- **Document** which pipeline patterns are fusible and which are
  not.

Example: `cat file | grep pattern` is fusible because `grep` can
accept a file argument directly. The two-cell pipeline has a
composite: a single cell (`grep pattern file`) that produces the
same result. The opcartesian cell is the equivalence witness.

`sort | uniq` is not fusible because `sort` buffers its entire
input before producing output. The pipeline exists as a sequence
— two cells — and the sequence does not have a composite.

This is the advantage of the virtual setting. The framework reasons
about pipelines as sequences of cells without requiring fusion. The
Segal condition tells you when fusion is available as an
optimization, but the uncomposed sequence is the primary structure.

### 9.5 Named Cells over Eval

Whenever a construct would traditionally require `eval` (indirection,
computed variable names, dynamic command construction), the designer
should ask: can this be expressed as a named cell, a name reference,
or a discipline function instead?

- **Variable indirection?** → Name reference (a vertical arrow in
  the VDC — an interface transformation, not a re-parse).
- **Dynamic command dispatch?** → A function table (a collection
  of named cells), not `eval`.
- **Computed field access?** → Accessor notation with a variable
  key, not `eval`.

`eval` is the "force the Segal condition" escape hatch — it takes a
sequence of horizontal arrows (the argument list), forces them into
a single composite arrow (the concatenated string), and re-parses
the composite to recover structure. This is exactly the Segal
condition applied destructively: force the composite to exist, even
when it doesn't naturally. **The information loss happens in the
composition step (string concatenation erases boundaries), not in
the re-parse — the re-parse can only find whatever boundaries the
concatenation preserved, which is generally none.**

The fact that `eval` must be explicitly invoked — that it is a
command, not an implicit behavior — is the design expression of the
virtual principle. Composition is not the default. You must ask for
it. Every use of `eval` in an existing shell script is a design
smell: it indicates a case where the shell's type system should have
provided a structural solution but didn't.


---


## 10. Concluding Remarks

The thread that runs through this report is: **Duff's "not a macro processor" principle is a theorem about the preservation of algebraic structure under substitution, and the appropriate algebraic structure is a virtual double category.**

The Bourne shell computes in a monoidal category: values are strings, composition (IFS splitting + glob expansion + rescanning) is a total operation, and every substitution forces the composite. This destroys structure and creates the entire class of bugs that plague Bourne shell programming.

Rc computes in a multicategory: values are lists (sequences of strings), substitution splices lists into lists, and composition is not forced. The boundaries between list elements are data, not artifacts of parsing. This eliminates rescanning and its associated bugs.

A typed shell grounded in the sequent calculus computes in a virtual double category: commands are cells with typed multi-source (argument sequences) and typed target (output channels), composition is available when the Segal condition holds (pipeline fusion) but is not required, and the polarity discipline of the focused sequent calculus governs evaluation order. The no-rescan invariant is built into the framework: horizontal arrows are typed at creation time and never re-parsed, because the virtual double category does not require (or permit) implicit composition.

The contribution of this perspective is not a new shell, but a new way of seeing the shell that already exists. Rc's design is already virtually double-categorical in structure. The sequent calculus makes the typing explicit. The virtual double category makes the composition theory explicit. Together, they provide a foundation on which a typed, session-aware, formally grounded shell can be built — one that preserves every aspect of Duff's original insight while gaining the expressiveness and safety of a genuine type system.


---


## References

All references use canonical keys from `docs/citations.md`. Full
citations with venue, year, and DOI are in the bibliography.

**Directly cited in this document:**

- `[Duf90]` — Duff, "Rc — The Plan 9 Shell." Founding reference for the entire analysis.
- `[CH00]` — Curien, Herbelin, "The Duality of Computation." ICFP, 2000. The λμμ̃-calculus.
- `[Mun13]` — Munch-Maccagnoni, "Syntax and Models of a Non-Associative Composition." Thesis, 2013. Non-associative composition, focusing.
- `[Spi14]` — Spiwack, "A Dissection of L." 2014. Shift placement, L-calculus analysis.
- `[BTMO23]` — Binder, Tzschentke, Müller, Ostermann, "Grokking the Sequent Calculus." 2023. Accessible sequent calculus presentation.
- `[CS10]` — Cruttwell, Shulman, "A unified framework for generalized multicategories." TAC, 2010. Virtual double categories.
- `[Lei04]` — Leinster, *Higher Operads, Higher Categories.* CUP, 2004. fc-multicategories.
- `[Bur71]` — Burroni, "T-catégories." 1971. Original multicatégorie structure.

**Cited by the spec for constructs discussed in this document:**

- `[MMM]` — Mangel, Melliès, Munch-Maccagnoni, "Duploids." Composition laws (§8).
- `[CMM10]` — Curien, Munch-Maccagnoni, "Duality Under Focus." TCS, 2010. Focusing discipline.
- `[CBG24]` — Clarke, Boisseau, Gibbons, "Profunctor Optics." Compositionality, 2024. MonadicLens structure for discipline functions.
- `[CMS]` — Carbone, Marin, Schürmann, "Async Multiparty Compatibility." MCutF admissibility for coprocess star topology.
- `[HVK98]` — Honda, Vasconcelos, Kubo, "Session Types." ESOP, 1998. Binary sessions for coprocesses.
- `[Lev04]` — Levy, *Call-by-Push-Value.* 2004. CBPV for `let` binding.
