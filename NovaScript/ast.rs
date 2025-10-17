use crate::tokenizer::Position;
use std::collections::HashMap;

/// Represents a NovaScript type annotation
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Null,
    Array(Box<Type>),
    Object(HashMap<String, Type>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Custom(String),
    Inferred, // For type inference
}

impl Type {
    pub fn to_string(&self) -> String {
        match self {
            Type::Int => "int".to_string(),
            Type::Float => "float".to_string(),
            Type::String => "string".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Null => "null".to_string(),
            Type::Array(inner) => format!("[{}]", inner.to_string()),
            Type::Object(fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", field_strs.join(", "))
            }
            Type::Function {
                params,
                return_type,
            } => {
                let param_types: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                format!(
                    "({}) -> {}",
                    param_types.join(", "),
                    return_type.to_string()
                )
            }
            Type::Custom(name) => name.clone(),
            Type::Inferred => "auto".to_string(),
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    In,
    NotIn,
    Is,
    IsNot,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Minus,
    Plus,
    BitwiseNot,
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>), // Changed from HashMap for parser compatibility
}

/// Expressions in NovaScript
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        value: Literal,
        position: Position,
    },
    Identifier {
        name: String,
        position: Position,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
        position: Position,
    },
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
        position: Position,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        position: Position,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        position: Position,
    },
    Member {
        object: Box<Expr>,
        property: String,
        position: Position,
    },
    // Add string interpolation and template if referenced
    StringInterpolation {
        parts: Vec<StringPart>,
        position: Position,
    },
    If {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        position: Position,
    },
    StringTemplate {
        parts: Vec<StringPart>,
        position: Position,
    },
    Assignment {
        target: Box<Expr>,
        value: Box<Expr>,
        position: Position,
    },
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
        position: Position,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        position: Position,
    },
    // @ZNOTE[NovaCore Hook]: Async expressions will need special handling in bytecode
    Async {
        expr: Box<Expr>,
        position: Position,
    },
    Await {
        expr: Box<Expr>,
        position: Position,
    },
    Conditional {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        position: Position,
    },
    List {
        elements: Vec<Expr>,
        position: Position,
    },
    Tuple {
        elements: Vec<Expr>,
        position: Position,
    },
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
        position: Position,
    },
    Comprehension {
        element: Box<Expr>,
        variable: String,
        iterable: Box<Expr>,
        condition: Option<Box<Expr>>,
        position: Position,
    },
}

impl Expr {
    pub fn position(&self) -> &Position {
        match self {
            Expr::Literal { position, .. } => position,
            Expr::Identifier { position, .. } => position,
            Expr::Binary { position, .. } => position,
            Expr::Unary { position, .. } => position,
            Expr::Call { position, .. } => position,
            Expr::Index { position, .. } => position,
            Expr::Member { position, .. } => position,
            Expr::StringInterpolation { position, .. } => position,
            Expr::If { position, .. } => position,
            Expr::StringTemplate { position, .. } => position,
            Expr::Assignment { position, .. } => position,
            Expr::Lambda { position, .. } => position,
            Expr::Match { position, .. } => position,
            Expr::Async { position, .. } => position,
            Expr::Await { position, .. } => position,
            Expr::Conditional { position, .. } => position,
            Expr::List { position, .. } => position,
            Expr::Tuple { position, .. } => position,
            Expr::Range { position, .. } => position,
            Expr::Comprehension { position, .. } => position,
        }
    }
}

/// Parts of a string interpolation
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Literal(String),
    Expression(Expr),
}

/// Pattern matching patterns
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Literal),
    Identifier(String),
    Wildcard,
    List(Vec<Pattern>),
    Object(Vec<(String, Pattern)>),
    Tuple(Vec<Pattern>),
    Constructor {
        name: String,
        fields: Vec<Pattern>,
    },
    Range {
        start: Box<Pattern>,
        end: Box<Pattern>,
    },
    Guard {
        pattern: Box<Pattern>,
        condition: Expr,
    },
}

/// Match expression arms
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

/// Function parameters
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub param_type: Type,
    pub default_value: Option<Expr>,
    pub position: Position,
}

/// Variable declarations
#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub name: String,
    pub var_type: Type,
    pub is_mutable: bool,
    pub initializer: Option<Expr>,
    pub position: Position,
}

/// Function declarations
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Stmt>,
    pub is_async: bool,
    pub visibility: Visibility,
    pub position: Position,
}

/// Class declarations
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub superclass: Option<String>,
    pub methods: Vec<FunctionDecl>,
    pub fields: Vec<VariableDecl>,
    pub visibility: Visibility,
    pub position: Position,
}

/// Interface declarations
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: String,
    pub methods: Vec<FunctionSignature>,
    pub superinterfaces: Vec<String>,
    pub position: Position,
}

/// Function signatures (for interfaces)
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub is_async: bool,
    pub position: Position,
}

/// Import declarations
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub module: String,
    pub items: Vec<String>, // Empty for wildcard imports
    pub alias: Option<String>,
    pub position: Position,
}

/// Export declarations
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDecl {
    pub item: ExportItem,
    pub position: Position,
}

/// Items that can be exported
#[derive(Debug, Clone, PartialEq)]
pub enum ExportItem {
    Function(FunctionDecl),
    Variable(VariableDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Type(TypeDecl),
    Module(String),
}

/// Visibility modifiers
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Statements in NovaScript
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expression {
        expr: Expr,
        position: Position,
    },
    VariableDecl {
        decl: VariableDecl,
    },
    FunctionDecl {
        decl: FunctionDecl,
    },
    ClassDecl {
        decl: ClassDecl,
    },
    InterfaceDecl {
        decl: InterfaceDecl,
    },
    ImportDecl {
        decl: ImportDecl,
    },
    ExportDecl {
        decl: ExportDecl,
    },
    Block {
        statements: Vec<Stmt>,
        position: Position,
    },
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        position: Position,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
        position: Position,
    },
    For {
        variable: String,
        iterable: Expr,
        body: Box<Stmt>,
        position: Position,
    },
    ForIn {
        variable: String,
        iterable: Expr,
        body: Box<Stmt>,
        position: Position,
    },
    ForOf {
        variable: String,
        iterable: Expr,
        body: Box<Stmt>,
        position: Position,
    },
    Loop {
        body: Box<Stmt>,
        position: Position,
    },
    Return {
        value: Option<Expr>,
        position: Position,
    },
    Break {
        label: Option<String>,
        position: Position,
    },
    Continue {
        label: Option<String>,
        position: Position,
    },
    Try {
        try_block: Box<Stmt>,
        catch_blocks: Vec<CatchBlock>,
        finally_block: Option<Box<Stmt>>,
        position: Position,
    },
    Throw {
        expr: Expr,
        position: Position,
    },
    // @ZNOTE[NovaStdLib Hook]: Panic statements will integrate with NovaOS error handling
    Panic {
        message: Option<Expr>,
        position: Position,
    },
    Defer {
        stmt: Box<Stmt>,
        position: Position,
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
        position: Position,
    },
    With {
        expr: Expr,
        body: Box<Stmt>,
        position: Position,
    },
    Switch {
        expr: Expr,
        cases: Vec<SwitchCase>,
        default_case: Option<Box<Stmt>>,
        position: Position,
    },
    Label {
        name: String,
        stmt: Box<Stmt>,
        position: Position,
    },
    Goto {
        label: String,
        position: Position,
    },
}

impl Stmt {
    pub fn position(&self) -> &Position {
        match self {
            Stmt::Expression { position, .. } => position,
            Stmt::VariableDecl { decl } => &decl.position,
            Stmt::FunctionDecl { decl } => &decl.position,
            Stmt::ClassDecl { decl } => &decl.position,
            Stmt::InterfaceDecl { decl } => &decl.position,
            Stmt::ImportDecl { decl } => &decl.position,
            Stmt::ExportDecl { decl } => &decl.position,
            Stmt::Block { position, .. } => position,
            Stmt::If { position, .. } => position,
            Stmt::While { position, .. } => position,
            Stmt::For { position, .. } => position,
            Stmt::ForIn { position, .. } => position,
            Stmt::ForOf { position, .. } => position,
            Stmt::Loop { position, .. } => position,
            Stmt::Return { position, .. } => position,
            Stmt::Break { position, .. } => position,
            Stmt::Continue { position, .. } => position,
            Stmt::Try { position, .. } => position,
            Stmt::Throw { position, .. } => position,
            Stmt::Panic { position, .. } => position,
            Stmt::Defer { position, .. } => position,
            Stmt::Match { position, .. } => position,
            Stmt::With { position, .. } => position,
            Stmt::Switch { position, .. } => position,
            Stmt::Label { position, .. } => position,
            Stmt::Goto { position, .. } => position,
        }
    }
}

/// Switch case
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub values: Vec<Expr>,
    pub body: Vec<Stmt>,
    pub position: Position,
}

/// Catch blocks for try-catch statements
#[derive(Debug, Clone, PartialEq)]
pub struct CatchBlock {
    pub exception_type: Option<Type>,
    pub variable: Option<String>,
    pub body: Box<Stmt>,
    pub position: Position,
}

/// Complete NovaScript program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
    pub position: Position,
}

/// Namespace declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Namespace {
    pub name: String,
    pub items: Vec<NamespaceItem>,
    pub position: Position,
}

/// Items that can be in a namespace
#[derive(Debug, Clone, PartialEq)]
pub enum NamespaceItem {
    Function(FunctionDecl),
    Variable(VariableDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Type(TypeDecl),
    Namespace(Namespace),
}

/// Type declarations (for future extensibility)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name: String,
    pub type_def: Type,
    pub position: Position,
}

/// Enum declarations
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub position: Position,
}

/// Enum variant
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<Expr>,
    pub fields: Vec<Type>,
    pub position: Position,
}

/// Trait declarations
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    pub name: String,
    pub methods: Vec<FunctionSignature>,
    pub associated_types: Vec<String>,
    pub supertraits: Vec<String>,
    pub position: Position,
}

/// Implementation blocks
#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    pub target_type: Type,
    pub trait_name: Option<String>,
    pub methods: Vec<FunctionDecl>,
    pub associated_items: Vec<AssociatedItem>,
    pub position: Position,
}

/// Associated items in impl blocks
#[derive(Debug, Clone, PartialEq)]
pub enum AssociatedItem {
    Function(FunctionDecl),
    Type(TypeDecl),
    Constant(VariableDecl),
}

/// Utility struct for AST traversal and manipulation
pub struct AstVisitor<T> {
    pub visit_expr: Option<Box<dyn Fn(&Expr) -> T>>,
    pub visit_stmt: Option<Box<dyn Fn(&Stmt) -> T>>,
}

impl<T> AstVisitor<T> {
    pub fn new() -> Self {
        Self {
            visit_expr: None,
            visit_stmt: None,
        }
    }
}

/// AST utility functions
impl Program {
    pub fn new(statements: Vec<Stmt>, position: Position) -> Self {
        Self {
            statements,
            position,
        }
    }

    /// Find all function declarations in the program
    pub fn find_functions(&self) -> Vec<&FunctionDecl> {
        let mut functions = Vec::new();
        for stmt in &self.statements {
            if let Stmt::FunctionDecl { decl } = stmt {
                functions.push(decl);
            }
        }
        functions
    }

    /// Find all variable declarations in the program
    pub fn find_variables(&self) -> Vec<&VariableDecl> {
        let mut variables = Vec::new();
        for stmt in &self.statements {
            if let Stmt::VariableDecl { decl } = stmt {
                variables.push(decl);
            }
        }
        variables
    }

    /// Find all class declarations in the program
    pub fn find_classes(&self) -> Vec<&ClassDecl> {
        let mut classes = Vec::new();
        for stmt in &self.statements {
            if let Stmt::ClassDecl { decl } = stmt {
                classes.push(decl);
            }
        }
        classes
    }

    /// Find all interface declarations in the program
    pub fn find_interfaces(&self) -> Vec<&InterfaceDecl> {
        let mut interfaces = Vec::new();
        for stmt in &self.statements {
            if let Stmt::InterfaceDecl { decl } = stmt {
                interfaces.push(decl);
            }
        }
        interfaces
    }

    /// Find all import declarations in the program
    pub fn find_imports(&self) -> Vec<&ImportDecl> {
        let mut imports = Vec::new();
        for stmt in &self.statements {
            if let Stmt::ImportDecl { decl } = stmt {
                imports.push(decl);
            }
        }
        imports
    }

    /// Find all export declarations in the program
    pub fn find_exports(&self) -> Vec<&ExportDecl> {
        let mut exports = Vec::new();
        for stmt in &self.statements {
            if let Stmt::ExportDecl { decl } = stmt {
                exports.push(decl);
            }
        }
        exports
    }
}

/// Helper functions for creating AST nodes
impl Expr {
    pub fn literal(value: Literal, position: Position) -> Self {
        Expr::Literal { value, position }
    }

    pub fn identifier(name: String, position: Position) -> Self {
        Expr::Identifier { name, position }
    }

    pub fn binary(left: Expr, operator: BinaryOp, right: Expr, position: Position) -> Self {
        Expr::Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
            position,
        }
    }

    pub fn unary(operator: UnaryOp, operand: Expr, position: Position) -> Self {
        Expr::Unary {
            operator,
            operand: Box::new(operand),
            position,
        }
    }

    pub fn call(callee: Expr, args: Vec<Expr>, position: Position) -> Self {
        Expr::Call {
            callee: Box::new(callee),
            args,
            position,
        }
    }

    pub fn index(object: Expr, index: Expr, position: Position) -> Self {
        Expr::Index {
            object: Box::new(object),
            index: Box::new(index),
            position,
        }
    }

    pub fn member(object: Expr, property: String, position: Position) -> Self {
        Expr::Member {
            object: Box::new(object),
            property,
            position,
        }
    }

    pub fn assignment(target: Expr, value: Expr, position: Position) -> Self {
        Expr::Assignment {
            target: Box::new(target),
            value: Box::new(value),
            position,
        }
    }

    pub fn lambda(params: Vec<String>, body: Expr, position: Position) -> Self {
        Expr::Lambda {
            params,
            body: Box::new(body),
            position,
        }
    }

    pub fn conditional(
        condition: Expr,
        then_expr: Expr,
        else_expr: Expr,
        position: Position,
    ) -> Self {
        Expr::Conditional {
            condition: Box::new(condition),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
            position,
        }
    }

    pub fn list(elements: Vec<Expr>, position: Position) -> Self {
        Expr::List { elements, position }
    }

    pub fn tuple(elements: Vec<Expr>, position: Position) -> Self {
        Expr::Tuple { elements, position }
    }

    pub fn range(
        start: Option<Expr>,
        end: Option<Expr>,
        step: Option<Expr>,
        position: Position,
    ) -> Self {
        Expr::Range {
            start: start.map(Box::new),
            end: end.map(Box::new),
            step: step.map(Box::new),
            position,
        }
    }
}

impl Stmt {
    pub fn expression(expr: Expr, position: Position) -> Self {
        Stmt::Expression { expr, position }
    }

    pub fn variable_decl(decl: VariableDecl) -> Self {
        Stmt::VariableDecl { decl }
    }

    pub fn function_decl(decl: FunctionDecl) -> Self {
        Stmt::FunctionDecl { decl }
    }

    pub fn class_decl(decl: ClassDecl) -> Self {
        Stmt::ClassDecl { decl }
    }

    pub fn interface_decl(decl: InterfaceDecl) -> Self {
        Stmt::InterfaceDecl { decl }
    }

    pub fn import_decl(decl: ImportDecl) -> Self {
        Stmt::ImportDecl { decl }
    }

    pub fn export_decl(decl: ExportDecl) -> Self {
        Stmt::ExportDecl { decl }
    }

    pub fn if_stmt(
        condition: Expr,
        then_branch: Stmt,
        else_branch: Option<Stmt>,
        position: Position,
    ) -> Self {
        Stmt::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
            position,
        }
    }

    pub fn while_stmt(condition: Expr, body: Stmt, position: Position) -> Self {
        Stmt::While {
            condition,
            body: Box::new(body),
            position,
        }
    }

    pub fn for_stmt(variable: String, iterable: Expr, body: Stmt, position: Position) -> Self {
        Stmt::For {
            variable,
            iterable,
            body: Box::new(body),
            position,
        }
    }

    pub fn block(statements: Vec<Stmt>, position: Position) -> Self {
        Stmt::Block {
            statements,
            position,
        }
    }

    pub fn return_stmt(value: Option<Expr>, position: Position) -> Self {
        Stmt::Return { value, position }
    }

    pub fn break_stmt(label: Option<String>, position: Position) -> Self {
        Stmt::Break { label, position }
    }

    pub fn continue_stmt(label: Option<String>, position: Position) -> Self {
        Stmt::Continue { label, position }
    }

    pub fn try_stmt(
        try_block: Stmt,
        catch_blocks: Vec<CatchBlock>,
        finally_block: Option<Stmt>,
        position: Position,
    ) -> Self {
        Stmt::Try {
            try_block: Box::new(try_block),
            catch_blocks,
            finally_block: finally_block.map(Box::new),
            position,
        }
    }

    pub fn throw_stmt(expr: Expr, position: Position) -> Self {
        Stmt::Throw { expr, position }
    }

    pub fn panic_stmt(message: Option<Expr>, position: Position) -> Self {
        Stmt::Panic { message, position }
    }

    pub fn defer_stmt(stmt: Stmt, position: Position) -> Self {
        Stmt::Defer {
            stmt: Box::new(stmt),
            position,
        }
    }

    pub fn match_stmt(expr: Expr, arms: Vec<MatchArm>, position: Position) -> Self {
        Stmt::Match {
            expr,
            arms,
            position,
        }
    }

    pub fn switch_stmt(
        expr: Expr,
        cases: Vec<SwitchCase>,
        default_case: Option<Stmt>,
        position: Position,
    ) -> Self {
        Stmt::Switch {
            expr,
            cases,
            default_case: default_case.map(Box::new),
            position,
        }
    }

    pub fn label_stmt(name: String, stmt: Stmt, position: Position) -> Self {
        Stmt::Label {
            name,
            stmt: Box::new(stmt),
            position,
        }
    }

    pub fn goto_stmt(label: String, position: Position) -> Self {
        Stmt::Goto { label, position }
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::Inferred
    }
}

/// @ZNOTE[NovaCore Integration]: This AST structure is designed to be easily serialized
/// for NovaCore bytecode compilation. Each node contains position information for
/// debugging and error reporting in the compiled bytecode.

/// @ZNOTE[NovaStdLib Hook]: Expression evaluation will need to interface with NovaStdLib
/// functions for built-in operations like arithmetic, string manipulation, and I/O.

/// @ZNOTE[Future Extension]: Pattern matching and async/await are included for future
/// language features that will be implemented in later NovaScript versions.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_creation() {
        let pos = Position::new(1, 1, 0);
        // Test literal creation
        let literal = Expr::literal(Literal::Integer(42), pos.clone());
        assert!(matches!(
            literal,
            Expr::Literal {
                value: Literal::Integer(42),
                ..
            }
        ));
        // Test identifier creation
        let identifier = Expr::identifier("x".to_string(), pos.clone());
        assert!(matches!(identifier, Expr::Identifier { name, .. } if name == "x"));
    }

    #[test]
    fn test_binary_expr() {
        let pos = Position::new(1, 1, 0);
        // Test binary expression creation and matching
        let left = Expr::literal(Literal::Integer(5), pos.clone());
        let right = Expr::literal(Literal::Integer(3), pos.clone());
        let binary = Expr::binary(left.clone(), BinaryOp::Add, right.clone(), pos.clone());
        if let Expr::Binary {
            left: l,
            operator,
            right: r,
            ..
        } = binary
        {
            assert!(matches!(
                *l,
                Expr::Literal {
                    value: Literal::Integer(5),
                    ..
                }
            ));
            assert_eq!(operator, BinaryOp::Add);
            assert!(matches!(
                *r,
                Expr::Literal {
                    value: Literal::Integer(3),
                    ..
                }
            ));
        } else {
            panic!("Expected binary expression");
        }
    }

    #[test]
    fn test_function_decl() {
        let pos = Position::new(1, 1, 0);
        // Test function declaration struct
        let param = Parameter {
            name: "x".to_string(),
            param_type: Type::Int,
            default_value: None,
            position: pos.clone(),
        };
        let func_decl = FunctionDecl {
            name: "test_func".to_string(),
            params: vec![param],
            return_type: Type::Int,
            body: vec![],
            is_async: false,
            visibility: Visibility::Private,
            position: pos.clone(),
        };
        assert_eq!(func_decl.name, "test_func");
        assert_eq!(func_decl.params.len(), 1);
        assert_eq!(func_decl.return_type, Type::Int);
        assert!(!func_decl.is_async);
    }

    #[test]
    fn test_variable_decl() {
        let pos = Position::new(1, 1, 0);
        // Test variable declaration struct
        let var_decl = VariableDecl {
            name: "x".to_string(),
            var_type: Type::Int,
            is_mutable: false,
            initializer: Some(Expr::literal(Literal::Integer(42), pos.clone())),
            position: pos.clone(),
        };
        assert_eq!(var_decl.name, "x");
        assert_eq!(var_decl.var_type, Type::Int);
        assert!(!var_decl.is_mutable);
        assert!(var_decl.initializer.is_some());
    }

    #[test]
    fn test_class_decl() {
        let pos = Position::new(1, 1, 0);
        // Test class declaration struct
        let class_decl = ClassDecl {
            name: "TestClass".to_string(),
            superclass: None,
            methods: vec![],
            fields: vec![],
            visibility: Visibility::Public,
            position: pos.clone(),
        };
        assert_eq!(class_decl.name, "TestClass");
        assert!(class_decl.superclass.is_none());
        assert_eq!(class_decl.methods.len(), 0);
        assert_eq!(class_decl.fields.len(), 0);
    }

    #[test]
    fn test_pattern_matching() {
        let pos = Position::new(1, 1, 0);
        // Test match arm struct
        let pattern = Pattern::Literal(Literal::Integer(42));
        let guard = Expr::literal(Literal::Boolean(true), pos.clone());
        let body = Expr::literal(Literal::String("matched".to_string()), pos.clone());
        let match_arm = MatchArm {
            pattern,
            guard: Some(guard),
            body,
        };
        assert!(matches!(
            match_arm.pattern,
            Pattern::Literal(Literal::Integer(42))
        ));
        assert!(match_arm.guard.is_some());
    }

    #[test]
    fn test_type_to_string() {
        // Test type to string conversion
        assert_eq!(Type::Int.to_string(), "int");
        assert_eq!(Type::Float.to_string(), "float");
        assert_eq!(Type::String.to_string(), "string");
        assert_eq!(Type::Bool.to_string(), "bool");
        assert_eq!(Type::Null.to_string(), "null");
        assert_eq!(Type::Inferred.to_string(), "auto");
        let array_type = Type::Array(Box::new(Type::Int));
        assert_eq!(array_type.to_string(), "[int]");
        let func_type = Type::Function {
            params: vec![Type::Int, Type::String],
            return_type: Box::new(Type::Bool),
        };
        assert_eq!(func_type.to_string(), "(int, string) -> bool");
    }

    #[test]
    fn test_expression_position() {
        let pos = Position::new(5, 10, 25);
        // Test position retrieval for expressions
        let expr = Expr::literal(Literal::Integer(42), pos.clone());
        assert_eq!(expr.position(), &pos);
        let binary_expr = Expr::binary(
            Expr::literal(Literal::Integer(1), pos.clone()),
            BinaryOp::Add,
            Expr::literal(Literal::Integer(2), pos.clone()),
            pos.clone(),
        );
        assert_eq!(binary_expr.position(), &pos);
    }

    #[test]
    fn test_statement_position() {
        let pos = Position::new(3, 7, 15);
        // Test position retrieval for statements
        let expr_stmt = Stmt::expression(
            Expr::literal(Literal::Integer(42), pos.clone()),
            pos.clone(),
        );
        assert_eq!(expr_stmt.position(), &pos);
        let block_stmt = Stmt::block(vec![], pos.clone());
        assert_eq!(block_stmt.position(), &pos);
    }

    #[test]
    fn test_program_utilities() {
        let pos = Position::new(1, 1, 0);
        // Test program utility methods
        let func_decl = FunctionDecl {
            name: "test".to_string(),
            params: vec![],
            return_type: Type::Inferred,
            body: vec![],
            is_async: false,
            visibility: Visibility::Private,
            position: pos.clone(),
        };
        let var_decl = VariableDecl {
            name: "x".to_string(),
            var_type: Type::Int,
            is_mutable: false,
            initializer: None,
            position: pos.clone(),
        };
        let program = Program::new(
            vec![
                Stmt::function_decl(func_decl),
                Stmt::variable_decl(var_decl),
            ],
            pos.clone(),
        );
        assert_eq!(program.find_functions().len(), 1);
        assert_eq!(program.find_variables().len(), 1);
        assert_eq!(program.find_classes().len(), 0);
        assert_eq!(program.find_interfaces().len(), 0);
        assert_eq!(program.find_imports().len(), 0);
        assert_eq!(program.find_exports().len(), 0);
    }

    #[test]
    fn test_literal_variants() {
        // Test all literal variants
        let int_lit = Literal::Integer(42);
        let float_lit = Literal::Float(3.14);
        let string_lit = Literal::String("hello".to_string());
        let bool_lit = Literal::Boolean(true);
        let null_lit = Literal::Null;
        let array_lit = Literal::Array(vec![]);
        let object_lit = Literal::Object(Vec::new());

        assert!(matches!(int_lit, Literal::Integer(42)));
        assert!(matches!(float_lit, Literal::Float(f) if (f - 3.14).abs() < f64::EPSILON));
        assert!(matches!(string_lit, Literal::String(ref s) if s == "hello"));
        assert!(matches!(bool_lit, Literal::Boolean(true)));
        assert!(matches!(null_lit, Literal::Null));
        assert!(matches!(array_lit, Literal::Array(_)));
        assert!(matches!(object_lit, Literal::Object(_)));
    }

    #[test]
    fn test_pattern_variants() {
        // Test all pattern variants
        let int_pattern = Pattern::Literal(Literal::Integer(42));
        let id_pattern = Pattern::Identifier("x".to_string());
        let wildcard_pattern = Pattern::Wildcard;
        let list_pattern = Pattern::List(vec![]);
        let object_pattern = Pattern::Object(Vec::new());
        let tuple_pattern = Pattern::Tuple(vec![]);
        assert!(matches!(
            int_pattern,
            Pattern::Literal(Literal::Integer(42))
        ));
        assert!(matches!(id_pattern, Pattern::Identifier(ref s) if s == "x"));
        assert!(matches!(wildcard_pattern, Pattern::Wildcard));
        assert!(matches!(list_pattern, Pattern::List(_)));
        assert!(matches!(object_pattern, Pattern::Object(_)));
        assert!(matches!(tuple_pattern, Pattern::Tuple(_)));
    }

    #[test]
    fn test_default_implementations() {
        // Test default trait implementations
        assert_eq!(Visibility::default(), Visibility::Private);
        assert_eq!(Type::default(), Type::Inferred);
    }

    #[test]
    fn test_binary_operators() {
        // Test all binary operators for Eq/Clone
        let ops = vec![
            BinaryOp::Add,
            BinaryOp::Subtract,
            BinaryOp::Multiply,
            BinaryOp::Divide,
            BinaryOp::Modulo,
            BinaryOp::Power,
            BinaryOp::Equal,
            BinaryOp::NotEqual,
            BinaryOp::Less,
            BinaryOp::Greater,
            BinaryOp::LessEqual,
            BinaryOp::GreaterEqual,
            BinaryOp::And,
            BinaryOp::Or,
            BinaryOp::BitwiseAnd,
            BinaryOp::BitwiseOr,
            BinaryOp::BitwiseXor,
            BinaryOp::LeftShift,
            BinaryOp::RightShift,
            BinaryOp::In,
            BinaryOp::NotIn,
            BinaryOp::Is,
            BinaryOp::IsNot,
        ];
        for op in ops {
            assert_eq!(op, op.clone());
        }
    }

    #[test]
    fn test_unary_operators() {
        // Test all unary operators for Eq/Clone
        let ops = vec![
            UnaryOp::Not,
            UnaryOp::Minus,
            UnaryOp::Plus,
            UnaryOp::BitwiseNot,
        ];
        for op in ops {
            assert_eq!(op, op.clone());
        }
    }

    #[test]
    fn test_complex_expressions() {
        let pos = Position::new(1, 1, 0);
        // Test lambda expression
        let lambda = Expr::lambda(
            vec![],
            Expr::literal(Literal::Integer(42), pos.clone()),
            pos.clone(),
        );
        assert!(matches!(lambda, Expr::Lambda { .. }));
        // Test conditional expression
        let conditional = Expr::conditional(
            Expr::literal(Literal::Boolean(true), pos.clone()),
            Expr::literal(Literal::Integer(1), pos.clone()),
            Expr::literal(Literal::Integer(2), pos.clone()),
            pos.clone(),
        );
        assert!(matches!(conditional, Expr::Conditional { .. }));
        // Test list expression
        let list = Expr::list(
            vec![
                Expr::literal(Literal::Integer(1), pos.clone()),
                Expr::literal(Literal::Integer(2), pos.clone()),
            ],
            pos.clone(),
        );
        assert!(matches!(list, Expr::List { .. }));
        // Test tuple expression
        let tuple = Expr::tuple(
            vec![
                Expr::literal(Literal::String("hello".to_string()), pos.clone()),
                Expr::literal(Literal::Integer(42), pos.clone()),
            ],
            pos.clone(),
        );
        assert!(matches!(tuple, Expr::Tuple { .. }));
        // Test range expression
        let range = Expr::range(
            Some(Expr::literal(Literal::Integer(1), pos.clone())),
            Some(Expr::literal(Literal::Integer(10), pos.clone())),
            None,
            pos.clone(),
        );
        assert!(matches!(range, Expr::Range { .. }));
    }

    #[test]
    fn test_complex_statements() {
        let pos = Position::new(1, 1, 0);
        // Test if statement
        let if_stmt = Stmt::if_stmt(
            Expr::literal(Literal::Boolean(true), pos.clone()),
            Stmt::expression(Expr::literal(Literal::Integer(1), pos.clone()), pos.clone()),
            Some(Stmt::expression(
                Expr::literal(Literal::Integer(2), pos.clone()),
                pos.clone(),
            )),
            pos.clone(),
        );
        assert!(matches!(if_stmt, Stmt::If { .. }));
        // Test while statement
        let while_stmt = Stmt::while_stmt(
            Expr::literal(Literal::Boolean(true), pos.clone()),
            Stmt::expression(
                Expr::literal(Literal::Integer(42), pos.clone()),
                pos.clone(),
            ),
            pos.clone(),
        );
        assert!(matches!(while_stmt, Stmt::While { .. }));
        // Test for statement
        let for_stmt = Stmt::for_stmt(
            "i".to_string(),
            Expr::literal(Literal::Array(vec![]), pos.clone()),
            Stmt::expression(
                Expr::literal(Literal::Integer(42), pos.clone()),
                pos.clone(),
            ),
            pos.clone(),
        );
        assert!(matches!(for_stmt, Stmt::For { .. }));
    }
}

/// Additional utility implementations
impl Parameter {
    pub fn new(name: String, param_type: Type, position: Position) -> Self {
        Self {
            name,
            param_type,
            default_value: None,
            position,
        }
    }

    pub fn with_default(mut self, default_value: Expr) -> Self {
        self.default_value = Some(default_value);
        self
    }
}

impl VariableDecl {
    pub fn new(name: String, var_type: Type, position: Position) -> Self {
        Self {
            name,
            var_type,
            is_mutable: false,
            initializer: None,
            position,
        }
    }

    pub fn mutable(mut self) -> Self {
        self.is_mutable = true;
        self
    }

    pub fn with_initializer(mut self, initializer: Expr) -> Self {
        self.initializer = Some(initializer);
        self
    }
}

impl FunctionDecl {
    pub fn new(name: String, return_type: Type, position: Position) -> Self {
        Self {
            name,
            params: Vec::new(),
            return_type,
            body: Vec::new(),
            is_async: false,
            visibility: Visibility::Private,
            position,
        }
    }

    pub fn with_params(mut self, params: Vec<Parameter>) -> Self {
        self.params = params;
        self
    }

    pub fn with_body(mut self, body: Vec<Stmt>) -> Self {
        self.body = body;
        self
    }

    pub fn async_fn(mut self) -> Self {
        self.is_async = true;
        self
    }
}

impl ClassDecl {
    pub fn new(name: String, position: Position) -> Self {
        Self {
            name,
            superclass: None,
            methods: vec![],
            fields: vec![],
            visibility: Visibility::Private,
            position,
        }
    }

    pub fn with_superclass(mut self, superclass: String) -> Self {
        self.superclass = Some(superclass);
        self
    }

    pub fn with_methods(mut self, methods: Vec<FunctionDecl>) -> Self {
        self.methods = methods;
        self
    }

    pub fn with_fields(mut self, fields: Vec<VariableDecl>) -> Self {
        self.fields = fields;
        self
    }

    pub fn public(mut self) -> Self {
        self.visibility = Visibility::Public;
        self
    }
}

impl InterfaceDecl {
    pub fn new(name: String, position: Position) -> Self {
        Self {
            name,
            methods: vec![],
            superinterfaces: vec![],
            position,
        }
    }

    pub fn with_methods(mut self, methods: Vec<FunctionSignature>) -> Self {
        self.methods = methods;
        self
    }

    pub fn with_superinterfaces(mut self, superinterfaces: Vec<String>) -> Self {
        self.superinterfaces = superinterfaces;
        self
    }
}

impl ImportDecl {
    pub fn new(module: String, position: Position) -> Self {
        Self {
            module,
            items: vec![],
            alias: None,
            position,
        }
    }

    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }

    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }
}

impl ExportDecl {
    pub fn new(item: ExportItem, position: Position) -> Self {
        Self { item, position }
    }
}

/// Error types for AST operations
#[derive(Debug, Clone, PartialEq)]
pub enum AstError {
    InvalidExpression(String),
    InvalidStatement(String),
    TypeMismatch {
        expected: Type,
        found: Type,
        position: Position,
    },
    UndefinedVariable {
        name: String,
        position: Position,
    },
    UndefinedFunction {
        name: String,
        position: Position,
    },
    InvalidArguments {
        expected: usize,
        found: usize,
        position: Position,
    },
    DuplicateDeclaration {
        name: String,
        position: Position,
    },
}

impl std::fmt::Display for AstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AstError::InvalidExpression(msg) => write!(f, "Invalid expression: {}", msg),
            AstError::InvalidStatement(msg) => write!(f, "Invalid statement: {}", msg),
            AstError::TypeMismatch {
                expected,
                found,
                position,
            } => {
                write!(
                    f,
                    "Type mismatch at {}:{}: expected {}, found {}",
                    position.line,
                    position.column,
                    expected.to_string(),
                    found.to_string()
                )
            }
            AstError::UndefinedVariable { name, position } => {
                write!(
                    f,
                    "Undefined variable '{}' at {}:{}",
                    name, position.line, position.column
                )
            }
            AstError::UndefinedFunction { name, position } => {
                write!(
                    f,
                    "Undefined function '{}' at {}:{}",
                    name, position.line, position.column
                )
            }
            AstError::InvalidArguments {
                expected,
                found,
                position,
            } => {
                write!(
                    f,
                    "Invalid number of arguments at {}:{}: expected {}, found {}",
                    position.line, position.column, expected, found
                )
            }
            AstError::DuplicateDeclaration { name, position } => {
                write!(
                    f,
                    "Duplicate declaration of '{}' at {}:{}",
                    name, position.line, position.column
                )
            }
        }
    }
}

impl std::error::Error for AstError {}

/// Result type for AST operations
pub type AstResult<T> = Result<T, AstError>;
