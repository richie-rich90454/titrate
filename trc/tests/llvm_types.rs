//! Integration tests verifying the Titrate → LLVM type mapping.
//!
//! These tests assert that each Titrate type documented in Section 3.2 of
//! `alpha04_spec.txt` maps to the documented LLVM type via the public API in
//! `trc::codegen::llvm::types`. They complement the unit tests embedded in
//! `types.rs` by exercising the mapping through the crate's public surface
//! from an integration-test crate.
//!
//! Spec mapping (Section 3.2):
//!
//! | Titrate       | LLVM                       |
//! |---------------|----------------------------|
//! | void          | void                       |
//! | bool          | i1                         |
//! | byte          | i8                         |
//! | short         | i16                        |
//! | int           | i32                        |
//! | long          | i64                        |
//! | vast          | i128                       |
//! | uvast         | i128 (unsigned semantic)   |
//! | float         | f32                        |
//! | double        | f64                        |
//! | half          | half                       |
//! | quad          | fp128                      |
//! | char          | i32                        |
//! | string        | { i64, i8* }               |
//! | size          | i64                        |
//! | Owned<T>      | T* (opaque ptr)            |
//! | &T / &mut T   | T* (opaque ptr)            |
//! | array<T>      | { i64, T* }                |
//! | Class         | opaque ptr (Phase 1)       |
//! | Interface     | opaque ptr (Phase 1)       |
//! | Enum          | opaque ptr (Phase 1)       |
//! | Tuple         | anonymous struct           |
//!
//! Note: `uvast` maps to `i128` because LLVM IR has no distinct `u128` type;
//! signedness is a property of operations, not types. This matches the spec
//! intent (`u128` is the unsigned interpretation of `i128`).
//!
//! Heap-allocated user types (classes, interfaces, enums) and generic
//! containers (`ArrayList<T>`, `HashMap<K,V>`, ...) are represented as
//! opaque `ptr` values in Phase 1, as documented in `types.rs`. The spec's
//! `{ vtable*, field0, ... }` layout is a Phase 2 goal.
//!
//! LLVM dev files must be installed for these tests to run (they create an
//! inkwell `Context`). They are NOT marked `#[ignore]` because they do not
//! invoke the system linker — only the in-process LLVM type machinery.
use inkwell::context::Context;
use trc::ast::Type;
use trc::codegen::llvm::types::{
    array_type, is_array, is_bool, is_char, is_float, is_integer, is_owned, is_ref, is_result,
    is_string, is_tuple, is_void, llvm_type, llvm_type_or_void, pointer_to, result_type,
    string_type, integer_bit_width,
};
/// Render an LLVM type as its IR string (e.g. "i32", "{ i64, ptr }").
fn llvm_type_str(ty: &Type) -> String {
    let context = Context::create();
    let llvm = llvm_type(&context, ty).expect("type should map to an LLVM basic type");
    llvm.print_to_string().to_string()
}
/// Like `llvm_type_str` but for types that may be `void`.
fn llvm_type_or_void_str(ty: Option<&Type>) -> Option<String> {
    let context = Context::create();
    llvm_type_or_void(&context, ty)
        .expect("type should map or be void")
        .map(|v| v.print_to_string().to_string())
}
// ---- Primitive integer types (Section 3.2) ----
#[test]
fn void_is_not_a_basic_type() {
    let ty = Type::simple("void");
    let context = Context::create();
    assert!(llvm_type(&context, &ty).is_err(), "void must not be a BasicType");
    assert_eq!(llvm_type_or_void_str(Some(&ty)), None, "void must lower to None");
    assert_eq!(llvm_type_or_void_str(None), None, "absent type must lower to None");
    assert!(is_void(&ty));
}
#[test]
fn bool_maps_to_i1() {
    let ty = Type::simple("bool");
    assert_eq!(llvm_type_str(&ty), "i1");
    assert!(is_bool(&ty));
    assert_eq!(integer_bit_width(&ty), Some(1));
}
#[test]
fn byte_maps_to_i8() {
    let ty = Type::simple("byte");
    assert_eq!(llvm_type_str(&ty), "i8");
    assert_eq!(integer_bit_width(&ty), Some(8));
}
#[test]
fn short_maps_to_i16() {
    let ty = Type::simple("short");
    assert_eq!(llvm_type_str(&ty), "i16");
    assert_eq!(integer_bit_width(&ty), Some(16));
}
#[test]
fn int_maps_to_i32() {
    let ty = Type::simple("int");
    assert_eq!(llvm_type_str(&ty), "i32");
    assert_eq!(integer_bit_width(&ty), Some(32));
}
#[test]
fn long_maps_to_i64() {
    let ty = Type::simple("long");
    assert_eq!(llvm_type_str(&ty), "i64");
    assert_eq!(integer_bit_width(&ty), Some(64));
}
#[test]
fn vast_maps_to_i128() {
    let ty = Type::simple("vast");
    assert_eq!(llvm_type_str(&ty), "i128");
    assert_eq!(integer_bit_width(&ty), Some(128));
}
#[test]
fn uvast_maps_to_i128_unsigned_semantic() {
    // Spec says `u128`; LLVM has no distinct u128 type, so i128 carries the
    // unsigned interpretation (signedness is on operations, not types).
    let ty = Type::simple("uvast");
    assert_eq!(llvm_type_str(&ty), "i128");
    assert_eq!(integer_bit_width(&ty), Some(128));
}
#[test]
fn char_maps_to_i32() {
    let ty = Type::simple("char");
    assert_eq!(llvm_type_str(&ty), "i32");
    assert!(is_char(&ty));
    assert_eq!(integer_bit_width(&ty), Some(32));
}
#[test]
fn size_maps_to_i64() {
    let ty = Type::simple("size");
    assert_eq!(llvm_type_str(&ty), "i64");
    assert_eq!(integer_bit_width(&ty), Some(64));
}
#[test]
fn unsigned_int_family_maps_correctly() {
    assert_eq!(llvm_type_str(&Type::simple("u8")), "i8");
    assert_eq!(llvm_type_str(&Type::simple("u16")), "i16");
    assert_eq!(llvm_type_str(&Type::simple("u32")), "i32");
    assert_eq!(llvm_type_str(&Type::simple("u64")), "i64");
}
// ---- Float types (Section 3.2) ----
#[test]
fn float_maps_to_f32() {
    let ty = Type::simple("float");
    assert_eq!(llvm_type_str(&ty), "float");
    assert!(is_float(&ty));
}
#[test]
fn double_maps_to_f64() {
    let ty = Type::simple("double");
    assert_eq!(llvm_type_str(&ty), "double");
    assert!(is_float(&ty));
}
#[test]
fn half_maps_to_half() {
    let ty = Type::simple("half");
    assert_eq!(llvm_type_str(&ty), "half");
    assert!(is_float(&ty));
}
#[test]
fn quad_maps_to_fp128() {
    let ty = Type::simple("quad");
    assert_eq!(llvm_type_str(&ty), "fp128");
    assert!(is_float(&ty));
}
// ---- String (Section 3.2: { i64, i8* }) ----
#[test]
fn string_maps_to_struct_with_i64_and_ptr() {
    let ty = Type::simple("string");
    let s = llvm_type_str(&ty);
    assert!(is_string(&ty));
    // The struct must contain an i64 (length) and a ptr (UTF-8 buffer).
    assert!(s.contains("i64"), "string type must contain i64, got: {}", s);
    assert!(s.contains("ptr"), "string type must contain ptr, got: {}", s);
    // Verify the dedicated constructor agrees.
    let context = Context::create();
    let direct = string_type(&context).print_to_string().to_string();
    assert_eq!(s, direct, "llvm_type(string) must match string_type()");
}
// ---- array<T> (Section 3.2: { i64, T* }) ----
#[test]
fn array_maps_to_struct_with_len_and_ptr() {
    let ty = Type::generic("array", vec![Type::simple("int")]);
    let s = llvm_type_str(&ty);
    assert!(is_array(&ty));
    assert!(s.contains("i64"), "array type must contain i64, got: {}", s);
    assert!(s.contains("ptr"), "array type must contain ptr, got: {}", s);
    // Verify the dedicated constructor agrees.
    let context = Context::create();
    let elem = llvm_type(&context, &Type::simple("int")).unwrap();
    let direct = array_type(&context, elem).print_to_string().to_string();
    assert_eq!(s, direct, "llvm_type(array<int>) must match array_type()");
}
// ---- Owned<T> (Section 3.2: T*) ----
#[test]
fn owned_maps_to_pointer() {
    let ty = Type::generic("Owned", vec![Type::simple("int")]);
    let s = llvm_type_str(&ty);
    assert!(is_owned(&ty));
    // In LLVM 15+ all pointers are opaque `ptr`.
    assert!(s.starts_with("ptr"), "Owned<T> must lower to ptr, got: {}", s);
}
#[test]
fn owned_of_composite_type_maps_to_pointer() {
    let ty = Type::generic("Owned", vec![Type::simple("string")]);
    let s = llvm_type_str(&ty);
    assert!(s.starts_with("ptr"), "Owned<string> must lower to ptr, got: {}", s);
}
// ---- &T and &mut T (Section 3.2: T*) ----
#[test]
fn ref_maps_to_pointer() {
    let ty = Type::Ref(Box::new(Type::simple("int")));
    let s = llvm_type_str(&ty);
    assert!(is_ref(&ty));
    assert!(s.starts_with("ptr"), "&T must lower to ptr, got: {}", s);
}
#[test]
fn mut_ref_maps_to_pointer() {
    let ty = Type::MutRef(Box::new(Type::simple("double")));
    let s = llvm_type_str(&ty);
    assert!(is_ref(&ty));
    assert!(s.starts_with("ptr"), "&mut T must lower to ptr, got: {}", s);
}
#[test]
fn ref_of_composite_maps_to_pointer() {
    let ty = Type::Ref(Box::new(Type::generic("ArrayList", vec![Type::simple("string")])));
    let s = llvm_type_str(&ty);
    assert!(s.starts_with("ptr"), "&ArrayList<...> must lower to ptr, got: {}", s);
}
// ---- pointer_to helper ----
#[test]
fn pointer_to_returns_opaque_ptr() {
    let context = Context::create();
    let inner = llvm_type(&context, &Type::simple("int")).unwrap();
    let p = pointer_to(inner, &context);
    assert_eq!(p.print_to_string().to_string(), "ptr");
}
// ---- Result<T, E> ({ i32, i8* }) ----
#[test]
fn result_maps_to_struct_with_tag_and_payload() {
    let ty = Type::generic("Result", vec![Type::simple("int"), Type::simple("string")]);
    let s = llvm_type_str(&ty);
    assert!(is_result(&ty));
    assert!(s.contains("i32"), "Result must contain i32 tag, got: {}", s);
    assert!(s.contains("ptr"), "Result must contain ptr payload, got: {}", s);
    let context = Context::create();
    let direct = result_type(&context).print_to_string().to_string();
    assert_eq!(s, direct, "llvm_type(Result<...>) must match result_type()");
}
// ---- Tuple (anonymous struct) ----
#[test]
fn tuple_maps_to_anonymous_struct() {
    let ty = Type::Tuple(vec![Type::simple("int"), Type::simple("double")]);
    let s = llvm_type_str(&ty);
    assert!(is_tuple(&ty));
    assert!(s.contains("i32"), "tuple must contain i32, got: {}", s);
    assert!(s.contains("double"), "tuple must contain double, got: {}", s);
}
#[test]
fn empty_tuple_maps_to_empty_struct() {
    let ty = Type::Tuple(vec![]);
    let s = llvm_type_str(&ty);
    assert!(s.contains("{}"), "empty tuple must be empty struct, got: {}", s);
}
// ---- Heap-allocated user types (Phase 1: opaque ptr) ----
#[test]
fn user_class_is_opaque_pointer() {
    let ty = Type::simple("MyClass");
    let s = llvm_type_str(&ty);
    assert!(s.starts_with("ptr"), "class must be opaque ptr in Phase 1, got: {}", s);
}
#[test]
fn generic_container_is_opaque_pointer() {
    let ty = Type::generic("ArrayList", vec![Type::simple("string")]);
    let s = llvm_type_str(&ty);
    assert!(s.starts_with("ptr"), "ArrayList must be opaque ptr, got: {}", s);
}
#[test]
fn interface_is_opaque_pointer() {
    let ty = Type::simple("Comparable");
    let s = llvm_type_str(&ty);
    assert!(s.starts_with("ptr"), "interface must be opaque ptr in Phase 1, got: {}", s);
}
#[test]
fn enum_is_opaque_pointer() {
    let ty = Type::simple("Color");
    let s = llvm_type_str(&ty);
    assert!(s.starts_with("ptr"), "enum must be opaque ptr in Phase 1, got: {}", s);
}
// ---- Predicate coverage ----
#[test]
fn integer_predicates_cover_all_int_types() {
    for name in &["byte", "short", "int", "long", "vast", "uvast", "u8", "u16", "u32", "u64", "size"] {
        assert!(is_integer(&Type::simple(name)), "{} should be integer", name);
    }
    // bool and char are NOT in INTEGER_TYPES (bool is logical, char is its own category).
    assert!(!is_integer(&Type::simple("bool")));
    assert!(!is_integer(&Type::simple("double")));
}
#[test]
fn float_predicates_cover_all_float_types() {
    for name in &["float", "double", "half", "quad"] {
        assert!(is_float(&Type::simple(name)), "{} should be float", name);
    }
    assert!(!is_float(&Type::simple("int")));
}
// ---- Full pipeline: compile a program and inspect the IR for type evidence ----
#[test]
fn compile_program_emits_typed_i32_alloca() {
    use trc::analyzer;
    use trc::codegen::llvm;
    use trc::lexer;
    use trc::parser;
    let source = r#"
public fn main(): void {
    let x: int = 42;
    io::println(x);
}
"#;
    let tokens = lexer::tokenize(source).expect("tokenize");
    let ast = parser::parse(tokens).expect("parse");
    let typed = analyzer::analyze(&ast).expect("analyze");
    let ir = llvm::compile_to_ir_text(&typed).expect("compile to IR");
    // The IR must contain an i32-typed alloca for `x`.
    assert!(ir.contains("i32"), "IR should contain i32, got:\n{}", ir);
    assert!(ir.contains("alloca"), "IR should contain an alloca, got:\n{}", ir);
}
#[test]
fn compile_program_emits_f64_for_double_var() {
    use trc::analyzer;
    use trc::codegen::llvm;
    use trc::lexer;
    use trc::parser;
    let source = r#"
public fn main(): void {
    let pi: double = 3.14;
    io::println(pi);
}
"#;
    let tokens = lexer::tokenize(source).expect("tokenize");
    let ast = parser::parse(tokens).expect("parse");
    let typed = analyzer::analyze(&ast).expect("analyze");
    let ir = llvm::compile_to_ir_text(&typed).expect("compile to IR");
    assert!(ir.contains("double"), "IR should contain double, got:\n{}", ir);
}
#[test]
fn compile_program_emits_string_struct_for_string_var() {
    use trc::analyzer;
    use trc::codegen::llvm;
    use trc::lexer;
    use trc::parser;
    let source = r#"
public fn main(): void {
    let s: string = "hello";
    io::println(s);
}
"#;
    let tokens = lexer::tokenize(source).expect("tokenize");
    let ast = parser::parse(tokens).expect("parse");
    let typed = analyzer::analyze(&ast).expect("analyze");
    let ir = llvm::compile_to_ir_text(&typed).expect("compile to IR");
    // The string alloca must be a struct of { i64, ptr }.
    assert!(ir.contains("i64"), "IR should contain i64 for string len, got:\n{}", ir);
    assert!(ir.contains("ptr"), "IR should contain ptr for string buffer, got:\n{}", ir);
}
#[test]
fn compile_program_void_return_emits_void_function() {
    use trc::analyzer;
    use trc::codegen::llvm;
    use trc::lexer;
    use trc::parser;
    let source = r#"
public fn main(): void {
    io::println("ok");
}
"#;
    let tokens = lexer::tokenize(source).expect("tokenize");
    let ast = parser::parse(tokens).expect("parse");
    let typed = analyzer::analyze(&ast).expect("analyze");
    let ir = llvm::compile_to_ir_text(&typed).expect("compile to IR");
    // main must be declared as `define void @main()`.
    assert!(ir.contains("define") && ir.contains("void") && ir.contains("@main"),
        "IR should define void @main, got:\n{}", ir);
}
