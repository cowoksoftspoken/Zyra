//! Bytecode Compiler for Zyra
//!
//! Compiles AST to stack-based bytecode

pub mod bytecode;

pub use bytecode::{Bytecode, FunctionDef, Instruction, Value, WindowState};

use crate::error::{ZyraError, ZyraResult};
use crate::parser::ast::*;
use std::collections::HashSet;

/// Bytecode compiler
pub struct Compiler {
    bytecode: Bytecode,
    loop_starts: Vec<usize>,
    loop_ends: Vec<Vec<usize>>,
    /// Tracks which methods/functions are actually called (for dead code elimination)
    used_methods: HashSet<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            bytecode: Bytecode::new(),
            loop_starts: Vec::new(),
            loop_ends: Vec::new(),
            used_methods: HashSet::new(),
        }
    }

    /// Compile a program to bytecode
    pub fn compile(&mut self, program: &Program) -> ZyraResult<Bytecode> {
        // Pass 0: Collect used method/function names for dead code elimination
        self.collect_used_methods(&program.statements);

        // First pass: compile function definitions
        for stmt in &program.statements {
            if let Statement::Function {
                name, params, body, ..
            } = stmt
            {
                self.compile_function(name, params, body)?;
            }
        }

        // Second pass: compile top-level statements
        for stmt in &program.statements {
            match stmt {
                Statement::Function { .. } => {
                    // Already compiled
                }
                _ => {
                    self.compile_statement(stmt)?;
                }
            }
        }

        // Add halt instruction
        self.bytecode.emit(Instruction::Halt);

        Ok(self.bytecode.clone())
    }

    /// Collect all used method/function names from the AST (for dead code elimination)
    fn collect_used_methods(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.collect_from_statement(stmt);
        }
    }

    fn collect_from_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { value, .. } => self.collect_from_expression(value),
            Statement::Function { body, .. } => {
                for s in &body.statements {
                    self.collect_from_statement(s);
                }
                if let Some(expr) = &body.expression {
                    self.collect_from_expression(expr);
                }
            }
            Statement::Expression { expr, .. } => self.collect_from_expression(expr),
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_from_expression(expr);
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_from_expression(condition);
                for s in &then_block.statements {
                    self.collect_from_statement(s);
                }
                if let Some(expr) = &then_block.expression {
                    self.collect_from_expression(expr);
                }
                if let Some(else_blk) = else_block {
                    for s in &else_blk.statements {
                        self.collect_from_statement(s);
                    }
                    if let Some(expr) = &else_blk.expression {
                        self.collect_from_expression(expr);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            }
            | Statement::For {
                start: condition,
                body,
                ..
            } => {
                self.collect_from_expression(condition);
                for s in &body.statements {
                    self.collect_from_statement(s);
                }
                if let Some(expr) = &body.expression {
                    self.collect_from_expression(expr);
                }
            }
            Statement::Impl { methods, .. } => {
                for method in methods {
                    self.collect_from_statement(method);
                }
            }
            Statement::Block(block) => {
                for s in &block.statements {
                    self.collect_from_statement(s);
                }
                if let Some(expr) = &block.expression {
                    self.collect_from_expression(expr);
                }
            }
            _ => {}
        }
    }

    fn collect_from_expression(&mut self, expr: &Expression) {
        match expr {
            // Function call: func_name(...) or object.method(...)
            Expression::Call {
                callee, arguments, ..
            } => {
                // Extract function/method name from callee
                if let Expression::Identifier { name, .. } = callee.as_ref() {
                    self.used_methods.insert(name.clone());
                } else if let Expression::FieldAccess { object, field, .. } = callee.as_ref() {
                    // Could be either:
                    // 1. Static method call: Type::method (object is type identifier)
                    // 2. Instance method call: obj.method() (object is variable)
                    // We record both the full name AND just the method name to be safe
                    if let Expression::Identifier { name, .. } = object.as_ref() {
                        let method_name = format!("{}::{}", name, field);
                        self.used_methods.insert(method_name);
                    }
                    // Also record just the method name (for instance method calls)
                    self.used_methods.insert(field.clone());
                    // Recurse into the object
                    self.collect_from_expression(object);
                }
                // Recurse into arguments
                for arg in arguments {
                    self.collect_from_expression(arg);
                }
            }

            Expression::Binary { left, right, .. } => {
                self.collect_from_expression(left);
                self.collect_from_expression(right);
            }
            Expression::Unary { operand, .. } => {
                self.collect_from_expression(operand);
            }
            Expression::Assignment { value, .. } => {
                self.collect_from_expression(value);
            }
            Expression::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_from_expression(condition);
                for s in &then_block.statements {
                    self.collect_from_statement(s);
                }
                if let Some(expr) = &then_block.expression {
                    self.collect_from_expression(expr);
                }
                if let Some(else_blk) = else_block {
                    for s in &else_blk.statements {
                        self.collect_from_statement(s);
                    }
                    if let Some(expr) = &else_blk.expression {
                        self.collect_from_expression(expr);
                    }
                }
            }

            Expression::Object { fields, .. } => {
                for (_, field_expr) in fields {
                    self.collect_from_expression(field_expr);
                }
            }
            Expression::List { elements, .. } => {
                for elem in elements {
                    self.collect_from_expression(elem);
                }
            }
            Expression::Index { object, index, .. } => {
                self.collect_from_expression(object);
                self.collect_from_expression(index);
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_from_expression(object);
            }
            Expression::Reference { value, .. } | Expression::Dereference { value, .. } => {
                self.collect_from_expression(value);
            }
            _ => {}
        }
    }

    fn compile_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        body: &Block,
    ) -> ZyraResult<()> {
        let start_address = self.bytecode.current_address();

        // Enter function scope
        self.bytecode.emit(Instruction::EnterScope);

        // Parameters are passed on the stack, store them in order (first arg is deepest)
        for param in params.iter() {
            // Normalize self parameter names: &self, &mut self, mut self -> self
            let var_name = if param.name == "&self"
                || param.name == "&mut self"
                || param.name == "mut self"
                || param.name == "self"
            {
                "self".to_string()
            } else if param.name.starts_with("mut ") {
                // Handle 'mut name' - strip the mut prefix for storage
                param.name[4..].to_string()
            } else {
                param.name.clone()
            };
            self.bytecode.emit(Instruction::StoreVar(var_name));
        }

        // Compile body
        self.compile_block(body)?;

        // Implicit return if no explicit return
        self.bytecode.emit(Instruction::Return);

        let end_address = self.bytecode.current_address();

        // Register function
        self.bytecode.functions.insert(
            name.to_string(),
            FunctionDef {
                name: name.to_string(),
                params: params.iter().map(|p| p.name.clone()).collect(),
                start_address,
                end_address,
            },
        );

        Ok(())
    }

    fn compile_statement(&mut self, stmt: &Statement) -> ZyraResult<()> {
        match stmt {
            Statement::Let { name, value, .. } => {
                self.compile_expression(value)?;
                self.bytecode.emit(Instruction::StoreVar(name.clone()));
                Ok(())
            }

            Statement::Function { .. } => {
                // Functions are compiled in the first pass
                Ok(())
            }

            Statement::Expression { expr, .. } => {
                // Check if this is an assignment expression - assignments don't leave a value on stack
                // Because StoreVar consumes the value without pushing anything back
                let is_assignment = matches!(expr, Expression::Assignment { .. });
                self.compile_expression(expr)?;
                // Pop the result for expressions that leave a value on the stack
                // Assignments don't leave a value (StoreVar consumes it), so don't pop
                if !is_assignment {
                    self.bytecode.emit(Instruction::Pop);
                }
                Ok(())
            }

            Statement::Import { .. } => {
                // Imports are handled at compile time
                Ok(())
            }

            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.compile_expression(expr)?;
                } else {
                    self.bytecode.emit(Instruction::LoadConst(Value::None));
                }
                self.bytecode.emit(Instruction::Return);
                Ok(())
            }

            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.compile_expression(condition)?;

                // Jump to else or end if condition is false
                let jump_to_else = self.bytecode.emit(Instruction::JumpIfFalse(0));

                // Compile then block
                self.compile_block(then_block)?;

                if let Some(else_blk) = else_block {
                    // Jump over else block
                    let jump_over_else = self.bytecode.emit(Instruction::Jump(0));

                    // Patch jump to else
                    let else_start = self.bytecode.current_address();
                    self.bytecode.patch_jump(jump_to_else, else_start);

                    // Compile else block
                    self.compile_block(else_blk)?;

                    // Patch jump over else
                    let end = self.bytecode.current_address();
                    self.bytecode.patch_jump(jump_over_else, end);
                } else {
                    // Patch jump to end
                    let end = self.bytecode.current_address();
                    self.bytecode.patch_jump(jump_to_else, end);
                }

                Ok(())
            }

            Statement::While {
                condition, body, ..
            } => {
                let loop_start = self.bytecode.current_address();
                self.loop_starts.push(loop_start);
                self.loop_ends.push(Vec::new());

                // Compile condition
                self.compile_expression(condition)?;

                // Jump to end if false
                let jump_to_end = self.bytecode.emit(Instruction::JumpIfFalse(0));

                // Compile body
                self.bytecode.emit(Instruction::EnterScope);
                self.compile_block(body)?;
                self.bytecode.emit(Instruction::ExitScope);

                // Jump back to start
                self.bytecode.emit(Instruction::Jump(loop_start));

                // Patch jump to end
                let loop_end = self.bytecode.current_address();
                self.bytecode.patch_jump(jump_to_end, loop_end);

                // Patch any break statements
                let breaks = self.loop_ends.pop().unwrap();
                for addr in breaks {
                    self.bytecode.patch_jump(addr, loop_end);
                }
                self.loop_starts.pop();

                Ok(())
            }

            Statement::For {
                variable,
                start,
                end,
                inclusive,
                body,
                ..
            } => {
                self.bytecode.emit(Instruction::EnterScope);

                // Initialize loop variable
                self.compile_expression(start)?;
                self.bytecode.emit(Instruction::StoreVar(variable.clone()));

                // Compile end value
                self.compile_expression(end)?;
                self.bytecode
                    .emit(Instruction::StoreVar("__loop_end".to_string()));

                let loop_start = self.bytecode.current_address();
                self.loop_starts.push(loop_start);
                self.loop_ends.push(Vec::new());

                // Check condition: variable < end (or <= for inclusive)
                self.bytecode.emit(Instruction::LoadVar(variable.clone()));
                self.bytecode
                    .emit(Instruction::LoadVar("__loop_end".to_string()));
                if *inclusive {
                    self.bytecode.emit(Instruction::Lte);
                } else {
                    self.bytecode.emit(Instruction::Lt);
                }

                let jump_to_end = self.bytecode.emit(Instruction::JumpIfFalse(0));

                // Compile body
                self.compile_block(body)?;

                // Increment loop variable
                self.bytecode.emit(Instruction::LoadVar(variable.clone()));
                self.bytecode.emit(Instruction::LoadConst(Value::Int(1)));
                self.bytecode.emit(Instruction::Add);
                self.bytecode.emit(Instruction::StoreVar(variable.clone()));

                // Jump back to start
                self.bytecode.emit(Instruction::Jump(loop_start));

                // Patch jumps
                let loop_end = self.bytecode.current_address();
                self.bytecode.patch_jump(jump_to_end, loop_end);

                let breaks = self.loop_ends.pop().unwrap();
                for addr in breaks {
                    self.bytecode.patch_jump(addr, loop_end);
                }
                self.loop_starts.pop();

                self.bytecode.emit(Instruction::ExitScope);

                Ok(())
            }

            Statement::Block(block) => {
                self.bytecode.emit(Instruction::EnterScope);
                self.compile_block(block)?;
                self.bytecode.emit(Instruction::ExitScope);
                Ok(())
            }

            // Type definitions - these don't generate bytecode directly
            // They're registered in a type registry during semantic analysis
            Statement::Struct { .. } => {
                // Struct definitions are handled at compile time
                Ok(())
            }

            Statement::Enum { .. } => {
                // Enum definitions are handled at compile time
                Ok(())
            }

            Statement::Impl {
                target_type,
                trait_name,
                methods,
                ..
            } => {
                // Compile impl methods as functions with namespaced names
                for method in methods {
                    // Extract function name and compile with prefixed name
                    if let Statement::Function {
                        name, params, body, ..
                    } = method.as_ref()
                    {
                        // Generate method name based on impl type:
                        // - impl Type { method }         -> Type::method
                        // - impl Trait for Type { method } -> <Trait as Type>::method
                        let prefixed_name = if let Some(trait_n) = trait_name {
                            // Trait impl: use <TraitName as Type>::method format
                            format!("<{} as {}>::{}", trait_n, target_type, name)
                        } else {
                            // Inherent impl: use Type::method format
                            format!("{}::{}", target_type, name)
                        };

                        // Dead Code Elimination: Skip compiling if method is not used
                        // Conservative approach for VM trait fallback safety:
                        // - For inherent impls: check Type::method format
                        // - For trait impls: include if EITHER:
                        //   1. Exact trait name is used (<Trait as Type>::method)
                        //   2. Method name only is used (e.g., "update")
                        //   3. Inherent method is used (Type::method) - VM fallback may resolve to trait
                        let inherent_method_name = format!("{}::{}", target_type, name);
                        let is_used = self.used_methods.contains(&prefixed_name)
                            || self.used_methods.contains(name)
                            || self.used_methods.contains(&inherent_method_name)
                            // For trait impls, also check if any variant is called
                            || (trait_name.is_some() && self.used_methods.iter().any(|m| {
                                // Check if any used method ends with ::methodname for this type
                                m.ends_with(&format!("::{}", name)) && 
                                (m.contains(target_type) || m.starts_with('<'))
                            }));

                        if is_used {
                            self.compile_function(&prefixed_name, params, body)?;
                        }
                        // If not used, skip compilation (dead code elimination)
                    } else {
                        self.compile_statement(method)?;
                    }
                }
                Ok(())
            }

            Statement::Trait { .. } => {
                // Trait definitions are handled at compile time
                Ok(())
            }
        }
    }

    fn compile_block(&mut self, block: &Block) -> ZyraResult<()> {
        for stmt in &block.statements {
            self.compile_statement(stmt)?;
        }

        if let Some(ref expr) = block.expression {
            self.compile_expression(expr)?;
        }

        Ok(())
    }

    fn compile_expression(&mut self, expr: &Expression) -> ZyraResult<()> {
        match expr {
            Expression::Int { value, .. } => {
                self.bytecode
                    .emit(Instruction::LoadConst(Value::Int(*value)));
                Ok(())
            }

            Expression::Float { value, .. } => {
                self.bytecode
                    .emit(Instruction::LoadConst(Value::Float(*value)));
                Ok(())
            }

            Expression::Bool { value, .. } => {
                self.bytecode
                    .emit(Instruction::LoadConst(Value::Bool(*value)));
                Ok(())
            }

            Expression::Char { value, .. } => {
                self.bytecode
                    .emit(Instruction::LoadConst(Value::Char(*value)));
                Ok(())
            }

            Expression::String { value, .. } => {
                self.bytecode
                    .emit(Instruction::LoadConst(Value::String(value.clone())));
                Ok(())
            }

            Expression::Identifier { name, .. } => {
                self.bytecode.emit(Instruction::LoadVar(name.clone()));
                Ok(())
            }

            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => {
                self.compile_expression(left)?;
                self.compile_expression(right)?;

                let instruction = match operator {
                    BinaryOp::Add => Instruction::Add,
                    BinaryOp::Subtract => Instruction::Sub,
                    BinaryOp::Multiply => Instruction::Mul,
                    BinaryOp::Divide => Instruction::Div,
                    BinaryOp::Modulo => Instruction::Mod,
                    BinaryOp::Equal => Instruction::Eq,
                    BinaryOp::NotEqual => Instruction::Neq,
                    BinaryOp::Less => Instruction::Lt,
                    BinaryOp::LessEqual => Instruction::Lte,
                    BinaryOp::Greater => Instruction::Gt,
                    BinaryOp::GreaterEqual => Instruction::Gte,
                    BinaryOp::And => Instruction::And,
                    BinaryOp::Or => Instruction::Or,
                };

                self.bytecode.emit(instruction);
                Ok(())
            }

            Expression::Unary {
                operator, operand, ..
            } => {
                self.compile_expression(operand)?;

                let instruction = match operator {
                    UnaryOp::Negate => Instruction::Neg,
                    UnaryOp::Not => Instruction::Not,
                };

                self.bytecode.emit(instruction);
                Ok(())
            }

            Expression::Assignment { target, value, .. } => {
                self.compile_expression(value)?;

                match target.as_ref() {
                    Expression::Identifier { name, .. } => {
                        self.bytecode.emit(Instruction::StoreVar(name.clone()));
                    }
                    Expression::FieldAccess { object, field, .. } => {
                        self.compile_expression(object)?;
                        self.bytecode.emit(Instruction::SetField(field.clone()));
                    }
                    Expression::Index { object, index, .. } => {
                        // For nested index assignment like `matrix[0][0] = 10`:
                        // We need to:
                        // 1. Collect all indices from innermost to outermost
                        // 2. Load the root variable
                        // 3. For each level except the last: GetIndex to navigate deeper
                        // 4. SetIndex with the value at the deepest level
                        // 5. Propagate changes back up by SetIndex at each level
                        // 6. StoreVar back to root

                        // Collect indices from outermost to innermost
                        fn collect_indices(
                            expr: &Expression,
                            indices: &mut Vec<Expression>,
                        ) -> Option<String> {
                            match expr {
                                Expression::Identifier { name, .. } => Some(name.clone()),
                                Expression::Index { object, index, .. } => {
                                    indices.push((**index).clone());
                                    collect_indices(object, indices)
                                }
                                _ => None,
                            }
                        }

                        let mut indices = vec![(**index).clone()];
                        let root_name = collect_indices(object, &mut indices);

                        // Reverse to get from root to deepest
                        indices.reverse();

                        if let Some(root) = root_name {
                            // Value is already on stack (compiled before target)

                            if indices.len() == 1 {
                                // Simple case: arr[i] = value
                                // Stack: [value]
                                // Need: [value, arr, i] for SetIndex
                                self.bytecode.emit(Instruction::LoadVar(root.clone()));
                                self.compile_expression(&indices[0])?;
                                self.bytecode.emit(Instruction::SetIndex);
                                self.bytecode.emit(Instruction::StoreVar(root));
                            } else {
                                // Nested case: matrix[i][j] = value (or deeper)
                                // Stack: [value]

                                // Load root and navigate to second-to-last level
                                self.bytecode.emit(Instruction::LoadVar(root.clone()));
                                for idx_expr in &indices[..indices.len() - 1] {
                                    self.compile_expression(idx_expr)?;
                                    self.bytecode.emit(Instruction::GetIndex);
                                }
                                // Stack: [value, inner_array]

                                // Now set at the deepest level
                                // We need: [value, inner_array, deepest_index]
                                // But value is at bottom, inner_array is at top
                                // We need to reorder: compile index, swap, then SetIndex
                                self.compile_expression(&indices[indices.len() - 1])?;
                                // Stack: [value, inner_array, deepest_index]
                                // But SetIndex expects [value, obj, idx] in order: pop idx, pop obj, pop value
                                // Our stack: bottom->[value], [inner_array], [deepest_index]<-top
                                // This is: idx at top, obj below, value at bottom - correct order!
                                self.bytecode.emit(Instruction::SetIndex);
                                // Stack: [modified_inner_array]

                                // Now propagate back up - for each level from second-deepest back to root
                                // We need to: load parent, swap with modified child, set child at index, store
                                // This is complex - for now let's handle 2-level nesting
                                // For matrix[i][j], after modifying row, we need to set it back

                                // Load root again, set the modified inner at first index
                                self.bytecode.emit(Instruction::LoadVar(root.clone()));
                                // Stack: [modified_inner, matrix]
                                // We need [modified_inner, matrix, first_index] then swap/reorder for SetIndex
                                self.compile_expression(&indices[0])?;
                                // Stack: [modified_inner, matrix, first_index]
                                // SetIndex pops: idx, obj, value -> gives us modified obj
                                // But our stack has modified_inner at bottom, not at "value" position

                                // We need to restructure: SetIndex wants [value_to_set, container, index]
                                // We have [modified_inner, matrix, first_index]
                                // This is already correct order!
                                self.bytecode.emit(Instruction::SetIndex);
                                // Stack: [modified_matrix]

                                self.bytecode.emit(Instruction::StoreVar(root));
                            }
                        }
                    }
                    _ => {
                        return Err(ZyraError::runtime_error("Invalid assignment target"));
                    }
                }

                Ok(())
            }

            Expression::Call {
                callee, arguments, ..
            } => {
                // Get function name and handle method calls specially
                match callee.as_ref() {
                    Expression::Identifier { name, .. } => {
                        // Regular function call: compile arguments then call
                        for arg in arguments {
                            self.compile_expression(arg)?;
                        }
                        self.bytecode
                            .emit(Instruction::Call(name.clone(), arguments.len()));
                    }
                    Expression::FieldAccess { object, field, .. } => {
                        // Method call: push receiver FIRST, then arguments
                        // VM expects: [receiver, arg1, arg2, ...] on stack
                        self.compile_expression(object)?;
                        for arg in arguments {
                            self.compile_expression(arg)?;
                        }
                        // Emit MethodCall with method name and arg count (not including receiver)
                        self.bytecode
                            .emit(Instruction::MethodCall(field.clone(), arguments.len()));
                    }
                    _ => {
                        return Err(ZyraError::runtime_error("Invalid call target"));
                    }
                }

                Ok(())
            }

            Expression::FieldAccess { object, field, .. } => {
                self.compile_expression(object)?;
                self.bytecode.emit(Instruction::GetField(field.clone()));
                Ok(())
            }

            Expression::Index { object, index, .. } => {
                self.compile_expression(object)?;
                self.compile_expression(index)?;
                self.bytecode.emit(Instruction::GetIndex);
                Ok(())
            }

            Expression::List { elements, .. } => {
                // Array literal [a, b, c] - compiles to Value::Array
                for elem in elements {
                    self.compile_expression(elem)?;
                }
                self.bytecode.emit(Instruction::MakeList(elements.len()));
                Ok(())
            }

            Expression::VecLiteral { elements, .. } => {
                // Vec literal vec[a, b, c] - compiles to Value::Vec
                for elem in elements {
                    self.compile_expression(elem)?;
                }
                self.bytecode.emit(Instruction::MakeVec(elements.len()));
                Ok(())
            }

            Expression::Object { fields, .. } => {
                for (name, value) in fields {
                    self.bytecode
                        .emit(Instruction::LoadConst(Value::String(name.clone())));
                    self.compile_expression(value)?;
                }
                self.bytecode.emit(Instruction::MakeObject(fields.len()));
                Ok(())
            }

            Expression::Reference {
                mutable: _mutable,
                value,
                ..
            } => {
                // Auto-dereference: references load the actual value
                // The borrow checking is done at compile-time in semantic analysis
                // At runtime, references behave like the value they point to
                if let Expression::Identifier { name, .. } = value.as_ref() {
                    self.bytecode.emit(Instruction::LoadVar(name.clone()));
                } else {
                    self.compile_expression(value)?;
                }
                Ok(())
            }

            Expression::Dereference { value, .. } => {
                self.compile_expression(value)?;
                // Dereference is handled at runtime
                Ok(())
            }

            Expression::Range { start, end, .. } => {
                // Ranges are typically used in for loops, handled there
                self.compile_expression(start)?;
                self.compile_expression(end)?;
                Ok(())
            }

            Expression::Grouped { inner, .. } => self.compile_expression(inner),

            // If expression - returns value from branch
            Expression::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.compile_expression(condition)?;

                // Jump to else or end if condition is false
                let jump_to_else = self.bytecode.emit(Instruction::JumpIfFalse(0));

                // Compile then block - leaves value on stack
                self.compile_block(then_block)?;

                if let Some(else_blk) = else_block {
                    // Jump over else block
                    let jump_over_else = self.bytecode.emit(Instruction::Jump(0));

                    // Patch jump to else
                    let else_start = self.bytecode.current_address();
                    self.bytecode.patch_jump(jump_to_else, else_start);

                    // Compile else block - leaves value on stack
                    self.compile_block(else_blk)?;

                    // Patch jump over else
                    let end = self.bytecode.current_address();
                    self.bytecode.patch_jump(jump_over_else, end);
                } else {
                    // No else block - push None/Null as result
                    let end = self.bytecode.current_address();
                    self.bytecode.patch_jump(jump_to_else, end);
                    self.bytecode.emit(Instruction::LoadConst(Value::None));
                }

                Ok(())
            }

            // Struct instantiation: StructName { field: value, ... }
            Expression::StructInit { name, fields, .. } => {
                // Create an Object value with _type field for struct name
                // VM pops: value first, then key. So push: key first, then value
                for (field_name, field_value) in fields {
                    // Push key first (will be popped second)
                    self.bytecode
                        .emit(Instruction::LoadConst(Value::String(field_name.clone())));
                    // Push value second (will be popped first)
                    self.compile_expression(field_value)?;
                }
                // Include struct type as _type field (key first, then value)
                self.bytecode
                    .emit(Instruction::LoadConst(Value::String("_type".to_string())));
                self.bytecode
                    .emit(Instruction::LoadConst(Value::String(name.clone())));
                // Create object with field count + 1 for _type field
                self.bytecode
                    .emit(Instruction::MakeObject(fields.len() + 1));
                Ok(())
            }

            // Enum variant: EnumName::Variant or EnumName::Variant(data)
            Expression::EnumVariant {
                enum_name,
                variant,
                data,
                ..
            } => {
                // Create object with _type = "EnumName::Variant" and optional _data field
                // Push variant info
                self.bytecode
                    .emit(Instruction::LoadConst(Value::String("_type".to_string())));
                self.bytecode
                    .emit(Instruction::LoadConst(Value::String(format!(
                        "{}::{}",
                        enum_name, variant
                    ))));

                let mut field_count = 1;

                if let Some(data_expr) = data {
                    // Push data field
                    self.bytecode
                        .emit(Instruction::LoadConst(Value::String("_data".to_string())));
                    self.compile_expression(data_expr)?;
                    field_count = 2;
                }

                self.bytecode.emit(Instruction::MakeObject(field_count));
                Ok(())
            }

            // Match expression: match scrutinee { pattern => body, ... }
            Expression::Match {
                scrutinee,
                arms,
                span,
            } => {
                // Phase 1: Compile scrutinee - leaves value on stack
                self.compile_expression(scrutinee)?;

                // Store scrutinee in a temp variable for pattern matching
                let scrutinee_var =
                    format!("__match_scrutinee_{}", self.bytecode.current_address());
                self.bytecode
                    .emit(Instruction::StoreVar(scrutinee_var.clone()));

                // Track jump addresses for patching
                let mut end_jumps: Vec<usize> = Vec::new();

                // Compile each arm as a chained conditional
                for (_i, arm) in arms.iter().enumerate() {
                    // Load scrutinee for pattern check
                    self.bytecode
                        .emit(Instruction::LoadVar(scrutinee_var.clone()));

                    // Compile pattern matching check
                    self.compile_pattern_check(&arm.pattern)?;

                    // If guard present, add guard check
                    if let Some(ref guard) = arm.guard {
                        // Only check guard if pattern matched
                        let skip_guard = self.bytecode.emit(Instruction::JumpIfFalse(0));
                        self.compile_expression(guard)?;
                        // Combine pattern match and guard result (already on stack from guard)
                        let guard_end = self.bytecode.current_address();
                        self.bytecode.patch_jump(skip_guard, guard_end);
                    }

                    // Jump to next arm if pattern doesn't match
                    let jump_to_next = self.bytecode.emit(Instruction::JumpIfFalse(0));

                    // Pattern matched - bind variables and compile body
                    self.bytecode
                        .emit(Instruction::LoadVar(scrutinee_var.clone()));
                    self.compile_pattern_bindings(&arm.pattern)?;
                    self.compile_expression(&arm.body)?;

                    // Jump to end after successful match
                    let jump_to_end = self.bytecode.emit(Instruction::Jump(0));
                    end_jumps.push(jump_to_end);

                    // Patch jump to next arm
                    let next_arm_addr = self.bytecode.current_address();
                    self.bytecode.patch_jump(jump_to_next, next_arm_addr);
                }

                // If no arm matched, push error value (match exhaustiveness should prevent this)
                self.bytecode.emit(Instruction::LoadConst(Value::None));

                // Patch all end jumps to here
                let end_addr = self.bytecode.current_address();
                for jump in end_jumps {
                    self.bytecode.patch_jump(jump, end_addr);
                }

                let _ = span; // Mark span as used
                Ok(())
            }

            // Type cast expression: expr as Type
            Expression::Cast {
                expr, target_type, ..
            } => {
                // Compile the expression to cast
                self.compile_expression(expr)?;

                // Get target type name for VM cast instruction
                use crate::semantic::types::ZyraType;
                let target = ZyraType::from_ast_type(target_type);
                let type_name = target.display_name();

                // Emit cast instruction
                self.bytecode.emit(Instruction::Cast(type_name));
                Ok(())
            }

            // Closure expression: |params| body
            Expression::Closure { params, body, .. } => {
                // Generate unique closure function name
                static CLOSURE_COUNTER: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                let closure_id = CLOSURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let func_name = format!("__closure_{}", closure_id);

                // Compile closure as a function:
                // 1. Jump over the closure body (it will be called later)
                let jump_over = self.bytecode.emit(Instruction::Jump(0));

                // 2. Record function start
                let func_start = self.bytecode.current_address();

                // 3. Enter scope and store parameters
                self.bytecode.emit(Instruction::EnterScope);

                // Store params in reverse order (they're on stack from caller)
                for param in params.iter().rev() {
                    self.bytecode
                        .emit(Instruction::StoreVar(param.name.clone()));
                }

                // 4. Compile closure body
                self.compile_expression(body)?;

                // 5. Return (exit scope and return to caller)
                self.bytecode.emit(Instruction::ExitScope);
                self.bytecode.emit(Instruction::Return);

                // 6. Record function end
                let func_end = self.bytecode.current_address();

                // 7. Patch jump to skip over closure body
                self.bytecode.patch_jump(jump_over, func_end);

                // 8. Register closure as a function
                self.bytecode.functions.insert(
                    func_name.clone(),
                    FunctionDef {
                        name: func_name.clone(),
                        params: params.iter().map(|p| p.name.clone()).collect(),
                        start_address: func_start,
                        end_address: func_end,
                    },
                );

                // 9. Emit MakeClosure instruction to create the closure value
                self.bytecode.emit(Instruction::MakeClosure {
                    func_name,
                    param_count: params.len(),
                });

                Ok(())
            }
        }
    }

    /// Compile pattern matching check - leaves bool on stack
    fn compile_pattern_check(&mut self, pattern: &crate::parser::ast::Pattern) -> ZyraResult<()> {
        use crate::parser::ast::Pattern;
        match pattern {
            Pattern::Wildcard { .. } => {
                // Wildcard always matches
                self.bytecode.emit(Instruction::Pop); // Remove scrutinee
                self.bytecode
                    .emit(Instruction::LoadConst(Value::Bool(true)));
            }
            Pattern::Identifier { .. } => {
                // Simple binding always matches
                self.bytecode.emit(Instruction::Pop);
                self.bytecode
                    .emit(Instruction::LoadConst(Value::Bool(true)));
            }
            Pattern::RefBinding { .. } => {
                // Ref binding always matches
                self.bytecode.emit(Instruction::Pop);
                self.bytecode
                    .emit(Instruction::LoadConst(Value::Bool(true)));
            }
            Pattern::Literal { value, .. } => {
                // Compare with literal
                use crate::parser::ast::LiteralPattern;
                match value {
                    LiteralPattern::Int(n) => {
                        self.bytecode.emit(Instruction::LoadConst(Value::Int(*n)));
                    }
                    LiteralPattern::Float(f) => {
                        self.bytecode.emit(Instruction::LoadConst(Value::Float(*f)));
                    }
                    LiteralPattern::Bool(b) => {
                        self.bytecode.emit(Instruction::LoadConst(Value::Bool(*b)));
                    }
                    LiteralPattern::Char(c) => {
                        self.bytecode.emit(Instruction::LoadConst(Value::Char(*c)));
                    }
                    LiteralPattern::String(s) => {
                        self.bytecode
                            .emit(Instruction::LoadConst(Value::String(s.clone())));
                    }
                }
                self.bytecode.emit(Instruction::Eq);
            }
            Pattern::Variant { variant, inner, .. } => {
                // Check if scrutinee._type ends with variant name
                self.bytecode
                    .emit(Instruction::GetField("_type".to_string()));
                self.bytecode
                    .emit(Instruction::LoadConst(Value::String(variant.clone())));
                self.bytecode.emit(Instruction::StrContains);
                // TODO: Check inner pattern if present
                let _ = inner;
            }
            Pattern::Struct { type_name, .. } => {
                // Check if scrutinee._type matches struct type
                self.bytecode
                    .emit(Instruction::GetField("_type".to_string()));
                self.bytecode
                    .emit(Instruction::LoadConst(Value::String(type_name.clone())));
                self.bytecode.emit(Instruction::Eq);
            }
            Pattern::Tuple { .. } => {
                // TODO: Tuple pattern matching
                self.bytecode.emit(Instruction::Pop);
                self.bytecode
                    .emit(Instruction::LoadConst(Value::Bool(true)));
            }
        }
        Ok(())
    }

    /// Compile pattern variable bindings
    fn compile_pattern_bindings(
        &mut self,
        pattern: &crate::parser::ast::Pattern,
    ) -> ZyraResult<()> {
        use crate::parser::ast::Pattern;
        match pattern {
            Pattern::Identifier { name, .. } => {
                // Bind the value to the variable name
                self.bytecode.emit(Instruction::StoreVar(name.clone()));
            }
            Pattern::RefBinding { name, .. } => {
                // Bind as reference (same as regular for now)
                self.bytecode.emit(Instruction::StoreVar(name.clone()));
            }
            Pattern::Struct { fields, .. } => {
                // Extract and bind each field
                for field in fields {
                    // Duplicate scrutinee for each field
                    self.bytecode.emit(Instruction::Dup);
                    self.bytecode
                        .emit(Instruction::GetField(field.field_name.clone()));
                    self.compile_pattern_bindings(&field.pattern)?;
                }
                self.bytecode.emit(Instruction::Pop); // Remove final scrutinee copy
            }
            Pattern::Variant { inner, .. } => {
                if let Some(inner_pattern) = inner {
                    // Extract _data and bind
                    self.bytecode
                        .emit(Instruction::GetField("_data".to_string()));
                    self.compile_pattern_bindings(inner_pattern)?;
                } else {
                    self.bytecode.emit(Instruction::Pop);
                }
            }
            _ => {
                // Wildcard, Literal, Tuple - no bindings
                self.bytecode.emit(Instruction::Pop);
            }
        }
        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
