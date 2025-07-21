mod tokenizer;
mod parser;
mod ast;
mod interpreter;

use crate::tokenizer::Tokenizer;
use crate::parser::Parser;

use std::fs;
use std::env;
use std::process;
use interpreter::Interpreter;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let source = if args.len() > 1 {
        // Read from file if provided
        let filename = &args[1];
        match fs::read_to_string(filename) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", filename, e);
                process::exit(1);
            }
        }
    } else {
        // Default example program
        r#"
        let x: int = 5 + 6 * 2;
        let y: int = x * 3;
        y + 1
        "#.to_string()
    };

    println!("NovaScript Interpreter");
    println!("===================");
    println!("Source code:");
    println!("{}", source);
    println!();

    // Tokenize
    println!("Tokenizing...");
    let mut tokenizer = Tokenizer::new(&source);
    let tokens = match tokenizer.tokenize() {
        Ok(toks) => {
            println!("✓ Tokenization successful ({} tokens)", toks.len());
            toks
        },
        Err(e) => {
            eprintln!("✗ Tokenizer error: {}", e);
            process::exit(1);
        }
    };

    // Debug: Print tokens if verbose mode
    if args.contains(&"--verbose".to_string()) || args.contains(&"-v".to_string()) {
        println!("Tokens: {:#?}", tokens);
        println!();
    }

    // Parse
    println!("Parsing...");
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(ast) => {
            println!("✓ Parsing successful");
            ast
        },
        Err(e) => {
            eprintln!("✗ Parser error: {}", e);
            process::exit(1);
        }
    };

    // Debug: Print AST if verbose mode
    if args.contains(&"--verbose".to_string()) || args.contains(&"-v".to_string()) {
        println!("Parsed AST:");
        println!("{:#?}", program);
        println!();
    }

    // Interpret
    println!("Interpreting...");
    let mut interp = Interpreter::new();
    match interp.eval_program(&program) {
        Ok(Some(val)) => {
            println!("✓ Execution successful");
            println!("Result: {:?}", val);
        },
        Ok(None) => {
            println!("✓ Program executed successfully (no return value)");
        },
        Err(e) => {
            eprintln!("✗ Runtime error: {:?}", e);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let source = "5 + 3 * 2";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut interp = Interpreter::new();
        
        match interp.eval_program(&program) {
            Ok(Some(val)) => {
                // This should evaluate to 11 (5 + (3 * 2))
                assert_eq!(format!("{:?}", val), "Int(11)");
            },
            _ => panic!("Expected successful evaluation"),
        }
    }

    #[test]
    fn test_variable_assignment() {
        let source = "let x: int = 10;\nlet y: int = x + 5;\ny";
        
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut interp = Interpreter::new();
        
        match interp.eval_program(&program) {
            Ok(Some(val)) => {
                // This should evaluate to 15
                assert_eq!(format!("{:?}", val), "Int(15)");
            },
            _ => panic!("Expected successful evaluation"),
        }
    }

    #[test]
    fn test_complex_expression() {
        let source = "let x: int = 5 + 6 * 2;\nlet y: int = x * 3;\ny + 1";
        
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut interp = Interpreter::new();
        
        match interp.eval_program(&program) {
            Ok(Some(val)) => {
                // x = 5 + 12 = 17, y = 17 * 3 = 51, result = 51 + 1 = 52
                assert_eq!(format!("{:?}", val), "Int(52)");
            },
            _ => panic!("Expected successful evaluation"),
        }
    }

    #[test]
    fn test_tokenizer_error() {
        let source = "let x = @invalid_token";
        let mut tokenizer = Tokenizer::new(source);
        assert!(tokenizer.tokenize().is_err());
    }

    #[test]
    fn test_parser_error() {
        let source = "let x: int = ;"; // Missing expression
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        assert!(parser.parse().is_err());
    }
}

// Helper functions for development and debugging
#[allow(dead_code)]
fn print_usage() {
    println!("Usage: novascript [OPTIONS] [FILE]");
    println!();
    println!("Options:");
    println!("  -v, --verbose    Enable verbose output (show tokens and AST)");
    println!("  -h, --help       Show this help message");
    println!();
    println!("Arguments:");
    println!("  FILE            NovaScript file to execute (optional)");
    println!();
    println!("If no file is provided, a default example program will be executed.");
}

#[allow(dead_code)]
fn run_interactive_mode() {
    println!("NovaScript Interactive Mode");
    println!("Type 'exit' to quit, 'help' for help");
    println!();
    
    loop {
        print!("nova> ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                
                if input == "exit" {
                    println!("Goodbye!");
                    break;
                } else if input == "help" {
                    println!("Available commands:");
                    println!("  exit - Exit the interpreter");
                    println!("  help - Show this help");
                    println!("  Any NovaScript expression or statement");
                    continue;
                } else if input.is_empty() {
                    continue;
                }
                
                // Process the input
                let mut tokenizer = Tokenizer::new(input);
                let tokens = match tokenizer.tokenize() {
                    Ok(toks) => toks,
                    Err(e) => {
                        eprintln!("Tokenizer error: {}", e);
                        continue;
                    }
                };
                
                let mut parser = Parser::new(tokens);
                let program = match parser.parse() {
                    Ok(ast) => ast,
                    Err(e) => {
                        eprintln!("Parser error: {}", e);
                        continue;
                    }
                };
                
                let mut interp = Interpreter::new();
                match interp.eval_program(&program) {
                    Ok(Some(val)) => println!("=> {:?}", val),
                    Ok(None) => println!("OK"),
                    Err(e) => eprintln!("Runtime error: {:?}", e),
                }
            },
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
}