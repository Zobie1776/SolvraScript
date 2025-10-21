#![allow(dead_code)]

use crate::ast::{
    BinaryOp, BindingKind, CatchBlock, Expr, FunctionDecl, ImportDecl, Literal, MatchArm,
    Parameter, Pattern, Program, Stmt, StringPart, Type, UnaryOp, VariableDecl, Visibility,
};
use crate::tokenizer::{Position, Token, TokenKind};

/// Parser error types
#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: TokenKind,
        position: Position,
    },
    UnexpectedEndOfInput {
        expected: String,
        position: Position,
    },
    InvalidSyntax {
        message: String,
        position: Position,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken {
                expected,
                found,
                position,
            } => {
                write!(
                    f,
                    "Expected {} but found {:?} at line {}, column {}",
                    expected, found, position.line, position.column
                )
            }
            ParseError::UnexpectedEndOfInput { expected, position } => {
                write!(
                    f,
                    "Unexpected end of input, expected {} at line {}, column {}",
                    expected, position.line, position.column
                )
            }
            ParseError::InvalidSyntax { message, position } => {
                write!(
                    f,
                    "Invalid syntax: {} at line {}, column {}",
                    message, position.line, position.column
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Recursive descent parser for NovaScript
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Parse a complete NovaScript program
    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let position = self.current_position();
        let mut statements = Vec::new();

        while !self.is_at_end() {
            // Skip whitespace and comments at the top level
            match &self.peek().kind {
                TokenKind::Newline
                | TokenKind::Indent
                | TokenKind::Dedent
                | TokenKind::Comment(_) => {
                    self.advance();
                    continue;
                }
                _ => {}
            }
            statements.push(self.parse_statement()?);
        }

        Ok(Program::new(statements, position))
    }

    /// Parse a single statement
    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        match &self.peek().kind {
            TokenKind::Let => {
                let start_pos = self.current_position();
                self.advance();
                self.parse_variable_declaration(start_pos, BindingKind::Let)
            }
            TokenKind::Const => {
                let start_pos = self.current_position();
                self.advance();
                self.parse_variable_declaration(start_pos, BindingKind::Const)
            }
            TokenKind::Fn => self.parse_function_declaration(),
            TokenKind::Import => self.parse_import_declaration(),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Break => self.parse_break_statement(),
            TokenKind::Continue => self.parse_continue_statement(),
            TokenKind::Try => self.parse_try_statement(),
            TokenKind::Panic => self.parse_panic_statement(),
            TokenKind::LeftBrace => self.parse_block_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    /// Parse variable declaration: `let`/`const` name [: type] [= value];
    fn parse_variable_declaration(
        &mut self,
        start_pos: Position,
        binding: BindingKind,
    ) -> Result<Stmt, ParseError> {
        let is_mutable = match binding {
            BindingKind::Let => {
                if self.check(&TokenKind::Mut) {
                    self.advance();
                    true
                } else {
                    false
                }
            }
            BindingKind::Const => {
                if self.check(&TokenKind::Mut) {
                    return Err(ParseError::InvalidSyntax {
                        message: "const bindings cannot be declared as mutable".to_string(),
                        position: self.current_position(),
                    });
                }
                false
            }
        };

        let name = self.consume_identifier("Expected variable name")?;

        let var_type = if self.check(&TokenKind::Colon) {
            self.advance();
            self.parse_type()?
        } else {
            Type::Inferred
        };

        let initializer = if self.check(&TokenKind::Equal) {
            self.advance();
            Some(self.parse_expression()?)
        } else if matches!(binding, BindingKind::Const) {
            return Err(ParseError::InvalidSyntax {
                message: "const bindings require an initializer".to_string(),
                position: self.current_position(),
            });
        } else {
            None
        };

        self.consume_statement_terminator()?;

        let decl = VariableDecl {
            name,
            var_type,
            binding,
            is_mutable,
            initializer,
            position: start_pos,
        };

        Ok(Stmt::VariableDecl { decl })
    }

    /// Parse function declaration: fn name(params) -> return_type { body }
    fn parse_function_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();

        let is_async = if self.check(&TokenKind::Async) {
            self.advance();
            true
        } else {
            false
        };

        self.consume(&TokenKind::Fn, "Expected 'fn'")?;
        let name = self.consume_identifier("Expected function name")?;

        self.consume(&TokenKind::LeftParen, "Expected '(' after function name")?;
        let mut params = Vec::new();

        if !self.check(&TokenKind::RightParen) {
            loop {
                let param_pos = self.current_position();
                let param_name = self.consume_identifier("Expected parameter name")?;

                let param_type = if self.check(&TokenKind::Colon) {
                    self.advance();
                    self.parse_type()?
                } else {
                    Type::Inferred
                };

                let default_value = if self.check(&TokenKind::Equal) {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                params.push(Parameter {
                    name: param_name,
                    param_type,
                    default_value,
                    position: param_pos,
                });

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.consume(&TokenKind::RightParen, "Expected ')' after parameters")?;

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            self.parse_type()?
        } else {
            Type::Inferred
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' before function body")?;
        let body = self.parse_block_body()?;

        let decl = FunctionDecl {
            name,
            params,
            return_type,
            body,
            is_async,
            visibility: Visibility::Private,
            position: start_pos,
        };

        Ok(Stmt::FunctionDecl { decl })
    }

    /// Parse import declaration: import module [as alias];
    fn parse_import_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Import, "Expected 'import'")?;

        let (module, items) = if self.check(&TokenKind::LeftBrace) {
            self.advance();
            let mut items = Vec::new();
            while !self.check(&TokenKind::RightBrace) {
                let item = self.consume_identifier("Expected imported item name")?;
                items.push(item);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.consume(&TokenKind::RightBrace, "Expected '}' after import list")?;
            self.expect_identifier_keyword("from")?;
            let module = self.consume_identifier("Expected module name")?;
            (module, items)
        } else {
            (self.consume_identifier("Expected module name")?, Vec::new())
        };

        let alias = if self.match_identifier("as") {
            Some(self.consume_identifier("Expected alias name")?)
        } else {
            None
        };

        self.consume_statement_terminator()?;

        let decl = ImportDecl {
            module,
            items,
            alias,
            position: start_pos,
        };

        Ok(Stmt::ImportDecl { decl })
    }

    /// Parse if statement: if condition { body } [else { body }]
    fn parse_if_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::If, "Expected 'if'")?;

        let condition = self.parse_expression()?;
        let then_branch = Box::new(self.parse_statement()?);

        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            position: start_pos,
        })
    }

    /// Parse while statement: while condition { body }
    fn parse_while_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::While, "Expected 'while'")?;

        let condition = self.parse_expression()?;
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::While {
            condition,
            body,
            position: start_pos,
        })
    }

    /// Parse for statement: for variable in iterable { body }
    fn parse_for_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::For, "Expected 'for'")?;

        let variable = self.consume_identifier("Expected variable name")?;
        self.consume(&TokenKind::In, "Expected 'in' after for variable")?;

        let iterable = self.parse_expression()?;
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::For {
            variable,
            iterable,
            body,
            position: start_pos,
        })
    }

    /// Parse return statement: return [expression];
    fn parse_return_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Return, "Expected 'return'")?;

        let value = if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Newline) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume_statement_terminator()?;

        Ok(Stmt::Return {
            value,
            position: start_pos,
        })
    }

    /// Parse break statement: break;
    fn parse_break_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Break, "Expected 'break'")?;
        self.consume_statement_terminator()?;

        Ok(Stmt::Break {
            label: None,
            position: start_pos,
        })
    }

    /// Parse continue statement: continue;
    fn parse_continue_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Continue, "Expected 'continue'")?;
        self.consume_statement_terminator()?;

        Ok(Stmt::Continue {
            label: None,
            position: start_pos,
        })
    }

    /// Parse try statement: try { body } catch [type] [var] { body }
    fn parse_try_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Try, "Expected 'try'")?;

        let try_block = Box::new(self.parse_block_statement()?);
        let mut catch_blocks = Vec::new();

        while self.check(&TokenKind::Catch) {
            let catch_pos = self.current_position();
            self.advance();

            let exception_type = if self.check(&TokenKind::LeftParen) {
                self.advance();
                let typ = Some(self.parse_type()?);
                self.consume(&TokenKind::RightParen, "Expected ')' after exception type")?;
                typ
            } else {
                None
            };

            let variable = if let TokenKind::Identifier(name) = &self.peek().kind {
                let name = name.clone();
                self.advance();
                Some(name)
            } else {
                None
            };

            let body = Box::new(self.parse_block_statement()?);

            catch_blocks.push(CatchBlock {
                exception_type,
                variable,
                body,
                position: catch_pos,
            });
        }

        let finally_block = if self.check(&TokenKind::Identifier("finally".to_string())) {
            self.advance();
            Some(Box::new(self.parse_block_statement()?))
        } else {
            None
        };

        Ok(Stmt::Try {
            try_block,
            catch_blocks,
            finally_block,
            position: start_pos,
        })
    }

    /// Parse panic statement: panic [expression];
    fn parse_panic_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Panic, "Expected 'panic'")?;

        let message = if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Newline) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume_statement_terminator()?;

        Ok(Stmt::Panic {
            message,
            position: start_pos,
        })
    }

    /// Parse block statement: { statements }
    fn parse_block_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::LeftBrace, "Expected '{'")?;

        let statements = self.parse_block_body()?;

        Ok(Stmt::Block {
            statements,
            position: start_pos,
        })
    }

    /// Parse block body (statements inside braces)
    fn parse_block_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();

        while {
            let is_end = self.check(&TokenKind::RightBrace) || self.is_at_end();
            !is_end
        } {
            // Skip whitespace and comments inside blocks
            match &self.peek().kind {
                TokenKind::Newline
                | TokenKind::Indent
                | TokenKind::Dedent
                | TokenKind::Comment(_) => {
                    self.advance();
                    continue;
                }
                _ => {}
            }
            statements.push(self.parse_statement()?);
        }

        self.consume(&TokenKind::RightBrace, "Expected '}'")?;
        Ok(statements)
    }

    /// Parse expression statement: expression;
    fn parse_expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expression()?;
        let position = expr.position().clone();
        self.consume_statement_terminator()?;

        Ok(Stmt::Expression { expr, position })
    }

    /// Parse expression with precedence climbing
    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_assignment()
    }

    /// Parse assignment expression: target = value
    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_logical_or()?;

        if self.check(&TokenKind::Equal) {
            let start_pos = self.current_position();
            self.advance();
            let value = self.parse_assignment()?;

            return Ok(Expr::Assignment {
                target: Box::new(expr),
                value: Box::new(value),
                position: start_pos,
            });
        }

        Ok(expr)
    }

    /// Parse logical OR expression: left || right
    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_and()?;

        while self.check(&TokenKind::Or) {
            let start_pos = self.current_position();
            self.advance();
            let right = self.parse_logical_and()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::Or,
                right: Box::new(right),
                position: start_pos,
            };
        }

        Ok(expr)
    }

    /// Parse logical AND expression: left && right
    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_equality()?;

        while self.check(&TokenKind::And) {
            let start_pos = self.current_position();
            self.advance();
            let right = self.parse_equality()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::And,
                right: Box::new(right),
                position: start_pos,
            };
        }

        Ok(expr)
    }

    /// Parse equality expression: left == right, left != right
    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;

        while let Some(op) = self.match_binary_op(&[TokenKind::EqualEqual, TokenKind::NotEqual]) {
            let start_pos = self.current_position();
            let right = self.parse_comparison()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
                position: start_pos,
            };
        }

        Ok(expr)
    }

    /// Parse comparison expression: <, >, <=, >=
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;

        while let Some(op) = self.match_binary_op(&[
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::LessEqual,
            TokenKind::GreaterEqual,
        ]) {
            let start_pos = self.current_position();
            let right = self.parse_term()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
                position: start_pos,
            };
        }

        Ok(expr)
    }

    /// Parse term expression: +, -
    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;

        while let Some(op) = self.match_binary_op(&[TokenKind::Plus, TokenKind::Minus]) {
            let start_pos = self.current_position();
            let right = self.parse_factor()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
                position: start_pos,
            };
        }

        Ok(expr)
    }

    /// Parse factor expression: *, /, %
    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;

        while let Some(op) =
            self.match_binary_op(&[TokenKind::Star, TokenKind::Slash, TokenKind::Percent])
        {
            let start_pos = self.current_position();
            let right = self.parse_unary()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
                position: start_pos,
            };
        }

        Ok(expr)
    }

    /// Parse unary expression: !, -
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if let Some(op) = self.match_unary_op(&[TokenKind::Not, TokenKind::Minus]) {
            let start_pos = self.current_position();
            let operand = self.parse_unary()?;

            return Ok(Expr::Unary {
                operator: op,
                operand: Box::new(operand),
                position: start_pos,
            });
        }

        self.parse_call()
    }

    /// Parse call expression: callee(args)
    fn parse_call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            // Store peeked token kind in a local variable to avoid borrow checker issues
            let kind = &self.peek().kind;
            if kind == &TokenKind::LeftParen {
                let start_pos = self.current_position();
                self.advance();

                let mut args = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }

                self.consume(&TokenKind::RightParen, "Expected ')' after arguments")?;

                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    position: start_pos,
                };
            } else if kind == &TokenKind::Dot {
                let start_pos = self.current_position();
                self.advance();
                let property = self.consume_identifier("Expected property name after '.'")?;

                expr = Expr::Member {
                    object: Box::new(expr),
                    property,
                    position: start_pos,
                };
            } else if kind == &TokenKind::LeftBracket {
                let start_pos = self.current_position();
                self.advance();
                let index = self.parse_expression()?;
                self.consume(&TokenKind::RightBracket, "Expected ']' after array index")?;

                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    position: start_pos,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse primary expression: literals, identifiers, parenthesized expressions
    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.peek().clone();
        let position = token.position.clone();

        match &token.kind {
            TokenKind::Integer(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::Integer(n),
                    position,
                })
            }
            TokenKind::Float(f) => {
                let f = *f;
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::Float(f),
                    position,
                })
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::StringTemplate {
                    parts: vec![StringPart::Literal(s)],
                    position,
                })
            }
            TokenKind::StringTemplate(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::StringTemplate {
                    parts: vec![StringPart::Literal(s)],
                    position,
                })
            }
            TokenKind::Boolean(b) => {
                let b = *b;
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::Boolean(b),
                    position,
                })
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Literal {
                    value: Literal::Null,
                    position,
                })
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Identifier { name, position })
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(&TokenKind::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                self.advance();
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
                self.consume(
                    &TokenKind::RightBracket,
                    "Expected ']' after array elements",
                )?;
                Ok(Expr::Literal {
                    value: Literal::Array(elements),
                    position,
                })
            }
            TokenKind::LeftBrace => self.parse_object_literal(),
            TokenKind::Match => self.parse_match_expression(),
            TokenKind::If => self.parse_if_expression(),
            TokenKind::Lambda => self.parse_lambda_expression(),
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: token.kind.clone(),
                position,
            }),
        }
    }

    /// Parse object literal: { key: value, ... }
    fn parse_object_literal(&mut self) -> Result<Expr, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::LeftBrace, "Expected '{'")?;

        let mut properties = Vec::new();

        if !self.check(&TokenKind::RightBrace) {
            loop {
                // Store peeked token kind in a local variable to avoid borrow checker issues
                let kind = &self.peek().kind;
                let key = if let TokenKind::Identifier(name) = kind {
                    name.clone()
                } else if let TokenKind::String(s) = kind {
                    s.clone()
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "property name".to_string(),
                        found: self.peek().kind.clone(),
                        position: self.peek().position.clone(),
                    });
                };
                self.advance();

                self.consume(&TokenKind::Colon, "Expected ':' after property name")?;
                let value = self.parse_expression()?;

                properties.push((key, value));

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after object literal")?;

        Ok(Expr::Literal {
            value: Literal::Object(properties),
            position: start_pos,
        })
    }

    /// Parse match expression: match expr { patterns }
    fn parse_match_expression(&mut self) -> Result<Expr, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Match, "Expected 'match'")?;

        let expr = Box::new(self.parse_expression()?);
        self.consume(&TokenKind::LeftBrace, "Expected '{' after match expression")?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            if self.check(&TokenKind::Newline) {
                self.advance();
                continue;
            }

            let pattern = self.parse_pattern()?;

            let guard = if self.check(&TokenKind::If) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            self.consume(&TokenKind::Arrow, "Expected '=>' after match pattern")?;
            let body = self.parse_expression()?;

            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });

            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        self.consume(&TokenKind::RightBrace, "Expected '}' after match arms")?;

        Ok(Expr::Match {
            expr,
            arms,
            position: start_pos,
        })
    }

    /// Parse if expression: if condition then expr else expr
    fn parse_if_expression(&mut self) -> Result<Expr, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::If, "Expected 'if'")?;
        let condition = Box::new(self.parse_expression()?);
        self.consume(&TokenKind::Then, "Expected 'then' after if condition")?;
        let then_expr = Box::new(self.parse_expression()?);
        self.consume(&TokenKind::Else, "Expected 'else' after then expression")?;
        let else_expr = Box::new(self.parse_expression()?);

        Ok(Expr::If {
            condition,
            then_expr,
            else_expr,
            position: start_pos,
        })
    }

    /// Parse lambda expression: |params| -> expr
    fn parse_lambda_expression(&mut self) -> Result<Expr, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Lambda, "Expected '|'")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::Lambda) {
            loop {
                let param_name = self.consume_identifier("Expected parameter name")?;
                params.push(param_name);

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.consume(&TokenKind::Lambda, "Expected closing '|'")?;
        self.consume(&TokenKind::Arrow, "Expected '->' after lambda parameters")?;

        let body = Box::new(self.parse_expression()?);

        Ok(Expr::Lambda {
            params,
            body,
            position: start_pos,
        })
    }

    /// Parse pattern for match expressions
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        match &self.peek().kind {
            TokenKind::Integer(n) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Integer(n)))
            }
            TokenKind::Float(f) => {
                let f = *f;
                self.advance();
                Ok(Pattern::Literal(Literal::Float(f)))
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::String(s)))
            }
            TokenKind::Boolean(b) => {
                let b = *b;
                self.advance();
                Ok(Pattern::Literal(Literal::Boolean(b)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Pattern::Literal(Literal::Null))
            }
            TokenKind::Identifier(name) => {
                if name == "_" {
                    self.advance();
                    Ok(Pattern::Wildcard)
                } else {
                    let name = name.clone();
                    self.advance();
                    Ok(Pattern::Identifier(name))
                }
            }
            TokenKind::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parse_pattern()?);
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.consume(&TokenKind::RightBracket, "Expected ']' after list pattern")?;
                Ok(Pattern::List(elements))
            }
            TokenKind::LeftBrace => {
                self.advance();
                let mut fields = Vec::new();
                if !self.check(&TokenKind::RightBrace) {
                    loop {
                        let key =
                            self.consume_identifier("Expected property name in object pattern")?;
                        let value = if self.check(&TokenKind::Colon) {
                            self.advance();
                            self.parse_pattern()?
                        } else {
                            Pattern::Identifier(key.clone())
                        };
                        fields.push((key, value));
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.consume(&TokenKind::RightBrace, "Expected '}' after object pattern")?;
                Ok(Pattern::Object(fields))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "pattern".to_string(),
                found: self.peek().kind.clone(),
                position: self.peek().position.clone(),
            }),
        }
    }

    // Utility: peek at current token
    fn peek(&self) -> &Token {
        self.tokens
            .get(self.current)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    // Utility: advance to next token and return previous
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.tokens.get(self.current - 1).unwrap()
    }

    // Utility: check if current token matches kind
    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            &self.peek().kind == kind
        }
    }

    // Utility: consume token of expected kind, or error
    fn consume(&mut self, kind: &TokenKind, _msg: &str) -> Result<&Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", kind),
                found: self.peek().kind.clone(),
                position: self.peek().position.clone(),
            })
        }
    }

    // Utility: consume identifier and return its name
    fn consume_identifier(&mut self, _msg: &str) -> Result<String, ParseError> {
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: self.peek().kind.clone(),
                position: self.peek().position.clone(),
            }),
        }
    }

    fn match_identifier(&mut self, expected: &str) -> bool {
        if let TokenKind::Identifier(name) = &self.peek().kind
            && name == expected
        {
            self.advance();
            return true;
        }
        false
    }

    fn expect_identifier_keyword(&mut self, keyword: &str) -> Result<(), ParseError> {
        if self.match_identifier(keyword) {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("'{}'", keyword),
                found: self.peek().kind.clone(),
                position: self.peek().position.clone(),
            })
        }
    }

    // Utility: consume statement terminator (semicolon or newline)
    fn consume_statement_terminator(&mut self) -> Result<(), ParseError> {
        if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Newline) {
            self.advance();
            Ok(())
        } else {
            // Allow EOF as statement terminator at end of input
            if self.is_at_end() {
                Ok(())
            } else {
                Err(ParseError::UnexpectedToken {
                    expected: "statement terminator".to_string(),
                    found: self.peek().kind.clone(),
                    position: self.peek().position.clone(),
                })
            }
        }
    }

    // Utility: are we at end of input?
    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    // Utility: get current position
    fn current_position(&self) -> Position {
        self.peek().position.clone()
    }

    // Utility: match binary operator and return BinaryOp
    fn match_binary_op(&mut self, kinds: &[TokenKind]) -> Option<BinaryOp> {
        for kind in kinds {
            if self.check(kind) {
                self.advance();
                return Some(match kind {
                    TokenKind::Plus => BinaryOp::Add,
                    TokenKind::Minus => BinaryOp::Subtract,
                    TokenKind::Star => BinaryOp::Multiply,
                    TokenKind::Slash => BinaryOp::Divide,
                    TokenKind::Percent => BinaryOp::Modulo,
                    TokenKind::EqualEqual => BinaryOp::Equal,
                    TokenKind::NotEqual => BinaryOp::NotEqual,
                    TokenKind::Less => BinaryOp::Less,
                    TokenKind::Greater => BinaryOp::Greater,
                    TokenKind::LessEqual => BinaryOp::LessEqual,
                    TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
                    TokenKind::And => BinaryOp::And,
                    TokenKind::Or => BinaryOp::Or,
                    _ => continue,
                });
            }
        }
        None
    }

    // Utility: match unary operator and return UnaryOp
    fn match_unary_op(&mut self, kinds: &[TokenKind]) -> Option<UnaryOp> {
        for kind in kinds {
            if self.check(kind) {
                self.advance();
                return Some(match kind {
                    TokenKind::Not => UnaryOp::Not,
                    TokenKind::Minus => UnaryOp::Minus,
                    TokenKind::Plus => UnaryOp::Plus,
                    _ => continue,
                });
            }
        }
        None
    }

    // Utility: parse type annotation (stub for now)
    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match &self.peek().kind {
            TokenKind::IntType => {
                self.advance();
                Ok(Type::Int)
            }
            TokenKind::FloatType => {
                self.advance();
                Ok(Type::Float)
            }
            TokenKind::StringType => {
                self.advance();
                Ok(Type::String)
            }
            TokenKind::BoolType => {
                self.advance();
                Ok(Type::Bool)
            }
            TokenKind::LeftBracket => {
                self.advance();
                let inner = self.parse_type()?;
                self.consume(&TokenKind::RightBracket, "Expected ']' after array type")?;
                Ok(Type::Array(Box::new(inner)))
            }
            TokenKind::LeftParen => {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        items.push(self.parse_type()?);
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.consume(&TokenKind::RightParen, "Expected ')' after tuple type")?;
                Ok(Type::Tuple(items))
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Type::Custom(name))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "type annotation".to_string(),
                found: self.peek().kind.clone(),
                position: self.peek().position.clone(),
            }),
        }
    }
} // End of Parser implementation
