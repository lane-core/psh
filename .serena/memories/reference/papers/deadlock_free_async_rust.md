---
type: reference
status: current
created: 2026-04-11
last_updated: 2026-04-11
importance: high
keywords: [rust, multiparty-session-type, deadlock-free, async, message-reordering, phantom-type, implementation, substrate]
agents: [psh-architect, psh-session-type-agent]
---

# Reference: Deadlock-Free Asynchronous Message Reordering in Rust with Multiparty Session Types

**Path.** `/Users/lane/gist/deadlock-free-asynchronous-message-reordering-in-Rust-with-multiparty-session-types/`

**Status.** Primary implementation reference. **The Rust substrate for psh's coprocess protocol.**

## Summary

Demonstrates deadlock-free asynchronous message reordering in Rust using phantom types to encode multiparty session types. The technique uses zero-runtime-cost marker types (`PhantomData<T>`) to carry the session state at compile time, so the Rust type checker enforces protocol discipline at build time.

Key pattern:

```rust
trait Session: Send + 'static {
    type Dual: Session<Dual = Self>;
}
struct Send<T, S: Session = ()>(PhantomData<(T, S)>);
struct Recv<T, S: Session = ()>(PhantomData<(T, S)>);
```

The session types live in the type signatures; the compiler enforces them. No `par`, no async runtime, no session-type library dependency.

## Concepts it informs in psh

- **`decision/coprocess_9p_discipline`** — the Rust implementation substrate. psh uses **~40 lines of phantom session types** following this pattern. The compiler enforces `Send<Req, Recv<Resp, End>>` per tag at build time.
- **Deadlock freedom via type discipline** — the Rust type system becomes the deadlock checker.
- **No `par` dependency** — psh deliberately doesn't use the `par` library precisely because this lightweight pattern suffices.

## Who consults it

- **psh-architect** (primary, canonical): for the Rust implementation of coprocesses. This is the how-to reference.
- **session type agent** (secondary): for the connection between session type theory and Rust's phantom-type pattern.

## Note

The full paper has more machinery than psh needs. psh extracts the lightweight phantom-type pattern; the rest (complex protocol composition, arbitrary async runtime integration) is overkill for star-topology coprocesses.
