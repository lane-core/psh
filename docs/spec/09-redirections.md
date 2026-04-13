# Redirections and profunctor structure

## Profunctor structure

### Redirections as profunctor maps

A traditional shell AST bolts redirections onto commands as a
flat list. This representation is silent about evaluation
order. psh encodes redirections as wrapping:

    Redirect(
        Redirect(
            Simple(cmd),
            Output { fd: 1, target: File("file") }
        ),
        Dup { dst: 2, src: 1 }
    )

The profunctor structure:

- `Output` = rmap (post-compose on output continuation)
- `Input` = lmap (pre-compose on input source)

Dup and Close are structural rules on the fd context, not
profunctor maps:

- `Dup` = contraction (two fds alias one resource)
- `Close` = weakening (discard a resource)

Duff: "Redirections are evaluated from left to right" [Duf90,
§Advanced I/O Redirection]. The wrapped representation makes
the only legal evaluation order the correct one. The profunctor
laws hold by construction.

This is the minimal system — two genuine optics survive in the
rc-compatible base:

1. **Redirections** — Adapter (Profunctor constraint only)
2. **fd save/restore** — Lens (Cartesian constraint)

Word expansion has Kleisli structure — each stage is a function
`Word → Val` with possible effects, composing sequentially.
This is a composition pattern in the shell's effect monad,
not an optic. The `.set` update side of the discipline system's
mixed monadic lens lives in the same `Kl(Ψ)` the expansion
pipeline lives in; the `.get` view is pure `W`.

The full optic hierarchy (Prism, AffineTraversal, Traversal)
activates when products and coproducts are added.

### Word expansion as Kleisli pipeline

ksh93's `macro.c` expansion pipeline (tilde → parameter →
command sub → arithmetic → field split → glob) is Kleisli
composition [SPEC, §"The monadic side"]. psh's `eval_term` has
a simpler pipeline:

1. **Literal** → identity (pure, no effects)
2. **Var** → read the stored slot, invoking the variable's
   `.get` cell (pure `W(S, A)` — default is the identity slot
   reader; a user-defined `.get` must remain pure). The result
   is thunkable, hence central [MMM, Prop 8550] (forward
   direction; dialogue structure not required here), and reused
   within the expression by CBV focusing. No polarity frame —
   `.get` has no
   effects to guard. Effectful state refresh is the job of
   `.refresh`, invoked separately as an imperative command.
3. **Count** → lookup then measure
4. **CommandSub** → polarity shift (↓→↑: fork, capture, return)
5. **Concat** → rc's `^` (pairwise or broadcast join)

Each stage is a function `Word → Val` with possible effects.
They compose by structural recursion over the `Term` AST.

**Glob no-match behavior.** A glob pattern that matches no
filenames stands for itself — it is not replaced by the empty
list and is not an error. rc heritage [Duf90, §Patterns]: "a
pattern matching no names is not replaced by the empty list;
rather it stands for itself." `ls *.xyz` passes the literal
string `*.xyz` to `ls`, which prints its own error. This is
the Plan 9 convention and avoids the zsh trap where
`rm *.bak` in an empty directory is a shell error before `rm`
ever runs.


