//! Zyra Virtual Machine
//!
//! Stack-based bytecode interpreter with scope management

pub mod heap;
pub mod value;

use crate::compiler::{Bytecode, FunctionDef, Instruction};
use crate::error::{ZyraError, ZyraResult};
use crate::stdlib::StdLib;
pub use heap::{Heap, HeapId, HeapObject};
pub use value::Value;

use std::collections::HashMap;

/// Call stack frame
#[derive(Debug, Clone)]
struct CallFrame {
    // function_name: String,
    return_address: usize,
    base_pointer: usize,
}

/// Scope for variable storage
#[derive(Debug, Clone)]
struct Scope {
    variables: HashMap<String, Value>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
}

/// Zyra Virtual Machine
pub struct VM {
    stack: Vec<Value>,
    call_stack: Vec<CallFrame>,
    scopes: Vec<Scope>,
    ip: usize,
    stdlib: StdLib,
    halted: bool,
    main_called: bool, // Track if main() was already called
    /// Heap for reference-counted objects (structs, enums, vecs, strings)
    heap: Heap,
}

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            call_stack: Vec::new(),
            scopes: vec![Scope::new()],
            ip: 0,
            stdlib: StdLib::new(),
            halted: false,
            main_called: false,
            heap: Heap::new(),
        }
    }

    /// Run bytecode program
    pub fn run(&mut self, bytecode: &Bytecode) -> ZyraResult<Option<Value>> {
        self.ip = 0;
        self.halted = false;

        // Skip function definitions (they're registered but not executed sequentially)
        // Find the first non-function instruction
        for (i, _instr) in bytecode.instructions.iter().enumerate() {
            let in_function = bytecode
                .functions
                .values()
                .any(|f| i >= f.start_address && i < f.end_address);
            if !in_function {
                self.ip = i;
                break;
            }
        }

        while self.ip < bytecode.instructions.len() && !self.halted {
            let instruction = bytecode.instructions[self.ip].clone();

            // If we hit Halt and main() exists but hasn't been called, call it first
            if matches!(instruction, Instruction::Halt) {
                if !self.main_called {
                    if let Some(main_func) = bytecode.functions.get("main") {
                        // Check if main has no parameters (valid entry point)
                        if main_func.params.is_empty() {
                            // Mark main as called to prevent double-execution
                            self.main_called = true;
                            // Call main function
                            self.call_function(main_func, Vec::new())?;
                            // Continue execution (don't execute Halt yet)
                            continue;
                        }
                    }
                }
            }

            self.ip += 1;
            self.execute_instruction(&instruction, bytecode)?;
        }

        // Return top of stack if any
        if self.stack.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.stack.pop().unwrap()))
        }
    }

    fn execute_instruction(
        &mut self,
        instruction: &Instruction,
        bytecode: &Bytecode,
    ) -> ZyraResult<()> {
        match instruction {
            Instruction::LoadConst(value) => {
                self.stack.push(value.clone());
            }

            Instruction::LoadVar(name) => {
                let value = self.get_variable(name)?;
                if let Value::Ref(heap_id) = value {
                    let _ = self.heap.inc_ref(heap_id);
                }
                self.stack.push(value);
            }

            Instruction::StoreVar(name) => {
                let value = self.pop()?;
                // Note: set_variable overwrites old. We need to handle old value decrement inside set_variable
                // OR we check here? set_variable logic handles scopes.
                // We should update set_variable (helper) instead of here if possible,
                // but if we do it here, we might miss other calls.
                // Let's rely on updated set_variable (Task: Update set_variable next).
                self.set_variable(name, value);
            }

            Instruction::Pop => {
                let val = self.pop()?;
                if let Value::Ref(heap_id) = val {
                    let _ = self.heap.dec_ref(heap_id);
                }
            }

            // Arithmetic
            Instruction::Add => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a.add(&b).ok_or_else(|| {
                    ZyraError::runtime_error(&format!(
                        "Cannot add {} and {}",
                        a.type_name(),
                        b.type_name()
                    ))
                })?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            Instruction::Sub => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a.sub(&b).ok_or_else(|| {
                    ZyraError::runtime_error(&format!(
                        "Cannot subtract {} from {}",
                        b.type_name(),
                        a.type_name()
                    ))
                })?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            Instruction::Mul => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a.mul(&b).ok_or_else(|| {
                    ZyraError::runtime_error(&format!(
                        "Cannot multiply {} and {}",
                        a.type_name(),
                        b.type_name()
                    ))
                })?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            Instruction::Div => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a.div(&b).ok_or_else(|| {
                    ZyraError::runtime_error("Division error (possibly division by zero)")
                })?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            Instruction::Mod => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a
                    .modulo(&b)
                    .ok_or_else(|| ZyraError::runtime_error("Modulo error"))?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            Instruction::Neg => {
                let a = self.pop()?;
                let result = a.neg().ok_or_else(|| {
                    ZyraError::runtime_error(&format!("Cannot negate {}", a.type_name()))
                })?;

                // Cleanup operand
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            // Comparison
            Instruction::Eq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(a.eq(&b));

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }
            }

            Instruction::Neq => {
                let b = self.pop()?;
                let a = self.pop()?;
                let eq = a.eq(&b);
                self.stack.push(eq.not());

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }
            }

            Instruction::Lt => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a.lt(&b).ok_or_else(|| {
                    ZyraError::runtime_error(&format!(
                        "Cannot compare {} and {}",
                        a.type_name(),
                        b.type_name()
                    ))
                })?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            Instruction::Lte => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a.lte(&b).ok_or_else(|| {
                    ZyraError::runtime_error(&format!(
                        "Cannot compare {} and {}",
                        a.type_name(),
                        b.type_name()
                    ))
                })?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            Instruction::Gt => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a.gt(&b).ok_or_else(|| {
                    ZyraError::runtime_error(&format!(
                        "Cannot compare {} and {}",
                        a.type_name(),
                        b.type_name()
                    ))
                })?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            Instruction::Gte => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = a.gte(&b).ok_or_else(|| {
                    ZyraError::runtime_error(&format!(
                        "Cannot compare {} and {}",
                        a.type_name(),
                        b.type_name()
                    ))
                })?;

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }

                self.stack.push(result);
            }

            // Logical
            Instruction::And => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(Value::Bool(a.is_truthy() && b.is_truthy()));

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }
            }

            Instruction::Or => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(Value::Bool(a.is_truthy() || b.is_truthy()));

                // Cleanup operands
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
                if let Value::Ref(id) = b {
                    let _ = self.heap.dec_ref(id);
                }
            }

            Instruction::Not => {
                let a = self.pop()?;
                self.stack.push(a.not());

                // Cleanup operand
                if let Value::Ref(id) = a {
                    let _ = self.heap.dec_ref(id);
                }
            }

            // Control flow
            Instruction::Jump(address) => {
                self.ip = *address;
            }

            Instruction::JumpIfFalse(address) => {
                let condition = self.pop()?;
                if !condition.is_truthy() {
                    self.ip = *address;
                }
            }

            Instruction::Call(name, arg_count) => {
                // Collect arguments
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();

                // Check for built-in functions first
                if let Some(result) = self.stdlib.call(name, &args)? {
                    self.stack.push(result);
                } else if let Some(func) = bytecode.functions.get(name) {
                    // User-defined function
                    self.call_function(func, args)?;
                } else if name.contains('.') {
                    // Method call: try to dispatch dynamically based on object's _type
                    // Format: "var.method" - use first arg to find type
                    if let Some(method_name) = name.split('.').last() {
                        if !args.is_empty() {
                            if let Value::Object(fields) = &args[0] {
                                if let Some(Value::String(type_name)) = fields.get("_type") {
                                    let full_method_name =
                                        format!("{}::{}", type_name, method_name);
                                    if let Some(func) = bytecode.functions.get(&full_method_name) {
                                        self.call_function(func, args)?;
                                    } else {
                                        return Err(ZyraError::runtime_error(&format!(
                                            "Unknown method: '{}'",
                                            full_method_name
                                        )));
                                    }
                                } else {
                                    return Err(ZyraError::runtime_error(&format!(
                                        "Cannot call method '{}' on non-struct value",
                                        name
                                    )));
                                }
                            } else {
                                return Err(ZyraError::runtime_error(&format!(
                                    "Cannot call method '{}' on non-struct value",
                                    name
                                )));
                            }
                        } else {
                            return Err(ZyraError::runtime_error(&format!(
                                "Method call '{}' requires a receiver",
                                name
                            )));
                        }
                    } else {
                        return Err(ZyraError::runtime_error(&format!(
                            "Unknown function: '{}'",
                            name
                        )));
                    }
                } else {
                    return Err(ZyraError::runtime_error(&format!(
                        "Unknown function: '{}'",
                        name
                    )));
                }
            }

            Instruction::MethodCall(method_name, arg_count) => {
                // MethodCall: receiver is pushed first, then args
                // Stack order: [receiver, arg1, arg2, ...]
                // Pop args first (in reverse), then receiver
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();

                // Pop the receiver (first argument is the struct)
                let receiver = self.pop()?;

                // Get the type from the receiver's _type field
                // Handle both Value::Ref (heap-allocated) and Value::Object (legacy)
                let type_name_opt = match &receiver {
                    Value::Ref(heap_id) => {
                        // Dereference from heap
                        if let Some(heap_obj) = self.heap.get(*heap_id) {
                            if let Value::Object(fields) = &heap_obj.data {
                                fields.get("_type").and_then(|v| {
                                    if let Value::String(s) = v {
                                        Some(s.clone())
                                    } else {
                                        None
                                    }
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    Value::Object(fields) => fields.get("_type").and_then(|v| {
                        if let Value::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    }),
                    _ => None,
                };

                if let Some(type_name) = type_name_opt {
                    let full_method_name = format!("{}::{}", type_name, method_name);
                    if let Some(func) = bytecode.functions.get(&full_method_name) {
                        // Phase 8: Access Control
                        // Check if the method requires unique mutable access ("mut self")
                        let is_mutable = func
                            .params
                            .first()
                            .map(|p| p.contains("mut self"))
                            .unwrap_or(false);

                        if is_mutable {
                            if let Value::Ref(heap_id) = receiver {
                                if let Some(heap_obj) = self.heap.get(heap_id) {
                                    // Rule: "Effective Uniqueness"
                                    // 1. Owner variable (if loaded from var) -> +1
                                    // 2. Stack temporary (receiver arg) -> +1
                                    // Total valid count = 2.
                                    // If > 2, it is shared by multiple variables -> Unsafe for mutation.
                                    // If called on Rvalue (make_obj().mut()), count is 1. Safe.
                                    if heap_obj.ref_count > 2 {
                                        return Err(ZyraError::runtime_error(&format!(
                                            "Cannot borrow mutable reference to object with shared ownership (ref_count = {}). Ensure the object is unique or move it.",
                                            heap_obj.ref_count
                                        )));
                                    }
                                }
                            }
                        }

                        // Prepend receiver to args for self parameter
                        let mut all_args = vec![receiver.clone()];
                        all_args.extend(args);
                        self.call_function(func, all_args)?;
                    } else {
                        return Err(ZyraError::runtime_error(&format!(
                            "Unknown method: '{}' on type '{}'",
                            method_name, type_name
                        )));
                    }
                } else {
                    return Err(ZyraError::runtime_error(&format!(
                        "Cannot call method '{}' on non-struct value (no _type field)",
                        method_name
                    )));
                }
            }

            Instruction::Return => {
                let return_value = self.stack.pop().unwrap_or(Value::None);

                // Since we are about to drop scopes, if return_value was a local variable,
                // it would be dec_ref'd. We must ensure it survives.
                // Assuming "Stack Owned" convention, return_value already has +1.
                // But since logic isn't fully migrated, we'll implement a safety increment if it matches a stack convention,
                // OR we rely on LoadVar cloning. For now, we assume LoadVar clones.
                // IF LoadVar implies +1, then return_value has +1.
                // When we drop scope, local `x` goes -1.
                // So return_value (+1) survives. Correct.

                if let Some(frame) = self.call_stack.pop() {
                    // Restore scope: Pop all scopes up to base_pointer
                    while self.scopes.len() > frame.base_pointer {
                        if let Some(scope) = self.scopes.pop() {
                            for (_, value) in scope.variables {
                                if let Value::Ref(heap_id) = value {
                                    let _ = self.heap.dec_ref(heap_id);
                                }
                            }
                        }
                    }
                    self.ip = frame.return_address;
                    self.stack.push(return_value);
                } else {
                    // Return from main
                    self.stack.push(return_value);
                    self.halted = true;
                }
            }

            // Data structures
            Instruction::MakeList(count) => {
                let mut elements = Vec::new();
                for _ in 0..*count {
                    elements.push(self.pop()?);
                }
                elements.reverse();
                self.stack.push(Value::List(elements));
            }

            Instruction::MakeObject(count) => {
                let mut fields = HashMap::new();
                for _ in 0..*count {
                    let value = self.pop()?;
                    let key = self.pop()?;
                    if let Value::String(k) = key {
                        fields.insert(k, value);
                    }
                }
                // Allocate object on heap and push reference
                let heap_id = self.heap.alloc(Value::Object(fields));
                self.stack.push(Value::Ref(heap_id));
            }

            Instruction::GetField(field) => {
                let obj = self.pop()?;
                match obj {
                    Value::Object(fields) => {
                        let value = fields.get(field).cloned().unwrap_or(Value::None);
                        self.stack.push(value);
                    }
                    Value::Ref(heap_id) => {
                        // Auto-deref: get field from heap object
                        if let Some(heap_obj) = self.heap.get(heap_id) {
                            if let Value::Object(fields) = &heap_obj.data {
                                let value = fields.get(field).cloned().unwrap_or(Value::None);
                                self.stack.push(value);
                            } else {
                                return Err(ZyraError::runtime_error(&format!(
                                    "Cannot access field '{}' on non-object heap value",
                                    field
                                )));
                            }
                        } else {
                            return Err(ZyraError::runtime_error(&format!(
                                "Invalid heap reference: {}",
                                heap_id
                            )));
                        }
                    }
                    Value::Window(state) => {
                        // Window method access
                        match field.as_str() {
                            "is_open" => self.stack.push(Value::Bool(state.is_open)),
                            "width" => self.stack.push(Value::Int(state.width as i64)),
                            "height" => self.stack.push(Value::Int(state.height as i64)),
                            _ => self.stack.push(Value::None),
                        }
                    }
                    Value::String(s) => match field.as_str() {
                        "len" => self.stack.push(Value::Int(s.len() as i64)),
                        _ => self.stack.push(Value::None),
                    },
                    Value::List(l) => match field.as_str() {
                        "len" | "length" => self.stack.push(Value::Int(l.len() as i64)),
                        _ => self.stack.push(Value::None),
                    },
                    _ => {
                        return Err(ZyraError::runtime_error(&format!(
                            "Cannot access field '{}' on {}",
                            field,
                            obj.type_name()
                        )));
                    }
                }
            }

            Instruction::SetField(field) => {
                // Stack order: [value, obj] - obj on top (pushed last by compiler)
                let obj = self.pop()?;
                let value = self.pop()?;
                match obj {
                    Value::Ref(heap_id) => {
                        if let Some(heap_obj) = self.heap.get_mut(heap_id) {
                            if let Value::Object(ref mut fields) = heap_obj.data {
                                if let Some(old_val) = fields.insert(field.clone(), value) {
                                    if let Value::Ref(old_id) = old_val {
                                        let _ = self.heap.dec_ref(old_id);
                                    }
                                }
                            }
                        }
                        // Push back the ref (for chaining)
                        self.stack.push(Value::Ref(heap_id));
                    }
                    Value::Object(mut fields) => {
                        fields.insert(field.clone(), value);
                        self.stack.push(Value::Object(fields));
                    }
                    _ => {
                        self.stack.push(obj);
                    }
                }
            }

            Instruction::GetIndex => {
                let index = self.pop()?;
                let obj = self.pop()?;

                match (&obj, &index) {
                    (Value::List(list), Value::Int(i)) => {
                        let idx = *i as usize;
                        if idx < list.len() {
                            self.stack.push(list[idx].clone());
                        } else {
                            return Err(ZyraError::runtime_error(&format!(
                                "Index {} out of bounds for list of length {}",
                                i,
                                list.len()
                            )));
                        }
                    }
                    (Value::String(s), Value::Int(i)) => {
                        let idx = *i as usize;
                        if idx < s.len() {
                            self.stack
                                .push(Value::String(s.chars().nth(idx).unwrap().to_string()));
                        } else {
                            return Err(ZyraError::runtime_error(&format!(
                                "Index {} out of bounds for string of length {}",
                                i,
                                s.len()
                            )));
                        }
                    }
                    _ => {
                        return Err(ZyraError::runtime_error(&format!(
                            "Cannot index {} with {}",
                            obj.type_name(),
                            index.type_name()
                        )));
                    }
                }
            }

            Instruction::SetIndex => {
                let index = self.pop()?;
                let mut obj = self.pop()?;
                let value = self.pop()?;

                if let (Value::List(ref mut list), Value::Int(i)) = (&mut obj, &index) {
                    let idx = *i as usize;
                    if idx < list.len() {
                        list[idx] = value;
                    }
                }
                self.stack.push(obj);
            }

            // Scope management
            Instruction::EnterScope => {
                self.scopes.push(Scope::new());
            }

            Instruction::ExitScope => {
                if let Some(scope) = self.scopes.pop() {
                    // Decrement ref counts for all variables in scope
                    for (_, value) in scope.variables {
                        if let Value::Ref(heap_id) = value {
                            let _ = self.heap.dec_ref(heap_id);
                        }
                    }
                }
            }

            Instruction::Print => {
                let value = self.pop()?;
                println!("{}", value);
                if let Value::Ref(id) = value {
                    let _ = self.heap.dec_ref(id);
                }
            }

            Instruction::Nop => {}

            // Memory management instructions
            Instruction::Alloc => {
                // For now, alloc just marks the value as heap-allocated
                // Real implementation would use HeapManager
                // This is a placeholder - the value is already on stack
            }

            Instruction::Move(from, to) => {
                // Move ownership from one variable to another
                if let Some(value) = self.remove_variable(from) {
                    self.set_variable(to, value);
                }
            }

            Instruction::BorrowShared(source) => {
                // Create an immutable reference
                let _value = self.get_variable(source)?;
                self.stack.push(Value::Reference {
                    name: source.clone(),
                    mutable: false,
                });
            }

            Instruction::BorrowMut(source) => {
                // Create a mutable reference
                let _value = self.get_variable(source)?;
                self.stack.push(Value::Reference {
                    name: source.clone(),
                    mutable: true,
                });
            }

            Instruction::Drop(var) => {
                // Explicitly drop a variable
                self.remove_variable(&var);
            }

            Instruction::EndBorrow(borrower) => {
                // End a borrow - remove the reference
                self.remove_variable(&borrower);
            }

            Instruction::Halt => {
                self.halted = true;
            }
        }

        Ok(())
    }

    fn call_function(&mut self, func: &FunctionDef, args: Vec<Value>) -> ZyraResult<()> {
        // Push call frame
        self.call_stack.push(CallFrame {
            // function_name: func.name.clone(),
            return_address: self.ip,
            base_pointer: self.scopes.len(),
        });

        // Push arguments onto stack in reverse order so StoreVar can pop them
        // The compiled function has EnterScope then StoreVar for each param in reverse
        for arg in args.into_iter().rev() {
            self.stack.push(arg);
        }

        // Jump to function (function will EnterScope and StoreVar the params)
        self.ip = func.start_address;

        Ok(())
    }

    fn pop(&mut self) -> ZyraResult<Value> {
        self.stack
            .pop()
            .ok_or_else(|| ZyraError::runtime_error("Stack underflow"))
    }

    fn get_variable(&self, name: &str) -> ZyraResult<Value> {
        // Search from innermost scope outward
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.variables.get(name) {
                return Ok(value.clone());
            }
        }

        // Check for module-style access (e.g., input.key)
        if name.contains('.') {
            // This is handled by the stdlib
            return Ok(Value::None);
        }

        Err(ZyraError::runtime_error(&format!(
            "Undefined variable: '{}'",
            name
        )))
    }

    fn set_variable(&mut self, name: &str, value: Value) {
        // First, check if the variable exists in any outer scope and update it there
        for scope in self.scopes.iter_mut().rev() {
            if scope.variables.contains_key(name) {
                if let Some(old_value) = scope.variables.insert(name.to_string(), value) {
                    if let Value::Ref(heap_id) = old_value {
                        let _ = self.heap.dec_ref(heap_id);
                    }
                }
                return;
            }
        }
        // If not found, create it in the innermost scope (for new let bindings)
        if let Some(scope) = self.scopes.last_mut() {
            // Note: If we are initializing a new variable, insert returns None.
            // If we are shadowing/overwriting in the same scope (if feasible), it returns Some.
            if let Some(old_value) = scope.variables.insert(name.to_string(), value) {
                if let Value::Ref(heap_id) = old_value {
                    let _ = self.heap.dec_ref(heap_id);
                }
            }
        }
    }

    // Public methods for stdlib access
    pub fn get_var(&self, name: &str) -> Option<Value> {
        self.get_variable(name).ok()
    }

    pub fn set_var(&mut self, name: &str, value: Value) {
        self.set_variable(name, value);
    }

    /// Remove a variable from all scopes (for move/drop)
    fn remove_variable(&mut self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(value) = scope.variables.remove(name) {
                // If it's a reference type (heap ptr), decrement ref count
                if let Value::Ref(heap_id) = value {
                    let _ = self.heap.dec_ref(heap_id);
                }
                return Some(value);
            }
        }
        None
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
