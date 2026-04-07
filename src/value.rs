//! Runtime values for psh.
//!
//! psh's value model: a typed enum extending rc's list-of-strings
//! heritage with discriminated types. Seven atoms (Unit, Bool, Int,
//! Str, Path, ExitCode, List), one product constructor (Tuple),
//! one coproduct constructor (Sum), one thunk constructor (Thunk).
//! Products give users Lenses. Coproducts give users Prisms.
//! Thunks are optic leaves (atomic, like ExitCode).
//!
//! rc heritage: lists are first-class, concat is pairwise/broadcast,
//! truth is non-emptiness. The typed model adds inference in let
//! contexts while preserving rc's string-valued identity for bare
//! assignments.

use std::{fmt, path::PathBuf};

use crate::ast::Command;

/// A psh value — typed, with rc-heritage list semantics.
///
/// Seven atoms: Unit (empty/false), Bool, Int, Str, Path,
/// ExitCode (reified computation outcome), List.
/// Three constructors: Tuple (product, Lens), Sum (coproduct, Prism),
/// Thunk (suspended computation, CBPV's U(A → F(B))).
///
/// rc compatibility: bare assignment (`x = val`) stays Str-valued;
/// `let` bindings run type inference via `Val::infer`.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    /// The empty value — false, zero-length, displays as "".
    Unit,
    /// Boolean value.
    Bool(bool),
    /// 64-bit signed integer.
    Int(i64),
    /// String value (the rc default).
    Str(String),
    /// Filesystem path.
    Path(PathBuf),
    /// Reified computation outcome (i32). Distinct from Int —
    /// ExitCode(0) is true (success), not false (zero).
    /// Enters Val only through `try`. Never produced by infer().
    ExitCode(i32),
    /// Heterogeneous list.
    List(Vec<Val>),
    /// Product type — comma-separated construction: `(42, 7)`.
    /// Always ≥2 elements by construction. 0-based indexing.
    Tuple(Vec<Val>),
    /// Coproduct type — tag + payload. Construction: `tag payload`.
    /// Display shows payload only (tag stripped).
    Sum(String, Box<Val>),
    /// Suspended computation — CBPV's U(A → F(B)). Named params,
    /// capture-by-value for free variables. At lambda construction
    /// time, free $var references (minus params) are snapshotted
    /// from the current scope. At force time, captures are restored
    /// into the scope before running the body. Named functions
    /// (`fn name { body }`) do NOT capture — they use dynamic
    /// resolution with positional params. Capture is lambda-only.
    /// PartialEq is structural (same params + same body + same
    /// captures = equal).
    Thunk {
        params: Vec<String>,
        body: Vec<Command>,
        captures: Vec<(String, Val)>,
    },
}

impl Val {
    /// The empty value — false.
    pub fn empty() -> Self {
        Val::Unit
    }

    /// A single string value (migration helper — rc heritage).
    pub fn scalar(s: impl Into<String>) -> Self {
        Val::Str(s.into())
    }

    /// A list of string values (migration helper — rc heritage).
    pub fn list(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let v: Vec<Val> = items.into_iter().map(|s| Val::Str(s.into())).collect();
        if v.is_empty() {
            Val::Unit
        } else {
            Val::List(v)
        }
    }

    /// Is this value true? Non-empty and non-zero/non-false = true.
    ///
    /// ExitCode truthiness inverts Int: ExitCode(0) = true (success),
    /// ExitCode(n≠0) = false (failure). These are different sorts —
    /// ExitCode is a reified computation outcome, Int is data.
    ///
    /// Tuple is always true (≥2 elements by construction).
    /// Sum is always true (has tag + payload).
    pub fn is_true(&self) -> bool {
        match self {
            Val::Unit => false,
            Val::Bool(b) => *b,
            Val::Int(n) => *n != 0,
            Val::Str(s) => !s.is_empty(),
            Val::Path(p) => !p.as_os_str().is_empty(),
            Val::ExitCode(code) => *code == 0,
            Val::List(v) => !v.is_empty(),
            Val::Tuple(_) => true,
            Val::Sum(_, _) => true,
            Val::Thunk { .. } => true,
        }
    }

    /// Number of elements. Scalars return 1 (Unit returns 0).
    /// Tuple returns its element count. Sum returns 1 (one tagged value).
    pub fn count(&self) -> usize {
        match self {
            Val::Unit => 0,
            Val::List(v) => v.len(),
            Val::Tuple(v) => v.len(),
            _ => 1,
        }
    }

    /// Index (1-based, rc convention). Returns Unit on out-of-bounds.
    /// Scalars self-index at 1. Tuple uses 0-based indexing — use
    /// `tuple_index` for structural projection.
    pub fn index(&self, i: usize) -> Val {
        match self {
            Val::List(v) => match v.get(i.wrapping_sub(1)) {
                Some(val) => val.clone(),
                None => Val::Unit,
            },
            // Tuple uses 1-based indexing in this method for consistency
            // with rc's $x(n) syntax. Accessor .0/.1 uses tuple_index().
            Val::Tuple(v) => match v.get(i.wrapping_sub(1)) {
                Some(val) => val.clone(),
                None => Val::Unit,
            },
            _ => {
                if i == 1 {
                    self.clone()
                } else {
                    Val::Unit
                }
            }
        }
    }

    /// 0-based tuple projection — structural access via .0, .1 etc.
    /// Returns Unit on out-of-bounds or non-Tuple values.
    pub fn tuple_index(&self, i: usize) -> Val {
        match self {
            Val::Tuple(v) => v.get(i).cloned().unwrap_or(Val::Unit),
            _ => Val::Unit,
        }
    }

    /// As a single string (for contexts expecting a scalar).
    pub fn as_str(&self) -> &str {
        match self {
            Val::Unit => "",
            Val::Bool(true) => "true",
            Val::Bool(false) => "false",
            Val::Str(s) => s.as_str(),
            // Int, Path, ExitCode, Tuple, Sum can't return &str — use to_string()
            _ => "",
        }
    }

    /// Concatenate with another value (rc's ^ operator).
    ///
    /// Both operands are coerced to Str via Display, then pairwise
    /// or broadcast concatenation applies. Result is always Str or
    /// List of Str.
    pub fn concat(&self, other: &Val) -> Val {
        let left = self.to_string_vec();
        let right = other.to_string_vec();

        if left.is_empty() || right.is_empty() {
            return Val::Unit;
        }
        if left.len() == right.len() {
            let result: Vec<Val> = left
                .iter()
                .zip(&right)
                .map(|(a, b)| Val::Str(format!("{a}{b}")))
                .collect();
            if result.len() == 1 {
                result.into_iter().next().unwrap()
            } else {
                Val::List(result)
            }
        } else if left.len() == 1 {
            let a = &left[0];
            let result: Vec<Val> = right.iter().map(|b| Val::Str(format!("{a}{b}"))).collect();
            if result.len() == 1 {
                result.into_iter().next().unwrap()
            } else {
                Val::List(result)
            }
        } else if right.len() == 1 {
            let b = &right[0];
            let result: Vec<Val> = left.iter().map(|a| Val::Str(format!("{a}{b}"))).collect();
            if result.len() == 1 {
                result.into_iter().next().unwrap()
            } else {
                Val::List(result)
            }
        } else {
            // Mismatched non-singleton lists — error in rc.
            Val::Unit
        }
    }

    /// Flatten to a Vec<String> for process argv.
    pub fn to_args(&self) -> Vec<String> {
        match self {
            Val::Unit => vec![],
            Val::List(v) | Val::Tuple(v) => v.iter().map(|val| val.to_string()).collect(),
            Val::Sum(_, payload) => vec![payload.to_string()],
            other => vec![other.to_string()],
        }
    }

    /// Type inference for let bindings.
    ///
    /// "true"/"false" → Bool, parseable as i64 (no leading zeros
    /// except "0") → Int, starts with /, ./, ../, ~/ → Path,
    /// everything else → Str.
    ///
    /// ExitCode is NEVER inferred — it enters Val only through `try`.
    /// Tuple and Sum are not literal-constructible via infer().
    pub fn infer(s: &str) -> Val {
        match s {
            "true" => return Val::Bool(true),
            "false" => return Val::Bool(false),
            "" => return Val::Str(String::new()),
            _ => {}
        }

        // Int: parseable as i64, no leading zeros except "0" itself
        if let Ok(n) = s.parse::<i64>() {
            // Reject leading zeros: "042" stays Str, "0" is Int,
            // "-0" is Int(0), negative numbers with leading zeros
            // like "-042" stay Str.
            let is_leading_zero = if let Some(rest) = s.strip_prefix('-') {
                rest.len() > 1 && rest.starts_with('0')
            } else {
                s.len() > 1 && s.starts_with('0')
            };
            if !is_leading_zero {
                return Val::Int(n);
            }
        }

        // Path: starts with /, ./, ../, ~/
        if s.starts_with('/') || s.starts_with("./") || s.starts_with("../") || s.starts_with("~/")
        {
            return Val::Path(PathBuf::from(s));
        }

        Val::Str(s.to_string())
    }

    /// Helper: flatten to a Vec of display strings for concat.
    fn to_string_vec(&self) -> Vec<String> {
        match self {
            Val::Unit => vec![],
            Val::List(v) => v.iter().map(|val| val.to_string()).collect(),
            Val::Tuple(v) => v.iter().map(|val| val.to_string()).collect(),
            other => vec![other.to_string()],
        }
    }

    /// Iterate over elements — for List, yields each element;
    /// for Tuple, yields each element; for Sum/Thunk, yields self;
    /// for scalars, yields self; for Unit, yields nothing.
    /// Used by for-loops and argument expansion.
    pub fn iter_elements(&self) -> ValIter<'_> {
        match self {
            Val::Unit => ValIter::Empty,
            Val::List(v) | Val::Tuple(v) => ValIter::List(v.iter()),
            _ => ValIter::Scalar(Some(self)),
        }
    }
}

/// Iterator over Val elements.
pub enum ValIter<'a> {
    Empty,
    Scalar(Option<&'a Val>),
    List(std::slice::Iter<'a, Val>),
}

impl<'a> Iterator for ValIter<'a> {
    type Item = &'a Val;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ValIter::Empty => None,
            ValIter::Scalar(opt) => opt.take(),
            ValIter::List(iter) => iter.next(),
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Unit => Ok(()),
            Val::Bool(b) => write!(f, "{b}"),
            Val::Int(n) => write!(f, "{n}"),
            Val::Str(s) => write!(f, "{s}"),
            Val::Path(p) => write!(f, "{}", p.display()),
            Val::ExitCode(code) => write!(f, "{code}"),
            Val::List(v) | Val::Tuple(v) => {
                let mut first = true;
                for val in v {
                    if !first {
                        write!(f, " ")?;
                    }
                    write!(f, "{val}")?;
                    first = false;
                }
                Ok(())
            }
            // Sum displays payload only — tag is control-flow metadata
            Val::Sum(_, payload) => write!(f, "{payload}"),
            // Thunk display is diagnostic, not round-trippable
            Val::Thunk { params, .. } => {
                write!(f, "fn({})", params.join(" "))?;
                write!(f, "{{...}}")
            }
        }
    }
}

impl From<String> for Val {
    fn from(s: String) -> Self {
        Val::Str(s)
    }
}

impl From<&str> for Val {
    fn from(s: &str) -> Self {
        Val::Str(s.to_string())
    }
}

impl From<Vec<String>> for Val {
    fn from(v: Vec<String>) -> Self {
        if v.is_empty() {
            Val::Unit
        } else {
            Val::List(v.into_iter().map(Val::Str).collect())
        }
    }
}

impl From<bool> for Val {
    fn from(b: bool) -> Self {
        Val::Bool(b)
    }
}

impl From<i64> for Val {
    fn from(n: i64) -> Self {
        Val::Int(n)
    }
}

impl From<PathBuf> for Val {
    fn from(p: PathBuf) -> Self {
        Val::Path(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Truthiness ──────────────────────────────────────────

    #[test]
    fn unit_is_false() {
        assert!(!Val::Unit.is_true());
        assert!(!Val::empty().is_true());
    }

    #[test]
    fn bool_truth() {
        assert!(Val::Bool(true).is_true());
        assert!(!Val::Bool(false).is_true());
    }

    #[test]
    fn int_truth() {
        assert!(Val::Int(42).is_true());
        assert!(Val::Int(-1).is_true());
        assert!(!Val::Int(0).is_true());
    }

    #[test]
    fn str_truth() {
        assert!(Val::scalar("hello").is_true());
        assert!(!Val::Str(String::new()).is_true());
    }

    #[test]
    fn path_truth() {
        assert!(Val::Path(PathBuf::from("/tmp")).is_true());
    }

    #[test]
    fn exit_code_truth_inverts_int() {
        // ExitCode(0) = success = true (opposite of Int(0))
        assert!(Val::ExitCode(0).is_true());
        // ExitCode(nonzero) = failure = false
        assert!(!Val::ExitCode(1).is_true());
        assert!(!Val::ExitCode(127).is_true());
        assert!(!Val::ExitCode(-1).is_true());
    }

    #[test]
    fn list_truth() {
        assert!(Val::list(["a"]).is_true());
        assert!(!Val::List(vec![]).is_true());
    }

    #[test]
    fn tuple_always_true() {
        // Tuples always have ≥2 elements by construction
        assert!(Val::Tuple(vec![Val::Int(0), Val::Int(0)]).is_true());
        assert!(Val::Tuple(vec![Val::Unit, Val::Unit]).is_true());
    }

    #[test]
    fn sum_always_true() {
        assert!(Val::Sum("ok".into(), Box::new(Val::Unit)).is_true());
        assert!(Val::Sum("err".into(), Box::new(Val::ExitCode(1))).is_true());
    }

    // ── Count ───────────────────────────────────────────────

    #[test]
    fn count_scalars() {
        assert_eq!(Val::Unit.count(), 0);
        assert_eq!(Val::Int(42).count(), 1);
        assert_eq!(Val::scalar("x").count(), 1);
        assert_eq!(Val::ExitCode(0).count(), 1);
        assert_eq!(Val::Bool(true).count(), 1);
    }

    #[test]
    fn count_list() {
        assert_eq!(Val::list(["a", "b", "c"]).count(), 3);
    }

    #[test]
    fn count_tuple() {
        assert_eq!(Val::Tuple(vec![Val::Int(1), Val::Int(2)]).count(), 2);
        assert_eq!(
            Val::Tuple(vec![Val::Int(1), Val::Int(2), Val::Int(3)]).count(),
            3
        );
    }

    #[test]
    fn count_sum() {
        assert_eq!(Val::Sum("ok".into(), Box::new(Val::Int(42))).count(), 1);
    }

    // ── Indexing ─────────────────────────────────────────────

    #[test]
    fn list_index_1_based() {
        let v = Val::list(["a", "b", "c"]);
        assert_eq!(v.index(1), Val::scalar("a"));
        assert_eq!(v.index(2), Val::scalar("b"));
        assert_eq!(v.index(3), Val::scalar("c"));
        assert_eq!(v.index(0), Val::Unit);
        assert_eq!(v.index(4), Val::Unit);
    }

    #[test]
    fn scalar_self_index() {
        let v = Val::Int(42);
        assert_eq!(v.index(1), Val::Int(42));
        assert_eq!(v.index(2), Val::Unit);
    }

    #[test]
    fn tuple_index_via_method() {
        let t = Val::Tuple(vec![Val::Int(42), Val::scalar("hello")]);
        // 0-based structural projection
        assert_eq!(t.tuple_index(0), Val::Int(42));
        assert_eq!(t.tuple_index(1), Val::scalar("hello"));
        assert_eq!(t.tuple_index(2), Val::Unit);
    }

    #[test]
    fn tuple_index_1_based_via_index() {
        // The general index() method uses 1-based for rc compat
        let t = Val::Tuple(vec![Val::Int(42), Val::scalar("hello")]);
        assert_eq!(t.index(1), Val::Int(42));
        assert_eq!(t.index(2), Val::scalar("hello"));
        assert_eq!(t.index(0), Val::Unit);
    }

    #[test]
    fn exit_code_self_index() {
        let v = Val::ExitCode(0);
        assert_eq!(v.index(1), Val::ExitCode(0));
        assert_eq!(v.index(2), Val::Unit);
    }

    #[test]
    fn sum_self_index() {
        let v = Val::Sum("ok".into(), Box::new(Val::Int(42)));
        assert_eq!(v.index(1), v);
        assert_eq!(v.index(2), Val::Unit);
    }

    // ── Concat ──────────────────────────────────────────────

    #[test]
    fn concat_pairwise() {
        let a = Val::list(["x", "y"]);
        let b = Val::list(["1", "2"]);
        assert_eq!(a.concat(&b), Val::list(["x1", "y2"]));
    }

    #[test]
    fn concat_broadcast_left() {
        let a = Val::scalar("prefix-");
        let b = Val::list(["a", "b", "c"]);
        assert_eq!(
            a.concat(&b),
            Val::list(["prefix-a", "prefix-b", "prefix-c"])
        );
    }

    #[test]
    fn concat_broadcast_right() {
        let a = Val::list(["a", "b", "c"]);
        let b = Val::scalar(".txt");
        assert_eq!(a.concat(&b), Val::list(["a.txt", "b.txt", "c.txt"]));
    }

    #[test]
    fn concat_mismatched_is_empty() {
        let a = Val::list(["x", "y"]);
        let b = Val::list(["1", "2", "3"]);
        assert_eq!(a.concat(&b), Val::Unit);
    }

    #[test]
    fn concat_empty_is_empty() {
        let a = Val::list(["x", "y"]);
        assert_eq!(a.concat(&Val::empty()), Val::Unit);
        assert_eq!(Val::empty().concat(&a), Val::Unit);
    }

    #[test]
    fn concat_typed_coercion() {
        assert_eq!(
            Val::Int(42).concat(&Val::Str("px".into())),
            Val::Str("42px".into())
        );
    }

    #[test]
    fn concat_exit_code_coerces() {
        assert_eq!(
            Val::ExitCode(1).concat(&Val::Str(" failed".into())),
            Val::Str("1 failed".into())
        );
    }

    #[test]
    fn concat_tuple_coerces() {
        // Tuple's to_string_vec flattens to individual elements,
        // so concat broadcasts the scalar across each element
        let t = Val::Tuple(vec![Val::Int(1), Val::Int(2)]);
        assert_eq!(
            t.concat(&Val::Str(" items".into())),
            Val::list(["1 items", "2 items"])
        );
    }

    #[test]
    fn concat_sum_coerces_payload_only() {
        let s = Val::Sum("ok".into(), Box::new(Val::Int(42)));
        assert_eq!(s.concat(&Val::Str("!".into())), Val::Str("42!".into()));
    }

    // ── Inference ───────────────────────────────────────────

    #[test]
    fn infer_int() {
        assert_eq!(Val::infer("42"), Val::Int(42));
        assert_eq!(Val::infer("0"), Val::Int(0));
        assert_eq!(Val::infer("-1"), Val::Int(-1));
    }

    #[test]
    fn infer_leading_zero_stays_str() {
        assert_eq!(Val::infer("042"), Val::Str("042".into()));
    }

    #[test]
    fn infer_bool() {
        assert_eq!(Val::infer("true"), Val::Bool(true));
        assert_eq!(Val::infer("false"), Val::Bool(false));
    }

    #[test]
    fn infer_path() {
        assert_eq!(Val::infer("/tmp"), Val::Path(PathBuf::from("/tmp")));
        assert_eq!(Val::infer("./foo"), Val::Path(PathBuf::from("./foo")));
        assert_eq!(Val::infer("../bar"), Val::Path(PathBuf::from("../bar")));
        assert_eq!(Val::infer("~/bin"), Val::Path(PathBuf::from("~/bin")));
    }

    #[test]
    fn infer_str() {
        assert_eq!(Val::infer("hello"), Val::Str("hello".into()));
        assert_eq!(Val::infer(""), Val::Str(String::new()));
    }

    #[test]
    fn infer_never_produces_exit_code() {
        // "0" infers to Int(0), never ExitCode(0)
        assert_eq!(Val::infer("0"), Val::Int(0));
        assert_eq!(Val::infer("1"), Val::Int(1));
        assert_eq!(Val::infer("127"), Val::Int(127));
    }

    // ── Display ─────────────────────────────────────────────

    #[test]
    fn display_atoms() {
        assert_eq!(Val::Unit.to_string(), "");
        assert_eq!(Val::Bool(true).to_string(), "true");
        assert_eq!(Val::Int(42).to_string(), "42");
        assert_eq!(Val::Str("hello".into()).to_string(), "hello");
        assert_eq!(Val::Path(PathBuf::from("/tmp")).to_string(), "/tmp");
        assert_eq!(Val::ExitCode(0).to_string(), "0");
        assert_eq!(Val::ExitCode(127).to_string(), "127");
    }

    #[test]
    fn display_list() {
        assert_eq!(Val::list(["a", "b"]).to_string(), "a b");
    }

    #[test]
    fn display_tuple() {
        let t = Val::Tuple(vec![Val::Int(42), Val::Int(7)]);
        assert_eq!(t.to_string(), "42 7");
    }

    #[test]
    fn display_sum_payload_only() {
        // Tag is stripped — payload only
        let s = Val::Sum("ok".into(), Box::new(Val::Int(42)));
        assert_eq!(s.to_string(), "42");
        let e = Val::Sum("err".into(), Box::new(Val::ExitCode(1)));
        assert_eq!(e.to_string(), "1");
    }

    // ── to_args ─────────────────────────────────────────────

    #[test]
    fn to_args_scalars() {
        assert_eq!(Val::Unit.to_args(), Vec::<String>::new());
        assert_eq!(Val::Int(42).to_args(), vec!["42"]);
        assert_eq!(Val::ExitCode(0).to_args(), vec!["0"]);
    }

    #[test]
    fn to_args_list() {
        assert_eq!(Val::list(["a", "b"]).to_args(), vec!["a", "b"]);
    }

    #[test]
    fn to_args_tuple() {
        let t = Val::Tuple(vec![Val::Int(1), Val::Int(2)]);
        assert_eq!(t.to_args(), vec!["1", "2"]);
    }

    #[test]
    fn to_args_sum() {
        let s = Val::Sum("ok".into(), Box::new(Val::Int(42)));
        assert_eq!(s.to_args(), vec!["42"]);
    }

    // ── iter_elements ───────────────────────────────────────

    #[test]
    fn iter_unit_is_empty() {
        assert_eq!(Val::Unit.iter_elements().count(), 0);
    }

    #[test]
    fn iter_scalar_yields_self() {
        let v = Val::Int(42);
        let elems: Vec<_> = v.iter_elements().collect();
        assert_eq!(elems, vec![&Val::Int(42)]);
    }

    #[test]
    fn iter_list_yields_elements() {
        let v = Val::List(vec![Val::Int(1), Val::Int(2)]);
        let elems: Vec<_> = v.iter_elements().collect();
        assert_eq!(elems, vec![&Val::Int(1), &Val::Int(2)]);
    }

    #[test]
    fn iter_tuple_yields_elements() {
        let v = Val::Tuple(vec![Val::Int(1), Val::Int(2)]);
        let elems: Vec<_> = v.iter_elements().collect();
        assert_eq!(elems, vec![&Val::Int(1), &Val::Int(2)]);
    }

    #[test]
    fn iter_sum_yields_self() {
        let v = Val::Sum("ok".into(), Box::new(Val::Int(42)));
        let elems: Vec<_> = v.iter_elements().collect();
        assert_eq!(elems.len(), 1);
        assert_eq!(elems[0], &v);
    }

    #[test]
    fn iter_exit_code_yields_self() {
        let v = Val::ExitCode(0);
        let elems: Vec<_> = v.iter_elements().collect();
        assert_eq!(elems, vec![&Val::ExitCode(0)]);
    }

    // ── From impls ──────────────────────────────────────────

    #[test]
    fn from_string() {
        let v: Val = "hello".to_string().into();
        assert_eq!(v, Val::Str("hello".into()));
    }

    #[test]
    fn from_str_ref() {
        let v: Val = "hello".into();
        assert_eq!(v, Val::Str("hello".into()));
    }

    #[test]
    fn from_bool() {
        let v: Val = true.into();
        assert_eq!(v, Val::Bool(true));
    }

    #[test]
    fn from_i64() {
        let v: Val = 42i64.into();
        assert_eq!(v, Val::Int(42));
    }

    #[test]
    fn from_pathbuf() {
        let v: Val = PathBuf::from("/tmp").into();
        assert_eq!(v, Val::Path(PathBuf::from("/tmp")));
    }

    #[test]
    fn from_vec_string() {
        let v: Val = vec!["a".to_string(), "b".to_string()].into();
        assert_eq!(v, Val::list(["a", "b"]));

        let empty: Val = Vec::<String>::new().into();
        assert_eq!(empty, Val::Unit);
    }

    // ── as_str ──────────────────────────────────────────────

    #[test]
    fn as_str_returns_reference() {
        assert_eq!(Val::Unit.as_str(), "");
        assert_eq!(Val::Bool(true).as_str(), "true");
        assert_eq!(Val::Bool(false).as_str(), "false");
        assert_eq!(Val::scalar("hello").as_str(), "hello");
        // Types that can't return &str return ""
        assert_eq!(Val::Int(42).as_str(), "");
        assert_eq!(Val::ExitCode(0).as_str(), "");
        assert_eq!(Val::Tuple(vec![Val::Int(1), Val::Int(2)]).as_str(), "");
        assert_eq!(Val::Sum("ok".into(), Box::new(Val::Int(42))).as_str(), "");
    }

    // ── to_string_vec ───────────────────────────────────────

    #[test]
    fn to_string_vec_via_concat() {
        // Verify via concat behavior that to_string_vec works for new types
        let ec = Val::ExitCode(42);
        assert_eq!(ec.concat(&Val::scalar("!")), Val::Str("42!".into()));

        let tuple = Val::Tuple(vec![Val::scalar("a"), Val::scalar("b")]);
        // Tuple flattens to multiple strings in to_string_vec
        assert_eq!(tuple.concat(&Val::scalar("!")), Val::list(["a!", "b!"]));
    }
}
