---
type: decision
status: current
created: 2026-04-10
last_updated: 2026-04-10
importance: high
keywords: [coprocess, 9p, session, per-tag, binary-session, star-topology, forwarder, deadlock-freedom, pending-reply, wire-format, socketpair]
agents: [psh-session-type-agent, psh-architect, vdc-theory, plan9-systems-engineer]
related: [decision/unified_trap_three_forms, decision/every_variable_is_a_list]
---

# Decision: coprocess protocol is 9P-shaped with per-tag binary sessions and star topology

## Decision

psh extracts Plan 9's 9P protocol **shape** (not its wire protocol) as the session discipline for bidirectional shell↔child channels:

1. **Negotiate** — one round-trip confirming both sides speak the same protocol (`"psh protocol v1"` handshake; validates protocol version only).
2. **Request-response pairs** — every request gets a response. No fire-and-forget. No ambiguity about whose turn it is.
3. **Error at any step** — failure is always a valid response, not a special case.
4. **Orderly teardown** — explicit close with reason; EOF is the crash fallback.

**Per-tag binary sessions.** Tags multiplex independent binary sessions over one socketpair. Each tag has session type `Send<Req, Recv<Resp, End>>` — exactly one legal action at each step. Tag space is `uint16` (65535); practical limit is backpressure (socketpair buffer full → sender blocks), not the constant.

**Star topology.** The shell is the **hub**. No coprocess-to-coprocess communication. Deadlock freedom by asymmetric initiator/responder discipline (shell always initiates; coprocess always responds).

**Wire format.** Length-prefixed binary frames:
```
frame = length[4 bytes LE u32] tag[2 bytes LE u16] payload[length - 2]
error = length[4 bytes LE u32] tag[2 bytes LE u16] '!' error_message
```
`MAX_FRAME_SIZE` = 16 MiB.

**User protocol.** `print -p name 'request'` returns an Int tag. `read -p name reply` reads the oldest outstanding response; `read -p name -t $tag reply` reads a specific tag.

## Why

ksh93 introduced coprocesses (`cmd |&`) as untyped byte streams with no protocol discipline. Bash extended them with named coprocesses, still no discipline. Neither could multiplex concurrent requests or guarantee response ordering. Plan 9's 9P protocol (session-typed IPC over a byte stream) is the design inspiration — psh takes its **conversation shape**, not its bytes.

**Star topology justification:** Carbone-Marin-Schürmann ("Logical Interpretation of Asynchronous Multiparty Compatibility") prove forwarders capture all multiparty compatible compositions via linear logic. The shell IS the forwarder, so any coprocess composition is reducible to star-shaped sessions and automatically deadlock-free.

**Per-tag multiplexing** gives concurrent requests without abandoning session discipline: each tag is its own binary session, and the tags correlate requests to responses (same role as 9P's uint16 transaction tags). Tag reuse is managed by shell-internal `PendingReply` tracking with affine obligation handles; drop-as-cancel sends a Tflush-equivalent.

**Length-prefixed wire format** applies Duff's principle at the byte level: frame boundaries are data (the length prefix), not artifacts of parsing (no scanning for delimiters). This matches `docs/vdc-framework.md` §9.1 "Duff's Principle Generalized".

## Consequences

- `print -p myserver 'query'` → returns an `Int` tag (a list of one element, `$#tag` is 1).
- Pipelined out-of-order reads: `let t1 = print -p db 'slow'; let t2 = print -p db 'fast'; read -p db -t $t2 fast; read -p db -t $t1 slow`.
- Named coprocesses: `server |& myserver` starts a named coprocess; multiple simultaneous coprocesses supported.
- Lifecycle: reaped on scope exit (subshell close, function return) or explicit close. Rust's `Drop` on `Coproc` handles cleanup.
- No `par` dependency. ~40 lines of phantom session types in the Rust implementation; compiler enforces session discipline at build time.
- Error responses produce nonzero status on `read -p` with the error message bound to the reply variable. Standard ⊕ error handling applies.

Spec: `docs/specification.md` §"Coprocesses (9P-shaped discipline)" — authoritative and complete. Framework: `docs/vdc-framework.md` §4-5 (VDC / horizontal arrows as typed channels), §9.1 (Duff's principle generalized), §9.2 (horizontal arrow discipline). Ground: Carbone-Marin-Schürmann (forwarders); Plan 9 9P (manual section 5).
