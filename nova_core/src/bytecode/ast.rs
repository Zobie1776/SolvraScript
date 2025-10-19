//! Minimal AST used by the NovaCore assembler.
//!
//! The NovaScript front-end exposes a much richer AST but for the runtime we only need a
//! compact subset that captures expression evaluation.  The assembler consumes this AST and
//! emits bytecode consumable by the interpreter.

#[derive(Debug, Clone, PartialEq)]
pub struct Ast {
    pub expressions: Vec<Expr>,
}

impl Ast {
    pub fn from_expr(expr: Expr) -> Self {
        Self {
            expressions: vec![expr],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Boolean(bool),
    String(String),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

impl Expr {
    pub fn number(value: f64) -> Self {
        Expr::Number(value)
    }

    pub fn boolean(value: bool) -> Self {
        Expr::Boolean(value)
    }

    pub fn string(value: impl Into<String>) -> Self {
        Expr::String(value.into())
    }

    pub fn binary(op: BinaryOp, left: Expr, right: Expr) -> Self {
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
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
        assert_eq!(ast.expressions.len(), 1);
    }
}
