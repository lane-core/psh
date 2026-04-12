psh — a pleasant shell
======================

psh is a system shell descended from Plan 9 rc with typed values,
pattern matching, bidirectional type checking, and structured
coprocess IPC. It runs as a login shell on any Unix without
additional infrastructure.

```
x = 42
for(f in *.txt) => wc -l $f

let config = {'host': 'localhost', 'port': '8080'}
echo "connecting to $config['host']"

let result = fetch $url
echo $result.ok ?? 'request failed'

match($result) {
    ok(body) => process $body;
    err(msg) => echo "error: $msg"
}
```

Status
------

**Design complete, implementation not yet started.** The
specification and grammar are at `docs/specification.md` and
`docs/syntax.md`. The source tree is a stub.

Requirements
------------

- Rust 1.70+
- A Unix-like operating system (Linux, macOS, *BSD)

Heritage
--------

rc (Duff 1990) for the value model, quoting, and free carets.
ksh93 for discipline functions, coprocesses, and bracket
subscripts. The type theory draws on the λμμ̃-calculus, duploid
semantics, profunctor optics, and session types.

License
-------

See LICENSE.
