//! Bytecode Compiler for Zyra
//!
//! Compiles AST to stack-based bytecode

pub mod bytecode;

pub use bytecode::{Bytecode, FunctionDef, Instruction, Value, WindowState};

use crate::error::{ZyraError, ZyraResult};
use crate::parser::ast::*;

/// Bytecode compiler
pub struct Compiler {
    bytecode: Bytecode,
    loop_starts: Vec<usize>,
    loop_ends: Vec<Vec<usize>>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            bytecode: Bytecode::new(),
            loop_starts: Vec::new(),
            loop_ends: Vec::new(),
        }
    }

    /// Compile a program to bytecode
    pub fn compile(&mut self, program: &Program) -> ZyraResult<Bytecode> {
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

    fn compile_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        body: &Block,
    ) -> ZyraResult<()> {
        let start_address = self.bytecode.current_address();

        // Enter function scope
        self.bytecode.emit(Instruction::EnterScope);

        // Parameters are passed on the stack, store them in reverse order
        for param in params.iter().rev() {
            self.bytecode
                .emit(Instruction::StoreVar(param.name.clone()));
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

            Statement::Impl { methods, .. } => {
                // Compile impl methods as functions
                for method in methods {
                    self.compile_statement(method)?;
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
                        // For index assignment like `arr[0] = 99`:
                        // Stack before SetIndex: [value, object, index]
                        // After SetIndex: [modified_object]
                        // We need to store it back to the variable
                        self.compile_expression(object)?;
                        self.compile_expression(index)?;
                        self.bytecode.emit(Instruction::SetIndex);

                        // Store the modified object back to the variable
                        if let Expression::Identifier { name, .. } = object.as_ref() {
                            self.bytecode.emit(Instruction::StoreVar(name.clone()));
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
                // Compile arguments
                for arg in arguments {
                    self.compile_expression(arg)?;
                }

                // Get function name
                match callee.as_ref() {
                    Expression::Identifier { name, .. } => {
                        self.bytecode
                            .emit(Instruction::Call(name.clone(), arguments.len()));
                    }
                    Expression::FieldAccess { object, field, .. } => {
                        // Method call: push object as first argument
                        self.compile_expression(object)?;
                        let full_name = if let Expression::Identifier { name, .. } = object.as_ref()
                        {
                            format!("{}.{}", name, field)
                        } else {
                            field.clone()
                        };
                        self.bytecode
                            .emit(Instruction::Call(full_name, arguments.len() + 1));
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
                for elem in elements {
                    self.compile_expression(elem)?;
                }
                self.bytecode.emit(Instruction::MakeList(elements.len()));
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
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
