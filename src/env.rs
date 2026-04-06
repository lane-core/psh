//! Variable environment for psh.
//!
//! Variables are lists of strings (rc heritage). Each variable
//! can optionally have discipline functions (ksh93 heritage) —
//! .get fires on read, .set fires on write. Discipline functions
//! are the MonadicLens at the shell level: view (co-Kleisli,
//! pure observation) and set (Kleisli, effectful mutation).

use std::collections::HashMap;

use crate::value::Val;

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
    /// Whether this variable is exported to child processes.
    pub exported: bool,
    /// Whether this variable is read-only.
    pub readonly: bool,
    /// Discipline functions, if any.
    pub discipline: Option<Discipline>,
}

impl Var {
    pub fn new(value: Val) -> Self {
        Var {
            value,
            exported: false,
            readonly: false,
            discipline: None,
        }
    }

    pub fn exported(value: Val) -> Self {
        Var {
            value,
            exported: true,
            readonly: false,
            discipline: None,
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
    pub fn import_process_env(&mut self) {
        for (key, value) in std::env::vars() {
            self.scopes[0].set(key, Var::exported(Val::scalar(value)));
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
    pub fn get_value(&self, name: &str) -> Val {
        self.get(name)
            .map(|v| v.value.clone())
            .unwrap_or_else(Val::empty)
    }

    /// Set a variable. If it exists in any scope, updates in place.
    /// Otherwise creates in the current (topmost) scope.
    ///
    /// Returns `true` on success, `false` if the variable is readonly
    /// or the current scope is readonly. The caller must report the error.
    pub fn set_value(&mut self, name: &str, value: Val) -> bool {
        // If the topmost scope is readonly, reject all mutations
        if self.is_readonly() {
            return false;
        }

        // Search existing scopes for the variable
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                if var.readonly {
                    return false;
                }
                var.value = value;
                return true;
            }
        }
        // Not found — create in current scope
        let scope = self.scopes.last_mut().unwrap();
        scope.set(name.into(), Var::new(value));
        true
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_resolution() {
        let mut env = Env::new();
        env.set_value("x", Val::scalar("global"));
        assert_eq!(env.get_value("x"), Val::scalar("global"));

        env.push_scope();
        // Inner scope can see outer
        assert_eq!(env.get_value("x"), Val::scalar("global"));

        // Inner scope shadows outer
        env.set_value("x", Val::scalar("local"));
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
        env.set_value("y", Val::scalar("inner"));
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
        env.set_value("x", Val::scalar("original"));
        env.push_readonly_scope();
        let ok = env.set_value("x", Val::scalar("changed"));
        assert!(!ok);
        env.pop_scope();
        assert_eq!(env.get_value("x"), Val::scalar("original"));
    }

    #[test]
    fn readonly_var_rejects_mutation() {
        let mut env = Env::new();
        // Manually create a readonly var
        env.scopes[0].set(
            "ro".into(),
            Var {
                value: Val::scalar("frozen"),
                exported: false,
                readonly: true,
                discipline: None,
            },
        );
        let ok = env.set_value("ro", Val::scalar("changed"));
        assert!(!ok);
        assert_eq!(env.get_value("ro"), Val::scalar("frozen"));
    }
}
