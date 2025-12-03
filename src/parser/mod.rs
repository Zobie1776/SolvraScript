//=============================================
// solvra_script/parser.rs
//=============================================
// Author: SolvraOS Contributors
// License: Duality Public License (DPL v1.0)
// Goal: SolvraScript recursive descent parser implementation
// Objective: Transform token streams into AST nodes consumed by interpreter
// Formatting: Zobie.format (.solvraformat)
//=============================================

//=============================================
//            Section 1: Crate Attributes & Imports
//=============================================

#![allow(dead_code)]

use crate::ast::{
    AssignTarget, BinaryOp, BindingKind, CatchBlock, ExportDecl, ExportItem, Expr, FunctionDecl,
    ImportDecl, ImportSource, Literal, MatchArm, MemberKind, Parameter, Pattern, Program, Span,
    Stmt, StringPart, Type, TypeNode, UnaryOp, VariableDecl, Visibility, next_node_id,
};
use crate::symbol::Symbol;
use crate::tokenizer::{Position, Token, TokenKind};

//=============================================/*
//  Collects AST type dependencies and tokenizer traits required for parsing.
//============================================*/
//=============================================
//            Section 2: Parse Errors
//=============================================

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

//=============================================/*
//  Captures parser-facing diagnostics enriched with token position metadata.
//============================================*/
//=============================================
//            Section 3: Parser State
//=============================================

/// Recursive descent parser for SolvraScript
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    expr_depth: usize,
}

const MAX_EXPRESSION_DEPTH: usize = 2048;

//=============================================/*
//  Maintains parser state across token streams for SolvraScript compilation.
//============================================*/
impl Parser {
    //Function: new
    //Purpose: Initialize parser with token stream and reset cursor
    //Inputs: tokens: Vec<Token>
    //Returns: Self
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            expr_depth: 0,
        }
    }

    //=============================================
    //            Section 4: Token Navigation
    //=============================================
    fn skip_layout_tokens(&mut self) {
        while matches!(
            &self.peek().kind,
            TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::Comment(_)
        ) {
            self.advance();
        }
    }

    //=============================================
    //            Section 5: Statement Parsing
    //=============================================
    /// Parse a complete SolvraScript program
    //Function: parse
    //Purpose: Consume tokens and produce a SolvraScript program AST
    //Inputs: &mut self
    //Returns: Result<Program, ParseError>
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

        let mut program = Program::new(statements, position);
        program.ensure_entry_point();
        Ok(program)
    }

    /// Parse a single expression and ensure the stream is fully consumed.
    pub fn parse_expression_only(&mut self) -> Result<Expr, ParseError> {
        let expression = self.parse_expression()?;
        if !self.is_at_end() {
            let token = self.peek();
            return Err(ParseError::UnexpectedToken {
                expected: "end of expression".into(),
                found: token.kind.clone(),
                position: token.position.clone(),
            });
        }
        Ok(expression)
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
            TokenKind::Async => {
                if matches!(self.peek_next().kind, TokenKind::Fn) {
                    self.parse_function_declaration()
                } else {
                    self.parse_expression_statement()
                }
            }
            TokenKind::Import => self.parse_import_declaration(),
            TokenKind::Export => self.parse_export_statement(),
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
                }
                true
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

        let (var_type, type_annotation) = if self.check(&TokenKind::Colon) {
            self.advance();
            let (ty, node) = self.parse_type_node()?;
            (ty, Some(node))
        } else {
            (Type::Inferred, None)
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
            type_annotation,
            binding,
            is_mutable,
            initializer,
            position: start_pos,
            node_id: next_node_id(),
        };

        Ok(Stmt::VariableDecl { decl })
    }

    fn parse_export_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.advance(); // consume 'export'

        match &self.peek().kind {
            TokenKind::Async | TokenKind::Fn => {
                let stmt = self.parse_function_declaration()?;
                if let Stmt::FunctionDecl { mut decl } = stmt {
                    decl.visibility = Visibility::Public;
                    Ok(Stmt::ExportDecl {
                        decl: ExportDecl::new(ExportItem::Function(decl), start_pos),
                    })
                } else {
                    Err(ParseError::InvalidSyntax {
                        message: "Expected function declaration after export".into(),
                        position: start_pos,
                    })
                }
            }
            TokenKind::Let => {
                self.advance();
                let stmt = self.parse_variable_declaration(start_pos.clone(), BindingKind::Let)?;
                if let Stmt::VariableDecl { decl } = stmt {
                    Ok(Stmt::ExportDecl {
                        decl: ExportDecl::new(ExportItem::Variable(decl), start_pos),
                    })
                } else {
                    Err(ParseError::InvalidSyntax {
                        message: "Expected variable declaration after export".into(),
                        position: start_pos,
                    })
                }
            }
            TokenKind::Const => {
                self.advance();
                let stmt =
                    self.parse_variable_declaration(start_pos.clone(), BindingKind::Const)?;
                if let Stmt::VariableDecl { decl } = stmt {
                    Ok(Stmt::ExportDecl {
                        decl: ExportDecl::new(ExportItem::Variable(decl), start_pos),
                    })
                } else {
                    Err(ParseError::InvalidSyntax {
                        message: "Expected const declaration after export".into(),
                        position: start_pos,
                    })
                }
            }
            _ => {
                let (name, alias) = self.parse_export_symbol_spec()?;
                self.consume_statement_terminator()?;
                Ok(Stmt::ExportDecl {
                    decl: ExportDecl::new(ExportItem::Symbol { name, alias }, start_pos),
                })
            }
        }
    }

    fn parse_export_symbol_spec(&mut self) -> Result<(Symbol, Option<Symbol>), ParseError> {
        let name = self.consume_export_name()?;
        let alias = if self.match_identifier("as") {
            Some(self.consume_identifier("Expected alias name")?)
        } else {
            None
        };
        Ok((name, alias))
    }

    fn consume_export_name(&mut self) -> Result<Symbol, ParseError> {
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let sym = name.clone();
                self.advance();
                Ok(sym)
            }
            TokenKind::StringType => {
                self.advance();
                Ok(Symbol::from("string"))
            }
            TokenKind::IntType => {
                self.advance();
                Ok(Symbol::from("int"))
            }
            TokenKind::FloatType => {
                self.advance();
                Ok(Symbol::from("float"))
            }
            TokenKind::BoolType => {
                self.advance();
                Ok(Symbol::from("bool"))
            }
            other => Err(ParseError::InvalidSyntax {
                message: format!("Unexpected token after export: {:?}", other),
                position: self.peek().position.clone(),
            }),
        }
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

                let (param_type, type_annotation) = if self.check(&TokenKind::Colon) {
                    self.advance();
                    let (ty, node) = self.parse_type_node()?;
                    (ty, Some(node))
                } else {
                    (Type::Inferred, None)
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
                    type_annotation,
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

        let (return_type, return_type_node) = if self.check(&TokenKind::Arrow) {
            self.advance();
            let (ty, node) = self.parse_type_node()?;
            (ty, Some(node))
        } else {
            (Type::Inferred, None)
        };

        self.consume(&TokenKind::LeftBrace, "Expected '{' before function body")?;
        let body = self.parse_block_body()?;

        let decl = FunctionDecl {
            name,
            params,
            return_type,
            return_type_node,
            body,
            is_async,
            visibility: Visibility::Private,
            position: start_pos,
            node_id: next_node_id(),
        };

        Ok(Stmt::FunctionDecl { decl })
    }

    /// Parse import declaration: import module [as alias];
    fn parse_import_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::Import, "Expected 'import'")?;

        let (source, items) = if self.check(&TokenKind::LeftBrace) {
            self.advance();
            let mut items = Vec::new();
            while !self.check(&TokenKind::RightBrace) {
                let item = self.consume_identifier("Expected imported item name")?;
                items.push(item.to_string());
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.consume(&TokenKind::RightBrace, "Expected '}' after import list")?;
            self.expect_identifier_keyword("from")?;
            let source = self.parse_import_source()?;
            (source, items)
        } else {
            (self.parse_import_source()?, Vec::new())
        };

        let alias = if self.match_identifier("as") {
            Some(self.consume_identifier("Expected alias name")?.to_string())
        } else {
            None
        };

        self.consume_statement_terminator()?;

        let decl = ImportDecl {
            source,
            items,
            alias,
            position: start_pos,
        };

        Ok(Stmt::ImportDecl { decl })
    }

    fn parse_import_source(&mut self) -> Result<ImportSource, ParseError> {
        match &self.peek().kind {
            TokenKind::String(path) => {
                let path = path.to_string();
                self.advance();
                Ok(ImportSource::ScriptPath(path))
            }
            TokenKind::Less => {
                self.advance();
                let name = self
                    .consume_identifier("Expected standard module name")?
                    .to_string();
                self.consume(&TokenKind::Greater, "Expected '>' after module name")?;
                Ok(ImportSource::StandardModule(name))
            }
            TokenKind::Identifier(_) => {
                let name = self.parse_module_path()?;
                Ok(ImportSource::BareModule(name))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "module path (\"file.svs\" or <module>)".to_string(),
                found: self.peek().kind.clone(),
                position: self.peek().position.clone(),
            }),
        }
    }

    fn parse_module_path(&mut self) -> Result<String, ParseError> {
        let mut name = self.consume_identifier("Expected module name")?.to_string();
        while self.check(&TokenKind::Dot) {
            self.advance();
            name.push('.');
            name.push_str(
                &self
                    .consume_identifier("Expected module segment")?
                    .to_string(),
            );
        }
        Ok(name)
    }

    /// Parse if statement: if condition { body } [else { body }]
    fn parse_if_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_pos = self.current_position();
        self.consume(&TokenKind::If, "Expected 'if'")?;

        let condition = self.parse_expression()?;
        let then_branch = Box::new(self.parse_statement()?);

        let else_branch = self.parse_optional_else_clause()?;

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            position: start_pos,
        })
    }

    fn parse_optional_else_clause(&mut self) -> Result<Option<Box<Stmt>>, ParseError> {
        if self.check(&TokenKind::Else) {
            self.advance();
            return Ok(Some(Box::new(self.parse_statement()?)));
        }

        if self.check(&TokenKind::Elif) {
            let start_pos = self.current_position();
            self.advance();
            let condition = self.parse_expression()?;
            let then_branch = Box::new(self.parse_statement()?);
            let else_branch = self.parse_optional_else_clause()?;
            let nested = Stmt::If {
                condition,
                then_branch,
                else_branch,
                position: start_pos,
            };
            return Ok(Some(Box::new(nested)));
        }

        Ok(None)
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
            node_id: next_node_id(),
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
                let name = name.to_string();
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

        let finally_block = if self.check(&TokenKind::Identifier(Symbol::from("finally"))) {
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

    //=============================================/*
    //  Handles SolvraScript statements including declarations and control flow structures.
    //============================================*/
    //=============================================
    //            Section 6: Expression Parsing
    //=============================================

    fn enter_expression(&mut self) -> Result<(), ParseError> {
        if self.expr_depth >= MAX_EXPRESSION_DEPTH {
            return Err(ParseError::InvalidSyntax {
                message: format!(
                    "expression recursion limit of {} exceeded",
                    MAX_EXPRESSION_DEPTH
                ),
                position: self.peek().position.clone(),
            });
        }
        self.expr_depth += 1;
        Ok(())
    }

    fn exit_expression(&mut self) {
        if self.expr_depth > 0 {
            self.expr_depth -= 1;
        }
    }

    /// Parse expression with precedence climbing
    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.enter_expression()?;
        let result = self.parse_assignment();
        self.exit_expression();
        result
    }

    /// Parse assignment expression: target = value
    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_logical_or()?;

        if let Some(op) = self.match_compound_assignment() {
            let assign_pos = expr.position().clone();
            let target_expr = expr.clone();
            let target = self.assignment_target_from_expr(expr)?;
            let value = self.parse_assignment()?;
            let combined = Expr::Binary {
                left: Box::new(target_expr),
                operator: op,
                right: Box::new(value),
                position: assign_pos.clone(),
            };
            return Ok(Expr::assignment(target, combined, assign_pos));
        }

        if self.check(&TokenKind::Equal) {
            let assign_pos = expr.position().clone();
            self.advance();
            let value = self.parse_assignment()?;
            let target = self.assignment_target_from_expr(expr)?;

            return Ok(Expr::assignment(target, value, assign_pos));
        }

        Ok(expr)
    }

    fn match_compound_assignment(&mut self) -> Option<BinaryOp> {
        let op = match self.peek().kind {
            TokenKind::PlusEqual => Some(BinaryOp::Add),
            TokenKind::MinusEqual => Some(BinaryOp::Subtract),
            TokenKind::StarEqual => Some(BinaryOp::Multiply),
            TokenKind::SlashEqual => Some(BinaryOp::Divide),
            _ => None,
        }?;
        self.advance();
        Some(op)
    }

    fn assignment_target_from_expr(&self, expr: Expr) -> Result<AssignTarget, ParseError> {
        match expr {
            Expr::Identifier { name, .. } => Ok(AssignTarget::Variable(name)),
            Expr::Index { object, index, .. } => Ok(AssignTarget::Index {
                array: object,
                index,
            }),
            Expr::Member {
                object, property, ..
            } => Ok(AssignTarget::Member { object, property }),
            other => Err(ParseError::InvalidSyntax {
                message: "unsupported assignment target".to_string(),
                position: other.position().clone(),
            }),
        }
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
            TokenKind::Is,
        ]) {
            let start_pos = self.current_position();
            let mut operator = op;
            if matches!(operator, BinaryOp::Is) && self.check(&TokenKind::Not) {
                self.advance();
                operator = BinaryOp::IsNot;
            }
            let right = self.parse_term()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
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
        if self.check(&TokenKind::Async) {
            let position = self.current_position();
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Async {
                expr: Box::new(expr),
                position,
            });
        }

        if self.check(&TokenKind::Await) {
            let position = self.current_position();
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Await {
                expr: Box::new(expr),
                position,
            });
        }

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
            let kind = self.peek().kind.clone();
            if kind == TokenKind::LeftParen {
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

                expr = match expr {
                    Expr::Member {
                        object,
                        property,
                        kind: MemberKind::Dot,
                        ..
                    } => Expr::MethodCall {
                        receiver: object,
                        method: property,
                        args,
                        position: start_pos,
                    },
                    other => Expr::Call {
                        callee: Box::new(other),
                        args,
                        position: start_pos,
                    },
                };
            } else if matches!(kind, TokenKind::Dot | TokenKind::DoubleColon) {
                let start_pos = self.current_position();
                self.advance();
                let property = self.consume_identifier("Expected property name after '.'")?;

                expr = Expr::Member {
                    object: Box::new(expr),
                    property,
                    position: start_pos,
                    kind: if kind == TokenKind::Dot {
                        MemberKind::Dot
                    } else {
                        MemberKind::DoubleColon
                    },
                };
            } else if kind == TokenKind::LeftBracket {
                let start_pos = self.current_position();
                self.advance();
                let mut start = None;
                let mut end = None;
                let mut step = None;

                if !self.check(&TokenKind::Colon)
                    && !self.check(&TokenKind::DoubleColon)
                    && !self.check(&TokenKind::RightBracket)
                {
                    start = Some(self.parse_expression()?);
                }

                if self.check(&TokenKind::DoubleColon) {
                    self.advance();
                    if !self.check(&TokenKind::RightBracket) {
                        step = Some(self.parse_expression()?);
                    }
                    self.consume(&TokenKind::RightBracket, "Expected ']' after slice")?;
                    let end_pos = self.previous_position();
                    expr = Expr::Slice {
                        object: Box::new(expr),
                        start: start.map(Box::new),
                        end: end.map(Box::new),
                        step: step.map(Box::new),
                        span: Span::new(start_pos, end_pos),
                        node_id: next_node_id(),
                    };
                } else if self.check(&TokenKind::Colon) {
                    self.advance();
                    if !self.check(&TokenKind::Colon) && !self.check(&TokenKind::RightBracket) {
                        end = Some(self.parse_expression()?);
                    }
                    if self.check(&TokenKind::Colon) {
                        self.advance();
                        if !self.check(&TokenKind::RightBracket) {
                            step = Some(self.parse_expression()?);
                        }
                    }
                    self.consume(&TokenKind::RightBracket, "Expected ']' after slice")?;
                    let end_pos = self.previous_position();
                    expr = Expr::Slice {
                        object: Box::new(expr),
                        start: start.map(Box::new),
                        end: end.map(Box::new),
                        step: step.map(Box::new),
                        span: Span::new(start_pos, end_pos),
                        node_id: next_node_id(),
                    };
                } else {
                    self.consume(&TokenKind::RightBracket, "Expected ']' after array index")?;
                    let index = start.ok_or_else(|| ParseError::InvalidSyntax {
                        message: "missing index expression".to_string(),
                        position: self.current_position(),
                    })?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        position: start_pos,
                    };
                }
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
                let s = Symbol::from(s.as_str());
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
                Ok(Expr::Identifier {
                    name,
                    position,
                    node_id: next_node_id(),
                })
            }
            TokenKind::StringType => {
                self.advance();
                Ok(Expr::Identifier {
                    name: Symbol::from("string"),
                    position,
                    node_id: next_node_id(),
                })
            }
            TokenKind::IntType => {
                self.advance();
                Ok(Expr::Identifier {
                    name: Symbol::from("int"),
                    position,
                    node_id: next_node_id(),
                })
            }
            TokenKind::FloatType => {
                self.advance();
                Ok(Expr::Identifier {
                    name: Symbol::from("float"),
                    position,
                    node_id: next_node_id(),
                })
            }
            TokenKind::BoolType => {
                self.advance();
                Ok(Expr::Identifier {
                    name: Symbol::from("bool"),
                    position,
                    node_id: next_node_id(),
                })
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
            TokenKind::Lambda => {
                self.advance();
                self.parse_lambda_expression()
            }
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
                self.skip_layout_tokens();
                if self.check(&TokenKind::RightBrace) {
                    break;
                }
                // Store peeked token kind in a local variable to avoid borrow checker issues
                let key = match &self.peek().kind {
                    TokenKind::Identifier(name) | TokenKind::String(name) => name.clone(),
                    TokenKind::StringTemplate(value) => Symbol::from(value.as_str()),
                    _ => {
                        return Err(ParseError::UnexpectedToken {
                            expected: "property name".to_string(),
                            found: self.peek().kind.clone(),
                            position: self.peek().position.clone(),
                        });
                    }
                };
                self.advance();

                self.consume(&TokenKind::Colon, "Expected ':' after property name")?;
                self.skip_layout_tokens();
                let value = self.parse_expression()?;
                self.skip_layout_tokens();

                properties.push((key, value));

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
                self.skip_layout_tokens();
            }
        }

        self.skip_layout_tokens();
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
        self.consume(&TokenKind::Pipe, "Expected '|' after lambda keyword")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::Pipe) {
            loop {
                let param_name = self.consume_identifier("Expected parameter name")?;
                params.push(param_name);

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.consume(&TokenKind::Pipe, "Expected closing '|'")?;
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
                if name.as_str() == "_" {
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

    fn peek_next(&self) -> &Token {
        if self.current + 1 >= self.tokens.len() {
            self.tokens.last().unwrap()
        } else {
            &self.tokens[self.current + 1]
        }
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
    fn consume_identifier(&mut self, _msg: &str) -> Result<Symbol, ParseError> {
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            TokenKind::StringType => {
                self.advance();
                Ok(Symbol::from("string"))
            }
            TokenKind::IntType => {
                self.advance();
                Ok(Symbol::from("int"))
            }
            TokenKind::FloatType => {
                self.advance();
                Ok(Symbol::from("float"))
            }
            TokenKind::BoolType => {
                self.advance();
                Ok(Symbol::from("bool"))
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
            && name.as_str() == expected
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

    fn previous_position(&self) -> Position {
        if self.current == 0 {
            self.current_position()
        } else {
            self.tokens[self.current - 1].position.clone()
        }
    }

    //=============================================/*
    //  Wraps token navigation helpers for layout-sensitive SolvraScript parsing.
    //============================================*/
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
                    TokenKind::Is => BinaryOp::Is,
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

    //=============================================/*
    //  Implements precedence-based expression parsing and operator binding.
    //============================================*/
    //=============================================
    //            Section 7: Type Parsing
    //=============================================

    fn parse_type_node(&mut self) -> Result<(Type, TypeNode), ParseError> {
        let start = self.current_position();
        let ty = self.parse_type()?;
        let end = self.previous_position();
        let span = Span::new(start, end);
        Ok((ty.clone(), TypeNode { ty, span }))
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
                let name = name.to_string();
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
    //=============================================/*
    //  Parses SolvraScript type annotations and composite signatures.
    //============================================*/
} // End of Parser implementation

//=============================================
// End Of solvra_script/parser.rs
//=============================================
// Notes:
// -[@TODOS] Expand type parsing once generics and interfaces solidify.
//=============================================
