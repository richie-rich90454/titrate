//! IR-level regression tests for LLVM native-bridge and codegen fixes.
//!
//! These tests do not invoke the system linker; they build a small in-process
//! LLVM module (or compile a Titrate source to IR text) and inspect the
//! emitted instructions. LLVM dev files are therefore required.

use std::path::PathBuf;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use trc::ast::Type;
use trc::codegen::llvm::native_bridge::{
    emit_native_call, marshal_to_titrate, titrate_value_type, unmarshal_from_titrate,
    unmarshal_from_titrate_declared, TV_HALF,
};

/// A freshly built function entry block and the module that owns it.
struct IrProbe<'ctx> {
    module: Module<'ctx>,
    builder: Builder<'ctx>,
}

fn probe<'a>(context: &'a Context, name: &str) -> IrProbe<'a> {
    let module = context.create_module(name);
    let builder = context.create_builder();
    let fn_ty = context.void_type().fn_type(&[], false);
    let f = module.add_function("probe_fn", fn_ty, None);
    let entry = context.append_basic_block(f, "entry");
    builder.position_at_end(entry);
    IrProbe { module, builder }
}

fn ir_text(p: &IrProbe<'_>) -> String {
    p.module.print_to_string().to_string()
}

/// Locate the workspace root by walking up from CARGO_MANIFEST_DIR.
fn workspace_root() -> PathBuf {
    let manifest =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by cargo");
    PathBuf::from(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .expect("trc should be inside the workspace")
}

fn compile(source: &str) -> Result<String, String> {
    let tokens = trc::lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
    let ast = trc::parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
    let typed = trc::analyzer::analyze(&ast).map_err(|e| format!("analyze: {:?}", e))?;
    trc::codegen::llvm::compile_to_ir_text(&typed, &workspace_root())
}

// ---------------------------------------------------------------------------
// Task 1: native-bridge marshalling for half / vast / uvast / quad
// ---------------------------------------------------------------------------

#[test]
fn marshal_half_widens_to_f32_and_uses_tv_half_tag() {
    let context = Context::create();
    let p = probe(&context, "marshal_half");
    // Load the f16 from memory so LLVM cannot constant-fold the fpext away.
    let half_slot = p
        .builder
        .build_alloca(context.f16_type(), "half.slot")
        .expect("alloca");
    p.builder
        .build_store(half_slot, context.f16_type().const_float(1.5))
        .expect("store half");
    let half = p
        .builder
        .build_load(context.f16_type(), half_slot, "half.value")
        .expect("load half");

    let _tv = marshal_to_titrate(&context, &p.builder, half, &Type::simple("half"))
        .expect("half must marshal");

    let ir = ir_text(&p);
    assert!(
        ir.contains(&format!("i32 {}", TV_HALF)),
        "expected TV_HALF tag, got:\n{}",
        ir
    );
    assert!(
        ir.contains("fpext half") && ir.contains("to float"),
        "expected f16->f32 widen, got:\n{}",
        ir
    );
}

#[test]
fn unmarshal_half_narrows_f32_to_f16() {
    let context = Context::create();
    let p = probe(&context, "unmarshal_half");

    // Assemble a TitrateValue carrying TV_HALF and an f32 payload (2.5).
    let payload_ty = context.i8_type().array_type(16);
    let payload_slot = p
        .builder
        .build_alloca(payload_ty, "payload.slot")
        .expect("alloca");
    p.builder
        .build_store(payload_slot, payload_ty.const_zero())
        .expect("zero payload");
    let raw_ptr = p
        .builder
        .build_bit_cast(
            payload_slot,
            context.ptr_type(Default::default()),
            "payload.raw",
        )
        .expect("bitcast")
        .into_pointer_value();
    p.builder
        .build_store(raw_ptr, context.f32_type().const_float(2.5))
        .expect("store f32");
    let payload = p
        .builder
        .build_load(payload_ty, payload_slot, "payload.value")
        .expect("load payload");

    let tv_ty = titrate_value_type(&context);
    let tag = context.i32_type().const_int(TV_HALF as u64, false);
    let tv = p
        .builder
        .build_insert_value(tv_ty.const_zero(), tag, 0, "tag")
        .expect("insert tag");
    let tv = p
        .builder
        .build_insert_value(tv, payload, 2, "payload")
        .expect("insert payload")
        .into_struct_value();

    let value = unmarshal_from_titrate(&context, &p.builder, tv, &Type::simple("half"))
        .expect("half must unmarshal");
    assert!(value.is_float_value(), "half must unmarshal to a float");
    assert_eq!(value.get_type().print_to_string().to_string(), "half");

    let ir = ir_text(&p);
    assert!(ir.contains("fptrunc float"), "expected f32->f16 narrow, got:\n{}", ir);
    assert!(ir.contains("to half"), "expected f16 result, got:\n{}", ir);
}

#[test]
fn marshal_rejects_vast_uvast_and_quad_explicitly() {
    let context = Context::create();
    let p = probe(&context, "marshal_unsupported");

    let i128_value = context.i128_type().const_int(1, false);
    for name in ["vast", "uvast"] {
        let err = marshal_to_titrate(&context, &p.builder, i128_value.into(), &Type::simple(name))
            .expect_err("128-bit integers must be rejected, not truncated");
        assert!(err.contains("cannot marshal"), "unexpected error: {}", err);
        assert!(err.contains(name), "error must name the offending type: {}", err);
    }

    let fp128_value = context.f128_type().const_float(1.0);
    let err = marshal_to_titrate(
        &context,
        &p.builder,
        fp128_value.into(),
        &Type::simple("quad"),
    )
    .expect_err("fp128 must be rejected, not narrowed to f64");
    assert!(err.contains("quad"), "error must name quad: {}", err);
}

#[test]
fn unmarshal_rejects_vast_uvast_and_quad_explicitly() {
    let context = Context::create();
    let p = probe(&context, "unmarshal_unsupported");
    let tv = titrate_value_type(&context).const_zero();

    for name in ["vast", "uvast", "quad"] {
        let err = unmarshal_from_titrate(&context, &p.builder, tv, &Type::simple(name))
            .expect_err("{} must be rejected, not silently misread");
        assert!(err.contains("cannot unmarshal"), "unexpected error: {}", err);
        assert!(err.contains(name), "error must name the offending type: {}", err);
    }
}

// ---------------------------------------------------------------------------
// Task 2: generic interface default methods carry a real body
// ---------------------------------------------------------------------------

#[test]
fn generic_interface_default_method_emits_real_body() {
    let src = r#"
        interface Sink<T> {
            fn accept(value: T): void {
                io::println("default sink body");
            }
        }

        public class IntSink implements Sink<int> {
            public fn init() {}
        }

        public fn main(): void {
            let s: IntSink = new IntSink();
        }
        "#;
    let tokens = trc::lexer::tokenize(src).expect("tokenize");
    let ast = trc::parser::parse(tokens).expect("parse");
    let typed = trc::analyzer::analyze(&ast).expect("analyze");
    let dir = std::env::temp_dir().join(format!("trc_llvm_parity_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let obj = dir.join("out.obj");
    let ir_path = dir.join("out.ll");
    trc::codegen::llvm::compile_with_ir(&typed, &obj, &ir_path, false, &workspace_root())
        .expect("generic interface default method must compile");
    let ir = std::fs::read_to_string(&ir_path).expect("read emitted IR");

    // The per-class clone IntSink_accept must exist and carry the body, not a
    // bare `ret void`. The emitted string constant proves the body ran.
    assert!(ir.contains("@IntSink_accept"), "missing default clone in IR:\n{}", ir);
    assert!(
        ir.contains("default sink body"),
        "default method body was dropped (empty clone):\n{}",
        ir
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Task 3: stdin reads are bridged through real natives, and Json.parse forces
// the pure-parser fallback instead of returning an opaque handle.
// ---------------------------------------------------------------------------

#[test]
fn io_read_line_uses_the_native_stdin_bridge() {
    let src = r#"
        public fn main(): void {
            let line: string = io::readLine();
            io::println(line);
        }
    "#;
    let ir = compile(src).expect("io::readLine should compile natively");
    assert!(
        ir.contains("IO_readLine"),
        "expected IO_readLine native call in IR"
    );
}

#[test]
fn io_read_all_uses_the_native_stdin_bridge() {
    let src = r#"
        public fn main(): void {
            let text: string = io::readAll();
            io::println(text);
        }
    "#;
    let ir = compile(src).expect("io::readAll should compile natively");
    assert!(
        ir.contains("IO_readAll"),
        "expected IO_readAll native call in IR"
    );
}

#[test]
#[ignore = "JsonParser requires iterator method dispatch (this.hasNext) not yet lowered in LLVM; tracked in native stdlib parity work"]
fn json_parse_forces_the_pure_parser_fallback() {
    let src = r#"
        import tt::json::Json;
        import tt::json::JsonValue;
        public fn main(): void {
            let value: JsonValue = Json.parse("{}");
        }
    "#;
    compile(src).expect("Json.parse should compile via the Titrate parser fallback");
}

// ---------------------------------------------------------------------------
// R6: declared return type wins over the native name table
// ---------------------------------------------------------------------------

fn tv_with_payload<'ctx>(
    context: &'ctx Context,
    p: &IrProbe<'ctx>,
    tag: i32,
    bits: u64,
) -> inkwell::values::StructValue<'ctx> {
    let payload_ty = context.i8_type().array_type(16);
    let slot = p
        .builder
        .build_alloca(payload_ty, "probe.payload.slot")
        .expect("payload alloca");
    p.builder
        .build_store(slot, payload_ty.const_zero())
        .expect("zero payload");
    let raw = p
        .builder
        .build_bit_cast(
            slot,
            context.ptr_type(Default::default()),
            "probe.payload.raw",
        )
        .expect("payload bitcast")
        .into_pointer_value();
    p.builder
        .build_store(raw, context.i64_type().const_int(bits, false))
        .expect("store payload bits");
    let payload = p
        .builder
        .build_load(payload_ty, slot, "probe.payload")
        .expect("load payload");
    let tv_ty = titrate_value_type(context);
    let tv = p
        .builder
        .build_insert_value(
            tv_ty.const_zero(),
            context.i32_type().const_int(tag as u64, false),
            0,
            "probe.tag",
        )
        .expect("insert tag");
    p.builder
        .build_insert_value(tv, payload, 2, "probe.payload.final")
        .expect("insert payload")
        .into_struct_value()
}

#[test]
fn declared_return_type_overrides_name_table() {
    let context = Context::create();
    let p = probe(&context, "declared_return_type");

    // Name table says `parseInt` -> int, but the call site declares a string.
    let tv = tv_with_payload(&context, &p, 12, 0);
    let value = unmarshal_from_titrate_declared(
        &context,
        &p.builder,
        tv,
        &Type::simple("int"),
        Some(&Type::simple("string")),
    )
    .expect("declared string must unmarshal");
    assert!(
        value.is_struct_value(),
        "declared string must yield the string struct, not the name-table i32: {:?}",
        value.get_type().print_to_string()
    );

    // Name table says `String_length` -> int, declared class must yield a
    // reconstructed object pointer instead of a truncated i32.
    let tv = tv_with_payload(&context, &p, 14, 1);
    let value = unmarshal_from_titrate_declared(
        &context,
        &p.builder,
        tv,
        &Type::simple("int"),
        Some(&Type::simple("Box")),
    )
    .expect("declared class must unmarshal");
    assert!(
        value.is_pointer_value(),
        "declared class must yield a pointer, got {:?}",
        value.get_type().print_to_string()
    );
    let ir = ir_text(&p);
    assert!(
        ir.contains("inttoptr"),
        "expected i64 bits to be reconstructed as a pointer, got:\n{}",
        ir
    );
}

#[test]
fn emit_native_call_uses_declared_return_type() {
    let context = Context::create();
    let p = probe(&context, "emit_native_declared");

    // `ArrayList_get` is listed as string in the name table; a generic class
    // element declares a class, so the bridge must return a pointer.
    let value = emit_native_call(
        &context,
        &p.builder,
        &p.module,
        "ArrayList_get",
        &[],
        &[],
        Some(&Type::simple("Box")),
    )
    .expect("emit with declared return type");
    assert!(
        value.is_pointer_value(),
        "declared class return must be a pointer, got {:?}",
        value.get_type().print_to_string()
    );

    // Without a declaration, the name-table string result is kept.
    let p2 = probe(&context, "emit_native_inferred");
    let inferred = emit_native_call(
        &context,
        &p2.builder,
        &p2.module,
        "ArrayList_get",
        &[],
        &[],
        None,
    )
    .expect("emit with inferred return type");
    assert!(
        inferred.is_struct_value(),
        "name-table fallback must keep the string struct, got {:?}",
        inferred.get_type().print_to_string()
    );
}

#[test]
fn marshal_pointer_argument_uses_class_instance_tag() {
    let context = Context::create();
    let p = probe(&context, "marshal_ptr_arg");
    let slot = p
        .builder
        .build_alloca(context.i64_type(), "obj.slot")
        .expect("alloca");
    let ptr = p
        .builder
        .build_bit_cast(
            slot,
            context.ptr_type(Default::default()),
            "obj.ptr",
        )
        .expect("bitcast")
        .into_pointer_value();

    let _tv = marshal_to_titrate(&context, &p.builder, ptr.into(), &Type::simple("Box"))
        .expect("object pointer must marshal");
    let ir = ir_text(&p);
    assert!(
        ir.contains(&format!("i32 {}", 14)),
        "expected TV_CLASS_INSTANCE tag for an object pointer, got:\n{}",
        ir
    );
    assert!(
        ir.contains("ptrtoint"),
        "expected pointer bits to be stored as i64, got:\n{}",
        ir
    );
}

#[test]
fn factored_interface_vtable_path_still_emits_vtables() {
    let src = r#"
        interface Greeter {
            fn greet(): string;
        }

        public class Hello implements Greeter {
            public fn init() {}

            public fn greet(): string {
                return "hello";
            }
        }

        public fn main(): void {
            let g: Greeter = new Hello();
        }
    "#;
    let tokens = trc::lexer::tokenize(src).expect("tokenize");
    let ast = trc::parser::parse(tokens).expect("parse");
    let typed = trc::analyzer::analyze(&ast).expect("analyze");
    let dir = std::env::temp_dir().join(format!("trc_llvm_ivtable_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let obj = dir.join("out.obj");
    let ir_path = dir.join("out.ll");
    trc::codegen::llvm::compile_with_ir(&typed, &obj, &ir_path, false, &workspace_root())
        .expect("interface vtable must compile");
    let ir = std::fs::read_to_string(&ir_path).expect("read emitted IR");
    assert!(
        ir.contains("_ivtable_Greeter_Hello"),
        "factored helper must still build the interface vtable:\n{}",
        ir
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// R8: generic class elements of containers are object pointers
// ---------------------------------------------------------------------------

#[test]
fn generic_container_element_access_uses_object_pointer() {
    let src = r#"
        public class Box<T> {
            public T value;

            public fn init(v: T) {
                this.value = v;
            }

            public fn getValue(): T {
                return this.value;
            }
        }

        public fn main(): void {
            let list: ArrayList<Box<int>> = new ArrayList<Box<int>>();
            list.add(new Box<int>(42));
            let b: Box<int> = list.get(0);
            io::println(b.getValue());
        }
    "#;
    let ir = compile(src).expect("generic container element access must compile");
    assert!(
        ir.contains("objarr.ptr"),
        "generic class element must be read as an object pointer (objarr.ptr), not a raw payload:\n{}",
        ir
    );
    assert!(
        !ir.contains("carr.value"),
        "generic class element must not be read as a raw {{ i64, ptr }} payload:\n{}",
        ir
    );
}
