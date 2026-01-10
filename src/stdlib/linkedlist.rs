//! LinkedList module for Zyra standard library
//!
//! Provides a memory-safe doubly-linked list implementation:
//! - push_front, push_back: Add elements
//! - pop_front, pop_back: Remove elements
//! - get, set: Access elements by index
//! - len, is_empty: Query list state

use crate::compiler::bytecode::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    /// Global storage for linked lists, keyed by ID
    static LINKED_LISTS: RefCell<HashMap<i64, LinkedList>> = RefCell::new(HashMap::new());
    static NEXT_LIST_ID: RefCell<i64> = RefCell::new(1);
}

/// Node in the linked list
#[derive(Clone)]
struct Node {
    value: Value,
    prev: Option<Rc<RefCell<Node>>>,
    next: Option<Rc<RefCell<Node>>>,
}

impl Node {
    fn new(value: Value) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Node {
            value,
            prev: None,
            next: None,
        }))
    }
}

/// Memory-safe doubly linked list
pub struct LinkedList {
    head: Option<Rc<RefCell<Node>>>,
    tail: Option<Rc<RefCell<Node>>>,
    len: usize,
}

impl LinkedList {
    /// Create a new empty linked list
    pub fn new() -> Self {
        LinkedList {
            head: None,
            tail: None,
            len: 0,
        }
    }

    /// Get the length of the list
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push a value to the front of the list
    pub fn push_front(&mut self, value: Value) {
        let new_node = Node::new(value);

        match self.head.take() {
            Some(old_head) => {
                old_head.borrow_mut().prev = Some(Rc::clone(&new_node));
                new_node.borrow_mut().next = Some(old_head);
                self.head = Some(new_node);
            }
            None => {
                self.head = Some(Rc::clone(&new_node));
                self.tail = Some(new_node);
            }
        }
        self.len += 1;
    }

    /// Push a value to the back of the list
    pub fn push_back(&mut self, value: Value) {
        let new_node = Node::new(value);

        match self.tail.take() {
            Some(old_tail) => {
                old_tail.borrow_mut().next = Some(Rc::clone(&new_node));
                new_node.borrow_mut().prev = Some(old_tail);
                self.tail = Some(new_node);
            }
            None => {
                self.head = Some(Rc::clone(&new_node));
                self.tail = Some(new_node);
            }
        }
        self.len += 1;
    }

    /// Pop a value from the front of the list
    pub fn pop_front(&mut self) -> Option<Value> {
        self.head.take().map(|old_head| {
            self.len -= 1;
            match old_head.borrow_mut().next.take() {
                Some(new_head) => {
                    new_head.borrow_mut().prev = None;
                    self.head = Some(new_head);
                }
                None => {
                    self.tail = None;
                }
            }
            // Safe to unwrap because we have the only strong reference now
            Rc::try_unwrap(old_head)
                .ok()
                .map(|cell| cell.into_inner().value)
                .unwrap_or(Value::None)
        })
    }

    /// Pop a value from the back of the list
    pub fn pop_back(&mut self) -> Option<Value> {
        self.tail.take().map(|old_tail| {
            self.len -= 1;
            match old_tail.borrow_mut().prev.take() {
                Some(new_tail) => {
                    new_tail.borrow_mut().next = None;
                    self.tail = Some(new_tail);
                }
                None => {
                    self.head = None;
                }
            }
            Rc::try_unwrap(old_tail)
                .ok()
                .map(|cell| cell.into_inner().value)
                .unwrap_or(Value::None)
        })
    }

    /// Get a value at the specified index
    pub fn get(&self, index: usize) -> Option<Value> {
        if index >= self.len {
            return None;
        }

        let mut current = self.head.clone();
        for _ in 0..index {
            current = current.and_then(|node| node.borrow().next.clone());
        }

        current.map(|node| node.borrow().value.clone())
    }

    /// Set a value at the specified index
    pub fn set(&mut self, index: usize, value: Value) -> bool {
        if index >= self.len {
            return false;
        }

        let mut current = self.head.clone();
        for _ in 0..index {
            current = current.and_then(|node| node.borrow().next.clone());
        }

        if let Some(node) = current {
            node.borrow_mut().value = value;
            true
        } else {
            false
        }
    }

    /// Convert list to an array
    pub fn to_array(&self) -> Vec<Value> {
        let mut result = Vec::with_capacity(self.len);
        let mut current = self.head.clone();

        while let Some(node) = current {
            result.push(node.borrow().value.clone());
            current = node.borrow().next.clone();
        }

        result
    }

    /// Clear the list
    pub fn clear(&mut self) {
        self.head = None;
        self.tail = None;
        self.len = 0;
    }
}

impl Default for LinkedList {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Public API Functions =====

/// Create a new linked list, returns list ID
pub fn list_new() -> Value {
    NEXT_LIST_ID.with(|id| {
        let list_id = *id.borrow();
        *id.borrow_mut() += 1;

        LINKED_LISTS.with(|lists| {
            lists.borrow_mut().insert(list_id, LinkedList::new());
        });

        Value::Int(list_id)
    })
}

/// Push to front of list
pub fn list_push_front(list_id: i64, value: Value) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow_mut().get_mut(&list_id) {
            list.push_front(value);
            Value::Bool(true)
        } else {
            Value::err(Value::String(format!("Invalid list ID: {}", list_id)))
        }
    })
}

/// Push to back of list
pub fn list_push_back(list_id: i64, value: Value) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow_mut().get_mut(&list_id) {
            list.push_back(value);
            Value::Bool(true)
        } else {
            Value::err(Value::String(format!("Invalid list ID: {}", list_id)))
        }
    })
}

/// Pop from front of list
pub fn list_pop_front(list_id: i64) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow_mut().get_mut(&list_id) {
            list.pop_front().unwrap_or(Value::None)
        } else {
            Value::None
        }
    })
}

/// Pop from back of list
pub fn list_pop_back(list_id: i64) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow_mut().get_mut(&list_id) {
            list.pop_back().unwrap_or(Value::None)
        } else {
            Value::None
        }
    })
}

/// Get element at index
pub fn list_get(list_id: i64, index: i64) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow().get(&list_id) {
            if index < 0 || index as usize >= list.len() {
                Value::err(Value::String(format!(
                    "Index out of bounds: {} (list size: {})",
                    index,
                    list.len()
                )))
            } else {
                list.get(index as usize).unwrap_or(Value::None)
            }
        } else {
            Value::err(Value::String(format!("Invalid list ID: {}", list_id)))
        }
    })
}

/// Set element at index
pub fn list_set(list_id: i64, index: i64, value: Value) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow_mut().get_mut(&list_id) {
            if index < 0 || index as usize >= list.len() {
                Value::err(Value::String(format!(
                    "Index out of bounds: {} (list size: {})",
                    index,
                    list.len()
                )))
            } else {
                Value::Bool(list.set(index as usize, value))
            }
        } else {
            Value::err(Value::String(format!("Invalid list ID: {}", list_id)))
        }
    })
}

/// Get list length
pub fn list_len(list_id: i64) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow().get(&list_id) {
            Value::Int(list.len() as i64)
        } else {
            Value::Int(0)
        }
    })
}

/// Check if list is empty
pub fn list_is_empty(list_id: i64) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow().get(&list_id) {
            Value::Bool(list.is_empty())
        } else {
            Value::Bool(true)
        }
    })
}

/// Convert list to array
pub fn list_to_array(list_id: i64) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow().get(&list_id) {
            Value::Array(list.to_array())
        } else {
            Value::Array(Vec::new())
        }
    })
}

/// Clear the list
pub fn list_clear(list_id: i64) -> Value {
    LINKED_LISTS.with(|lists| {
        if let Some(list) = lists.borrow_mut().get_mut(&list_id) {
            list.clear();
            Value::Bool(true)
        } else {
            Value::Bool(false)
        }
    })
}

/// Delete the list and free memory
pub fn list_delete(list_id: i64) -> Value {
    LINKED_LISTS.with(|lists| Value::Bool(lists.borrow_mut().remove(&list_id).is_some()))
}
