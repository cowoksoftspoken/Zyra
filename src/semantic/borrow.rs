//! Borrow Checker for Zyra
//!
//! Enforces Rust-inspired borrowing rules:
//! - Mutable borrows: exactly one at a time (exclusive)
//! - Immutable borrows: multiple allowed
//! - Cannot have mutable and immutable borrows simultaneously
//! - Borrows end at scope exit or last use

use std::collections::HashMap;

/// State of a borrow
#[derive(Debug, Clone, PartialEq)]
pub enum BorrowKind {
    /// Immutable borrow (&T)
    Shared,
    /// Mutable borrow (&mut T)  
    Mutable,
}

/// Information about an active borrow
#[derive(Debug, Clone)]
pub struct ActiveBorrow {
    /// The variable being borrowed
    pub source: String,
    /// Kind of borrow (shared or mutable)
    pub kind: BorrowKind,
    /// Name of the borrowing variable/reference
    pub borrower: String,
    /// Line where borrow was created
    pub created_at: usize,
    /// Scope depth where borrow is valid
    pub scope_depth: usize,
}

/// Borrow checker tracks active borrows and enforces borrow rules
pub struct BorrowChecker {
    /// Active borrows by source variable name
    borrows: HashMap<String, Vec<ActiveBorrow>>,
    /// Current scope depth
    scope_depth: usize,
    /// Variables that have been moved (owned value taken)
    moved: HashMap<String, MovedInfo>,
    /// Errors encountered
    errors: Vec<BorrowError>,
}

/// Information about a moved variable
#[derive(Debug, Clone)]
pub struct MovedInfo {
    pub moved_to: String,
    pub at_line: usize,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            borrows: HashMap::new(),
            scope_depth: 0,
            moved: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exit current scope, ending all borrows in this scope
    pub fn exit_scope(&mut self) -> Vec<String> {
        let current_scope = self.scope_depth;
        let mut ended_borrows = Vec::new();

        // End all borrows that were created in this scope
        for (source, borrows) in self.borrows.iter_mut() {
            let before_len = borrows.len();
            borrows.retain(|b| b.scope_depth < current_scope);
            if borrows.len() < before_len {
                ended_borrows.push(source.clone());
            }
        }

        // Clean up empty entries
        self.borrows.retain(|_, v| !v.is_empty());

        self.scope_depth -= 1;
        ended_borrows
    }

    /// Record that a variable has been moved
    pub fn record_move(&mut self, from: &str, to: &str, line: usize) -> Result<(), BorrowError> {
        // Cannot move if currently borrowed
        if let Some(borrows) = self.borrows.get(from) {
            if !borrows.is_empty() {
                return Err(BorrowError::MoveWhileBorrowed {
                    variable: from.to_string(),
                    borrowed_by: borrows[0].borrower.clone(),
                    at_line: line,
                });
            }
        }

        // Cannot move if already moved
        if let Some(info) = self.moved.get(from) {
            return Err(BorrowError::UseAfterMove {
                variable: from.to_string(),
                moved_to: info.moved_to.clone(),
                moved_at: info.at_line,
                used_at: line,
            });
        }

        self.moved.insert(
            from.to_string(),
            MovedInfo {
                moved_to: to.to_string(),
                at_line: line,
            },
        );

        Ok(())
    }

    /// Create an immutable borrow
    pub fn borrow_shared(
        &mut self,
        source: &str,
        borrower: &str,
        line: usize,
    ) -> Result<(), BorrowError> {
        // Check if moved
        if let Some(info) = self.moved.get(source) {
            return Err(BorrowError::UseAfterMove {
                variable: source.to_string(),
                moved_to: info.moved_to.clone(),
                moved_at: info.at_line,
                used_at: line,
            });
        }

        // Check for existing mutable borrow (conflict)
        if let Some(borrows) = self.borrows.get(source) {
            for borrow in borrows {
                if borrow.kind == BorrowKind::Mutable {
                    return Err(BorrowError::BorrowConflict {
                        variable: source.to_string(),
                        existing_kind: BorrowKind::Mutable,
                        existing_by: borrow.borrower.clone(),
                        new_kind: BorrowKind::Shared,
                        at_line: line,
                    });
                }
            }
        }

        // Add the shared borrow
        let borrow = ActiveBorrow {
            source: source.to_string(),
            kind: BorrowKind::Shared,
            borrower: borrower.to_string(),
            created_at: line,
            scope_depth: self.scope_depth,
        };

        self.borrows
            .entry(source.to_string())
            .or_insert_with(Vec::new)
            .push(borrow);

        Ok(())
    }

    /// Create a mutable borrow (exclusive)
    pub fn borrow_mutable(
        &mut self,
        source: &str,
        borrower: &str,
        line: usize,
    ) -> Result<(), BorrowError> {
        // Check if moved
        if let Some(info) = self.moved.get(source) {
            return Err(BorrowError::UseAfterMove {
                variable: source.to_string(),
                moved_to: info.moved_to.clone(),
                moved_at: info.at_line,
                used_at: line,
            });
        }

        // Check for any existing borrow (mutable borrows are exclusive)
        if let Some(borrows) = self.borrows.get(source) {
            if !borrows.is_empty() {
                let existing = &borrows[0];
                return Err(BorrowError::BorrowConflict {
                    variable: source.to_string(),
                    existing_kind: existing.kind.clone(),
                    existing_by: existing.borrower.clone(),
                    new_kind: BorrowKind::Mutable,
                    at_line: line,
                });
            }
        }

        // Add the mutable borrow
        let borrow = ActiveBorrow {
            source: source.to_string(),
            kind: BorrowKind::Mutable,
            borrower: borrower.to_string(),
            created_at: line,
            scope_depth: self.scope_depth,
        };

        self.borrows
            .entry(source.to_string())
            .or_insert_with(Vec::new)
            .push(borrow);

        Ok(())
    }

    /// Check if a variable can be used (not moved, not mutably borrowed elsewhere)
    pub fn can_use(&self, name: &str, line: usize) -> Result<(), BorrowError> {
        // Check if moved
        if let Some(info) = self.moved.get(name) {
            return Err(BorrowError::UseAfterMove {
                variable: name.to_string(),
                moved_to: info.moved_to.clone(),
                moved_at: info.at_line,
                used_at: line,
            });
        }

        Ok(())
    }

    /// Check if a variable can be mutated
    pub fn can_mutate(&self, name: &str, line: usize) -> Result<(), BorrowError> {
        // First check if can use at all
        self.can_use(name, line)?;

        // Check if borrowed (cannot mutate while borrowed)
        if let Some(borrows) = self.borrows.get(name) {
            if !borrows.is_empty() {
                let borrow = &borrows[0];
                return Err(BorrowError::MutateWhileBorrowed {
                    variable: name.to_string(),
                    borrowed_by: borrow.borrower.clone(),
                    borrow_kind: borrow.kind.clone(),
                    at_line: line,
                });
            }
        }

        Ok(())
    }

    /// End a specific borrow (e.g., when reference goes out of scope)
    pub fn end_borrow(&mut self, borrower: &str) {
        for borrows in self.borrows.values_mut() {
            borrows.retain(|b| b.borrower != borrower);
        }
        self.borrows.retain(|_, v| !v.is_empty());
    }

    /// Check if returning a reference to a local variable
    pub fn check_return_reference(
        &self,
        ref_name: &str,
        source_var: Option<&str>,
        function_scope: usize,
        line: usize,
    ) -> Result<(), BorrowError> {
        if let Some(source) = source_var {
            // If the source variable was created in the function scope,
            // returning a reference to it is a dangling reference
            if let Some(borrows) = self.borrows.get(source) {
                for borrow in borrows {
                    if borrow.borrower == ref_name && borrow.scope_depth >= function_scope {
                        return Err(BorrowError::DanglingReference {
                            reference: ref_name.to_string(),
                            source: source.to_string(),
                            at_line: line,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Get all errors
    pub fn errors(&self) -> &[BorrowError] {
        &self.errors
    }

    /// Add an error
    pub fn add_error(&mut self, error: BorrowError) {
        self.errors.push(error);
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get current scope depth
    pub fn current_scope(&self) -> usize {
        self.scope_depth
    }
}

impl Default for BorrowChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrow checking errors
#[derive(Debug, Clone)]
pub enum BorrowError {
    UseAfterMove {
        variable: String,
        moved_to: String,
        moved_at: usize,
        used_at: usize,
    },
    MoveWhileBorrowed {
        variable: String,
        borrowed_by: String,
        at_line: usize,
    },
    BorrowConflict {
        variable: String,
        existing_kind: BorrowKind,
        existing_by: String,
        new_kind: BorrowKind,
        at_line: usize,
    },
    MutateWhileBorrowed {
        variable: String,
        borrowed_by: String,
        borrow_kind: BorrowKind,
        at_line: usize,
    },
    DanglingReference {
        reference: String,
        source: String,
        at_line: usize,
    },
    DoubleFree {
        variable: String,
        at_line: usize,
    },
}

impl std::fmt::Display for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowError::UseAfterMove {
                variable,
                moved_to,
                moved_at,
                used_at,
            } => {
                write!(
                    f,
                    "Use of moved value: '{}' was moved to '{}' at line {}, cannot use at line {}",
                    variable, moved_to, moved_at, used_at
                )
            }
            BorrowError::MoveWhileBorrowed {
                variable,
                borrowed_by,
                at_line,
            } => {
                write!(
                    f,
                    "Cannot move '{}' while borrowed by '{}' (line {})",
                    variable, borrowed_by, at_line
                )
            }
            BorrowError::BorrowConflict {
                variable,
                existing_kind,
                existing_by,
                new_kind,
                at_line,
            } => {
                let existing_str = match existing_kind {
                    BorrowKind::Shared => "immutably",
                    BorrowKind::Mutable => "mutably",
                };
                let new_str = match new_kind {
                    BorrowKind::Shared => "immutable",
                    BorrowKind::Mutable => "mutable",
                };
                write!(
                    f,
                    "Cannot create {} borrow of '{}': already borrowed {} by '{}' (line {})",
                    new_str, variable, existing_str, existing_by, at_line
                )
            }
            BorrowError::MutateWhileBorrowed {
                variable,
                borrowed_by,
                borrow_kind,
                at_line,
            } => {
                let kind_str = match borrow_kind {
                    BorrowKind::Shared => "immutably",
                    BorrowKind::Mutable => "mutably",
                };
                write!(
                    f,
                    "Cannot mutate '{}' while {} borrowed by '{}' (line {})",
                    variable, kind_str, borrowed_by, at_line
                )
            }
            BorrowError::DanglingReference {
                reference,
                source,
                at_line,
            } => {
                write!(
                    f,
                    "Dangling reference: '{}' references local variable '{}' which goes out of scope (line {})",
                    reference, source, at_line
                )
            }
            BorrowError::DoubleFree { variable, at_line } => {
                write!(
                    f,
                    "Double free: '{}' already freed (line {})",
                    variable, at_line
                )
            }
        }
    }
}
