//! Abstract syntax tree representation consumed by the NovaCore assembler.
//!
//! The AST mirrors the high level constructs supported by NovaScript.  It purposefully keeps
//! expression semantics simple so the assembler can perform deterministic lowering into the
//! NovaCore intermediate representation.

use std::sync::Arc;

/// Source location associated with AST nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    file: Arc<str>,
    line: u32,
    column: u32,
}

impl Span {
    /// Creates a new span anchored at the specified location.
    pub fn new(file: impl Into<Arc<str>>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }

    /// Creates a span pointing to synthetic locations – useful for tests.
    pub fn synthetic() -> Self {
        Self {
            file: Arc::from("<synthetic>"),
            line: 0,
            column: 0,
        }
    }

    /// Returns the file portion of the span.
    pub fn file(&self) -> &Arc<str> {
        &self.file
    }

    /// Returns the line number (1-indexed).
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Returns the column number (1-indexed).
    pub fn column(&self) -> u32 {
        self.column
    }
}

/// Program level container grouping top-level statements and function declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct Ast {
    pub functions: Vec<Function>,
    pub body: Vec<Stmt>,
}

impl Ast {
    pub fn new(functions: Vec<Function>, body: Vec<Stmt>) -> Self {
        Self { functions, body }
    }

    /// Convenience helper that converts a single expression into a program returning that value.
    pub fn from_expr(expr: Expr) -> Self {
        let span = expr.span.clone();
        Self {
            functions: Vec::new(),
            body: vec![Stmt::Return(Some(expr), span)],
        }
    }
}

/// User defined function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

impl Function {
    pub fn new(name: impl Into<String>, params: Vec<String>, body: Vec<Stmt>, span: Span) -> Self {
        Self {
            name: name.into(),
            params,
            body,
            span,
        }
    }
}

/// Statements supported by the NovaScript subset consumed by NovaCore.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        expr: Expr,
        span: Span,
    },
    Assign {
        name: String,
        expr: Expr,
        span: Span,
    },
    Expr(Expr, Span),
    Return(Option<Expr>, Span),
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Vec<Stmt>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Try {
        try_block: Vec<Stmt>,
        catch_name: String,
        catch_block: Vec<Stmt>,
        finally_block: Vec<Stmt>,
        span: Span,
    },
    Throw {
        expr: Expr,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> &Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::Expr(_, span)
            | Stmt::Return(_, span)
            | Stmt::If { span, .. }
            | Stmt::While { span, .. }
            | Stmt::Try { span, .. }
            | Stmt::Throw { span, .. } => span,
        }
    }
}

/// Expressions supported by the NovaScript subset.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn number(value: f64) -> Self {
        Self {
            kind: ExprKind::Number(value),
            span: Span::synthetic(),
        }
    }

    pub fn boolean(value: bool) -> Self {
        Self {
            kind: ExprKind::Boolean(value),
            span: Span::synthetic(),
        }
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self {
            kind: ExprKind::String(value.into()),
            span: Span::synthetic(),
        }
    }

    pub fn identifier(name: impl Into<String>) -> Self {
        Self {
            kind: ExprKind::Identifier(name.into()),
            span: Span::synthetic(),
        }
    }

    pub fn binary(op: BinaryOp, left: Expr, right: Expr) -> Self {
        Self {
            kind: ExprKind::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            },
            span: Span::synthetic(),
        }
    }

    pub fn unary(op: UnaryOp, expr: Expr) -> Self {
        Self {
            kind: ExprKind::Unary {
                op,
                expr: Box::new(expr),
            },
            span: Span::synthetic(),
        }
    }

    pub fn call(callee: Expr, args: Vec<Expr>) -> Self {
        Self {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                args,
            },
            span: Span::synthetic(),
        }
    }

    pub fn lambda(params: Vec<String>, body: Vec<Stmt>) -> Self {
        Self {
            kind: ExprKind::Lambda { params, body },
            span: Span::synthetic(),
        }
    }

    pub fn list(elements: Vec<Expr>) -> Self {
        Self {
            kind: ExprKind::List(elements),
            span: Span::synthetic(),
        }
    }

    pub fn map(entries: Vec<(Expr, Expr)>) -> Self {
        Self {
            kind: ExprKind::Map(entries),
            span: Span::synthetic(),
        }
    }

    pub fn index(target: Expr, index: Expr) -> Self {
        Self {
            kind: ExprKind::Index {
                target: Box::new(target),
                index: Box::new(index),
            },
            span: Span::synthetic(),
        }
    }
}

/// Concrete expression kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Number(f64),
    Boolean(bool),
    String(String),
    Identifier(String),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Lambda {
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equals,
    NotEquals,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_ast_conveniently() {
        let ast = Ast::from_expr(Expr::binary(
            BinaryOp::Add,
            Expr::number(1.0),
            Expr::number(2.0),
        ));
        assert_eq!(ast.body.len(), 1);
    }
}
