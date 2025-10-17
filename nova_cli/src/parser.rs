//! Parser that understands NovaCLI command syntax including pipes and expansions.

use crate::command::{Argument, Command, Pipeline, Redirection, RedirectionKind, Statement, Word};
use anyhow::{bail, Context, Result};
use std::iter::Peekable;
use std::str::Chars;

/// Parser for shell-like command lines.
#[derive(Default)]
pub struct Parser;

impl Parser {
    /// Construct a new parser.
    pub fn new() -> Self {
        Self
    }

    /// Parse the provided input into a [`Statement`].
    pub fn parse(&mut self, input: &str) -> Result<Statement> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(Statement::Empty);
        }
        if let Some(expr) = trimmed.strip_prefix("=>") {
            return Ok(Statement::NovaExpression(expr.trim().to_string()));
        }
        if trimmed.starts_with("ns") {
            if let Some(block) = Self::extract_nova_block(input)? {
                return Ok(Statement::NovaBlock(block));
            }
        }
        self.parse_pipeline(input)
    }

    fn extract_nova_block(input: &str) -> Result<Option<String>> {
        let mut chars = input.chars().peekable();
        skip_whitespace(&mut chars);
        if !consume_literal(&mut chars, "ns") {
            return Ok(None);
        }
        skip_whitespace(&mut chars);
        if chars.next() != Some('{') {
            return Ok(None);
        }
        let mut depth = 1i32;
        let mut block = String::new();
        for ch in chars {
            match ch {
                '{' => {
                    depth += 1;
                    block.push(ch);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(Some(block.trim().to_string()));
                    }
                    block.push(ch);
                }
                _ => block.push(ch),
            }
        }
        bail!("unterminated NovaScript block")
    }

    fn parse_pipeline(&mut self, input: &str) -> Result<Statement> {
        let mut chars = input.chars().peekable();
        let mut commands = Vec::new();
        loop {
            skip_whitespace(&mut chars);
            if chars.peek().is_none() {
                break;
            }
            let command = self.parse_command(&mut chars)?;
            commands.push(command);
            skip_whitespace(&mut chars);
            if let Some('|') = chars.peek().copied() {
                chars.next();
                continue;
            } else if chars.peek().is_some() {
                bail!("unexpected characters after command");
            } else {
                break;
            }
        }
        Ok(Statement::Pipeline(Pipeline::new(commands)))
    }

    fn parse_command(&mut self, chars: &mut Peekable<Chars<'_>>) -> Result<Command> {
        let mut program: Option<Argument> = None;
        let mut args = Vec::new();
        let mut redirects = Vec::new();
        loop {
            skip_whitespace(chars);
            match chars.peek().copied() {
                None | Some('|') => break,
                Some('>') => {
                    chars.next();
                    let append = matches!(chars.peek(), Some('>'));
                    if append {
                        chars.next();
                    }
                    let target = self
                        .read_argument(chars)
                        .context("missing file after redirection")?;
                    redirects.push(Redirection::new(
                        if append {
                            RedirectionKind::Append
                        } else {
                            RedirectionKind::Output
                        },
                        target,
                    ));
                }
                Some('<') => {
                    chars.next();
                    let target = self
                        .read_argument(chars)
                        .context("missing file after input redirection")?;
                    redirects.push(Redirection::new(RedirectionKind::Input, target));
                }
                _ => {
                    let arg = self.read_argument(chars)?;
                    if program.is_none() {
                        program = Some(arg);
                    } else {
                        args.push(arg);
                    }
                }
            }
        }
        let program = program.context("command is missing a program")?;
        Ok(Command::new(program, args, redirects))
    }

    fn read_argument(&mut self, chars: &mut Peekable<Chars<'_>>) -> Result<Argument> {
        skip_whitespace(chars);
        let mut parts = Vec::new();
        let mut buffer = String::new();
        while let Some(&ch) = chars.peek() {
            match ch {
                ' ' | '\t' | '\n' | '|' | '>' | '<' => break,
                '\'' => {
                    chars.next();
                    self.flush_buffer(&mut buffer, &mut parts);
                    parts.push(Word::Text(read_single_quote(chars)?));
                }
                '"' => {
                    chars.next();
                    self.flush_buffer(&mut buffer, &mut parts);
                    parts.extend(read_double_quote(chars)?);
                }
                '\\' => {
                    chars.next();
                    if let Some(escaped) = chars.next() {
                        buffer.push(escaped);
                    }
                }
                '$' => {
                    chars.next();
                    self.flush_buffer(&mut buffer, &mut parts);
                    if let Some('(') = chars.peek().copied() {
                        chars.next();
                        let content = read_command_substitution(chars)?;
                        parts.push(Word::Command(content));
                    } else {
                        let name = read_identifier(chars);
                        if name.is_empty() {
                            buffer.push('$');
                        } else {
                            parts.push(Word::Env(name));
                        }
                    }
                }
                _ => {
                    chars.next();
                    buffer.push(ch);
                }
            }
        }
        self.flush_buffer(&mut buffer, &mut parts);
        Ok(Argument::new(parts))
    }

    fn flush_buffer(&self, buffer: &mut String, parts: &mut Vec<Word>) {
        if !buffer.is_empty() {
            parts.push(Word::Text(std::mem::take(buffer)));
        }
    }
}

fn skip_whitespace(chars: &mut Peekable<Chars<'_>>) {
    while matches!(chars.peek(), Some(' ' | '\t' | '\n')) {
        chars.next();
    }
}

fn consume_literal(chars: &mut Peekable<Chars<'_>>, literal: &str) -> bool {
    for expected in literal.chars() {
        match chars.peek() {
            Some(&actual) if actual == expected => {
                chars.next();
            }
            _ => return false,
        }
    }
    true
}

fn read_single_quote(chars: &mut Peekable<Chars<'_>>) -> Result<String> {
    let mut result = String::new();
    for ch in chars.by_ref() {
        if ch == '\'' {
            return Ok(result);
        }
        result.push(ch);
    }
    bail!("unterminated single quote")
}

#[allow(clippy::while_let_on_iterator)]
fn read_double_quote(chars: &mut Peekable<Chars<'_>>) -> Result<Vec<Word>> {
    let mut parts = Vec::new();
    let mut buffer = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if !buffer.is_empty() {
                    parts.push(Word::Text(buffer.clone()));
                    buffer.clear();
                }
                return Ok(parts);
            }
            '\\' => {
                if let Some(escaped) = chars.next() {
                    buffer.push(escaped);
                }
            }
            '$' => {
                if !buffer.is_empty() {
                    parts.push(Word::Text(std::mem::take(&mut buffer)));
                }
                if let Some('(') = chars.peek().copied() {
                    chars.next();
                    let content = read_command_substitution(chars)?;
                    parts.push(Word::Command(content));
                } else {
                    let name = read_identifier(chars);
                    if name.is_empty() {
                        buffer.push('$');
                    } else {
                        parts.push(Word::Env(name));
                    }
                }
            }
            _ => buffer.push(ch),
        }
    }
    bail!("unterminated double quote")
}

fn read_command_substitution(chars: &mut Peekable<Chars<'_>>) -> Result<String> {
    let mut depth = 1i32;
    let mut content = String::new();
    #[allow(clippy::while_let_on_iterator)]
    while let Some(ch) = chars.next() {
        match ch {
            '(' => {
                depth += 1;
                content.push(ch);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(content);
                }
                content.push(ch);
            }
            _ => content.push(ch),
        }
    }
    bail!("unterminated command substitution")
}

fn read_identifier(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut name = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            chars.next();
            name.push(ch);
        } else {
            break;
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_command() {
        let mut parser = Parser::new();
        let stmt = parser.parse("echo hello").unwrap();
        match stmt {
            Statement::Pipeline(pipeline) => {
                assert_eq!(pipeline.commands().len(), 1);
            }
            _ => panic!("unexpected statement"),
        }
    }

    #[test]
    fn parses_pipes() {
        let mut parser = Parser::new();
        let stmt = parser.parse("echo hi | grep h").unwrap();
        match stmt {
            Statement::Pipeline(pipeline) => assert_eq!(pipeline.commands().len(), 2),
            _ => panic!("expected pipeline"),
        }
    }

    #[test]
    fn parses_command_substitution() {
        let mut parser = Parser::new();
        let stmt = parser.parse("echo $(echo hi)").unwrap();
        match stmt {
            Statement::Pipeline(pipeline) => {
                let cmd = &pipeline.commands()[0];
                assert_eq!(
                    cmd.args()[0].parts()[0],
                    Word::Command("echo hi".to_string())
                );
            }
            _ => panic!("expected pipeline"),
        }
    }

    #[test]
    fn parses_redirection() {
        let mut parser = Parser::new();
        let stmt = parser.parse("cat file.txt > out.txt").unwrap();
        match stmt {
            Statement::Pipeline(pipeline) => {
                let cmd = &pipeline.commands()[0];
                assert_eq!(cmd.redirections().len(), 1);
                assert!(matches!(
                    cmd.redirections()[0].kind(),
                    RedirectionKind::Output
                ));
            }
            _ => panic!("expected pipeline"),
        }
    }

    #[test]
    fn parses_nova_block() {
        let mut parser = Parser::new();
        let stmt = parser
            .parse("ns {\nlet x: int = 1;\n}")
            .expect("nova block");
        assert!(matches!(stmt, Statement::NovaBlock(_)));
    }

    #[test]
    fn parses_nova_expression() {
        let mut parser = Parser::new();
        let stmt = parser.parse("=> 1 + 1").unwrap();
        assert!(matches!(stmt, Statement::NovaExpression(_)));
    }
}
