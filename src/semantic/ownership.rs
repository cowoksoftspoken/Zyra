//! Ownership and borrow checking for Zyra
//!
//! Implements Rust-inspired ownership semantics:
//! - Every value has a single owner
//! - Values are immutable by default
//! - Values are dropped when they go out of scope
//! - References are always safe

use std::collections::HashMap;

/// Ownership state of a variable
#[derive(Debug, Clone, PartialEq)]
pub enum OwnershipState {
    /// Variable owns its value
    Owned,
    /// Variable's value has been moved
    Moved { to: String, at_line: usize },
    /// Variable is borrowed immutably
    Borrowed { by: Vec<String> },
    /// Variable is borrowed mutably
    MutablyBorrowed { by: String },
}

/// Variable binding information
#[derive(Debug, Clone)]
pub struct Binding {
    pub name: String,
    pub mutable: bool,
    pub ownership: OwnershipState,
    pub defined_at: usize,
    pub scope_depth: usize,
    pub scope_id: usize, // Unique scope identifier
}

/// Ownership checker tracks variable ownership through scopes
pub struct OwnershipChecker {
    bindings: HashMap<String, Binding>,
    scope_depth: usize,
    scope_id: usize,         // Current unique scope ID
    next_scope_id: usize,    // Counter for generating unique scope IDs
    scope_stack: Vec<usize>, // Stack of scope IDs for tracking nested scopes
}

impl OwnershipChecker {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            scope_depth: 0,
            scope_id: 0,
            next_scope_id: 1,
            scope_stack: vec![0], // Root scope
        }
    }

    /// Enter a new scope with unique ID
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
        self.scope_id = self.next_scope_id;
        self.next_scope_id += 1;
        self.scope_stack.push(self.scope_id);
    }

    /// Exit current scope, dropping all bindings in this scope
    pub fn exit_scope(&mut self) -> Vec<String> {
        let current_scope_id = self.scope_id;
        let dropped: Vec<_> = self
            .bindings
            .iter()
            .filter(|(_, b)| b.scope_id == current_scope_id)
            .map(|(name, _)| name.clone())
            .collect();

        for name in &dropped {
            self.bindings.remove(name);
        }

        self.scope_depth -= 1;
        self.scope_stack.pop();
        // Restore previous scope ID
        self.scope_id = *self.scope_stack.last().unwrap_or(&0);
        dropped
    }

    /// Define a new binding with mangled name for scoped uniqueness
    pub fn define(&mut self, name: &str, mutable: bool, line: usize) -> Result<(), OwnershipError> {
        // Create a scoped key that includes the scope_id for uniqueness
        let scoped_key = format!("{}@{}", name, self.scope_id);

        // Check if the SAME variable already exists in the SAME scope
        if let Some(existing) = self.bindings.get(&scoped_key) {
            return Err(OwnershipError::AlreadyDefined {
                name: name.to_string(),
                original_line: existing.defined_at,
                duplicate_line: line,
            });
        }

        // Also check the plain name for shadowing in outer scope
        // This allows inner scope to shadow outer scope variables

        self.bindings.insert(
            scoped_key,
            Binding {
                name: name.to_string(),
                mutable,
                ownership: OwnershipState::Owned,
                defined_at: line,
                scope_depth: self.scope_depth,
                scope_id: self.scope_id,
            },
        );

        Ok(())
    }

    /// Find the scoped key for a binding, searching from current scope up to root
    fn find_binding_key(&self, name: &str) -> Option<String> {
        // Search from current scope up to root
        for &scope_id in self.scope_stack.iter().rev() {
            let scoped_key = format!("{}@{}", name, scope_id);
            if self.bindings.contains_key(&scoped_key) {
                return Some(scoped_key);
            }
        }
        None
    }

    /// Use a binding (read access)
    pub fn use_binding(&self, name: &str, line: usize) -> Result<&Binding, OwnershipError> {
        // Search for binding in scope hierarchy
        let key = self
            .find_binding_key(name)
            .ok_or_else(|| OwnershipError::NotDefined {
                name: name.to_string(),
                at_line: line,
            })?;

        let binding = self
            .bindings
            .get(&key)
            .ok_or_else(|| OwnershipError::NotDefined {
                name: name.to_string(),
                at_line: line,
            })?;

        if let OwnershipState::Moved { to, at_line } = &binding.ownership {
            return Err(OwnershipError::UsedAfterMove {
                name: name.to_string(),
                moved_to: to.clone(),
                moved_at: *at_line,
                used_at: line,
            });
        }

        Ok(binding)
    }

    /// Move a value from one binding to another
    pub fn move_value(&mut self, from: &str, to: &str, line: usize) -> Result<(), OwnershipError> {
        // Check source exists and is owned
        let binding = self.use_binding(from, line)?;

        if let OwnershipState::Borrowed { .. } | OwnershipState::MutablyBorrowed { .. } =
            &binding.ownership
        {
            return Err(OwnershipError::MovedWhileBorrowed {
                name: from.to_string(),
                at_line: line,
            });
        }

        // Mark as moved - use scoped key
        if let Some(key) = self.find_binding_key(from) {
            if let Some(b) = self.bindings.get_mut(&key) {
                b.ownership = OwnershipState::Moved {
                    to: to.to_string(),
                    at_line: line,
                };
            }
        }

        Ok(())
    }

    /// Assign to a mutable binding
    pub fn assign(&mut self, name: &str, line: usize) -> Result<(), OwnershipError> {
        let key = self
            .find_binding_key(name)
            .ok_or_else(|| OwnershipError::NotDefined {
                name: name.to_string(),
                at_line: line,
            })?;

        let binding = self
            .bindings
            .get(&key)
            .ok_or_else(|| OwnershipError::NotDefined {
                name: name.to_string(),
                at_line: line,
            })?;

        if !binding.mutable {
            return Err(OwnershipError::AssignToImmutable {
                name: name.to_string(),
                at_line: line,
                defined_at: binding.defined_at,
            });
        }

        if let OwnershipState::Borrowed { .. } | OwnershipState::MutablyBorrowed { .. } =
            &binding.ownership
        {
            return Err(OwnershipError::AssignWhileBorrowed {
                name: name.to_string(),
                at_line: line,
            });
        }

        Ok(())
    }

    /// Create an immutable borrow
    pub fn borrow(
        &mut self,
        name: &str,
        borrower: &str,
        line: usize,
    ) -> Result<(), OwnershipError> {
        let key = self
            .find_binding_key(name)
            .ok_or_else(|| OwnershipError::NotDefined {
                name: name.to_string(),
                at_line: line,
            })?;

        let binding = self
            .bindings
            .get_mut(&key)
            .ok_or_else(|| OwnershipError::NotDefined {
                name: name.to_string(),
                at_line: line,
            })?;

        match &mut binding.ownership {
            OwnershipState::Owned => {
                binding.ownership = OwnershipState::Borrowed {
                    by: vec![borrower.to_string()],
                };
            }
            OwnershipState::Borrowed { by } => {
                by.push(borrower.to_string());
            }
            OwnershipState::MutablyBorrowed { .. } => {
                return Err(OwnershipError::BorrowWhileMutablyBorrowed {
                    name: name.to_string(),
                    at_line: line,
                });
            }
            OwnershipState::Moved { at_line, .. } => {
                return Err(OwnershipError::UsedAfterMove {
                    name: name.to_string(),
                    moved_to: String::new(),
                    moved_at: *at_line,
                    used_at: line,
                });
            }
        }

        Ok(())
    }

    /// Create a mutable borrow
    pub fn borrow_mut(
        &mut self,
        name: &str,
        borrower: &str,
        line: usize,
    ) -> Result<(), OwnershipError> {
        let key = self
            .find_binding_key(name)
            .ok_or_else(|| OwnershipError::NotDefined {
                name: name.to_string(),
                at_line: line,
            })?;

        let binding = self
            .bindings
            .get_mut(&key)
            .ok_or_else(|| OwnershipError::NotDefined {
                name: name.to_string(),
                at_line: line,
            })?;

        if !binding.mutable {
            return Err(OwnershipError::MutBorrowOfImmutable {
                name: name.to_string(),
                at_line: line,
            });
        }

        match &binding.ownership {
            OwnershipState::Owned => {
                binding.ownership = OwnershipState::MutablyBorrowed {
                    by: borrower.to_string(),
                };
                Ok(())
            }
            OwnershipState::Borrowed { .. } => Err(OwnershipError::MutBorrowWhileBorrowed {
                name: name.to_string(),
                at_line: line,
            }),
            OwnershipState::MutablyBorrowed { .. } => {
                Err(OwnershipError::MutBorrowWhileMutablyBorrowed {
                    name: name.to_string(),
                    at_line: line,
                })
            }
            OwnershipState::Moved { at_line, .. } => Err(OwnershipError::UsedAfterMove {
                name: name.to_string(),
                moved_to: String::new(),
                moved_at: *at_line,
                used_at: line,
            }),
        }
    }

    /// Get binding info if it exists
    pub fn get(&self, name: &str) -> Option<&Binding> {
        self.find_binding_key(name)
            .and_then(|key| self.bindings.get(&key))
    }
}

impl Default for OwnershipChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Ownership errors
#[derive(Debug, Clone)]
pub enum OwnershipError {
    NotDefined {
        name: String,
        at_line: usize,
    },
    AlreadyDefined {
        name: String,
        original_line: usize,
        duplicate_line: usize,
    },
    UsedAfterMove {
        name: String,
        moved_to: String,
        moved_at: usize,
        used_at: usize,
    },
    MovedWhileBorrowed {
        name: String,
        at_line: usize,
    },
    AssignToImmutable {
        name: String,
        at_line: usize,
        defined_at: usize,
    },
    AssignWhileBorrowed {
        name: String,
        at_line: usize,
    },
    BorrowWhileMutablyBorrowed {
        name: String,
        at_line: usize,
    },
    MutBorrowOfImmutable {
        name: String,
        at_line: usize,
    },
    MutBorrowWhileBorrowed {
        name: String,
        at_line: usize,
    },
    MutBorrowWhileMutablyBorrowed {
        name: String,
        at_line: usize,
    },
}

impl std::fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnershipError::NotDefined { name, at_line } => {
                write!(
                    f,
                    "Variable '{}' is not defined (used at line {})",
                    name, at_line
                )
            }
            OwnershipError::AlreadyDefined {
                name,
                original_line,
                duplicate_line,
            } => {
                write!(
                    f,
                    "Variable '{}' is already defined at line {} (duplicate at line {})",
                    name, original_line, duplicate_line
                )
            }
            OwnershipError::UsedAfterMove {
                name,
                moved_at,
                used_at,
                ..
            } => {
                write!(
                    f,
                    "Variable '{}' was moved at line {} and cannot be used at line {}",
                    name, moved_at, used_at
                )
            }
            OwnershipError::MovedWhileBorrowed { name, at_line } => {
                write!(
                    f,
                    "Cannot move '{}' while it is borrowed (at line {})",
                    name, at_line
                )
            }
            OwnershipError::AssignToImmutable {
                name,
                at_line,
                defined_at,
            } => {
                write!(
                    f,
                    "Cannot assign to immutable variable '{}' at line {} (defined at line {}). \
                          Consider declaring with 'let mut'",
                    name, at_line, defined_at
                )
            }
            OwnershipError::AssignWhileBorrowed { name, at_line } => {
                write!(
                    f,
                    "Cannot assign to '{}' while it is borrowed (at line {})",
                    name, at_line
                )
            }
            OwnershipError::BorrowWhileMutablyBorrowed { name, at_line } => {
                write!(
                    f,
                    "Cannot borrow '{}' while it is mutably borrowed (at line {})",
                    name, at_line
                )
            }
            OwnershipError::MutBorrowOfImmutable { name, at_line } => {
                write!(
                    f,
                    "Cannot mutably borrow immutable variable '{}' (at line {}). \
                          Consider declaring with 'let mut'",
                    name, at_line
                )
            }
            OwnershipError::MutBorrowWhileBorrowed { name, at_line } => {
                write!(
                    f,
                    "Cannot mutably borrow '{}' while it is already borrowed (at line {})",
                    name, at_line
                )
            }
            OwnershipError::MutBorrowWhileMutablyBorrowed { name, at_line } => {
                write!(
                    f,
                    "Cannot mutably borrow '{}' while it is already mutably borrowed (at line {})",
                    name, at_line
                )
            }
        }
    }
}
