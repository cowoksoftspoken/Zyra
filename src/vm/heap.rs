//! Heap allocator for Zyra reference-counted objects
//!
//! All reference types (Struct, Enum, Vec, String) are heap-allocated.
//! Stack variables hold Value::Ref(HeapId) pointing to heap data.
//!
//! Invariants:
//! - Every Value::Ref creation must IncRef
//! - Every overwrite, scope exit, or drop must DecRef
//! - When ref_count reaches 0: drop fields depth-first, then release slot

use crate::error::{ZyraError, ZyraResult};
use crate::vm::Value;

/// Unique identifier for heap objects
pub type HeapId = usize;

/// Object stored on the heap with reference count
#[derive(Debug, Clone)]
pub struct HeapObject {
    /// The actual value stored
    pub data: Value,
    /// Reference count - freed when reaches 0
    pub ref_count: usize,
}

impl HeapObject {
    pub fn new(data: Value) -> Self {
        Self {
            data,
            ref_count: 1, // Start with count of 1 (creator owns it)
        }
    }
}

/// Heap storage for reference-counted objects
#[derive(Debug)]
pub struct Heap {
    /// Object storage - None means slot is free
    objects: Vec<Option<HeapObject>>,
    /// Free list for reusing slots
    free_list: Vec<HeapId>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            free_list: Vec::new(),
        }
    }

    /// Allocate a new object on the heap, returns HeapId
    /// The new object starts with ref_count = 1
    pub fn alloc(&mut self, value: Value) -> HeapId {
        let obj = HeapObject::new(value);

        if let Some(id) = self.free_list.pop() {
            // Reuse a freed slot
            self.objects[id] = Some(obj);
            id
        } else {
            // Allocate new slot
            let id = self.objects.len();
            self.objects.push(Some(obj));
            id
        }
    }

    /// Get immutable reference to heap object
    pub fn get(&self, id: HeapId) -> Option<&HeapObject> {
        self.objects.get(id).and_then(|opt| opt.as_ref())
    }

    /// Get mutable reference to heap object
    pub fn get_mut(&mut self, id: HeapId) -> Option<&mut HeapObject> {
        self.objects.get_mut(id).and_then(|opt| opt.as_mut())
    }

    /// Get the data value at a heap location
    pub fn get_value(&self, id: HeapId) -> Option<&Value> {
        self.get(id).map(|obj| &obj.data)
    }

    /// Get mutable data value at a heap location
    pub fn get_value_mut(&mut self, id: HeapId) -> Option<&mut Value> {
        self.get_mut(id).map(|obj| &mut obj.data)
    }

    /// Increment reference count
    /// Called when: creating Value::Ref, copying ref, passing to function
    pub fn inc_ref(&mut self, id: HeapId) -> ZyraResult<()> {
        if let Some(obj) = self.get_mut(id) {
            obj.ref_count = obj.ref_count.saturating_add(1);
            Ok(())
        } else {
            Err(ZyraError::runtime_error(&format!(
                "IncRef on invalid heap id: {}",
                id
            )))
        }
    }

    /// Decrement reference count
    /// Called when: overwriting variable, scope exit, drop
    /// Returns true if object was freed (ref_count reached 0)
    pub fn dec_ref(&mut self, id: HeapId) -> ZyraResult<bool> {
        let should_free = {
            if let Some(obj) = self.get_mut(id) {
                obj.ref_count = obj.ref_count.saturating_sub(1);
                obj.ref_count == 0
            } else {
                return Err(ZyraError::runtime_error(&format!(
                    "DecRef on invalid heap id: {}",
                    id
                )));
            }
        };

        if should_free {
            self.free(id)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Free a heap object and add slot to free list
    /// Drops fields depth-first, decrements refcounts of referenced fields
    fn free(&mut self, id: HeapId) -> ZyraResult<()> {
        // Get the object to free
        let obj = self
            .objects
            .get_mut(id)
            .ok_or_else(|| ZyraError::runtime_error(&format!("Free on invalid heap id: {}", id)))?;

        if let Some(heap_obj) = obj.take() {
            // Drop fields depth-first if it's an object
            if let Value::Object(fields) = &heap_obj.data {
                // Collect ref ids to decrement (avoid borrow issues)
                let mut refs_to_dec: Vec<HeapId> = Vec::new();
                for value in fields.values() {
                    if let Value::Ref(ref_id) = value {
                        refs_to_dec.push(*ref_id);
                    }
                }
                // Now decrement refs (may trigger recursive free)
                for ref_id in refs_to_dec {
                    // Ignore errors for now (defensive)
                    let _ = self.dec_ref(ref_id);
                }
            }

            // Add slot to free list
            self.free_list.push(id);
        }

        Ok(())
    }

    /// Get the reference count for an object
    pub fn ref_count(&self, id: HeapId) -> Option<usize> {
        self.get(id).map(|obj| obj.ref_count)
    }

    /// Check if &mut self is valid (ref_count == 1)
    /// Panics if ref_count > 1 as per runtime enforcement
    pub fn check_exclusive_borrow(&self, id: HeapId) -> ZyraResult<()> {
        if let Some(count) = self.ref_count(id) {
            if count == 1 {
                Ok(())
            } else {
                Err(ZyraError::runtime_error(&format!(
                    "Cannot mutably borrow: object has {} references (expected 1)",
                    count
                )))
            }
        } else {
            Err(ZyraError::runtime_error(&format!(
                "Cannot borrow invalid heap id: {}",
                id
            )))
        }
    }

    /// Debug: print heap state
    #[allow(dead_code)]
    pub fn debug_print(&self) {
        println!("=== Heap State ===");
        for (i, slot) in self.objects.iter().enumerate() {
            if let Some(obj) = slot {
                println!("  [{}] rc={} data={:?}", i, obj.ref_count, obj.data);
            } else {
                println!("  [{}] FREE", i);
            }
        }
        println!("  Free list: {:?}", self.free_list);
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_and_get() {
        let mut heap = Heap::new();
        let id = heap.alloc(Value::I32(42));
        assert_eq!(heap.ref_count(id), Some(1));
        assert!(matches!(heap.get_value(id), Some(Value::I32(42))));
    }

    #[test]
    fn test_inc_dec_ref() {
        let mut heap = Heap::new();
        let id = heap.alloc(Value::I32(42));

        // Inc ref
        heap.inc_ref(id).unwrap();
        assert_eq!(heap.ref_count(id), Some(2));

        // Dec ref
        let freed = heap.dec_ref(id).unwrap();
        assert!(!freed);
        assert_eq!(heap.ref_count(id), Some(1));

        // Dec ref again - should free
        let freed = heap.dec_ref(id).unwrap();
        assert!(freed);
        assert!(heap.get(id).is_none());
    }

    #[test]
    fn test_slot_reuse() {
        let mut heap = Heap::new();
        let id1 = heap.alloc(Value::I32(1));
        let _ = heap.dec_ref(id1); // Free it

        let id2 = heap.alloc(Value::I32(2));
        assert_eq!(id1, id2); // Should reuse the slot
    }
}
