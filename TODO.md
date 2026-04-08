# TODO

Short-term implementation tasks. See `PLAN.md` for the roadmap.

## Parser: Tuple literal syntax `(a, b)`

**Status:** Runtime supports `Val::Tuple`, parser lacks construction syntax.

**Grammar target:**
```
tuple_lit = '(' value (',' value)+ ')'
```

**Implementation notes:**
- Current `(a b c)` parses as `Value::List` 
- Need to detect comma vs space as separator
- Type inference: `(42, 7)` → `Tuple(Int, Int)`
- Comma is not in `word_char`, so it terminates word parsing naturally
- Add to `value_()` parser in `parse.rs` before `Value::List` attempt

## Parser: Sum construction syntax `tag[payload]`

**Status:** Runtime supports `Val::Sum`, parser lacks construction syntax.

**Grammar target:**
```
tagged_val = NAME '[' value ']'
```

**Design rationale:**
- `[]` visually distinct from function calls (space) and grouping (`()`)
- Mirrors type syntax: `Result[T]` type, `ok[v]` value
- Unambiguous commit: `NAME '['` is always Sum construction
- Payload is standard value position: primitives, lists, tuples, nested Sums

**Examples:**
```psh
let result : Result[Int] = ok[42]              # Sum("ok", Int(42))
let err : Result[Int] = err["not found"]       # Sum("err", Str(...))
let event = KeyEvent[(97, 0)]                  # Sum("KeyEvent", Tuple(...))
let opt : Maybe[Int] = none[()]                # Sum("none", Unit)
match $x { ok[v] => echo $v; err[e] => ... }   # Patterns mirror construction
```

**Parsing notes:**
- `[` is not in `word_char` or `var_char` — clean token boundary
- Payload parsed by `value_()`: handles literals, lists, lambdas, nested Sums
- Type annotation validates at binding site (Prism check)

## See also

- `PLAN.md` — roadmap and current state
- `docs/syntax.md` — full grammar specification
- `src/parse.rs` — parser implementation (`value_()` function)
