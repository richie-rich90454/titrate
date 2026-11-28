//! Cross-backend differential harness: generated programs are lowered to
//! bytecode and to native LLVM IR from the same typed AST. The VM output must
//! match the generated oracle and the native IR must verify and define `main`.
//!
//! Only IR validation is in scope here: the LLVM backend has no in-process
//! stdout-capture host, so native execution stays with the existing
//! `native_samples_parity` gate.

use std::panic::{catch_unwind, AssertUnwindSafe};

use trc::{analyzer, ast, bytecode, codegen, lexer, parser};

use crate::common::{self, Lcg};

/// Lex, parse, and analyze a source string with stage-labelled errors.
pub fn analyze_source(source: &str) -> Result<(ast::Program, ast::Program), String> {
    let tokens = lexer::tokenize(source).map_err(|error| format!("lexer: {}", error))?;
    let raw = parser::parse(tokens).map_err(|error| format!("parser: {}", error))?;
    let typed =
        analyzer::analyze(&raw).map_err(|error| format!("analyzer: {}", error.join("; ")))?;
    Ok((raw, typed))
}

/// One program in the VM/IR intersection, plus its exact expected output.
pub fn generated_case(rng: &mut Lcg) -> (String, String) {
    let value = rng.range(0, 1_000_000);
    let source = format!(
        "public fn main(): void {{ io::println(Integer.toString({})); }}",
        value
    );
    (source, value.to_string())
}

/// Compare the bytecode VM result with the native IR projection.
pub fn compare_case(typed: &ast::Program, expected: &str) -> Result<(), String> {
    let lines = bytecode::execute(typed)?;
    let actual = lines.join("\n");
    if actual.trim_end() != expected {
        return Err(format!("bytecode VM printed {:?}, expected {:?}", actual, expected));
    }
    let ir = codegen::llvm::compile_to_ir_text(typed, &common::workspace_root())?;
    if !ir.contains("define") || !ir.contains("main") {
        return Err("native IR did not define main".to_string());
    }
    Ok(())
}

pub fn run(iterations: usize, seed: u64) -> Result<(), String> {
    let mut rng = Lcg::new(seed ^ 0x4449_4646_4552_454E);
    for iteration in 0..iterations {
        let (source, expected) = generated_case(&mut rng);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let (_, typed) = analyze_source(&source)?;
            compare_case(&typed, &expected)
        }));
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                return Err(format!("iteration {} failed: {}\n{}", iteration, error, source))
            }
            Err(_) => return Err(format!("iteration {} panicked\n{}", iteration, source)),
        }
    }
    Ok(())
}
