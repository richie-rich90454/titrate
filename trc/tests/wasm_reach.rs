//! Smoke tests for real example files. Backend coverage is deliberately
//! partial; see docs/guide/wasm-backend.md for the unsupported list.

use std::fs;

fn compile_file(path: &str) -> Result<Vec<u8>, String> {
    let src = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let tokens = trc::lexer::tokenize(&src)?;
    let ast = trc::parser::parse(tokens)?;
    let typed = trc::analyzer::analyze(&ast).map_err(|e| e.join("\n"))?;
    trc::codegen::wasm::compile_to_wasm(&typed)
}

#[test]
fn supported_examples_compile() {
    for path in ["../examples/hello.tr", "../examples/fibonacci.tr"] {
        let bytes = compile_file(path).unwrap_or_else(|e| panic!("{} failed: {}", path, e));
        assert_eq!(&bytes[..4], b"\0asm", "{} should be a wasm module", path);
    }
}
