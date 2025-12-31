//! Zyra Parser
//!
//! Recursive descent parser implementing the Zyra BNF grammar

pub mod ast;

pub use ast::*;

use crate::error::{SourceLocation, ZyraError, ZyraResult};
use crate::lexer::{Span, Token, TokenKind};

/// Parser for Zyra source code
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Parse the token stream into an AST
    pub fn parse(&mut self) -> ZyraResult<Program> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    // ===== Statement Parsing =====

    fn parse_statement(&mut self) -> ZyraResult<Statement> {
        match self.peek().kind {
            TokenKind::Let => self.parse_let_statement(),
            TokenKind::Func => self.parse_function(),
            TokenKind::Import => self.parse_import(),
            TokenKind::Return => self.parse_return(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Struct => self.parse_struct(),
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Impl => self.parse_impl(),
            TokenKind::Trait => self.parse_trait(),
            TokenKind::LeftBrace => {
                let block = self.parse_block()?;
                Ok(Statement::Block(block))
            }
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'let'

        // Check for 'mut'
        let mutable = if self.check(&TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };

        // Parse variable name
        let name = self.expect_identifier("Expected variable name after 'let'")?;

        // Optional type annotation
        let type_annotation = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Expect '='
        self.expect(&TokenKind::Equal, "Expected '=' in let statement")?;

        // Parse value expression
        let value = self.parse_expression()?;

        // Expect ';'
        self.expect(&TokenKind::Semicolon, "Expected ';' after let statement")?;

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Statement::Let {
            name,
            mutable,
            type_annotation,
            value,
            span,
        })
    }

    fn parse_function(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'func'

        // Parse function name
        let name = self.expect_identifier("Expected function name after 'func'")?;

        // Optional lifetime parameters: <'a, 'b>
        let lifetimes = if self.check(&TokenKind::Less) {
            self.advance();
            let mut lifetimes = Vec::new();

            loop {
                if let TokenKind::Lifetime(lt) = &self.peek().kind {
                    lifetimes.push(lt.clone());
                    self.advance();
                } else {
                    break;
                }

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }

            self.expect(
                &TokenKind::Greater,
                "Expected '>' after lifetime parameters",
            )?;
            lifetimes
        } else {
            Vec::new()
        };

        // Parse parameters
        self.expect(&TokenKind::LeftParen, "Expected '(' after function name")?;
        let params = self.parse_parameters()?;
        self.expect(&TokenKind::RightParen, "Expected ')' after parameters")?;

        // Optional return type
        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse body
        let body = self.parse_block()?;

        // Optional semicolon after function (for top-level style: `func main() { ... };`)
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Statement::Function {
            name,
            lifetimes,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_parameters(&mut self) -> ZyraResult<Vec<Parameter>> {
        let mut params = Vec::new();

        if self.check(&TokenKind::RightParen) {
            return Ok(params);
        }

        loop {
            let param_span = self.peek().span;

            // Handle &self or &mut self
            if self.check(&TokenKind::Ampersand) {
                self.advance(); // consume &

                let is_mutable = if self.check(&TokenKind::Mut) {
                    self.advance(); // consume mut
                    true
                } else {
                    false
                };

                // Expect 'self' keyword
                if self.check(&TokenKind::SelfType) {
                    self.advance(); // consume self

                    let span = Span::new(
                        param_span.start,
                        self.previous().span.end,
                        param_span.line,
                        param_span.column,
                    );

                    // Create a special self parameter
                    params.push(Parameter {
                        name: if is_mutable {
                            "&mut self".to_string()
                        } else {
                            "&self".to_string()
                        },
                        param_type: Type::SelfType,
                        span,
                    });
                } else {
                    return Err(ZyraError::syntax_error(
                        "Expected 'self' after '&' or '&mut' in parameter",
                        SourceLocation::new("", param_span.line, param_span.column),
                    ));
                }
            } else if self.check(&TokenKind::SelfType) {
                // Plain self parameter (takes ownership)
                self.advance(); // consume self
                let span = Span::new(
                    param_span.start,
                    self.previous().span.end,
                    param_span.line,
                    param_span.column,
                );
                params.push(Parameter {
                    name: "self".to_string(),
                    param_type: Type::SelfType,
                    span,
                });
            } else if self.check(&TokenKind::Mut) {
                // Check for 'mut self'
                self.advance(); // consume mut
                if self.check(&TokenKind::SelfType) {
                    self.advance(); // consume self
                    let span = Span::new(
                        param_span.start,
                        self.previous().span.end,
                        param_span.line,
                        param_span.column,
                    );
                    params.push(Parameter {
                        name: "mut self".to_string(),
                        param_type: Type::SelfType,
                        span,
                    });
                } else {
                    // It's 'mut name: Type' - mutable parameter
                    let name = self.expect_identifier("Expected parameter name after 'mut'")?;
                    self.expect(&TokenKind::Colon, "Expected ':' after parameter name")?;
                    let param_type = self.parse_type()?;

                    let span = Span::new(
                        param_span.start,
                        self.previous().span.end,
                        param_span.line,
                        param_span.column,
                    );
                    params.push(Parameter {
                        name: format!("mut {}", name),
                        param_type,
                        span,
                    });
                }
            } else {
                // Regular parameter: name: Type
                let name = self.expect_identifier("Expected parameter name")?;
                self.expect(&TokenKind::Colon, "Expected ':' after parameter name")?;
                let param_type = self.parse_type()?;

                let span = Span::new(
                    param_span.start,
                    self.previous().span.end,
                    param_span.line,
                    param_span.column,
                );
                params.push(Parameter {
                    name,
                    param_type,
                    span,
                });
            }

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        Ok(params)
    }

    fn parse_import(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'import'

        // Parse namespace path: std::game::specific
        let mut path = vec![self.expect_identifier("Expected module name after 'import'")?];

        while self.check(&TokenKind::ColonColon) {
            self.advance(); // consume ::
            path.push(self.expect_identifier("Expected identifier after '::'")?);
        }

        // Check for specific imports: ::{Item1, Item2}
        let items = if self.check(&TokenKind::ColonColon) {
            self.advance(); // consume ::
            if self.check(&TokenKind::LeftBrace) {
                self.advance(); // consume {
                let mut items = Vec::new();

                if !self.check(&TokenKind::RightBrace) {
                    items.push(self.expect_identifier("Expected import item")?);

                    while self.check(&TokenKind::Comma) {
                        self.advance(); // consume ,
                        if self.check(&TokenKind::RightBrace) {
                            break; // trailing comma
                        }
                        items.push(self.expect_identifier("Expected import item")?);
                    }
                }

                self.expect(&TokenKind::RightBrace, "Expected '}' after import items")?;
                items
            } else {
                // Single item after ::
                vec![self.expect_identifier("Expected import item after '::'")?]
            }
        } else {
            Vec::new() // Import entire module
        };

        // Semicolon is optional
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Statement::Import { path, items, span })
    }

    fn parse_return(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'return'

        let value = if !self.check(&TokenKind::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(&TokenKind::Semicolon, "Expected ';' after return statement")?;

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Statement::Return { value, span })
    }

    fn parse_if(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'if'

        let condition = self.parse_expression()?;
        let then_block = self.parse_block()?;

        let else_block = if self.check(&TokenKind::Else) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };

        // Optional semicolon after if statement
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Statement::If {
            condition,
            then_block,
            else_block,
            span,
        })
    }

    fn parse_while(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'while'

        let condition = self.parse_expression()?;
        let body = self.parse_block()?;

        // Optional semicolon after while loop
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Statement::While {
            condition,
            body,
            span,
        })
    }

    fn parse_for(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'for'

        let variable = self.expect_identifier("Expected loop variable name")?;

        self.expect(&TokenKind::In, "Expected 'in' after loop variable")?;

        let start = self.parse_expression()?;

        // Check for .. or ..= (inclusive range)
        let inclusive = if self.check(&TokenKind::DotDotEq) {
            self.advance();
            true
        } else {
            self.expect(&TokenKind::DotDot, "Expected '..' or '..=' in range")?;
            false
        };

        let end = self.parse_expression()?;

        let body = self.parse_block()?;

        // Optional semicolon after for loop
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Statement::For {
            variable,
            start,
            end,
            inclusive,
            body,
            span,
        })
    }

    fn parse_expression_statement(&mut self) -> ZyraResult<Statement> {
        let start_span = self.peek().span;
        let expr = self.parse_expression()?;

        self.expect(&TokenKind::Semicolon, "Expected ';' after expression")?;

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Statement::Expression { expr, span })
    }

    fn parse_block(&mut self) -> ZyraResult<Block> {
        let start_span = self
            .expect(&TokenKind::LeftBrace, "Expected '{' to start block")?
            .span;

        let mut statements = Vec::new();
        let mut expression = None;

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Try to determine if this is a statement or a trailing expression
            // Check what kind of token we're looking at
            match self.peek().kind {
                // These are definitely statements
                TokenKind::Let
                | TokenKind::Func
                | TokenKind::Import
                | TokenKind::Return
                | TokenKind::While
                | TokenKind::For => {
                    statements.push(self.parse_statement()?);
                }
                // If statement - could be trailing expression or statement
                TokenKind::If => {
                    let if_stmt = self.parse_if()?;

                    // Check if this if statement should be a trailing expression
                    // (no semicolon after and followed by closing brace)
                    if self.check(&TokenKind::RightBrace) {
                        // Convert if statement to expression for trailing position
                        if let Statement::If {
                            condition,
                            then_block,
                            else_block,
                            span,
                        } = if_stmt
                        {
                            expression = Some(Box::new(Expression::If {
                                condition: Box::new(condition),
                                then_block,
                                else_block,
                                span,
                            }));
                        } else {
                            statements.push(if_stmt);
                        }
                    } else {
                        statements.push(if_stmt);
                    }
                }
                // Block statement
                TokenKind::LeftBrace => {
                    let block = self.parse_block()?;
                    statements.push(Statement::Block(block));
                }
                // Everything else might be an expression or expression statement
                _ => {
                    let expr_start = self.peek().span;
                    let expr = self.parse_expression()?;

                    // Check what follows the expression
                    if self.check(&TokenKind::Semicolon) {
                        // It's an expression statement with semicolon
                        self.advance();
                        let span = Span::new(
                            expr_start.start,
                            self.previous().span.end,
                            expr_start.line,
                            expr_start.column,
                        );
                        statements.push(Statement::Expression { expr, span });
                    } else if self.check(&TokenKind::RightBrace) {
                        // It's the trailing expression (no semicolon before closing brace)
                        expression = Some(Box::new(expr));
                    } else {
                        // Unexpected - report error
                        return Err(self.error("Expected ';' or '}' after expression"));
                    }
                }
            }
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' to end block")?;

        let span = Span::new(
            start_span.start,
            self.previous().span.end,
            start_span.line,
            start_span.column,
        );

        Ok(Block {
            statements,
            expression,
            span,
        })
    }

    // ===== Expression Parsing (Pratt Parser style with precedence) =====

    fn parse_expression(&mut self) -> ZyraResult<Expression> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> ZyraResult<Expression> {
        let expr = self.parse_or()?;

        if self.check(&TokenKind::Equal) {
            let start_span = self.advance().span;
            let value = self.parse_assignment()?;

            let span = Span::new(
                expr.span().start,
                value.span().end,
                start_span.line,
                start_span.column,
            );

            return Ok(Expression::Assignment {
                target: Box::new(expr),
                value: Box::new(value),
                span,
            });
        }

        // Compound assignment operators: +=, -=, *=, /=, %=
        // Desugar x += y to x = x + y
        let compound_op = if self.check(&TokenKind::PlusAssign) {
            Some(BinaryOp::Add)
        } else if self.check(&TokenKind::MinusAssign) {
            Some(BinaryOp::Subtract)
        } else if self.check(&TokenKind::StarAssign) {
            Some(BinaryOp::Multiply)
        } else if self.check(&TokenKind::SlashAssign) {
            Some(BinaryOp::Divide)
        } else if self.check(&TokenKind::PercentAssign) {
            Some(BinaryOp::Modulo)
        } else {
            None
        };

        if let Some(op) = compound_op {
            let start_span = self.advance().span;
            let right = self.parse_assignment()?;

            let span = Span::new(
                expr.span().start,
                right.span().end,
                start_span.line,
                start_span.column,
            );

            // Create: target = target op value
            let binary_expr = Expression::Binary {
                operator: op,
                left: Box::new(expr.clone()),
                right: Box::new(right),
                span,
            };

            return Ok(Expression::Assignment {
                target: Box::new(expr),
                value: Box::new(binary_expr),
                span,
            });
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> ZyraResult<Expression> {
        let mut left = self.parse_and()?;

        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );

            left = Expression::Binary {
                left: Box::new(left),
                operator: BinaryOp::Or,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> ZyraResult<Expression> {
        let mut left = self.parse_equality()?;

        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.parse_equality()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );

            left = Expression::Binary {
                left: Box::new(left),
                operator: BinaryOp::And,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> ZyraResult<Expression> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::EqualEqual => BinaryOp::Equal,
                TokenKind::NotEqual => BinaryOp::NotEqual,
                _ => break,
            };
            self.advance();

            let right = self.parse_comparison()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );

            left = Expression::Binary {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> ZyraResult<Expression> {
        let mut left = self.parse_term()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::Less => BinaryOp::Less,
                TokenKind::LessEqual => BinaryOp::LessEqual,
                TokenKind::Greater => BinaryOp::Greater,
                TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
                _ => break,
            };
            self.advance();

            let right = self.parse_term()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );

            left = Expression::Binary {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_term(&mut self) -> ZyraResult<Expression> {
        let mut left = self.parse_factor()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Subtract,
                _ => break,
            };
            self.advance();

            let right = self.parse_factor()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );

            left = Expression::Binary {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> ZyraResult<Expression> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                TokenKind::Percent => BinaryOp::Modulo,
                _ => break,
            };
            self.advance();

            let right = self.parse_unary()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );

            left = Expression::Binary {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> ZyraResult<Expression> {
        let start_span = self.peek().span;

        match self.peek().kind {
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                let span = Span::new(
                    start_span.start,
                    operand.span().end,
                    start_span.line,
                    start_span.column,
                );

                Ok(Expression::Unary {
                    operator: UnaryOp::Negate,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                let span = Span::new(
                    start_span.start,
                    operand.span().end,
                    start_span.line,
                    start_span.column,
                );

                Ok(Expression::Unary {
                    operator: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Ampersand => {
                self.advance();
                let mutable = if self.check(&TokenKind::Mut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let value = self.parse_unary()?;
                let span = Span::new(
                    start_span.start,
                    value.span().end,
                    start_span.line,
                    start_span.column,
                );

                Ok(Expression::Reference {
                    mutable,
                    value: Box::new(value),
                    span,
                })
            }
            TokenKind::Star => {
                self.advance();
                let value = self.parse_unary()?;
                let span = Span::new(
                    start_span.start,
                    value.span().end,
                    start_span.line,
                    start_span.column,
                );

                Ok(Expression::Dereference {
                    value: Box::new(value),
                    span,
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> ZyraResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::LeftParen) {
                self.advance();
                let arguments = self.parse_arguments()?;
                self.expect(&TokenKind::RightParen, "Expected ')' after arguments")?;

                let span = Span::new(
                    expr.span().start,
                    self.previous().span.end,
                    expr.span().line,
                    expr.span().column,
                );

                expr = Expression::Call {
                    callee: Box::new(expr),
                    arguments,
                    span,
                };
            } else if self.check(&TokenKind::Dot) {
                self.advance();
                let field = self.expect_identifier("Expected field name after '.'")?;

                let span = Span::new(
                    expr.span().start,
                    self.previous().span.end,
                    expr.span().line,
                    expr.span().column,
                );

                expr = Expression::FieldAccess {
                    object: Box::new(expr),
                    field,
                    span,
                };
            } else if self.check(&TokenKind::LeftBracket) {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(&TokenKind::RightBracket, "Expected ']' after index")?;

                let span = Span::new(
                    expr.span().start,
                    self.previous().span.end,
                    expr.span().line,
                    expr.span().column,
                );

                expr = Expression::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_arguments(&mut self) -> ZyraResult<Vec<Expression>> {
        let mut args = Vec::new();

        if self.check(&TokenKind::RightParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        Ok(args)
    }

    fn parse_primary(&mut self) -> ZyraResult<Expression> {
        let token = self.advance();
        let span = token.span;

        match token.kind {
            TokenKind::Int(value) => Ok(Expression::Int { value, span }),
            TokenKind::Float(value) => Ok(Expression::Float { value, span }),
            TokenKind::True => Ok(Expression::Bool { value: true, span }),
            TokenKind::False => Ok(Expression::Bool { value: false, span }),
            TokenKind::Char(value) => Ok(Expression::Char { value, span }),
            TokenKind::String(value) => Ok(Expression::String { value, span }),

            TokenKind::FormatString(parts) => {
                let mut parts_iter = parts.into_iter();

                // Helper to convert part to expression
                // We parse strings as String literals, and expressions by invoking a sub-parser
                let parse_part = |is_expr: bool, content: String| -> ZyraResult<Expression> {
                    if is_expr {
                        let mut lexer = crate::lexer::Lexer::new(&content, "interpolation");
                        let tokens = lexer.tokenize()?;
                        let mut parser = Parser::new(tokens);
                        parser.parse_expression()
                    } else {
                        Ok(Expression::String {
                            value: content,
                            span,
                        })
                    }
                };

                if let Some((is_expr, content)) = parts_iter.next() {
                    let mut expr = parse_part(is_expr, content)?;

                    for (is_expr, content) in parts_iter {
                        let right = parse_part(is_expr, content)?;

                        let span = Span::new(
                            expr.span().start,
                            right.span().end,
                            expr.span().line,
                            expr.span().column,
                        );

                        expr = Expression::Binary {
                            left: Box::new(expr),
                            operator: BinaryOp::Add,
                            right: Box::new(right),
                            span,
                        };
                    }
                    Ok(expr)
                } else {
                    Ok(Expression::String {
                        value: String::new(),
                        span,
                    })
                }
            }

            TokenKind::Identifier(name) => {
                // Check for qualified path (module::function or module::StructName)
                let mut full_path = name;
                while self.check(&TokenKind::ColonColon) {
                    self.advance(); // Consume ::
                    if let TokenKind::Identifier(segment) = &self.peek().kind {
                        full_path = format!("{}::{}", full_path, segment);
                        self.advance(); // Consume segment
                    } else {
                        return Err(ZyraError::syntax_error(
                            "Expected identifier after '::'",
                            SourceLocation::new("", self.peek().span.line, self.peek().span.column),
                        ));
                    }
                }

                // Check for struct instantiation: StructName { field: value, ... }
                // Ambiguity fix: Only parse as struct if name starts with Uppercase (PascalCase)
                // This prevents `if var {` from being parsed as `Struct {`
                let is_struct_name = full_path
                    .split("::")
                    .last()
                    .and_then(|s| s.chars().next())
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false);

                if is_struct_name && self.check(&TokenKind::LeftBrace) {
                    self.advance(); // Consume {
                    let mut fields = Vec::new();

                    while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                        // Parse field name
                        let field_name = if let TokenKind::Identifier(fname) = &self.peek().kind {
                            let fname = fname.clone();
                            self.advance();
                            fname
                        } else {
                            return Err(ZyraError::syntax_error(
                                "Expected field name in struct initializer",
                                SourceLocation::new(
                                    "",
                                    self.peek().span.line,
                                    self.peek().span.column,
                                ),
                            ));
                        };

                        // Check for shorthand initialization: Point { x, y }
                        let field_value = if self.check(&TokenKind::Colon) {
                            self.advance(); // Consumes :
                            self.parse_expression()?
                        } else if self.check(&TokenKind::Comma)
                            || self.check(&TokenKind::RightBrace)
                        {
                            // Shorthand: field implies field: field
                            Expression::Identifier {
                                name: field_name.clone(),
                                span: self.previous().span,
                            }
                        } else {
                            return Err(ZyraError::syntax_error(
                                "Expected ':' or ',' after field name in struct initializer",
                                SourceLocation::new(
                                    "",
                                    self.peek().span.line,
                                    self.peek().span.column,
                                ),
                            ));
                        };

                        fields.push((field_name, field_value));

                        // Expect comma or end of fields
                        if !self.check(&TokenKind::RightBrace) {
                            self.expect(
                                &TokenKind::Comma,
                                "Expected ',' or '}' after struct field",
                            )?;
                        }
                    }

                    self.expect(
                        &TokenKind::RightBrace,
                        "Expected '}' to close struct initializer",
                    )?;
                    let end_span = self.previous().span;
                    let span = Span::new(span.start, end_span.end, span.line, span.column);

                    Ok(Expression::StructInit {
                        name: full_path,
                        fields,
                        span,
                    })
                } else {
                    // Check if this looks like an enum variant: EnumName::Variant
                    // (contains :: and not followed by parentheses)
                    if full_path.contains("::") && !self.check(&TokenKind::LeftParen) {
                        // Parse as enum variant
                        let parts: Vec<&str> = full_path.rsplitn(2, "::").collect();
                        if parts.len() == 2 {
                            let variant = parts[0].to_string();
                            let enum_name = parts[1].to_string();
                            // Check for variant data: EnumName::Variant(data)
                            let data = if self.check(&TokenKind::LeftParen) {
                                self.advance(); // consume (
                                let expr = self.parse_expression()?;
                                self.expect(
                                    &TokenKind::RightParen,
                                    "Expected ')' after enum variant data",
                                )?;
                                Some(Box::new(expr))
                            } else {
                                None
                            };
                            return Ok(Expression::EnumVariant {
                                enum_name,
                                variant,
                                data,
                                span,
                            });
                        }
                    }
                    Ok(Expression::Identifier {
                        name: full_path,
                        span,
                    })
                }
            }

            // Handle 'self' keyword as expression (for method bodies)
            TokenKind::SelfType => Ok(Expression::Identifier {
                name: "self".to_string(),
                span,
            }),

            TokenKind::LeftParen => {
                let inner = self.parse_expression()?;
                self.expect(&TokenKind::RightParen, "Expected ')' after expression")?;

                let end_span = self.previous().span;
                let span = Span::new(span.start, end_span.end, span.line, span.column);

                Ok(Expression::Grouped {
                    inner: Box::new(inner),
                    span,
                })
            }

            TokenKind::LeftBracket => {
                let mut elements = Vec::new();

                if !self.check(&TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }

                self.expect(&TokenKind::RightBracket, "Expected ']' after list elements")?;

                let end_span = self.previous().span;
                let span = Span::new(span.start, end_span.end, span.line, span.column);

                Ok(Expression::List { elements, span })
            }

            _ => Err(self.error(&format!("Unexpected token: {}", token.kind))),
        }
    }

    // ===== Type Parsing =====

    fn parse_type(&mut self) -> ZyraResult<Type> {
        // Check for lifetime-annotated type ('a Type)
        if let TokenKind::Lifetime(lt) = &self.peek().kind {
            let lifetime = lt.clone();
            self.advance();
            let inner = self.parse_type()?;
            return Ok(Type::LifetimeAnnotated {
                lifetime,
                inner: Box::new(inner),
            });
        }

        // Check for reference type
        if self.check(&TokenKind::Ampersand) {
            self.advance();

            // Optional lifetime
            let lifetime = if let TokenKind::Lifetime(lt) = &self.peek().kind {
                let lt = lt.clone();
                self.advance();
                Some(lt)
            } else {
                None
            };

            // Optional mut
            let mutable = if self.check(&TokenKind::Mut) {
                self.advance();
                true
            } else {
                false
            };

            let inner = self.parse_type()?;

            return Ok(Type::Reference {
                lifetime,
                mutable,
                inner: Box::new(inner),
            });
        }

        // Parse base type
        let base = match self.peek().kind {
            TokenKind::TypeInt => {
                self.advance();
                Type::Int
            }
            TokenKind::TypeFloat => {
                self.advance();
                Type::Float
            }
            TokenKind::TypeBool => {
                self.advance();
                Type::Bool
            }
            TokenKind::TypeString => {
                self.advance();
                Type::String
            }
            TokenKind::TypeObject => {
                self.advance();
                Type::Object
            }
            TokenKind::TypeList => {
                self.advance();
                self.expect(&TokenKind::Less, "Expected '<' after 'List'")?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::Greater, "Expected '>' after list type")?;
                Type::List(Box::new(inner))
            }
            TokenKind::Identifier(ref name) => {
                let name = name.clone();
                self.advance();

                match name.as_str() {
                    "i8" => Type::I8,
                    "i32" => Type::I32,
                    "i64" => Type::I64,
                    "u8" => Type::U8,
                    "u32" => Type::U32,
                    "u64" => Type::U64,
                    "f32" => Type::F32,
                    "f64" => Type::F64,
                    "char" => Type::Char,
                    "Vec" => {
                        self.expect(&TokenKind::Less, "Expected '<' after 'Vec'")?;
                        let inner = self.parse_type()?;
                        self.expect(&TokenKind::Greater, "Expected '>' after vector type")?;
                        Type::Vec(Box::new(inner))
                    }
                    _ => Type::Named(name),
                }
            }
            TokenKind::LeftBracket => {
                self.advance();
                let inner = self.parse_type()?;
                self.expect(&TokenKind::Semicolon, "Expected ';' in array type")?;

                let size = if let TokenKind::Int(n) = self.peek().kind {
                    self.advance();
                    n as usize
                } else {
                    return Err(self.error("Expected integer size for array"));
                };

                self.expect(&TokenKind::RightBracket, "Expected ']' after array size")?;
                Type::Array {
                    elem: Box::new(inner),
                    size,
                }
            }
            _ => {
                return Err(self.error("Expected type"));
            }
        };

        Ok(base)
    }

    // ===== Helper Methods =====

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous().clone()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
        }
    }

    fn expect(&mut self, kind: &TokenKind, message: &str) -> ZyraResult<Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }

    fn expect_identifier(&mut self, message: &str) -> ZyraResult<String> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> ZyraError {
        let token = self.peek();
        ZyraError::syntax_error(
            message,
            SourceLocation::new("", token.span.line, token.span.column).with_snippet(&token.lexeme),
        )
    }

    // ===== Struct Parsing =====
    // struct Name { field1: Type1, field2: Type2 }
    fn parse_struct(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'struct'

        let name = self.expect_identifier("Expected struct name")?;

        self.expect(&TokenKind::LeftBrace, "Expected '{' after struct name")?;

        let mut fields = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let field_span = self.peek().span;
            let field_name = self.expect_identifier("Expected field name")?;
            self.expect(&TokenKind::Colon, "Expected ':' after field name")?;
            let field_type = self.parse_type()?;

            fields.push(StructField {
                name: field_name,
                field_type,
                span: field_span,
            });

            // Optional comma between fields
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' after struct fields")?;

        // Optional semicolon
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Statement::Struct {
            name,
            fields,
            span: start_span,
        })
    }

    // ===== Enum Parsing =====
    // enum Name { Variant1, Variant2(Type), Variant3 }
    fn parse_enum(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'enum'

        let name = self.expect_identifier("Expected enum name")?;

        self.expect(&TokenKind::LeftBrace, "Expected '{' after enum name")?;

        let mut variants = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let variant_span = self.peek().span;
            let variant_name = self.expect_identifier("Expected variant name")?;

            // Check for tuple variant: Variant(Type1, Type2)
            let data = if self.check(&TokenKind::LeftParen) {
                self.advance();
                let mut types = Vec::new();

                if !self.check(&TokenKind::RightParen) {
                    types.push(self.parse_type()?);
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        types.push(self.parse_type()?);
                    }
                }

                self.expect(&TokenKind::RightParen, "Expected ')' after variant types")?;
                Some(types)
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                data,
                span: variant_span,
            });

            // Optional comma between variants
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' after enum variants")?;

        // Optional semicolon
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Statement::Enum {
            name,
            variants,
            span: start_span,
        })
    }

    // ===== Impl Parsing =====
    // impl Name { func method() { } }
    // impl Trait for Name { func method() { } }
    fn parse_impl(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'impl'

        let first_name = self.expect_identifier("Expected type or trait name after 'impl'")?;

        // Check for trait implementation: impl Trait for Type
        let (trait_name, target_type) = if self.check(&TokenKind::For) {
            self.advance(); // Consume 'for'
            let target = self.expect_identifier("Expected type name after 'for'")?;
            (Some(first_name), target)
        } else {
            (None, first_name)
        };

        self.expect(&TokenKind::LeftBrace, "Expected '{' after impl declaration")?;

        let mut methods = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Only functions are allowed in impl blocks
            if self.check(&TokenKind::Func) {
                let method = self.parse_function()?;
                methods.push(Box::new(method));
            } else {
                return Err(self.error("Only functions are allowed in impl blocks"));
            }
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' after impl body")?;

        // Optional semicolon
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Statement::Impl {
            target_type,
            trait_name,
            methods,
            span: start_span,
        })
    }

    // ===== Trait Parsing =====
    // trait Name { func method(param: Type) -> ReturnType; }
    fn parse_trait(&mut self) -> ZyraResult<Statement> {
        let start_span = self.advance().span; // Consume 'trait'

        let name = self.expect_identifier("Expected trait name")?;

        self.expect(&TokenKind::LeftBrace, "Expected '{' after trait name")?;

        let mut methods = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            if !self.check(&TokenKind::Func) {
                return Err(self.error("Only method signatures are allowed in traits"));
            }

            let method_span = self.advance().span; // Consume 'func'
            let method_name = self.expect_identifier("Expected method name")?;

            // Parse parameters
            self.expect(&TokenKind::LeftParen, "Expected '(' after method name")?;
            let params = self.parse_parameters()?;
            self.expect(&TokenKind::RightParen, "Expected ')' after parameters")?;

            // Optional return type
            let return_type = if self.check(&TokenKind::Arrow) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };

            // Check for default implementation or just signature
            let default_impl = if self.check(&TokenKind::LeftBrace) {
                Some(self.parse_block()?)
            } else {
                // Just a signature, expect semicolon
                if self.check(&TokenKind::Semicolon) {
                    self.advance();
                }
                None
            };

            methods.push(TraitMethod {
                name: method_name,
                params,
                return_type,
                default_impl,
                span: method_span,
            });
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' after trait body")?;

        // Optional semicolon
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(Statement::Trait {
            name,
            methods,
            span: start_span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> ZyraResult<Program> {
        let mut lexer = Lexer::new(source, "test.zr");
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_let_statement() {
        let program = parse("let x = 5;").unwrap();
        assert_eq!(program.statements.len(), 1);

        if let Statement::Let { name, mutable, .. } = &program.statements[0] {
            assert_eq!(name, "x");
            assert!(!mutable);
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_mut_let() {
        let program = parse("let mut x = 10;").unwrap();

        if let Statement::Let { name, mutable, .. } = &program.statements[0] {
            assert_eq!(name, "x");
            assert!(mutable);
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_function() {
        let program = parse("func add(a: Int, b: Int) -> Int { a + b; }").unwrap();

        if let Statement::Function {
            name,
            params,
            return_type,
            ..
        } = &program.statements[0]
        {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert!(matches!(return_type, Some(Type::Int)));
        } else {
            panic!("Expected Function statement");
        }
    }

    #[test]
    fn test_binary_expression() {
        let program = parse("1 + 2 * 3;").unwrap();

        if let Statement::Expression { expr, .. } = &program.statements[0] {
            // Should be parsed as 1 + (2 * 3)
            if let Expression::Binary { operator, .. } = expr {
                assert_eq!(*operator, BinaryOp::Add);
            } else {
                panic!("Expected Binary expression");
            }
        } else {
            panic!("Expected Expression statement");
        }
    }
}
