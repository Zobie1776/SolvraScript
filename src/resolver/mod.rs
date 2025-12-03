//=====================================================
// File: resolver.rs
//=====================================================
// Author: ZobieLabs
// License: Duality Public License (DPL v1.0)
// Goal: Symbol resolution for SolvraScript AST
// Objective: Map identifier usages to their declarations with a simple scope stack
//=====================================================

use crate::ast::{
    AssignTarget, Expr, FunctionDecl, NodeId, Program, Stmt, StringPart, VariableDecl, next_node_id,
};
use crate::tokenizer::Position;
use std::collections::HashMap;

pub type Module = Program;

#[derive(Default)]
pub struct Diagnostics {
    unresolved: Vec<(String, Position)>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            unresolved: Vec::new(),
        }
    }

    pub fn record_unresolved(&mut self, name: &str, position: Position) {
        self.unresolved.push((name.to_string(), position));
    }

    pub fn has_errors(&self) -> bool {
        !self.unresolved.is_empty()
    }

    pub fn unresolved(&self) -> &[(String, Position)] {
        &self.unresolved
    }
}

pub struct SymbolResolution {
    #[allow(dead_code)]
    pub map: HashMap<NodeId, NodeId>,
}

/// Run name resolution over a module and capture identifier bindings.
pub fn resolve_module(ast: &Module, diagnostics: &mut Diagnostics) -> SymbolResolution {
    let mut resolver = Resolver::new(diagnostics);
    resolver.collect_function_decls(ast);
    resolver.resolve_statements(&ast.statements);
    SymbolResolution {
        map: resolver.resolutions,
    }
}

#[derive(Clone)]
struct Scope {
    parent: Option<usize>,
    bindings: HashMap<String, NodeId>,
}

struct Resolver<'a> {
    scopes: Vec<Scope>,
    current_scope: usize,
    resolutions: HashMap<NodeId, NodeId>,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> Resolver<'a> {
    fn new(diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            scopes: vec![Scope {
                parent: None,
                bindings: HashMap::new(),
            }],
            current_scope: 0,
            resolutions: HashMap::new(),
            diagnostics,
        }
    }

    fn collect_function_decls(&mut self, module: &Module) {
        for stmt in &module.statements {
            if let Stmt::FunctionDecl { decl } = stmt {
                self.define(&decl.name, decl.node_id);
            }
        }
    }

    fn resolve_statements(&mut self, statements: &[Stmt]) {
        for stmt in statements {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VariableDecl { decl } => {
                self.define_variable(decl);
            }
            Stmt::FunctionDecl { decl } => {
                self.define(&decl.name, decl.node_id);
                self.resolve_function(decl);
            }
            Stmt::Expression { expr, .. } => self.resolve_expr(expr),
            Stmt::Block { statements, .. } => {
                self.push_scope();
                self.resolve_statements(statements);
                self.pop_scope();
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.resolve_expr(condition);
                self.push_scope();
                self.resolve_stmt(then_branch);
                self.pop_scope();
                if let Some(else_branch) = else_branch {
                    self.push_scope();
                    self.resolve_stmt(else_branch);
                    self.pop_scope();
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.resolve_expr(condition);
                self.push_scope();
                self.resolve_stmt(body);
                self.pop_scope();
            }
            Stmt::For {
                variable,
                iterable,
                body,
                node_id,
                ..
            } => {
                self.resolve_expr(iterable);
                self.push_scope();
                self.define(variable, *node_id);
                self.resolve_stmt(body);
                self.pop_scope();
            }
            Stmt::Loop { body, .. } => {
                self.push_scope();
                self.resolve_stmt(body);
                self.pop_scope();
            }
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.resolve_expr(expr);
                }
            }
            Stmt::Try {
                try_block,
                catch_blocks,
                finally_block,
                ..
            } => {
                self.resolve_stmt(try_block);
                for block in catch_blocks {
                    self.resolve_stmt(&block.body);
                }
                if let Some(finally) = finally_block {
                    self.resolve_stmt(finally);
                }
            }
            Stmt::Match { expr, arms, .. } => {
                self.resolve_expr(expr);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.resolve_expr(guard);
                    }
                    self.resolve_expr(&arm.body);
                }
            }
            Stmt::With { expr, body, .. } => {
                self.resolve_expr(expr);
                self.resolve_stmt(body);
            }
            Stmt::Switch {
                expr,
                cases,
                default_case,
                ..
            } => {
                self.resolve_expr(expr);
                for case in cases {
                    for value in &case.values {
                        self.resolve_expr(value);
                    }
                    self.resolve_statements(&case.body);
                }
                if let Some(default) = default_case {
                    self.resolve_stmt(default);
                }
            }
            Stmt::Defer { stmt, .. } => self.resolve_stmt(stmt),
            Stmt::Throw { expr, .. }
            | Stmt::Panic {
                message: Some(expr),
                ..
            } => self.resolve_expr(expr),
            Stmt::ImportDecl { .. }
            | Stmt::ExportDecl { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Panic { .. } => {}
            _ => {}
        }
    }

    fn resolve_function(&mut self, decl: &FunctionDecl) {
        self.push_scope();
        for param in &decl.params {
            let param_id = next_node_id();
            self.define(&param.name, param_id);
        }
        self.resolve_statements(&decl.body);
        self.pop_scope();
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Identifier {
                name,
                position,
                node_id,
                ..
            } => {
                self.resolve_identifier(name.as_str(), *node_id, position.clone());
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Unary { operand, .. } => self.resolve_expr(operand),
            Expr::Call { callee, args, .. } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                self.resolve_expr(receiver);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::Member { object, .. } => self.resolve_expr(object),
            Expr::Index { object, index, .. } => {
                self.resolve_expr(object);
                self.resolve_expr(index);
            }
            Expr::Slice {
                object,
                start,
                end,
                step,
                ..
            } => {
                self.resolve_expr(object);
                if let Some(expr) = start {
                    self.resolve_expr(expr);
                }
                if let Some(expr) = end {
                    self.resolve_expr(expr);
                }
                if let Some(expr) = step {
                    self.resolve_expr(expr);
                }
            }
            Expr::Assign {
                target,
                value,
                position,
                ..
            } => {
                self.resolve_assign_target(target, position);
                self.resolve_expr(value);
            }
            Expr::If {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.resolve_expr(condition);
                self.resolve_expr(then_expr);
                self.resolve_expr(else_expr);
            }
            Expr::Conditional {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.resolve_expr(condition);
                self.resolve_expr(then_expr);
                self.resolve_expr(else_expr);
            }
            Expr::List { elements, .. } | Expr::Tuple { elements, .. } => {
                for element in elements {
                    self.resolve_expr(element);
                }
            }
            Expr::Range {
                start, end, step, ..
            } => {
                if let Some(expr) = start {
                    self.resolve_expr(expr);
                }
                if let Some(expr) = end {
                    self.resolve_expr(expr);
                }
                if let Some(expr) = step {
                    self.resolve_expr(expr);
                }
            }
            Expr::Comprehension {
                element,
                iterable,
                condition,
                variable,
                ..
            } => {
                self.resolve_expr(iterable);
                self.push_scope();
                self.define(variable, next_node_id());
                self.resolve_expr(element);
                if let Some(cond) = condition {
                    self.resolve_expr(cond);
                }
                self.pop_scope();
            }
            Expr::Lambda { params, body, .. } => {
                self.push_scope();
                for param in params {
                    self.define(param, next_node_id());
                }
                self.resolve_expr(body);
                self.pop_scope();
            }
            Expr::Match { expr, arms, .. } => {
                self.resolve_expr(expr);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.resolve_expr(guard);
                    }
                    self.resolve_expr(&arm.body);
                }
            }
            Expr::StringTemplate { parts, .. } | Expr::StringInterpolation { parts, .. } => {
                self.resolve_string_parts(parts);
            }
            Expr::Literal { .. } | Expr::Await { .. } | Expr::Async { .. } => {}
        }
    }

    fn resolve_string_parts(&mut self, parts: &[StringPart]) {
        for part in parts {
            if let StringPart::Expression(expr) = part {
                self.resolve_expr(expr);
            }
        }
    }

    fn resolve_assign_target(&mut self, target: &AssignTarget, position: &Position) {
        match target {
            AssignTarget::Variable(symbol) => {
                if self.lookup(symbol.as_str()).is_none() {
                    self.diagnostics
                        .record_unresolved(symbol.as_str(), position.clone());
                }
            }
            AssignTarget::Index { array, index } => {
                self.resolve_expr(array);
                self.resolve_expr(index);
            }
            AssignTarget::Member { object, .. } => {
                self.resolve_expr(object);
            }
        }
    }

    fn define_variable(&mut self, decl: &VariableDecl) {
        self.define(&decl.name, decl.node_id);
        if let Some(init) = &decl.initializer {
            self.resolve_expr(init);
        }
    }

    fn resolve_identifier(&mut self, name: &str, use_id: NodeId, position: Position) {
        if let Some(def_id) = self.lookup(name) {
            self.resolutions.insert(use_id, def_id);
        } else {
            self.diagnostics.record_unresolved(name, position);
        }
    }

    fn define(&mut self, name: &str, node_id: NodeId) {
        if let Some(scope) = self.scopes.get_mut(self.current_scope) {
            scope.bindings.insert(name.to_string(), node_id);
        }
    }

    fn lookup(&self, name: &str) -> Option<NodeId> {
        let mut scope_index = Some(self.current_scope);
        while let Some(index) = scope_index {
            if let Some(id) = self.scopes[index].bindings.get(name) {
                return Some(*id);
            }
            scope_index = self.scopes[index].parent;
        }
        None
    }

    fn push_scope(&mut self) {
        let parent = Some(self.current_scope);
        self.scopes.push(Scope {
            parent,
            bindings: HashMap::new(),
        });
        self.current_scope = self.scopes.len() - 1;
    }

    fn pop_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }
}

//=====================================================
// End of file
//=====================================================
