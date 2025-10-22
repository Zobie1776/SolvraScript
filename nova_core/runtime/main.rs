use std::env;
use std::process;

use nova_core::{NovaRuntime, RuntimeRepl, Value};

fn main() {
    let mut args = env::args().skip(1);
    let mut run_repl = true;
    let mut exec_path: Option<String> = None;
    let mut preload_paths: Vec<String> = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-V" => {
                println!("nova-runtime {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--repl" => {
                run_repl = true;
            }
            "--no-repl" => {
                run_repl = false;
            }
            "--exec" => {
                let Some(path) = args.next() else {
                    eprintln!("--exec expects a path argument");
                    process::exit(2);
                };
                exec_path = Some(path);
            }
            "--load" => {
                let Some(path) = args.next() else {
                    eprintln!("--load expects a path argument");
                    process::exit(2);
                };
                preload_paths.push(path);
            }
            other if other.starts_with('-') => {
                eprintln!("Unknown flag {other}. Use --help for usage information.");
                process::exit(2);
            }
            other => {
                exec_path = Some(other.to_string());
                run_repl = false;
            }
        }
    }

    let runtime = NovaRuntime::new();

    for path in &preload_paths {
        if let Err(err) = runtime.load_module_file(path) {
            eprintln!("Failed to load {path}: {err}");
            process::exit(1);
        }
    }

    if let Some(path) = exec_path.as_deref() {
        match runtime.execute_file(path) {
            Ok(value) => {
                print_value(&value);
            }
            Err(err) => {
                eprintln!("Execution failed: {err}");
                process::exit(1);
            }
        }
    }

    if run_repl {
        launch_repl(runtime);
    }
}

fn launch_repl(runtime: NovaRuntime) {
    let mut repl = RuntimeRepl::new(runtime);
    if let Err(err) = repl.run() {
        eprintln!("REPL terminated with error: {err}");
        process::exit(1);
    }
}

fn print_value(value: &Value) {
    match value {
        Value::Null => println!("null"),
        Value::Boolean(b) => println!("{b}"),
        Value::Integer(i) => println!("{i}"),
        Value::Float(f) => println!("{f}"),
        Value::String(s) => println!("{s}"),
        Value::Object(obj) => println!("<object {:?}>", obj.handle()),
    }
}

fn print_help() {
    println!("Nova runtime tool\n");
    println!("USAGE:");
    println!("    nova-runtime [OPTIONS] [FILE]\n");
    println!("OPTIONS:");
    println!("    -h, --help         Print this message");
    println!("    -V, --version      Print version information");
    println!("        --exec <FILE>  Execute the provided .novac bytecode file");
    println!("        --load <FILE>  Load a .novac module before entering the REPL (repeatable)");
    println!("        --repl         Force REPL mode after executing FILE or --exec");
    println!("        --no-repl      Disable REPL mode after executing FILE or --exec");
    println!("");
    println!("By default the tool enters the REPL. Providing a positional FILE executes it once.");
}
