//! Runtime values for psh.
//!
//! psh's value model: a typed enum extending rc's list-of-strings
//! heritage with discriminated types. Scalars are Unit, Bool, Int,
//! Str, or Path. Lists are heterogeneous Vec<Val>.
//!
//! rc heritage: lists are first-class, concat is pairwise/broadcast,
//! truth is non-emptiness. The typed model adds inference in let
//! contexts while preserving rc's string-valued identity for bare
//! assignments.

use std::{fmt, path::PathBuf};

/// A psh value — typed, with rc-heritage list semantics.
///
/// Scalars: Unit (empty/false), Bool, Int, Str, Path.
/// Compound: List (heterogeneous, ordered).
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
    /// Heterogeneous list.
    List(Vec<Val>),
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
    pub fn is_true(&self) -> bool {
        match self {
            Val::Unit => false,
            Val::Bool(b) => *b,
            Val::Int(n) => *n != 0,
            Val::Str(s) => !s.is_empty(),
            Val::Path(p) => !p.as_os_str().is_empty(),
            Val::List(v) => !v.is_empty(),
        }
    }

    /// Number of elements. Scalars return 1 (Unit returns 0).
    pub fn count(&self) -> usize {
        match self {
            Val::Unit => 0,
            Val::List(v) => v.len(),
            _ => 1,
        }
    }

    /// Index (1-based, rc convention). Returns Unit on out-of-bounds.
    /// Scalars self-index at 1.
    pub fn index(&self, i: usize) -> Val {
        match self {
            Val::List(v) => match v.get(i.wrapping_sub(1)) {
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

    /// As a single string (for contexts expecting a scalar).
    pub fn as_str(&self) -> &str {
        match self {
            Val::Unit => "",
            Val::Bool(true) => "true",
            Val::Bool(false) => "false",
            Val::Str(s) => s.as_str(),
            // Int and Path can't return &str — use to_string()
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
            Val::List(v) => v.iter().map(|val| val.to_string()).collect(),
            other => vec![other.to_string()],
        }
    }

    /// Type inference for let bindings.
    ///
    /// "true"/"false" → Bool, parseable as i64 (no leading zeros
    /// except "0") → Int, starts with /, ./, ../, ~/ → Path,
    /// everything else → Str.
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
            other => vec![other.to_string()],
        }
    }

    /// Iterate over elements — for List, yields each element;
    /// for scalars, yields self; for Unit, yields nothing.
    /// Used by for-loops and argument expansion.
    pub fn iter_elements(&self) -> ValIter<'_> {
        match self {
            Val::Unit => ValIter::Empty,
            Val::List(v) => ValIter::List(v.iter()),
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
            Val::List(v) => {
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

    #[test]
    fn empty_is_false() {
        assert!(!Val::empty().is_true());
        assert!(!Val::Unit.is_true());
    }

    #[test]
    fn scalar_is_true() {
        assert!(Val::scalar("hello").is_true());
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
    fn index_1_based() {
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
    fn count() {
        assert_eq!(Val::empty().count(), 0);
        assert_eq!(Val::scalar("x").count(), 1);
        assert_eq!(Val::Int(42).count(), 1);
        assert_eq!(Val::list(["a", "b", "c"]).count(), 3);
    }

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
        // Int concat Str → Str
        assert_eq!(
            Val::Int(42).concat(&Val::Str("px".into())),
            Val::Str("42px".into())
        );
    }

    // ── Inference tests ──────────────────────────────────────

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
    fn display_val() {
        assert_eq!(Val::Unit.to_string(), "");
        assert_eq!(Val::Bool(true).to_string(), "true");
        assert_eq!(Val::Int(42).to_string(), "42");
        assert_eq!(Val::Str("hello".into()).to_string(), "hello");
        assert_eq!(Val::Path(PathBuf::from("/tmp")).to_string(), "/tmp");
        assert_eq!(Val::list(["a", "b"]).to_string(), "a b");
    }

    #[test]
    fn to_args_flattens() {
        assert_eq!(Val::Unit.to_args(), Vec::<String>::new());
        assert_eq!(Val::Int(42).to_args(), vec!["42"]);
        assert_eq!(Val::list(["a", "b"]).to_args(), vec!["a", "b"]);
    }
}
