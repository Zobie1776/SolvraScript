use crate::error::SolvraIdeError;
use serde::{Deserialize, Serialize};
use solvrascript::ast::{FunctionDecl, Program, Stmt, Type, VariableDecl};
use solvrascript::parser::{ParseError, Parser};
use solvrascript::tokenizer::{Position as SolvraPosition, Tokenizer};

const KEYWORDS: &[&str] = &[
    "let", "mut", "fn", "if", "else", "while", "for", "return", "match", "try", "catch",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TextPosition {
    pub line: usize,
    pub character: usize,
}

impl From<(usize, usize)> for TextPosition {
    fn from(value: (usize, usize)) -> Self {
        Self {
            line: value.0,
            character: value.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompletionKind {
    Variable,
    Function,
    Keyword,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HoverResult {
    pub contents: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

pub struct SolvraLanguageServer;

impl SolvraLanguageServer {
    pub const fn new() -> Self {
        Self
    }

    pub fn complete(
        &self,
        source: &str,
        position: TextPosition,
    ) -> Result<Vec<CompletionItem>, SolvraIdeError> {
        let program = self.parse_program(source)?;
        let mut symbols = collect_symbols(&program);
        symbols.retain(|symbol| symbol.position.line <= position.line);
        let mut items: Vec<CompletionItem> = symbols
            .into_iter()
            .map(|symbol| CompletionItem {
                label: symbol.name,
                detail: symbol.detail,
                kind: symbol.kind,
            })
            .collect();
        items.extend(KEYWORDS.iter().map(|keyword| CompletionItem {
            label: (*keyword).to_string(),
            detail: Some("keyword".into()),
            kind: CompletionKind::Keyword,
        }));
        items.sort_by(|a, b| a.label.cmp(&b.label));
        items.dedup_by(|a, b| a.label == b.label);
        Ok(items)
    }

    pub fn hover(
        &self,
        source: &str,
        position: TextPosition,
    ) -> Result<Option<HoverResult>, SolvraIdeError> {
        let program = self.parse_program(source)?;
        for symbol in collect_symbols(&program) {
            if same_line(&symbol.position, position) {
                let detail = symbol
                    .detail
                    .unwrap_or_else(|| symbol.kind.as_str().to_string());
                return Ok(Some(HoverResult { contents: detail }));
            }
        }
        Ok(None)
    }

    pub fn goto_definition(
        &self,
        source: &str,
        symbol_name: &str,
    ) -> Result<Option<TextPosition>, SolvraIdeError> {
        let program = self.parse_program(source)?;
        for symbol in collect_symbols(&program) {
            if symbol.name == symbol_name {
                return Ok(Some(to_text_position(&symbol.position)));
            }
        }
        Ok(None)
    }

    pub fn diagnostics(&self, source: &str) -> Result<Vec<Diagnostic>, SolvraIdeError> {
        let mut tokenizer = Tokenizer::new(source);
        let tokens = match tokenizer.tokenize() {
            Ok(tokens) => tokens,
            Err(message) => {
                return Ok(vec![Diagnostic {
                    message,
                    line: 1,
                    column: 1,
                }]);
            }
        };

        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(_) => Ok(Vec::new()),
            Err(err) => Ok(vec![diagnostic_from_parse_error(err)]),
        }
    }

    fn parse_program(&self, source: &str) -> Result<Program, SolvraIdeError> {
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer
            .tokenize()
            .map_err(|err| SolvraIdeError::language(format!("tokenization failed: {err}")))?;
        let mut parser = Parser::new(tokens);
        parser
            .parse()
            .map_err(|err| SolvraIdeError::language(err.to_string()))
    }
}

impl Default for SolvraLanguageServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct SymbolRecord {
    name: String,
    kind: CompletionKind,
    detail: Option<String>,
    position: SolvraPosition,
}

fn collect_symbols(program: &Program) -> Vec<SymbolRecord> {
    let mut symbols = Vec::new();
    for stmt in &program.statements {
        collect_from_stmt(stmt, &mut symbols);
    }
    symbols
}

fn collect_from_stmt(stmt: &Stmt, symbols: &mut Vec<SymbolRecord>) {
    match stmt {
        Stmt::VariableDecl { decl } => symbols.push(variable_symbol(decl)),
        Stmt::FunctionDecl { decl } => {
            symbols.push(function_symbol(decl));
            for inner in &decl.body {
                collect_from_stmt(inner, symbols);
            }
        }
        Stmt::Block { statements, .. } => {
            for inner in statements {
                collect_from_stmt(inner, symbols);
            }
        }
        Stmt::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_from_stmt(then_branch, symbols);
            if let Some(else_branch) = else_branch {
                collect_from_stmt(else_branch, symbols);
            }
        }
        Stmt::While { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForIn { body, .. }
        | Stmt::ForOf { body, .. }
        | Stmt::Loop { body, .. } => collect_from_stmt(body, symbols),
        _ => {}
    }
}

fn variable_symbol(decl: &VariableDecl) -> SymbolRecord {
    SymbolRecord {
        name: decl.name.clone(),
        kind: CompletionKind::Variable,
        detail: Some(format!("variable: {}", display_type(&decl.var_type))),
        position: decl.position.clone(),
    }
}

fn function_symbol(decl: &FunctionDecl) -> SymbolRecord {
    let params: Vec<String> = decl
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, display_type(&param.param_type)))
        .collect();
    let signature = format!(
        "fn {}({}) -> {}",
        decl.name,
        params.join(", "),
        display_type(&decl.return_type)
    );
    SymbolRecord {
        name: decl.name.clone(),
        kind: CompletionKind::Function,
        detail: Some(signature),
        position: decl.position.clone(),
    }
}

fn display_type(ty: &Type) -> String {
    use std::fmt::Write;
    let mut buf = String::new();
    let _ = write!(&mut buf, "{ty}");
    buf
}

fn to_text_position(position: &SolvraPosition) -> TextPosition {
    TextPosition {
        line: position.line,
        character: position.column,
    }
}

fn same_line(position: &SolvraPosition, query: TextPosition) -> bool {
    position.line == query.line
}

fn diagnostic_from_parse_error(err: ParseError) -> Diagnostic {
    let position = match &err {
        ParseError::UnexpectedToken { position, .. }
        | ParseError::UnexpectedEndOfInput { position, .. }
        | ParseError::InvalidSyntax { position, .. } => position.clone(),
    };
    Diagnostic {
        message: err.to_string(),
        line: position.line,
        column: position.column,
    }
}

impl CompletionKind {
    fn as_str(&self) -> &'static str {
        match self {
            CompletionKind::Variable => "variable",
            CompletionKind::Function => "function",
            CompletionKind::Keyword => "keyword",
        }
    }
}
