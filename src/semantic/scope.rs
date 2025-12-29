//! Scope tracking for Zyra
//!
//! Provides unique ScopeId for each scope and tracks variable origins.

use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for generating unique ScopeIds
static SCOPE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u64);

impl ScopeId {
    /// Create a new unique ScopeId
    pub fn new() -> Self {
        Self(SCOPE_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Global/static scope (scope 0)
    pub fn global() -> Self {
        Self(0)
    }

    /// Check if this scope is the global scope
    pub fn is_global(&self) -> bool {
        self.0 == 0
    }

    /// Get the raw ID value
    pub fn id(&self) -> u64 {
        self.0
    }
}

impl Default for ScopeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Origin of a value - where it was created
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueOrigin {
    /// Function parameter - can be returned as reference
    Param,
    /// Local variable - cannot escape its scope
    Local,
    /// Temporary value - cannot be borrowed outside statement
    Temporary,
    /// Global/static value - can be referenced anywhere
    Global,
}

impl ValueOrigin {
    /// Can a reference to this origin be returned from a function?
    pub fn can_return_reference(&self) -> bool {
        matches!(self, ValueOrigin::Param | ValueOrigin::Global)
    }

    /// Can this value be borrowed?
    pub fn can_borrow(&self) -> bool {
        // Temporaries cannot be borrowed (outside their statement)
        !matches!(self, ValueOrigin::Temporary)
    }

    /// Display name for error messages
    pub fn display_name(&self) -> &'static str {
        match self {
            ValueOrigin::Param => "parameter",
            ValueOrigin::Local => "local variable",
            ValueOrigin::Temporary => "temporary value",
            ValueOrigin::Global => "global variable",
        }
    }
}

/// Information about a variable's scope and origin
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// Name of the variable
    pub name: String,
    /// Scope where this variable was declared
    pub decl_scope: ScopeId,
    /// Origin of the value
    pub origin: ValueOrigin,
    /// Line where declared
    pub decl_line: usize,
    /// Is it mutable?
    pub mutable: bool,
}

impl VariableInfo {
    pub fn new(
        name: &str,
        scope: ScopeId,
        origin: ValueOrigin,
        line: usize,
        mutable: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            decl_scope: scope,
            origin,
            decl_line: line,
            mutable,
        }
    }

    /// Create info for a function parameter
    pub fn param(name: &str, scope: ScopeId, line: usize, mutable: bool) -> Self {
        Self::new(name, scope, ValueOrigin::Param, line, mutable)
    }

    /// Create info for a local variable
    pub fn local(name: &str, scope: ScopeId, line: usize, mutable: bool) -> Self {
        Self::new(name, scope, ValueOrigin::Local, line, mutable)
    }
}

/// Information about a reference
#[derive(Debug, Clone)]
pub struct ReferenceInfo {
    /// Name of the reference variable
    pub ref_name: String,
    /// Name of the source variable being borrowed
    pub source_name: String,
    /// Scope where the reference was created
    pub use_scope: ScopeId,
    /// Origin scope of the source variable
    pub origin_scope: ScopeId,
    /// Origin type of the source value
    pub source_origin: ValueOrigin,
    /// Is it a mutable reference?
    pub is_mutable: bool,
    /// Line where reference was created
    pub created_at: usize,
}

impl ReferenceInfo {
    /// Check if this reference escapes its origin scope
    /// use_scope > origin_scope means escape (illegal for locals)
    pub fn escapes_scope(&self, current_scope: ScopeId) -> bool {
        // References to locals/temporaries cannot escape their origin scope
        match self.source_origin {
            ValueOrigin::Local | ValueOrigin::Temporary => {
                // If current scope is different from origin, it's an escape
                current_scope != self.origin_scope
            }
            ValueOrigin::Param | ValueOrigin::Global => {
                // Params and globals can be referenced anywhere
                false
            }
        }
    }

    /// Check if this reference can be returned from a function
    pub fn can_return(&self) -> bool {
        self.source_origin.can_return_reference()
    }
}

/// Scope stack for tracking nested scopes
#[derive(Debug)]
pub struct ScopeStack {
    /// Stack of scope IDs (innermost is last)
    scopes: Vec<ScopeId>,
    /// Function scope ID (for return checks)
    function_scope: Option<ScopeId>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![ScopeId::global()],
            function_scope: None,
        }
    }

    /// Enter a new scope, returns the new ScopeId
    pub fn enter(&mut self) -> ScopeId {
        let id = ScopeId::new();
        self.scopes.push(id);
        id
    }

    /// Enter a function scope
    pub fn enter_function(&mut self) -> ScopeId {
        let id = self.enter();
        self.function_scope = Some(id);
        id
    }

    /// Exit the function scope
    pub fn exit_function(&mut self) {
        self.function_scope = None;
    }

    /// Exit current scope, returns the exited ScopeId
    pub fn exit(&mut self) -> Option<ScopeId> {
        if self.scopes.len() > 1 {
            self.scopes.pop()
        } else {
            None
        }
    }

    /// Get current scope ID
    pub fn current(&self) -> ScopeId {
        *self.scopes.last().unwrap_or(&ScopeId::global())
    }

    /// Get function scope ID (for return checks)
    pub fn function_scope(&self) -> Option<ScopeId> {
        self.function_scope
    }

    /// Get scope depth
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// Check if a scope is still active (hasn't been exited)
    pub fn is_active(&self, scope: ScopeId) -> bool {
        self.scopes.contains(&scope)
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}
