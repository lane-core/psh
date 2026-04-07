//! Variable environment for psh.
//!
//! Variables are lists of strings (rc heritage). Each variable
//! can optionally have discipline functions (ksh93 heritage) —
//! .get fires on read, .set fires on write. Discipline functions
//! are the MonadicLens at the shell level: view (co-Kleisli,
//! pure observation) and set (Kleisli, effectful mutation).

use std::collections::HashMap;

use crate::{ast::TypeAnnotation, value::Val};

/// A discipline function pair — the shell-level MonadicLens.
///
/// The function names follow ksh93 convention: `varname.get`
/// and `varname.set`. These are stored as function names to be
/// looked up and executed by the interpreter, not as closures.
#[derive(Debug, Clone, PartialEq)]
pub struct Discipline {
    /// Function name to call on read (e.g., "x.get").
    /// If None, reads return the stored value directly.
    pub get: Option<String>,
    /// Function name to call on write (e.g., "x.set").
    /// If None, writes store the value directly.
    pub set: Option<String>,
}

/// A single variable binding.
#[derive(Debug, Clone)]
pub struct Var {
    /// The stored value.
    pub value: Val,
    /// Stderr from last failed try evaluation. For stored variables
    /// (tiers 1-2), always None. For computed variables (try {}),
    /// preserves the human-readable error string alongside the
    /// ExitCode for diagnostic use.
    pub error: Option<String>,
    /// Whether this variable is exported to child processes.
    pub exported: bool,
    /// Whether this variable is read-only.
    pub readonly: bool,
    /// Whether this variable can be reassigned.
    /// true for `x = val` (rc heritage) and `let mut`; false for plain `let`.
    pub mutable: bool,
    /// Type annotation from `let x : Type = val`. When present,
    /// reassignment validates against this type (Prism check).
    pub type_ann: Option<TypeAnnotation>,
    /// Discipline functions, if any.
    pub discipline: Option<Discipline>,
    /// ksh93 heritage: nameref target. When set, all accesses
    /// resolve through the named target variable instead.
    pub nameref: Option<String>,
}

impl Var {
    pub fn new(value: Val) -> Self {
        Var {
            value,
            error: None,
            exported: false,
            readonly: false,
            mutable: true,
            type_ann: None,
            discipline: None,
            nameref: None,
        }
    }

    pub fn exported(value: Val) -> Self {
        Var {
            value,
            error: None,
            exported: true,
            readonly: false,
            mutable: true,
            type_ann: None,
            discipline: None,
            nameref: None,
        }
    }

    /// Create a nameref variable pointing to `target`.
    pub fn nameref(target: String) -> Self {
        Var {
            value: Val::empty(),
            error: None,
            exported: false,
            readonly: false,
            mutable: true,
            type_ann: None,
            discipline: None,
            nameref: Some(target),
        }
    }
}

/// A scope in the environment stack.
///
/// Variable resolution walks the stack from top (innermost scope)
/// to bottom (global). This is AffineFold composition — each
/// scope may or may not contain the binding.
#[derive(Debug)]
pub struct Scope {
    vars: HashMap<String, Var>,
    /// When true, mutations through this scope are rejected.
    /// Used by .get discipline bodies to enforce purity.
    pub readonly: bool,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            vars: HashMap::new(),
            readonly: false,
        }
    }

    /// Create a readonly scope (.get discipline purity enforcement).
    pub fn readonly() -> Self {
        Scope {
            vars: HashMap::new(),
            readonly: true,
        }
    }

    pub fn get(&self, name: &str) -> Option<&Var> {
        self.vars.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Var> {
        self.vars.get_mut(name)
    }

    pub fn set(&mut self, name: String, var: Var) {
        self.vars.insert(name, var);
    }

    pub fn remove(&mut self, name: &str) -> Option<Var> {
        self.vars.remove(name)
    }

    /// Iterate all exported variables in this scope.
    pub fn exported(&self) -> impl Iterator<Item = (&str, &Val)> {
        self.vars
            .iter()
            .filter(|(_, v)| v.exported)
            .map(|(k, v)| (k.as_str(), &v.value))
    }
}

/// The full variable environment — a stack of scopes.
///
/// Global scope is at index 0. Each function call pushes a
/// new scope. Variable resolution walks from top to bottom.
#[derive(Debug)]
pub struct Env {
    scopes: Vec<Scope>,
    /// Function definitions (including discipline functions).
    /// Separate from variable scopes — functions are global
    /// in rc, and we follow that convention.
    functions: HashMap<String, Vec<crate::ast::Command>>,
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    pub fn new() -> Self {
        let mut global = Scope::new();

        // Seed with $pid and $status
        global.set(
            "pid".into(),
            Var::new(Val::scalar(std::process::id().to_string())),
        );
        global.set("status".into(), Var::new(Val::scalar("")));

        Env {
            scopes: vec![global],
            functions: HashMap::new(),
        }
    }

    /// Import the process environment as exported variables.
    ///
    /// Also sets rc-style lowercase aliases for common vars:
    /// $home = $HOME, $path = $PATH (as list split on :),
    /// $user = $USER.
    pub fn import_process_env(&mut self) {
        for (key, value) in std::env::vars() {
            self.scopes[0].set(key.clone(), Var::exported(Val::scalar(value.clone())));

            // rc-style lowercase aliases
            match key.as_str() {
                "HOME" => {
                    self.scopes[0].set("home".into(), Var::new(Val::scalar(value)));
                }
                "PATH" => {
                    // rc convention: $path is a list, not colon-separated
                    let dirs: Vec<String> = value.split(':').map(String::from).collect();
                    self.scopes[0].set("path".into(), Var::new(Val::from(dirs)));
                }
                "USER" => {
                    self.scopes[0].set("user".into(), Var::new(Val::scalar(value)));
                }
                _ => {}
            }
        }
    }

    /// Push a new scope (function call).
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Push a readonly scope (.get discipline body).
    pub fn push_readonly_scope(&mut self) {
        self.scopes.push(Scope::readonly());
    }

    /// Pop the innermost scope (function return).
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Whether the current (topmost) scope is readonly.
    pub fn is_readonly(&self) -> bool {
        self.scopes.last().is_some_and(|s| s.readonly)
    }

    /// Look up a variable by name. Walks scopes from top to bottom.
    pub fn get(&self, name: &str) -> Option<&Var> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var);
            }
        }
        None
    }

    /// Get the value of a variable, or empty if not found.
    /// Follows namerefs up to 8 levels deep (ksh93 convention).
    pub fn get_value(&self, name: &str) -> Val {
        let resolved = self.resolve_nameref(name);
        self.get(resolved)
            .map(|v| v.value.clone())
            .unwrap_or_else(Val::empty)
    }

    /// Set a variable. If it exists in any scope, updates in place.
    /// Otherwise creates in the current (topmost) scope.
    /// Follows namerefs: if `name` is a nameref, the target is set instead.
    ///
    /// Returns Ok(()) on success, Err(message) if the variable is readonly,
    /// immutable, or the value fails type validation.
    pub fn set_value(&mut self, name: &str, value: Val) -> Result<(), String> {
        // If the topmost scope is readonly, reject all mutations
        if self.is_readonly() {
            return Err(format!("{name}: readonly scope"));
        }

        // Resolve nameref chain before mutating
        let resolved = self.resolve_nameref(name).to_string();

        // Search existing scopes for the variable
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(&resolved) {
                if var.readonly {
                    return Err(format!("{resolved}: readonly variable"));
                }
                if !var.mutable {
                    return Err(format!("{resolved}: immutable variable (use let mut)"));
                }
                // Type-check if annotation present
                if let Some(ref ann) = var.type_ann {
                    let validated = validate_type(&value, ann)?;
                    var.value = validated;
                } else {
                    var.value = value;
                }
                return Ok(());
            }
        }
        // Not found — create in current scope
        let scope = self.scopes.last_mut().unwrap();
        scope.set(resolved, Var::new(value));
        Ok(())
    }

    /// Create a let-bound variable in the current (topmost) scope.
    /// Never walks up the scope chain — always local.
    pub fn let_value(
        &mut self,
        name: &str,
        value: Val,
        mutable: bool,
        exported: bool,
        type_ann: Option<TypeAnnotation>,
    ) -> Result<(), String> {
        let validated = if let Some(ref ann) = type_ann {
            validate_type(&value, ann)?
        } else {
            value
        };

        let var = Var {
            value: validated,
            error: None,
            exported,
            readonly: false,
            mutable,
            type_ann,
            discipline: None,
            nameref: None,
        };
        let scope = self.scopes.last_mut().unwrap();
        scope.set(name.to_string(), var);
        Ok(())
    }

    /// Get a mutable reference to a variable (for tests and internal use).
    pub fn get_mut_var(&mut self, name: &str) -> Option<&mut Var> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                return Some(var);
            }
        }
        None
    }

    /// Resolve a nameref chain, returning the final target name.
    /// Limits recursion to 8 levels (ksh93 convention) to prevent
    /// infinite loops from circular namerefs.
    fn resolve_nameref<'b>(&'b self, name: &'b str) -> &'b str {
        let mut current = name;
        for _ in 0..8 {
            match self.get(current) {
                Some(var) if var.nameref.is_some() => {
                    current = var.nameref.as_ref().unwrap();
                }
                _ => break,
            }
        }
        current
    }

    /// Get the nameref target for a variable, if it is a nameref.
    pub fn get_nameref_target(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|v| v.nameref.as_deref())
    }

    /// Create a nameref variable in the current scope.
    pub fn set_nameref(&mut self, name: &str, target: String) {
        let scope = self.scopes.last_mut().unwrap();
        scope.set(name.into(), Var::nameref(target));
    }

    /// Check if a variable has a discipline function.
    pub fn has_discipline(&self, name: &str, kind: &str) -> bool {
        let fn_name = format!("{name}.{kind}");
        self.functions.contains_key(&fn_name)
    }

    /// Define a function.
    pub fn define_fn(&mut self, name: String, body: Vec<crate::ast::Command>) {
        self.functions.insert(name, body);
    }

    /// Look up a function by name.
    pub fn get_fn(&self, name: &str) -> Option<&Vec<crate::ast::Command>> {
        self.functions.get(name)
    }

    /// Build the exported environment for a child process.
    pub fn to_process_env(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        // Walk from bottom to top, later scopes override
        for scope in &self.scopes {
            for (name, val) in scope.exported() {
                result.push((name.to_string(), val.to_string()));
            }
        }
        result
    }
}

/// Validate a value against a type annotation.
///
/// Coercion policy: widening (Int→Str, Bool→Str, Path→Str) is
/// allowed. Narrowing (Str→Int) is rejected. List[T] validates
/// each element. Tuple validates componentwise. Union passes if
/// any branch matches. Result[T] and Maybe[T] validate as their
/// desugared Sum forms.
fn validate_type(val: &Val, ann: &TypeAnnotation) -> Result<Val, String> {
    match (val, ann) {
        (Val::Unit, TypeAnnotation::Unit) => Ok(Val::Unit),
        (Val::Bool(_), TypeAnnotation::Bool) => Ok(val.clone()),
        (Val::Int(_), TypeAnnotation::Int) => Ok(val.clone()),
        (Val::Str(_), TypeAnnotation::Str) => Ok(val.clone()),
        (Val::Path(_), TypeAnnotation::Path) => Ok(val.clone()),
        (Val::ExitCode(_), TypeAnnotation::ExitCode) => Ok(val.clone()),
        (Val::List(_), TypeAnnotation::List(None)) => Ok(val.clone()),
        (Val::List(items), TypeAnnotation::List(Some(elem_ann))) => {
            let mut validated = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                validated.push(
                    validate_type(item, elem_ann).map_err(|e| format!("element {}: {e}", i + 1))?,
                );
            }
            Ok(Val::List(validated))
        }
        // Tuple: componentwise validation
        (Val::Tuple(items), TypeAnnotation::Tuple(anns)) => {
            if items.len() != anns.len() {
                return Err(format!(
                    "tuple length mismatch: expected {}, got {}",
                    anns.len(),
                    items.len()
                ));
            }
            let mut validated = Vec::with_capacity(items.len());
            for (i, (item, elem_ann)) in items.iter().zip(anns).enumerate() {
                validated.push(
                    validate_type(item, elem_ann).map_err(|e| format!("tuple element {i}: {e}"))?,
                );
            }
            Ok(Val::Tuple(validated))
        }
        // Union: value matches if it passes any branch
        (_, TypeAnnotation::Union(branches)) => {
            for branch in branches {
                if let Ok(v) = validate_type(val, branch) {
                    return Ok(v);
                }
            }
            Err(format!(
                "type mismatch: expected {}, got {}",
                type_ann_name(ann),
                val_type_name(val),
            ))
        }
        // Result[T]: Sum("ok", T) or Sum("err", ExitCode)
        (Val::Sum(tag, payload), TypeAnnotation::Result(inner)) => match tag.as_str() {
            "ok" => {
                let validated =
                    validate_type(payload, inner).map_err(|e| format!("Result ok branch: {e}"))?;
                Ok(Val::Sum("ok".into(), Box::new(validated)))
            }
            "err" => {
                let validated = validate_type(payload, &TypeAnnotation::ExitCode)
                    .map_err(|e| format!("Result err branch: {e}"))?;
                Ok(Val::Sum("err".into(), Box::new(validated)))
            }
            _ => Err(format!(
                "type mismatch: Result expects ok/err tags, got \"{tag}\""
            )),
        },
        // Maybe[T]: Sum("ok", T) or Sum("none", Unit)
        (Val::Sum(tag, payload), TypeAnnotation::Maybe(inner)) => match tag.as_str() {
            "ok" => {
                let validated =
                    validate_type(payload, inner).map_err(|e| format!("Maybe ok branch: {e}"))?;
                Ok(Val::Sum("ok".into(), Box::new(validated)))
            }
            "none" => {
                let validated = validate_type(payload, &TypeAnnotation::Unit)
                    .map_err(|e| format!("Maybe none branch: {e}"))?;
                Ok(Val::Sum("none".into(), Box::new(validated)))
            }
            _ => Err(format!(
                "type mismatch: Maybe expects ok/none tags, got \"{tag}\""
            )),
        },
        // Widening coercions: narrow type → Str
        (Val::Int(n), TypeAnnotation::Str) => Ok(Val::Str(n.to_string())),
        (Val::Bool(b), TypeAnnotation::Str) => Ok(Val::Str(b.to_string())),
        (Val::Path(p), TypeAnnotation::Str) => Ok(Val::Str(p.display().to_string())),
        (Val::ExitCode(c), TypeAnnotation::Str) => Ok(Val::Str(c.to_string())),
        // Narrowing from Str: attempt parse when assigning to typed variable.
        // This handles bare `x = 99` when x was declared `let mut x : Int`.
        (Val::Str(s), TypeAnnotation::Int) => s
            .parse::<i64>()
            .map(Val::Int)
            .map_err(|_| format!("type mismatch: expected Int, got Str(\"{s}\")")),
        (Val::Str(s), TypeAnnotation::Bool) => match s.as_str() {
            "true" => Ok(Val::Bool(true)),
            "false" => Ok(Val::Bool(false)),
            _ => Err(format!("type mismatch: expected Bool, got Str(\"{s}\")")),
        },
        (Val::Str(s), TypeAnnotation::Path) => Ok(Val::Path(std::path::PathBuf::from(s.as_str()))),
        // All other combinations are type errors
        _ => Err(format!(
            "type mismatch: expected {}, got {}",
            type_ann_name(ann),
            val_type_name(val),
        )),
    }
}

fn type_ann_name(ann: &TypeAnnotation) -> String {
    match ann {
        TypeAnnotation::Unit => "Unit".into(),
        TypeAnnotation::Bool => "Bool".into(),
        TypeAnnotation::Int => "Int".into(),
        TypeAnnotation::Str => "Str".into(),
        TypeAnnotation::Path => "Path".into(),
        TypeAnnotation::ExitCode => "ExitCode".into(),
        TypeAnnotation::List(None) => "List".into(),
        TypeAnnotation::List(Some(inner)) => format!("List[{}]", type_ann_name(inner)),
        TypeAnnotation::Tuple(anns) => {
            let parts: Vec<String> = anns.iter().map(type_ann_name).collect();
            format!("({})", parts.join(", "))
        }
        TypeAnnotation::Union(branches) => {
            let parts: Vec<String> = branches.iter().map(type_ann_name).collect();
            parts.join(" | ")
        }
        TypeAnnotation::Result(inner) => format!("Result[{}]", type_ann_name(inner)),
        TypeAnnotation::Maybe(inner) => format!("Maybe[{}]", type_ann_name(inner)),
    }
}

fn val_type_name(val: &Val) -> &'static str {
    match val {
        Val::Unit => "Unit",
        Val::Bool(_) => "Bool",
        Val::Int(_) => "Int",
        Val::Str(_) => "Str",
        Val::Path(_) => "Path",
        Val::ExitCode(_) => "ExitCode",
        Val::List(_) => "List",
        Val::Tuple(_) => "Tuple",
        Val::Sum(_, _) => "Sum",
        Val::Thunk { .. } => "Thunk",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_resolution() {
        let mut env = Env::new();
        let _ = env.set_value("x", Val::scalar("global"));
        assert_eq!(env.get_value("x"), Val::scalar("global"));

        env.push_scope();
        // Inner scope can see outer
        assert_eq!(env.get_value("x"), Val::scalar("global"));

        // Inner scope shadows outer
        let _ = env.set_value("x", Val::scalar("local"));
        assert_eq!(env.get_value("x"), Val::scalar("local"));

        env.pop_scope();
        // Outer scope unchanged... actually no — set_value
        // updates in place when it finds the var. So "x" was
        // modified in the global scope. This is rc's behavior.
        assert_eq!(env.get_value("x"), Val::scalar("local"));
    }

    #[test]
    fn new_var_in_current_scope() {
        let mut env = Env::new();
        env.push_scope();
        let _ = env.set_value("y", Val::scalar("inner"));
        assert_eq!(env.get_value("y"), Val::scalar("inner"));

        env.pop_scope();
        // y was in inner scope, now gone
        assert_eq!(env.get_value("y"), Val::empty());
    }

    #[test]
    fn pid_is_set() {
        let env = Env::new();
        let pid = env.get_value("pid");
        assert!(pid.is_true());
        assert!(!pid.as_str().is_empty());
    }

    #[test]
    fn discipline_lookup() {
        let mut env = Env::new();
        assert!(!env.has_discipline("x", "get"));

        env.define_fn("x.get".into(), vec![]);
        assert!(env.has_discipline("x", "get"));
        assert!(!env.has_discipline("x", "set"));
    }

    #[test]
    fn readonly_scope_rejects_mutation() {
        let mut env = Env::new();
        let _ = env.set_value("x", Val::scalar("original"));
        env.push_readonly_scope();
        let result = env.set_value("x", Val::scalar("changed"));
        assert!(result.is_err());
        env.pop_scope();
        assert_eq!(env.get_value("x"), Val::scalar("original"));
    }

    #[test]
    fn readonly_var_rejects_mutation() {
        let mut env = Env::new();
        env.scopes[0].set(
            "ro".into(),
            Var {
                value: Val::scalar("frozen"),
                error: None,
                exported: false,
                readonly: true,
                mutable: true,
                type_ann: None,
                discipline: None,
                nameref: None,
            },
        );
        let result = env.set_value("ro", Val::scalar("changed"));
        assert!(result.is_err());
        assert_eq!(env.get_value("ro"), Val::scalar("frozen"));
    }

    #[test]
    fn nameref_resolves_read() {
        let mut env = Env::new();
        let _ = env.set_value("target", Val::scalar("data"));
        env.set_nameref("alias", "target".into());
        assert_eq!(env.get_value("alias"), Val::scalar("data"));
    }

    #[test]
    fn nameref_resolves_write() {
        let mut env = Env::new();
        let _ = env.set_value("target", Val::scalar("old"));
        env.set_nameref("alias", "target".into());
        let _ = env.set_value("alias", Val::scalar("new"));
        assert_eq!(env.get_value("target"), Val::scalar("new"));
    }

    #[test]
    fn nameref_chain_resolves() {
        let mut env = Env::new();
        let _ = env.set_value("base", Val::scalar("value"));
        env.set_nameref("mid", "base".into());
        env.set_nameref("top", "mid".into());
        assert_eq!(env.get_value("top"), Val::scalar("value"));
    }

    #[test]
    fn nameref_depth_limit() {
        // Create a circular nameref — resolution should terminate
        // after 8 levels without panicking.
        let mut env = Env::new();
        env.set_nameref("a", "b".into());
        env.set_nameref("b", "a".into());
        let _ = env.get_value("a");
    }

    // ── Type validation: new variants ───────────────────────

    #[test]
    fn validate_exit_code() {
        let val = Val::ExitCode(0);
        assert!(validate_type(&val, &TypeAnnotation::ExitCode).is_ok());
        assert!(validate_type(&val, &TypeAnnotation::Int).is_err());
    }

    #[test]
    fn validate_exit_code_widens_to_str() {
        let val = Val::ExitCode(42);
        let result = validate_type(&val, &TypeAnnotation::Str).unwrap();
        assert_eq!(result, Val::Str("42".into()));
    }

    #[test]
    fn validate_tuple_componentwise() {
        let val = Val::Tuple(vec![Val::Int(42), Val::Bool(true)]);
        let ann = TypeAnnotation::Tuple(vec![TypeAnnotation::Int, TypeAnnotation::Bool]);
        assert!(validate_type(&val, &ann).is_ok());
    }

    #[test]
    fn validate_tuple_length_mismatch() {
        let val = Val::Tuple(vec![Val::Int(42)]);
        let ann = TypeAnnotation::Tuple(vec![TypeAnnotation::Int, TypeAnnotation::Bool]);
        assert!(validate_type(&val, &ann).is_err());
    }

    #[test]
    fn validate_tuple_element_mismatch() {
        let val = Val::Tuple(vec![Val::Int(42), Val::Int(7)]);
        let ann = TypeAnnotation::Tuple(vec![TypeAnnotation::Int, TypeAnnotation::Bool]);
        assert!(validate_type(&val, &ann).is_err());
    }

    #[test]
    fn validate_union_matches_any_branch() {
        let ann = TypeAnnotation::Union(vec![TypeAnnotation::Int, TypeAnnotation::Str]);
        assert!(validate_type(&Val::Int(42), &ann).is_ok());
        assert!(validate_type(&Val::scalar("hello"), &ann).is_ok());
        // Bool widens to Str, so it passes the Str branch
        assert!(validate_type(&Val::Bool(true), &ann).is_ok());
        // A type with no widening path fails
        let narrow = TypeAnnotation::Union(vec![TypeAnnotation::Int, TypeAnnotation::Path]);
        assert!(validate_type(&Val::Bool(true), &narrow).is_err());
    }

    #[test]
    fn validate_result_ok() {
        let val = Val::Sum("ok".into(), Box::new(Val::Int(42)));
        let ann = TypeAnnotation::Result(Box::new(TypeAnnotation::Int));
        let result = validate_type(&val, &ann).unwrap();
        assert_eq!(result, val);
    }

    #[test]
    fn validate_result_err() {
        let val = Val::Sum("err".into(), Box::new(Val::ExitCode(1)));
        let ann = TypeAnnotation::Result(Box::new(TypeAnnotation::Int));
        assert!(validate_type(&val, &ann).is_ok());
    }

    #[test]
    fn validate_result_wrong_tag() {
        let val = Val::Sum("bad".into(), Box::new(Val::Int(42)));
        let ann = TypeAnnotation::Result(Box::new(TypeAnnotation::Int));
        assert!(validate_type(&val, &ann).is_err());
    }

    #[test]
    fn validate_maybe_ok() {
        let val = Val::Sum("ok".into(), Box::new(Val::scalar("hello")));
        let ann = TypeAnnotation::Maybe(Box::new(TypeAnnotation::Str));
        assert!(validate_type(&val, &ann).is_ok());
    }

    #[test]
    fn validate_maybe_none() {
        let val = Val::Sum("none".into(), Box::new(Val::Unit));
        let ann = TypeAnnotation::Maybe(Box::new(TypeAnnotation::Str));
        assert!(validate_type(&val, &ann).is_ok());
    }

    #[test]
    fn validate_maybe_wrong_tag() {
        let val = Val::Sum("err".into(), Box::new(Val::Unit));
        let ann = TypeAnnotation::Maybe(Box::new(TypeAnnotation::Str));
        assert!(validate_type(&val, &ann).is_err());
    }

    #[test]
    fn var_error_field_defaults_none() {
        let var = Var::new(Val::Int(42));
        assert!(var.error.is_none());
    }
}
