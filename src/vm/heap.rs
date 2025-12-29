//! Heap Manager for Zyra VM
//!
//! Manages heap-allocated objects with explicit ownership tracking:
//! - Object IDs instead of Rust references
//! - Track Alive/Freed state
//! - Runtime checks for use-after-free
//! - Explicit deallocation

use super::Value;
use std::collections::HashMap;

/// Object ID for heap-allocated values
pub type HeapId = u64;

/// State of a heap object
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectState {
    /// Object is alive and valid
    Alive,
    /// Object has been freed  
    Freed,
    /// Object is currently borrowed (cannot be freed or moved)
    Borrowed { count: usize, mutable: bool },
}

/// A heap-allocated object
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub id: HeapId,
    pub value: Value,
    pub state: ObjectState,
    pub owner: Option<String>, // Name of owning variable
    pub allocated_at: usize,   // Line number
}

/// Heap manager tracks all heap-allocated objects
pub struct HeapManager {
    /// All objects by their ID
    objects: HashMap<HeapId, HeapObject>,
    /// Next available ID
    next_id: HeapId,
    /// Free list of reusable IDs
    free_list: Vec<HeapId>,
    /// Variable to HeapId mapping
    var_to_heap: HashMap<String, HeapId>,
    /// Reference to source mapping (which variable a reference points to)
    ref_sources: HashMap<String, String>,
}

impl HeapManager {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            next_id: 1, // Start from 1, 0 is reserved for null
            free_list: Vec::new(),
            var_to_heap: HashMap::new(),
            ref_sources: HashMap::new(),
        }
    }

    /// Allocate a new heap object
    pub fn alloc(&mut self, value: Value, owner: Option<&str>, line: usize) -> HeapId {
        let id = if let Some(reused_id) = self.free_list.pop() {
            reused_id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        };

        let object = HeapObject {
            id,
            value,
            state: ObjectState::Alive,
            owner: owner.map(|s| s.to_string()),
            allocated_at: line,
        };

        self.objects.insert(id, object);

        if let Some(var) = owner {
            self.var_to_heap.insert(var.to_string(), id);
        }

        id
    }

    /// Move ownership from one variable to another
    pub fn move_ownership(&mut self, from: &str, to: &str) -> Result<(), HeapError> {
        let id = self
            .var_to_heap
            .remove(from)
            .ok_or_else(|| HeapError::VariableNotFound {
                name: from.to_string(),
            })?;

        // Check if object is valid
        let obj = self
            .objects
            .get_mut(&id)
            .ok_or_else(|| HeapError::InvalidHeapId { id })?;

        match &obj.state {
            ObjectState::Freed => {
                return Err(HeapError::UseAfterFree {
                    id,
                    variable: from.to_string(),
                });
            }
            ObjectState::Borrowed { .. } => {
                return Err(HeapError::MoveWhileBorrowed {
                    variable: from.to_string(),
                });
            }
            ObjectState::Alive => {}
        }

        // Transfer ownership
        obj.owner = Some(to.to_string());
        self.var_to_heap.insert(to.to_string(), id);

        Ok(())
    }

    /// Create a shared (immutable) borrow
    pub fn borrow_shared(&mut self, source: &str, borrower: &str) -> Result<HeapId, HeapError> {
        let id = *self
            .var_to_heap
            .get(source)
            .ok_or_else(|| HeapError::VariableNotFound {
                name: source.to_string(),
            })?;

        let obj = self
            .objects
            .get_mut(&id)
            .ok_or_else(|| HeapError::InvalidHeapId { id })?;

        match &mut obj.state {
            ObjectState::Freed => {
                return Err(HeapError::UseAfterFree {
                    id,
                    variable: source.to_string(),
                });
            }
            ObjectState::Borrowed { mutable, count } if *mutable => {
                return Err(HeapError::BorrowConflict {
                    variable: source.to_string(),
                    reason: "already mutably borrowed".to_string(),
                });
            }
            ObjectState::Borrowed { count, .. } => {
                *count += 1;
            }
            ObjectState::Alive => {
                obj.state = ObjectState::Borrowed {
                    count: 1,
                    mutable: false,
                };
            }
        }

        // Track reference source
        self.ref_sources
            .insert(borrower.to_string(), source.to_string());

        Ok(id)
    }

    /// Create a mutable borrow (exclusive)
    pub fn borrow_mut(&mut self, source: &str, borrower: &str) -> Result<HeapId, HeapError> {
        let id = *self
            .var_to_heap
            .get(source)
            .ok_or_else(|| HeapError::VariableNotFound {
                name: source.to_string(),
            })?;

        let obj = self
            .objects
            .get_mut(&id)
            .ok_or_else(|| HeapError::InvalidHeapId { id })?;

        match &obj.state {
            ObjectState::Freed => {
                return Err(HeapError::UseAfterFree {
                    id,
                    variable: source.to_string(),
                });
            }
            ObjectState::Borrowed { .. } => {
                return Err(HeapError::BorrowConflict {
                    variable: source.to_string(),
                    reason: "already borrowed".to_string(),
                });
            }
            ObjectState::Alive => {
                obj.state = ObjectState::Borrowed {
                    count: 1,
                    mutable: true,
                };
            }
        }

        // Track reference source
        self.ref_sources
            .insert(borrower.to_string(), source.to_string());

        Ok(id)
    }

    /// End a borrow
    pub fn end_borrow(&mut self, borrower: &str) -> Result<(), HeapError> {
        let source =
            self.ref_sources
                .remove(borrower)
                .ok_or_else(|| HeapError::VariableNotFound {
                    name: borrower.to_string(),
                })?;

        let id = *self
            .var_to_heap
            .get(&source)
            .ok_or_else(|| HeapError::VariableNotFound {
                name: source.clone(),
            })?;

        let obj = self
            .objects
            .get_mut(&id)
            .ok_or_else(|| HeapError::InvalidHeapId { id })?;

        match &mut obj.state {
            ObjectState::Borrowed { count, mutable: _ } => {
                *count -= 1;
                if *count == 0 {
                    obj.state = ObjectState::Alive;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Free/drop a heap object
    pub fn free(&mut self, var: &str) -> Result<Value, HeapError> {
        let id = self
            .var_to_heap
            .remove(var)
            .ok_or_else(|| HeapError::VariableNotFound {
                name: var.to_string(),
            })?;

        let obj = self
            .objects
            .get_mut(&id)
            .ok_or_else(|| HeapError::InvalidHeapId { id })?;

        match &obj.state {
            ObjectState::Freed => {
                return Err(HeapError::DoubleFree {
                    variable: var.to_string(),
                });
            }
            ObjectState::Borrowed { .. } => {
                return Err(HeapError::FreeWhileBorrowed {
                    variable: var.to_string(),
                });
            }
            ObjectState::Alive => {}
        }

        obj.state = ObjectState::Freed;
        self.free_list.push(id);

        Ok(obj.value.clone())
    }

    /// Get value by variable name (for reading)
    pub fn get(&self, var: &str) -> Result<&Value, HeapError> {
        let id = self
            .var_to_heap
            .get(var)
            .ok_or_else(|| HeapError::VariableNotFound {
                name: var.to_string(),
            })?;

        let obj = self
            .objects
            .get(id)
            .ok_or_else(|| HeapError::InvalidHeapId { id: *id })?;

        match &obj.state {
            ObjectState::Freed => Err(HeapError::UseAfterFree {
                id: *id,
                variable: var.to_string(),
            }),
            _ => Ok(&obj.value),
        }
    }

    /// Get mutable value by variable name
    pub fn get_mut(&mut self, var: &str) -> Result<&mut Value, HeapError> {
        let id = *self
            .var_to_heap
            .get(var)
            .ok_or_else(|| HeapError::VariableNotFound {
                name: var.to_string(),
            })?;

        let obj = self
            .objects
            .get_mut(&id)
            .ok_or_else(|| HeapError::InvalidHeapId { id })?;

        match &obj.state {
            ObjectState::Freed => Err(HeapError::UseAfterFree {
                id,
                variable: var.to_string(),
            }),
            ObjectState::Borrowed { mutable: false, .. } => Err(HeapError::MutateWhileBorrowed {
                variable: var.to_string(),
            }),
            _ => Ok(&mut obj.value),
        }
    }

    /// Check if a variable is on heap
    pub fn is_heap_allocated(&self, var: &str) -> bool {
        self.var_to_heap.contains_key(var)
    }

    /// Get source of a reference
    pub fn get_ref_source(&self, borrower: &str) -> Option<&String> {
        self.ref_sources.get(borrower)
    }
}

impl Default for HeapManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Heap errors
#[derive(Debug, Clone)]
pub enum HeapError {
    VariableNotFound { name: String },
    InvalidHeapId { id: HeapId },
    UseAfterFree { id: HeapId, variable: String },
    DoubleFree { variable: String },
    MoveWhileBorrowed { variable: String },
    FreeWhileBorrowed { variable: String },
    BorrowConflict { variable: String, reason: String },
    MutateWhileBorrowed { variable: String },
}

impl std::fmt::Display for HeapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeapError::VariableNotFound { name } => {
                write!(f, "Variable '{}' not found on heap", name)
            }
            HeapError::InvalidHeapId { id } => {
                write!(f, "Invalid heap ID: {}", id)
            }
            HeapError::UseAfterFree { id, variable } => {
                write!(
                    f,
                    "Use after free: '{}' (heap ID {}) has been freed",
                    variable, id
                )
            }
            HeapError::DoubleFree { variable } => {
                write!(f, "Double free: '{}' already freed", variable)
            }
            HeapError::MoveWhileBorrowed { variable } => {
                write!(f, "Cannot move '{}' while borrowed", variable)
            }
            HeapError::FreeWhileBorrowed { variable } => {
                write!(f, "Cannot free '{}' while borrowed", variable)
            }
            HeapError::BorrowConflict { variable, reason } => {
                write!(f, "Borrow conflict for '{}': {}", variable, reason)
            }
            HeapError::MutateWhileBorrowed { variable } => {
                write!(f, "Cannot mutate '{}' while immutably borrowed", variable)
            }
        }
    }
}
