use solvrascript::{
    ast::{self, BindingKind, Expr, Literal, MatchArm, Pattern, Stmt, StringPart, VariableDecl},
    parser, tokenizer,
};

#[derive(Default)]
struct Coverage {
    import_decl: bool,
    export_decl: bool,
    function_decl: bool,
    async_function: bool,
    mutable_binding: bool,
    const_binding: bool,
    if_stmt: bool,
    else_branch: bool,
    while_stmt: bool,
    for_stmt: bool,
    try_stmt: bool,
    panic_stmt: bool,
    return_stmt: bool,
    break_stmt: bool,
    continue_stmt: bool,
    lambda_expr: bool,
    match_expr: bool,
    match_guard: bool,
    if_expr: bool,
    await_expr: bool,
    member_access: bool,
    index_expr: bool,
    assignment_expr: bool,
    binary_expr: bool,
    unary_expr: bool,
    call_expr: bool,
    array_literal: bool,
    object_literal: bool,
    string_literal: bool,
    boolean_literal: bool,
    null_literal: bool,
    pattern_object: bool,
    pattern_list: bool,
    pattern_wildcard: bool,
    pattern_identifier: bool,
}

#[test]
fn grammar_suite_covers_language_surface() {
    let program = parse_fixture();
    let mut coverage = Coverage::default();

    for stmt in &program.statements {
        visit_stmt(stmt, &mut coverage);
    }

    assert!(
        coverage.import_decl,
        "expected at least one import declaration"
    );
    assert!(coverage.export_decl, "expected export declarations");
    assert!(coverage.function_decl, "expected function declarations");
    assert!(coverage.async_function, "expected async function coverage");
    assert!(coverage.mutable_binding, "expected mutable let binding");
    assert!(coverage.const_binding, "expected const binding");
    assert!(
        coverage.if_stmt && coverage.else_branch,
        "expected if/else statement"
    );
    assert!(coverage.while_stmt, "expected while loop");
    assert!(coverage.for_stmt, "expected for loop");
    assert!(coverage.try_stmt, "expected try/catch/finally coverage");
    assert!(coverage.panic_stmt, "expected panic statement");
    assert!(coverage.return_stmt, "expected return statement");
    assert!(coverage.break_stmt, "expected break statement");
    assert!(coverage.continue_stmt, "expected continue statement");
    assert!(coverage.lambda_expr, "expected lambda expression");
    assert!(coverage.match_expr, "expected match expression");
    assert!(coverage.match_guard, "expected match guard expression");
    assert!(coverage.if_expr, "expected if-then-else expression");
    assert!(coverage.await_expr, "expected await expression");
    assert!(coverage.member_access, "expected member access expression");
    assert!(coverage.index_expr, "expected indexing expression");
    assert!(coverage.assignment_expr, "expected assignment expression");
    assert!(coverage.binary_expr, "expected binary expression");
    assert!(coverage.unary_expr, "expected unary expression");
    assert!(coverage.call_expr, "expected function call expression");
    assert!(coverage.array_literal, "expected array literal");
    assert!(coverage.object_literal, "expected object literal");
    assert!(coverage.string_literal, "expected string literal");
    assert!(coverage.boolean_literal, "expected boolean literal");
    assert!(coverage.null_literal, "expected null literal");
    assert!(coverage.pattern_object, "expected object pattern");
    assert!(coverage.pattern_list, "expected list pattern");
    assert!(coverage.pattern_wildcard, "expected wildcard pattern");
    assert!(coverage.pattern_identifier, "expected identifier pattern");
}

fn parse_fixture() -> ast::Program {
    let source = include_str!("grammar_validation.svs");
    let mut tokenizer = tokenizer::Tokenizer::new(source);
    let tokens = tokenizer
        .tokenize()
        .expect("tokenize grammar_validation.svs");
    let mut parser = parser::Parser::new(tokens);
    parser.parse().expect("parse grammar_validation.svs")
}

fn visit_stmt(stmt: &Stmt, coverage: &mut Coverage) {
    match stmt {
        Stmt::ImportDecl { decl } => {
            coverage.import_decl = true;
            if let Some(alias) = &decl.alias {
                assert!(
                    !alias.is_empty(),
                    "import aliases should not be empty in fixtures"
                );
            }
        }
        Stmt::ExportDecl { decl } => {
            coverage.export_decl = true;
            visit_export_item(&decl.item, coverage);
        }
        Stmt::VariableDecl { decl } => {
            record_binding(decl, coverage);
            if let Some(init) = &decl.initializer {
                visit_expr(init, coverage);
            }
        }
        Stmt::FunctionDecl { decl } => {
            coverage.function_decl = true;
            if decl.is_async {
                coverage.async_function = true;
            }
            for param in &decl.params {
                if let Some(default) = &param.default_value {
                    visit_expr(default, coverage);
                }
            }
            for inner in &decl.body {
                visit_stmt(inner, coverage);
            }
        }
        Stmt::Block { statements, .. } => {
            for inner in statements {
                visit_stmt(inner, coverage);
            }
        }
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            coverage.if_stmt = true;
            visit_expr(condition, coverage);
            visit_stmt(then_branch, coverage);
            if let Some(branch) = else_branch {
                coverage.else_branch = true;
                visit_stmt(branch, coverage);
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            coverage.while_stmt = true;
            visit_expr(condition, coverage);
            visit_stmt(body, coverage);
        }
        Stmt::For { iterable, body, .. }
        | Stmt::ForIn { iterable, body, .. }
        | Stmt::ForOf { iterable, body, .. } => {
            coverage.for_stmt = true;
            visit_expr(iterable, coverage);
            visit_stmt(body, coverage);
        }
        Stmt::Try {
            try_block,
            catch_blocks,
            finally_block,
            ..
        } => {
            coverage.try_stmt = true;
            visit_stmt(try_block, coverage);
            for catch in catch_blocks {
                if let Some(exc_type) = &catch.exception_type {
                    // Ensure type annotations parse.
                    match exc_type {
                        ast::Type::String | ast::Type::Inferred => {}
                        other => panic!("unexpected type in catch: {:?}", other),
                    }
                }
                if let Some(var) = &catch.variable {
                    assert!(
                        !var.is_empty(),
                        "catch variable must not be empty in fixture"
                    );
                }
                visit_stmt(&catch.body, coverage);
            }
            if let Some(finally) = finally_block {
                visit_stmt(finally, coverage);
            }
        }
        Stmt::Return { value, .. } => {
            coverage.return_stmt = true;
            if let Some(expr) = value {
                visit_expr(expr, coverage);
            }
        }
        Stmt::Break { .. } => {
            coverage.break_stmt = true;
        }
        Stmt::Continue { .. } => {
            coverage.continue_stmt = true;
        }
        Stmt::Panic { message, .. } => {
            coverage.panic_stmt = true;
            if let Some(expr) = message {
                visit_expr(expr, coverage);
            }
        }
        Stmt::Expression { expr, .. } => {
            visit_expr(expr, coverage);
        }
        other => {
            panic!("grammar fixture produced unsupported statement: {other:?}");
        }
    }
}

fn visit_export_item(item: &ast::ExportItem, coverage: &mut Coverage) {
    match item {
        ast::ExportItem::Function(func) => {
            coverage.function_decl = true;
            if func.is_async {
                coverage.async_function = true;
            }
            for stmt in &func.body {
                visit_stmt(stmt, coverage);
            }
        }
        ast::ExportItem::Variable(decl) => {
            record_binding(decl, coverage);
            if let Some(init) = &decl.initializer {
                visit_expr(init, coverage);
            }
        }
        ast::ExportItem::Module(_) => {}
        other => panic!("unexpected export item derived from grammar fixture: {other:?}"),
    }
}

fn record_binding(decl: &VariableDecl, coverage: &mut Coverage) {
    match decl.binding {
        BindingKind::Let => {
            if decl.is_mutable {
                coverage.mutable_binding = true;
            }
        }
        BindingKind::Const => {
            coverage.const_binding = true;
        }
    }
}

fn visit_expr(expr: &Expr, coverage: &mut Coverage) {
    match expr {
        Expr::Literal { value, .. } => visit_literal(value, coverage),
        Expr::StringTemplate { parts, .. } => {
            coverage.string_literal = true;
            for part in parts {
                match part {
                    StringPart::Literal(_) => {}
                    StringPart::Expression(inner) => visit_expr(inner, coverage),
                }
            }
        }
        Expr::StringInterpolation { parts, .. } => {
            for part in parts {
                match part {
                    StringPart::Literal(_) => {}
                    StringPart::Expression(inner) => visit_expr(inner, coverage),
                }
            }
        }
        Expr::Identifier { .. } => {}
        Expr::Assignment { target, value, .. } => {
            coverage.assignment_expr = true;
            visit_expr(target, coverage);
            visit_expr(value, coverage);
        }
        Expr::Binary { left, right, .. } => {
            coverage.binary_expr = true;
            visit_expr(left, coverage);
            visit_expr(right, coverage);
        }
        Expr::Unary { operand, .. } => {
            coverage.unary_expr = true;
            visit_expr(operand, coverage);
        }
        Expr::Call { callee, args, .. } => {
            coverage.call_expr = true;
            visit_expr(callee, coverage);
            for arg in args {
                visit_expr(arg, coverage);
            }
        }
        Expr::Index { object, index, .. } => {
            coverage.index_expr = true;
            visit_expr(object, coverage);
            visit_expr(index, coverage);
        }
        Expr::Member { object, .. } => {
            coverage.member_access = true;
            visit_expr(object, coverage);
        }
        Expr::Lambda { body, .. } => {
            coverage.lambda_expr = true;
            visit_expr(body, coverage);
        }
        Expr::Match {
            expr: subject,
            arms,
            ..
        } => {
            coverage.match_expr = true;
            visit_expr(subject, coverage);
            for MatchArm {
                pattern,
                guard,
                body,
            } in arms
            {
                visit_pattern(pattern, coverage);
                if let Some(guard_expr) = guard {
                    coverage.match_guard = true;
                    visit_expr(guard_expr, coverage);
                }
                visit_expr(body, coverage);
            }
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            coverage.if_expr = true;
            visit_expr(condition, coverage);
            visit_expr(then_expr, coverage);
            visit_expr(else_expr, coverage);
        }
        Expr::Await { expr: awaited, .. } => {
            coverage.await_expr = true;
            visit_expr(awaited, coverage);
        }
        Expr::Async { expr: inner, .. } => {
            visit_expr(inner, coverage);
        }
        Expr::Conditional {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            visit_expr(condition, coverage);
            visit_expr(then_expr, coverage);
            visit_expr(else_expr, coverage);
        }
        Expr::List { elements, .. } => {
            for element in elements {
                visit_expr(element, coverage);
            }
        }
        Expr::Tuple { elements, .. } => {
            for element in elements {
                visit_expr(element, coverage);
            }
        }
        Expr::Range {
            start, end, step, ..
        } => {
            if let Some(s) = start {
                visit_expr(s, coverage);
            }
            if let Some(e) = end {
                visit_expr(e, coverage);
            }
            if let Some(step) = step {
                visit_expr(step, coverage);
            }
        }
        Expr::Comprehension {
            element,
            iterable,
            condition,
            ..
        } => {
            visit_expr(element, coverage);
            visit_expr(iterable, coverage);
            if let Some(cond) = condition {
                visit_expr(cond, coverage);
            }
        }
    }
}

fn visit_literal(literal: &Literal, coverage: &mut Coverage) {
    match literal {
        Literal::Integer(_) | Literal::Float(_) => {}
        Literal::String(_) => {
            coverage.string_literal = true;
        }
        Literal::Boolean(val) => {
            coverage.boolean_literal = true;
            if *val {
                // nothing extra
            }
        }
        Literal::Null => {
            coverage.null_literal = true;
        }
        Literal::Array(elements) => {
            coverage.array_literal = true;
            for element in elements {
                visit_expr(element, coverage);
            }
        }
        Literal::Object(fields) => {
            coverage.object_literal = true;
            for (_, value) in fields {
                visit_expr(value, coverage);
            }
        }
    }
}

fn visit_pattern(pattern: &Pattern, coverage: &mut Coverage) {
    match pattern {
        Pattern::Literal(literal) => visit_literal(literal, coverage),
        Pattern::Identifier(_) => {
            coverage.pattern_identifier = true;
        }
        Pattern::Wildcard => {
            coverage.pattern_wildcard = true;
        }
        Pattern::List(elements) => {
            coverage.pattern_list = true;
            for element in elements {
                visit_pattern(element, coverage);
            }
        }
        Pattern::Object(fields) => {
            coverage.pattern_object = true;
            for (_, value) in fields {
                visit_pattern(value, coverage);
            }
        }
        Pattern::Tuple(elements) => {
            for element in elements {
                visit_pattern(element, coverage);
            }
        }
        Pattern::Constructor { fields, .. } => {
            for field in fields {
                visit_pattern(field, coverage);
            }
        }
        Pattern::Range { start, end } => {
            visit_pattern(start, coverage);
            visit_pattern(end, coverage);
        }
        Pattern::Guard { pattern, condition } => {
            visit_pattern(pattern, coverage);
            visit_expr(condition, coverage);
        }
    }
}
