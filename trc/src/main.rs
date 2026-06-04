use std::env;
use std::fs;
use std::process;

use trc::lexer;
use trc::parser;
use trc::analyzer;
use trc::interpreter;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: trc <file.tr>");
        process::exit(1);
    }

    // Let the titration begin – richie-rich90454
    let path = &args[1];
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            process::exit(1);
        }
    };

    let tokens = match lexer::tokenize(&source) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            process::exit(1);
        }
    };

    let ast = match parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    let typed_ast = match analyzer::analyze(&ast) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Semantic error: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = interpreter::interpret(&typed_ast) {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}
