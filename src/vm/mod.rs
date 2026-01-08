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
    /// IMPORTANT: Only main() is executed - no code outside functions runs
    pub fn run(&mut self, bytecode: &Bytecode) -> ZyraResult<Option<Value>> {
        self.ip = 0;
        self.halted = false;

        // *** MAIN-ONLY EXECUTION ***
        // Programs must have a main() function as the entry point.
        // No code outside functions is executed - stack starts clean from main().

        if let Some(main_func) = bytecode.functions.get("main") {
            // Verify main has no parameters (valid entry point)
            if !main_func.params.is_empty() {
                return Err(ZyraError::runtime_error(
                    "main() function must not have parameters.",
                ));
            }

            // Mark main as called and execute it
            self.main_called = true;

            // Set up main execution WITHOUT pushing a CallFrame
            // This way when main() returns, call_stack is empty and halted gets set to true
            self.scopes.push(Scope::new()); // Enter main's scope
            self.ip = main_func.start_address;

            // Execute instructions starting from main's body
            while self.ip < bytecode.instructions.len() && !self.halted {
                let instruction = bytecode.instructions[self.ip].clone();
                self.ip += 1;
                self.execute_instruction(&instruction, bytecode)?;
            }
        } else {
            // No main function found - error
            return Err(ZyraError::runtime_error(
                "No 'main' function found. Programs must have a 'func main() { ... }' as entry point.",
            ));
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
                // set_variable handles ref counting: decrements old value's ref if Ref
                self.set_variable(name, value);
            }

            Instruction::Pop => {
                // Gracefully handle empty stack (e.g., after void function calls)
                if let Some(val) = self.stack.pop() {
                    if let Value::Ref(heap_id) = val {
                        let _ = self.heap.dec_ref(heap_id);
                    }
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

                // Handle higher-order functions that need closure invocation
                match name.as_str() {
                    "vec_map" => {
                        // vec_map(array, closure) -> new array with closure applied to each element
                        if args.len() >= 2 {
                            let (arr, is_vec) = match &args[0] {
                                Value::Array(a) => (a.clone(), false),
                                Value::Vec(a) => (a.clone(), true),
                                _ => {
                                    return Err(ZyraError::runtime_error(
                                        "vec_map: first argument must be an array or vec",
                                    ))
                                }
                            };
                            let closure = &args[1];
                            let mut result = Vec::new();
                            for item in arr {
                                let mapped =
                                    self.call_closure_with_value(closure, vec![item], bytecode)?;
                                result.push(mapped);
                            }
                            // Preserve input type in output
                            if is_vec {
                                self.stack.push(Value::Vec(result));
                            } else {
                                self.stack.push(Value::Array(result));
                            }
                        } else {
                            return Err(ZyraError::runtime_error(
                                "vec_map requires 2 arguments: array and closure",
                            ));
                        }
                    }
                    "vec_filter" => {
                        // vec_filter(array, closure) -> new array with elements where closure returns true
                        if args.len() >= 2 {
                            let (arr, is_vec) = match &args[0] {
                                Value::Array(a) => (a.clone(), false),
                                Value::Vec(a) => (a.clone(), true),
                                _ => {
                                    return Err(ZyraError::runtime_error(
                                        "vec_filter: first argument must be an array or vec",
                                    ))
                                }
                            };
                            let closure = &args[1];
                            let mut result = Vec::new();
                            for item in arr {
                                let keep = self.call_closure_with_value(
                                    closure,
                                    vec![item.clone()],
                                    bytecode,
                                )?;
                                if keep.is_truthy() {
                                    result.push(item);
                                }
                            }
                            // Preserve input type in output
                            if is_vec {
                                self.stack.push(Value::Vec(result));
                            } else {
                                self.stack.push(Value::Array(result));
                            }
                        } else {
                            return Err(ZyraError::runtime_error(
                                "vec_filter requires 2 arguments: array and closure",
                            ));
                        }
                    }
                    "vec_fold" => {
                        // vec_fold(array, initial, closure) -> reduced value
                        if args.len() >= 3 {
                            let arr = match &args[0] {
                                Value::Array(a) => a.clone(),
                                Value::Vec(a) => a.clone(),
                                _ => {
                                    return Err(ZyraError::runtime_error(
                                        "vec_fold: first argument must be an array or vec",
                                    ))
                                }
                            };
                            let mut acc = args[1].clone();
                            let closure = &args[2];
                            for item in arr {
                                acc = self.call_closure_with_value(
                                    closure,
                                    vec![acc, item],
                                    bytecode,
                                )?;
                            }
                            self.stack.push(acc);
                        } else {
                            return Err(ZyraError::runtime_error(
                                "vec_fold requires 3 arguments: array, initial, closure",
                            ));
                        }
                    }
                    "vec_foreach" => {
                        // vec_foreach(array, closure) -> executes closure for each element
                        if args.len() >= 2 {
                            let arr = match &args[0] {
                                Value::Array(a) => a.clone(),
                                Value::Vec(a) => a.clone(),
                                _ => {
                                    return Err(ZyraError::runtime_error(
                                        "vec_foreach: first argument must be an array or vec",
                                    ))
                                }
                            };
                            let closure = &args[1];
                            for item in arr {
                                self.call_closure_with_value(closure, vec![item], bytecode)?;
                            }
                            self.stack.push(Value::None);
                        } else {
                            return Err(ZyraError::runtime_error(
                                "vec_foreach requires 2 arguments: array and closure",
                            ));
                        }
                    }
                    "vec_find" => {
                        // vec_find(array, closure) -> first element where closure returns true, or None
                        if args.len() >= 2 {
                            let arr = match &args[0] {
                                Value::Array(a) => a.clone(),
                                Value::Vec(a) => a.clone(),
                                _ => {
                                    return Err(ZyraError::runtime_error(
                                        "vec_find: first argument must be an array or vec",
                                    ))
                                }
                            };
                            let closure = &args[1];
                            let mut found = Value::None;
                            for item in arr {
                                let matches = self.call_closure_with_value(
                                    closure,
                                    vec![item.clone()],
                                    bytecode,
                                )?;
                                if matches.is_truthy() {
                                    found = item;
                                    break;
                                }
                            }
                            self.stack.push(found);
                        } else {
                            return Err(ZyraError::runtime_error(
                                "vec_find requires 2 arguments: array and closure",
                            ));
                        }
                    }
                    "vec_any" => {
                        // vec_any(array, closure) -> true if closure returns true for any element
                        if args.len() >= 2 {
                            let arr = match &args[0] {
                                Value::Array(a) => a.clone(),
                                Value::Vec(a) => a.clone(),
                                _ => {
                                    return Err(ZyraError::runtime_error(
                                        "vec_any: first argument must be an array or vec",
                                    ))
                                }
                            };
                            let closure = &args[1];
                            let mut any_true = false;
                            for item in arr {
                                let matches =
                                    self.call_closure_with_value(closure, vec![item], bytecode)?;
                                if matches.is_truthy() {
                                    any_true = true;
                                    break;
                                }
                            }
                            self.stack.push(Value::Bool(any_true));
                        } else {
                            return Err(ZyraError::runtime_error(
                                "vec_any requires 2 arguments: array and closure",
                            ));
                        }
                    }
                    "vec_all" => {
                        // vec_all(array, closure) -> true if closure returns true for all elements
                        if args.len() >= 2 {
                            let arr = match &args[0] {
                                Value::Array(a) => a.clone(),
                                Value::Vec(a) => a.clone(),
                                _ => {
                                    return Err(ZyraError::runtime_error(
                                        "vec_all: first argument must be an array or vec",
                                    ))
                                }
                            };
                            let closure = &args[1];
                            let mut all_true = true;
                            for item in arr {
                                let matches =
                                    self.call_closure_with_value(closure, vec![item], bytecode)?;
                                if !matches.is_truthy() {
                                    all_true = false;
                                    break;
                                }
                            }
                            self.stack.push(Value::Bool(all_true));
                        } else {
                            return Err(ZyraError::runtime_error(
                                "vec_all requires 2 arguments: array and closure",
                            ));
                        }
                    }
                    _ => {
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
                                        if let Some(Value::String(type_name)) = fields.get("_type")
                                        {
                                            let full_method_name =
                                                format!("{}::{}", type_name, method_name);
                                            if let Some(func) =
                                                bytecode.functions.get(&full_method_name)
                                            {
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
                        } else if let Ok(closure_val) = self.get_variable(name) {
                            // Check if it's a closure variable
                            match closure_val {
                                Value::Closure { .. } => {
                                    let result =
                                        self.call_closure_with_value(&closure_val, args, bytecode)?;
                                    self.stack.push(result);
                                }
                                _ => {
                                    return Err(ZyraError::runtime_error(&format!(
                                        "Variable '{}' is not callable (type: {})",
                                        name,
                                        closure_val.type_name()
                                    )));
                                }
                            }
                        } else {
                            return Err(ZyraError::runtime_error(&format!(
                                "Unknown function: '{}'",
                                name
                            )));
                        }
                    }
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

                // ===== ARRAY/VEC HOF METHODS =====
                // Handle method calls on Array and Vec types (map, filter, fold, etc.)
                match (&receiver, method_name.as_str()) {
                    (Value::Array(arr), "map") | (Value::Vec(arr), "map") => {
                        if args.is_empty() {
                            return Err(ZyraError::runtime_error(
                                "map requires a closure argument",
                            ));
                        }
                        let closure = &args[0];
                        let is_vec = matches!(&receiver, Value::Vec(_));
                        let mut result = Vec::new();
                        for item in arr.clone() {
                            let mapped =
                                self.call_closure_with_value(closure, vec![item], bytecode)?;
                            result.push(mapped);
                        }
                        if is_vec {
                            self.stack.push(Value::Vec(result));
                        } else {
                            self.stack.push(Value::Array(result));
                        }
                        return Ok(());
                    }
                    (Value::Array(arr), "filter") | (Value::Vec(arr), "filter") => {
                        if args.is_empty() {
                            return Err(ZyraError::runtime_error(
                                "filter requires a closure argument",
                            ));
                        }
                        let closure = &args[0];
                        let is_vec = matches!(&receiver, Value::Vec(_));
                        let mut result = Vec::new();
                        for item in arr.clone() {
                            let keep = self.call_closure_with_value(
                                closure,
                                vec![item.clone()],
                                bytecode,
                            )?;
                            if keep.is_truthy() {
                                result.push(item);
                            }
                        }
                        if is_vec {
                            self.stack.push(Value::Vec(result));
                        } else {
                            self.stack.push(Value::Array(result));
                        }
                        return Ok(());
                    }
                    (Value::Array(arr), "fold") | (Value::Vec(arr), "fold") => {
                        if args.len() < 2 {
                            return Err(ZyraError::runtime_error(
                                "fold requires initial value and closure arguments",
                            ));
                        }
                        let mut acc = args[0].clone();
                        let closure = &args[1];
                        for item in arr.clone() {
                            acc =
                                self.call_closure_with_value(closure, vec![acc, item], bytecode)?;
                        }
                        self.stack.push(acc);
                        return Ok(());
                    }
                    (Value::Array(arr), "forEach") | (Value::Vec(arr), "forEach") => {
                        if args.is_empty() {
                            return Err(ZyraError::runtime_error(
                                "forEach requires a closure argument",
                            ));
                        }
                        let closure = &args[0];
                        for item in arr.clone() {
                            self.call_closure_with_value(closure, vec![item], bytecode)?;
                        }
                        self.stack.push(Value::None);
                        return Ok(());
                    }
                    (Value::Array(arr), "find") | (Value::Vec(arr), "find") => {
                        if args.is_empty() {
                            return Err(ZyraError::runtime_error(
                                "find requires a closure argument",
                            ));
                        }
                        let closure = &args[0];
                        let mut found = Value::None;
                        for item in arr.clone() {
                            let matches = self.call_closure_with_value(
                                closure,
                                vec![item.clone()],
                                bytecode,
                            )?;
                            if matches.is_truthy() {
                                found = item;
                                break;
                            }
                        }
                        self.stack.push(found);
                        return Ok(());
                    }
                    (Value::Array(arr), "any") | (Value::Vec(arr), "any") => {
                        if args.is_empty() {
                            return Err(ZyraError::runtime_error(
                                "any requires a closure argument",
                            ));
                        }
                        let closure = &args[0];
                        let mut found = false;
                        for item in arr.clone() {
                            let matches =
                                self.call_closure_with_value(closure, vec![item], bytecode)?;
                            if matches.is_truthy() {
                                found = true;
                                break;
                            }
                        }
                        self.stack.push(Value::Bool(found));
                        return Ok(());
                    }
                    (Value::Array(arr), "all") | (Value::Vec(arr), "all") => {
                        if args.is_empty() {
                            return Err(ZyraError::runtime_error(
                                "all requires a closure argument",
                            ));
                        }
                        let closure = &args[0];
                        let mut all_match = true;
                        for item in arr.clone() {
                            let matches =
                                self.call_closure_with_value(closure, vec![item], bytecode)?;
                            if !matches.is_truthy() {
                                all_match = false;
                                break;
                            }
                        }
                        self.stack.push(Value::Bool(all_match));
                        return Ok(());
                    }
                    _ => {}
                }

                // ===== OBJECT/STRUCT METHODS =====
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
                        // Phase 8: Access Control - NOW HANDLED AT COMPILE TIME
                        // The semantic analyzer's borrow checker enforces &mut self exclusivity
                        // This runtime check is kept only in debug builds as a verification layer
                        #[cfg(debug_assertions)]
                        {
                            let is_mutable = func
                                .params
                                .first()
                                .map(|p| p.contains("mut self"))
                                .unwrap_or(false);

                            if is_mutable {
                                if let Value::Ref(heap_id) = receiver {
                                    if let Some(heap_obj) = self.heap.get(heap_id) {
                                        // Debug assertion: compile-time should have caught violations
                                        // If this triggers, there's a gap in semantic analysis
                                        if heap_obj.ref_count > 3 {
                                            eprintln!(
                                                "[DEBUG] Runtime borrow check triggered: ref_count={} for &mut self method '{}'. \
                                                This should have been caught at compile time.",
                                                heap_obj.ref_count, method_name
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Prepend receiver to args for self parameter
                        let mut all_args = vec![receiver.clone()];
                        all_args.extend(args);
                        self.call_function(func, all_args)?;
                    } else {
                        // Fallback: Try to find trait implementation methods
                        // Trait methods are compiled as "<TraitName as Type>::method"
                        // Search for any function matching the pattern <* as Type>::method
                        let trait_method_suffix = format!(" as {}>::{}", type_name, method_name);

                        let trait_func = bytecode
                            .functions
                            .iter()
                            .find(|(name, _)| {
                                name.starts_with('<') && name.ends_with(&trait_method_suffix)
                            })
                            .map(|(_, func)| func);

                        if let Some(func) = trait_func {
                            // Found trait method implementation
                            let mut all_args = vec![receiver.clone()];
                            all_args.extend(args);
                            self.call_function(func, all_args)?;
                        } else {
                            return Err(ZyraError::runtime_error(&format!(
                                "Unknown method: '{}' on type '{}'. No inherent or trait implementation found.",
                                method_name, type_name
                            )));
                        }
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
                self.stack.push(Value::Array(elements));
            }

            Instruction::MakeVec(count) => {
                let mut elements = Vec::new();
                for _ in 0..*count {
                    elements.push(self.pop()?);
                }
                elements.reverse();
                self.stack.push(Value::Vec(elements));
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
                    Value::Array(l) | Value::Vec(l) => match field.as_str() {
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
                    (Value::Array(list), Value::Int(i)) | (Value::Vec(list), Value::Int(i)) => {
                        let idx = *i as usize;
                        if idx < list.len() {
                            self.stack.push(list[idx].clone());
                        } else {
                            return Err(ZyraError::runtime_error(&format!(
                                "Index {} out of bounds for array of length {}",
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

                if let (Value::Array(ref mut list), Value::Int(i)) = (&mut obj, &index) {
                    let idx = *i as usize;
                    if idx < list.len() {
                        list[idx] = value;
                    }
                } else if let (Value::Vec(ref mut list), Value::Int(i)) = (&mut obj, &index) {
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

            // Pattern matching support
            Instruction::Dup => {
                // Duplicate top of stack
                if let Some(top) = self.stack.last().cloned() {
                    if let Value::Ref(id) = &top {
                        let _ = self.heap.inc_ref(*id);
                    }
                    self.stack.push(top);
                }
            }

            Instruction::StrContains => {
                // Check if string contains substring: [haystack, needle] => bool
                let needle = self.pop()?;
                let haystack = self.pop()?;
                let result = match (&haystack, &needle) {
                    (Value::String(h), Value::String(n)) => Value::Bool(h.contains(n.as_str())),
                    _ => Value::Bool(false),
                };
                self.stack.push(result);
            }

            Instruction::Halt => {
                self.halted = true;
            }

            Instruction::Cast(target_type) => {
                let value = self.pop()?;
                let cast_value = self.cast_value(value, target_type)?;
                self.stack.push(cast_value);
            }

            Instruction::MakeClosure {
                func_name,
                param_count,
            } => {
                // Create a closure value that references the compiled function
                let closure = Value::Closure {
                    func_name: func_name.clone(),
                    param_count: *param_count,
                };
                self.stack.push(closure);
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

    /// Call a closure with given arguments and return the result
    /// This is used for higher-order functions like map, filter, fold
    fn call_closure_with_value(
        &mut self,
        closure: &Value,
        args: Vec<Value>,
        bytecode: &Bytecode,
    ) -> ZyraResult<Value> {
        if let Value::Closure {
            func_name,
            param_count,
        } = closure
        {
            // Verify argument count
            if args.len() != *param_count {
                return Err(ZyraError::runtime_error(&format!(
                    "Closure expected {} arguments, got {}",
                    param_count,
                    args.len()
                )));
            }

            // Look up the closure function
            if let Some(func) = bytecode.functions.get(func_name) {
                // Save state
                let saved_ip = self.ip;
                let saved_stack_len = self.stack.len();

                // Call the closure
                self.call_function(func, args)?;

                // Execute until return
                while self.ip < bytecode.instructions.len() && !self.halted {
                    let instr = &bytecode.instructions[self.ip];
                    self.ip += 1;

                    // Check for Return instruction
                    if matches!(instr, Instruction::Return) {
                        if let Some(frame) = self.call_stack.pop() {
                            // Get return value from stack
                            let return_value = if self.stack.len() > saved_stack_len {
                                self.pop()?
                            } else {
                                Value::None
                            };

                            // Restore state
                            self.ip = saved_ip;

                            // Clean up any leftover stack values
                            while self.scopes.len() > frame.base_pointer {
                                self.scopes.pop();
                            }

                            return Ok(return_value);
                        }
                    }

                    self.execute_instruction(instr, bytecode)?;
                }

                // If we get here without returning, return None
                Ok(Value::None)
            } else {
                Err(ZyraError::runtime_error(&format!(
                    "Closure function '{}' not found",
                    func_name
                )))
            }
        } else {
            Err(ZyraError::runtime_error("Expected a closure value"))
        }
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

    /// Cast a value to a target type at runtime
    fn cast_value(&self, value: Value, target_type: &str) -> ZyraResult<Value> {
        match target_type {
            // Integer casts
            "i8" => {
                let n = self.value_to_i64(&value)?;
                Ok(Value::I8(n as i8))
            }
            "i32" => {
                let n = self.value_to_i64(&value)?;
                Ok(Value::I32(n as i32))
            }
            "i64" | "Int" => {
                let n = self.value_to_i64(&value)?;
                Ok(Value::I64(n))
            }
            // Unsigned integer casts
            "u8" => {
                let n = self.value_to_i64(&value)?;
                Ok(Value::U8(n as u8))
            }
            "u32" => {
                let n = self.value_to_i64(&value)?;
                Ok(Value::U32(n as u32))
            }
            "u64" => {
                let n = self.value_to_i64(&value)?;
                Ok(Value::U64(n as u64))
            }
            // Float casts
            "f32" => {
                let n = self.value_to_f64(&value)?;
                Ok(Value::F32(n as f32))
            }
            "f64" | "Float" => {
                let n = self.value_to_f64(&value)?;
                Ok(Value::Float(n))
            }
            // Same type - return as-is
            _ => Ok(value),
        }
    }

    /// Helper to extract i64 from any numeric value
    fn value_to_i64(&self, value: &Value) -> ZyraResult<i64> {
        match value {
            Value::I8(n) => Ok(*n as i64),
            Value::I32(n) => Ok(*n as i64),
            Value::I64(n) | Value::Int(n) => Ok(*n),
            Value::U8(n) => Ok(*n as i64),
            Value::U32(n) => Ok(*n as i64),
            Value::U64(n) => Ok(*n as i64),
            Value::F32(n) => Ok(*n as i64),
            Value::F64(n) | Value::Float(n) => Ok(*n as i64),
            Value::Bool(b) => Ok(if *b { 1 } else { 0 }),
            Value::Char(c) => Ok(*c as i64),
            _ => Err(ZyraError::runtime_error("Cannot cast value to integer")),
        }
    }

    /// Helper to extract f64 from any numeric value
    fn value_to_f64(&self, value: &Value) -> ZyraResult<f64> {
        match value {
            Value::I8(n) => Ok(*n as f64),
            Value::I32(n) => Ok(*n as f64),
            Value::I64(n) | Value::Int(n) => Ok(*n as f64),
            Value::U8(n) => Ok(*n as f64),
            Value::U32(n) => Ok(*n as f64),
            Value::U64(n) => Ok(*n as f64),
            Value::F32(n) => Ok(*n as f64),
            Value::F64(n) | Value::Float(n) => Ok(*n),
            _ => Err(ZyraError::runtime_error("Cannot cast value to float")),
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
