# Coprocesses

## Coprocesses (9P-shaped discipline)

### Design lineage

ksh93 introduced coprocesses (`cmd |&`) — bidirectional channels
between the shell and a child process. These were untyped byte
streams with no protocol discipline. Bash extended them with
named coprocesses. Neither had a conversation discipline.

Plan 9's 9P protocol [9P] is the design inspiration: a
session protocol imposed on a byte stream. The sequence
Tversion/Rversion, Tattach/Rattach, Twalk, Topen, Tread/Twrite,
Tclunk is a state machine — what Honda [HVK98] would later
formalize as session types. 9P session-typed its IPC informally,
enforced by runtime checks rather than compile-time types.

psh extracts 9P's conversation shape (not its wire protocol):

1. **Negotiate** — one round-trip confirming both sides speak
   the same protocol. Uses the standard wire format with tag 0
   (reserved for negotiate). The shell sends a request frame
   with payload `"psh/1"` on tag 0; the coprocess responds on
   tag 0 with `"psh/1"` (accept) or an error frame (reject).
   Mismatch or error kills the coprocess channel. Session type:
   `Send<Version, Recv<VersionAck, S>>` where S transitions to
   the per-tag multiplexed protocol. The negotiate step exists
   so that the protocol is self-describing from the first byte
   — no out-of-band assumptions about the peer.
2. **Request-response pairs** — every request gets a response.
   No fire-and-forget. No ambiguity about whose turn it is.
3. **Error at any step** — failure is always a valid response,
   not a special case.
4. **Orderly teardown** — explicit close via a close frame,
   not just EOF/SIGPIPE. The close frame uses tag 0 (the
   negotiate tag, repurposed after negotiate completes) with
   payload `"close"`. The coprocess acknowledges with a
   response on tag 0 and then closes its end of the
   socketpair. Outstanding per-tag sessions are cancelled
   (Tflush sent for each, responses drained) before the close
   frame is sent. EOF without a preceding close frame is the
   crash fallback — the shell treats it as an unclean death,
   fails all outstanding tags with error status, and reaps the
   coprocess. Channel state machine: negotiate → active →
   draining (outstanding tags being flushed) → close-sent →
   close-acked → closed.

### Per-tag binary sessions

Tags multiplex independent binary sessions over one channel.
Each tag has the session type `Send<Req, Recv<Resp, End>>` —
exactly one legal action at each step. The tag is a session
identifier, not a reason to abandon session discipline.

**Cancellation** extends each per-tag session type with an
internal choice (⊕) on the shell side after Send: the shell
may either await the response or cancel. Cancellation uses
a Tflush frame **on the same tag** being cancelled, and the
coprocess acknowledges with an Rflush on that tag. The
extended per-tag session type is:

    Send<Req, (Recv<Resp, End> ⊕ Cancel<Recv<Flush_Ack, End>>)>

The shell chooses one branch per tag. This mirrors 9P's
Tflush/Rflush transaction [9P, §flush]. Cancellation is
strictly shell-initiated, preserving the asymmetric initiator
discipline. Users interact only with per-tag sessions via
`print -p` / `read -p` and never see Tflush directly.

This mirrors 9P's multiplexing: tags are transaction
identifiers (one per outstanding request, like 9P's uint16
tags), and each tag identifies an independent request-response
exchange. In 9P, fids are the stateful session-like entities
(with lifecycles: walk → open → read/write → clunk); tags
correlate requests to responses across concurrent fids. psh's
coprocess tags serve the same correlation role.

The tag space is uint16 (65535). The practical limit comes
from backpressure (socketpair buffer full = sender blocks),
not from an artificial constant. Design for the ceiling,
operate at the floor.

### The user-visible protocol

`print -p name 'request'` sends a request to the named coprocess
and returns a `ReplyTag` identifying the outstanding request.
`let` binds the tag directly, per the CBPV rule that `let`
accepts effectful computations:

    let tag = print -p myserver 'query'
    # tag is a ReplyTag — a list of one element, $#tag is 1

`read -p name reply` reads the oldest outstanding response
(FIFO order) into `reply`. `read -p name -t $tag reply` reads
the response for a specific tag. Tags are **affine resources**
in the linear zone of psh's type system (§Linear resources).
`ReplyTag` is a distinct type from `Int`: using a tag twice is
a type error (contraction failure on a linear resource);
dropping a tag without consuming it is permitted (affine
discard) — the runtime sends a Tflush frame to cancel the
outstanding request (§Shell-internal tracking). The FIFO
pattern (no tag capture, `read -p name reply`) is unaffected:
tags allocated without `let` binding are consumed in order by
the shell's internal tracking, and their affine obligation is
discharged by the read. Within a `linear` block, tags are
strictly linear — unconsumed tags at block exit are a type
error, not a silent Tflush.

Simple FIFO pattern (no tag capture):

    print -p myserver 'query1'
    print -p myserver 'query2'
    read -p myserver reply1     # response to query1
    read -p myserver reply2     # response to query2

Pipelined pattern with out-of-order reads:

    let t1 = print -p db 'slow_query'
    let t2 = print -p db 'fast_query'
    read -p db -t $t2 fast      # read fast response first
    read -p db -t $t1 slow      # then the slow one

Error responses (the coprocess returns an error frame) produce
a nonzero status on `read -p`, with the error message bound to
the reply variable. Standard ⊕ error handling applies: check
status, use `try`/`catch`, etc.

### Shell-internal tracking

The shell maintains, per coprocess, a set of outstanding tags
(tags that have been sent but not yet read). `print -p`
allocates the lowest available tag, records it as outstanding,
and returns it. `read -p` (without `-t`) pops the oldest
outstanding tag when its response arrives. `read -p -t N`
removes tag N specifically when its response is read. Stale or
invalid tags produce a nonzero status with a descriptive error.

Internally, the shell tracks each outstanding tag with a
handle parameterised by a phantom session-state type. Rust's
type system enforces at compile time that a handle can only be
consumed in its `AwaitingReply` state and only once — the
consume method moves `self` and returns a handle in the
`Consumed` state. Compile-time use-site affinity, not a
true linear type discipline (Rust disallows specialised `Drop`
impls per `E0366`, so drop-as-cancel is a runtime invariant
rather than a type-level guarantee).

When a handle is dropped without being consumed (the tag's
response is never read), the shell sends a Tflush frame on
that tag, telling the coprocess to discard any pending work.
The tag then enters a **draining state**: it is still
outstanding from the shell's perspective, but no user code
owns it, and it is not available for reallocation. The tag
leaves the outstanding set only when the coprocess
acknowledges with an Rflush response on the same tag —
9P-style Tflush/Rflush pairing. Rresponse
frames for a tag in draining state are discarded silently:
they are the expected residual of a cancel race, not a
protocol violation.

Tag reuse is therefore gated on **session termination**, not
on cancel dispatch. The sequence `allocated → sent → (response
received | Tflush sent → Rflush received) → freed` is the
only state machine the shell maintains per tag, and the `End`
of the per-tag session corresponds to the free step. The
handle discipline is implementation detail — users see only
tag integers.

### Implementation

~40 lines of phantom session types:

    trait Session: Send + 'static {
        type Dual: Session<Dual = Self>;
    }
    impl Session for () { type Dual = (); }
    struct Send<T, S: Session = ()>(PhantomData<(T, S)>);
    struct Recv<T, S: Session = ()>(PhantomData<(T, S)>);
    // HasDual derived from Session::Dual

No par dependency. The session types live in the Rust
implementation's type signatures — verified by the compiler
when the builtins are written.

### Wire format

Length-prefixed frames in the 9P style [9P] — length and tag
headers, but without 9P's separate Tcode byte. Frame kind is
recovered from the first payload byte on the receiver side:

    request    = length[4 bytes, LE u32] tag[2 bytes, LE u16] payload[length - 2 bytes]
    response   = length[4 bytes, LE u32] tag[2 bytes, LE u16] payload[length - 2 bytes]
    error      = length[4 bytes, LE u32] tag[2 bytes, LE u16] '!' error_message
    tflush     = length[4 bytes, LE u32] tag[2 bytes, LE u16] '#'                         (length = 3)
    rflush     = length[4 bytes, LE u32] tag[2 bytes, LE u16] '#'                         (length = 3)

`'!'` marks an error response; `'#'` marks a flush transaction
(Tflush from shell, Rflush back from coprocess). All other
first-byte values are ordinary request/response payloads.
Length-prefixed rather than newline-delimited because payloads
may contain newlines (multi-line strings, command output,
heredocs). The tag is binary u16 for efficiency; the payload
is UTF-8 text (Display/FromStr). An error frame with an empty
`error_message` is a protocol violation; the shell tears down
the session on receipt.

**MAX_FRAME_SIZE** is 16 MiB. Any frame whose length prefix
exceeds this is a protocol violation: the channel is torn
down, outstanding tags fail with error status, and the
coprocess is killed. This is a defensive constant to bound
memory use against buggy or hostile peers — not a semantic
limit on legitimate payloads.

**Reserved first-byte values.** `'!'` (0x21) marks error
responses; `'#'` (0x23) marks flush transactions. Normal
request/response payloads must not begin with these bytes.
If a payload naturally starts with `!` or `#`, the sender
must prefix it with a NUL byte (0x00) as an escape; the
receiver strips a leading NUL. Direction (Tflush vs Rflush)
is determined by which side of the socketpair the frame
arrives on — the shell writes to its end and reads from the
coprocess's end.

**Tag 0 is reserved** for the negotiate and close protocols.
User-visible tags start at 1.

### Named coprocesses

Coprocesses are named. The shell holds a `HashMap<String,
Coproc>` — each coprocess has a name, its own socketpair, its
own independent tag space, and its own binary sessions.

    server |& myserver           # start named coprocess
    print -p myserver 'query'    # write to myserver
    read -p myserver reply       # read from myserver

    worker |& bg                 # another coprocess
    print -p bg 'task'           # independent channel

Anonymous `cmd |&` (no name) targets a default coprocess.
`print -p` / `read -p` without a name target the default.
This preserves ksh93 compatibility for simple cases while
enabling multiple simultaneous coprocesses.

**Lifecycle.** Named coprocesses are reaped on scope exit
(subshell close, function return) or explicit close. A dead
coprocess's name becomes available again — no zombie entries.
Rust's `Drop` on `Coproc` handles cleanup.

**Topology.** The shell is the hub. No coprocess-to-coprocess
communication — star topology. Each coprocess talks only to
the shell. Deadlock freedom follows from a simple per-channel
argument: each shell-to-coprocess channel is an independent
binary session with asymmetric initiative (shell always sends
first), so deadlock freedom is immediate by duality per
channel, and cross-channel deadlock is impossible because no
coprocess blocks on another coprocess.

Carbone, Marin, and Schürmann's forwarder logic [CMS] provides
the generalization path: their **MCutF admissibility theorem**
(§6) proves that multiparty compatible compositions can be
mediated by a forwarder. The current design does not use CMS
directly — the shell initiates and consumes, it does not
forward between coprocesses — but if psh ever adds
coprocess-to-coprocess routing, CMS provides the theoretical
foundation for deadlock freedom of the mediated composition.

Within this frame, psh restricts itself further: the shell
always initiates and the coprocess always responds on each
per-tag binary session `Send<Req, Recv<Resp, End>>`. This
asymmetric discipline makes each per-tag interaction duality-
safe (no interleaved cycles, no crossed initiative), so two-
party deadlock freedom per tag is immediate and multiparty
safety reduces to the forwarder correctness of the shell
itself.

