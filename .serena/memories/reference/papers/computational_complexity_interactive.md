---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: normal
keywords: [computational-complexity, interactive-behavior, session-type, composition, complexity-bound, decidability]
agents: [psh-session-type-agent]
---

# Reference: Computational Complexity of Interactive Behaviors

**Path.** `/Users/lane/gist/computational-complexity-of-interactive-behaviors.gist.txt`

**Status.** Theoretical reference. Complexity bounds for session type compositions.

## Summary

Studies the computational complexity of deciding various session-type properties: compatibility, deadlock freedom, composition, etc. Gives decidability results and complexity bounds (polynomial vs exponential) for different fragments of session type systems.

For psh, this is relevant when considering what the **type system can and cannot guarantee at compile time**. Simple binary sessions (what psh uses) have polynomial-time decidable properties; more expressive session type systems can push into exponential or even undecidable territory.

## Concepts it informs in psh

- **`decision/coprocess_9p_discipline`** — binary sessions per tag (what psh uses) sit in the tractable fragment. The Rust type checker can decide session compatibility in polynomial time.
- **Future extensions** — if psh ever adds dependent session types or refinement session types, the complexity profile changes. This paper tells you what you're buying.
- **Tooling** — for type-checker implementation, the complexity bounds tell you which algorithms to pick.

## Who consults it

- **session type agent**: for complexity-aware design decisions. "Can we decide this at compile time?" questions land here.

## Low-confidence rejection note

Not needed for the current psh design — star topology with binary sessions is well within the tractable fragment. Consult only if a more expressive session type extension is being proposed.
