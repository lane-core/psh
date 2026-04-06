//! Runtime values for psh.
//!
//! psh has one data type: the list of strings. A scalar is a
//! one-element list. The empty list is false; everything else
//! is true.

use std::fmt;

/// A psh value — a list of strings.
///
/// This is rc's value model. No maps, no typed values, no
/// structured data. Structured data lives in the namespace,
/// not in the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Val(pub Vec<String>);

impl Val {
    /// The empty list — false.
    pub fn empty() -> Self {
        Val(Vec::new())
    }

    /// A single-element list from a string.
    pub fn scalar(s: impl Into<String>) -> Self {
        Val(vec![s.into()])
    }

    /// A list from multiple strings.
    pub fn list(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Val(items.into_iter().map(Into::into).collect())
    }

    /// Is this value true? Non-empty = true.
    pub fn is_true(&self) -> bool {
        !self.0.is_empty()
    }

    /// Number of elements.
    pub fn count(&self) -> usize {
        self.0.len()
    }

    /// Index (1-based, rc convention). Returns empty on out-of-bounds.
    pub fn index(&self, i: usize) -> Val {
        match self.0.get(i.wrapping_sub(1)) {
            Some(s) => Val::scalar(s.clone()),
            None => Val::empty(),
        }
    }

    /// As a single string (first element, or empty).
    /// Used when a scalar is expected.
    pub fn as_str(&self) -> &str {
        self.0.first().map(|s| s.as_str()).unwrap_or("")
    }

    /// Concatenate with another value (rc's ^ operator).
    ///
    /// rc's rule: if both operands are lists of the same non-zero
    /// length, they are concatenated pairwise. If one operand is
    /// a single string, it is concatenated with each member of
    /// the other. Any other combination is an error (returns empty).
    pub fn concat(&self, other: &Val) -> Val {
        if self.0.is_empty() || other.0.is_empty() {
            return Val::empty();
        }
        if self.0.len() == other.0.len() {
            // Pairwise: (a b)^(1 2) = (a1 b2)
            let result = self.0.iter().zip(&other.0)
                .map(|(a, b)| format!("{a}{b}"))
                .collect();
            Val(result)
        } else if self.0.len() == 1 {
            // Broadcast left: x^(1 2) = (x1 x2)
            let a = &self.0[0];
            Val(other.0.iter().map(|b| format!("{a}{b}")).collect())
        } else if other.0.len() == 1 {
            // Broadcast right: (a b)^x = (ax bx)
            let b = &other.0[0];
            Val(self.0.iter().map(|a| format!("{a}{b}")).collect())
        } else {
            // Mismatched non-singleton lists — error in rc.
            // Return empty; the interpreter should report the error.
            Val::empty()
        }
    }

    /// Flatten to a single string with spaces (for command
    /// expansion where a list becomes arguments).
    pub fn to_args(&self) -> Vec<&str> {
        self.0.iter().map(|s| s.as_str()).collect()
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for s in &self.0 {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "{s}")?;
            first = false;
        }
        Ok(())
    }
}

impl From<String> for Val {
    fn from(s: String) -> Self {
        Val::scalar(s)
    }
}

impl From<&str> for Val {
    fn from(s: &str) -> Self {
        Val::scalar(s)
    }
}

impl From<Vec<String>> for Val {
    fn from(v: Vec<String>) -> Self {
        Val(v)
    }
}

impl From<bool> for Val {
    fn from(b: bool) -> Self {
        if b { Val::scalar("") } else { Val::empty() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_false() {
        assert!(!Val::empty().is_true());
    }

    #[test]
    fn scalar_is_true() {
        assert!(Val::scalar("hello").is_true());
    }

    #[test]
    fn index_1_based() {
        let v = Val::list(["a", "b", "c"]);
        assert_eq!(v.index(1), Val::scalar("a"));
        assert_eq!(v.index(2), Val::scalar("b"));
        assert_eq!(v.index(3), Val::scalar("c"));
        assert_eq!(v.index(0), Val::empty());
        assert_eq!(v.index(4), Val::empty());
    }

    #[test]
    fn count() {
        assert_eq!(Val::empty().count(), 0);
        assert_eq!(Val::scalar("x").count(), 1);
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
        assert_eq!(a.concat(&b), Val::list(["prefix-a", "prefix-b", "prefix-c"]));
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
        assert_eq!(a.concat(&b), Val::empty());
    }

    #[test]
    fn concat_empty_is_empty() {
        let a = Val::list(["x", "y"]);
        assert_eq!(a.concat(&Val::empty()), Val::empty());
        assert_eq!(Val::empty().concat(&a), Val::empty());
    }
}
