// String values, concatenation, and print lowering

use super::native_bridge;
use super::types as llvm_types;
use super::{LlvmBackend, StringValue};
use crate::ast::{Expr, Literal, Operator, Type};
use inkwell::AddressSpace;
use inkwell::module::Linkage;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue};

impl<'ctx> LlvmBackend<'ctx> {
    /// Declare the external C-ABI functions provided by `titrate_native`.
    pub(super) fn declare_natives(&self) {
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let f64_type = self.context.f64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let void_type = self.context.void_type();

        // void titrate_println(i64 len, i8* ptr)
        let println_fn = void_type.fn_type(&[i64_type.into(), i8_ptr.into()], false);
        self.module
            .add_function("titrate_println", println_fn, Some(Linkage::External));

        // i8* titrate_string_concat(i64 a_len, i8* a_ptr, i64 b_len, i8* b_ptr, i64* out_len)
        let concat_fn = i8_ptr.fn_type(
            &[
                i64_type.into(),
                i8_ptr.into(),
                i64_type.into(),
                i8_ptr.into(),
                i8_ptr.into(),
            ],
            false,
        );
        self.module
            .add_function("titrate_string_concat", concat_fn, Some(Linkage::External));

        // void titrate_free(i8* ptr)
        let free_fn = void_type.fn_type(&[i8_ptr.into()], false);
        self.module
            .add_function("titrate_free", free_fn, Some(Linkage::External));

        // i8* titrate_malloc(i64 size)
        let malloc_fn = i8_ptr.fn_type(&[i64_type.into()], false);
        self.module
            .add_function("titrate_malloc", malloc_fn, Some(Linkage::External));

        // Primitive printers.
        let println_int_fn = void_type.fn_type(&[i64_type.into()], false);
        self.module.add_function(
            "titrate_println_int",
            println_int_fn,
            Some(Linkage::External),
        );

        let println_double_fn = void_type.fn_type(&[f64_type.into()], false);
        self.module.add_function(
            "titrate_println_double",
            println_double_fn,
            Some(Linkage::External),
        );

        let println_bool_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function(
            "titrate_println_bool",
            println_bool_fn,
            Some(Linkage::External),
        );

        let println_char_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module.add_function(
            "titrate_println_char",
            println_char_fn,
            Some(Linkage::External),
        );

        // Print without newline (io::print support)
        let print_fn = void_type.fn_type(&[i64_type.into(), i8_ptr.into()], false);
        self.module
            .add_function("titrate_print", print_fn, Some(Linkage::External));

        let print_int_fn = void_type.fn_type(&[i64_type.into()], false);
        self.module
            .add_function("titrate_print_int", print_int_fn, Some(Linkage::External));

        let print_double_fn = void_type.fn_type(&[f64_type.into()], false);
        self.module.add_function(
            "titrate_print_double",
            print_double_fn,
            Some(Linkage::External),
        );

        let print_bool_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module
            .add_function("titrate_print_bool", print_bool_fn, Some(Linkage::External));

        let print_char_fn = void_type.fn_type(&[i32_type.into()], false);
        self.module
            .add_function("titrate_print_char", print_char_fn, Some(Linkage::External));

        // titrate_array_get_string(arr, index, out): out-pointer pattern.
        // The array is passed by pointer: passing {i64, ptr} by value is
        // classified differently by LLVM and the Rust/C ABI on Windows x64.
        // The string is returned through `out` for the same reason: struct
        // returns disagree between LLVM and rustc on Windows x64.
        let array_get_fn = void_type.fn_type(
            &[i8_ptr.into(), i64_type.into(), i8_ptr.into()],
            false,
        );
        self.module.add_function(
            "titrate_array_get_string",
            array_get_fn,
            Some(Linkage::External),
        );

        // Global exception pointer for throw/try-catch error propagation.
        // Phase 1: i8* points to a heap-allocated error payload.
        let exception_global = self.module.add_global(i8_ptr, None, "__titrate_exception");
        exception_global.set_initializer(&i8_ptr.const_null());
        exception_global.set_linkage(Linkage::Internal);
    }

    /// Create a global byte array holding the string's UTF-8 bytes and return
    /// a `(len, ptr)` pair pointing at it.
    pub(super) fn make_string_global(&mut self, s: &str) -> StringValue<'ctx> {
        let bytes = s.as_bytes();
        let len = bytes.len();
        let i8_type = self.context.i8_type();
        let i64_type = self.context.i64_type();

        let name = format!(".str.{}", self.string_counter);
        self.string_counter += 1;

        // Include a NUL terminator so the buffer is also a valid C string,
        // which is handy for debugging and for native interop.
        let mut const_bytes: Vec<IntValue> = bytes
            .iter()
            .map(|&b| i8_type.const_int(b as u64, false))
            .collect();
        const_bytes.push(i8_type.const_int(0, false));

        let arr_type = i8_type.array_type(const_bytes.len() as u32);
        let global = self.module.add_global(arr_type, None, &name);
        global.set_linkage(Linkage::Private);
        global.set_constant(true);
        global.set_initializer(&i8_type.const_array(&const_bytes));

        let len_val = i64_type.const_int(len as u64, false);
        let ptr_val = global.as_pointer_value();
        StringValue {
            len: len_val,
            ptr: ptr_val,
        }
    }

    /// Look up a native function by name.
    pub(super) fn get_function(&self, name: &str) -> FunctionValue<'ctx> {
        self.module
            .get_function(name)
            .unwrap_or_else(|| panic!("function '{}' not declared", name))
    }

    /// Emit a call to a native function via the titrate_native_call_out bridge.
    /// This is a fixed version of native_bridge::emit_native_call that correctly
    /// bitcasts pointer arguments to i8* to match the C function signature.
    pub(super) fn compile_native_call(
        &self,
        native_name: &str,
        arg_values: &[BasicValueEnum<'ctx>],
        arg_types: &[Type],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let tv_ty = native_bridge::titrate_value_type(self.context);
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let usize_type = self.context.i64_type();
        let void_type = self.context.void_type();

        // Declare titrate_native_call_out if not already declared.
        let out_fn_name = "titrate_native_call_out";
        if self.module.get_function(out_fn_name).is_none() {
            let fn_type = void_type.fn_type(
                &[
                    i8_ptr.into(),     // name_ptr
                    usize_type.into(), // name_len
                    i8_ptr.into(),     // args (i8*)
                    usize_type.into(), // arg_count
                    i8_ptr.into(),     // out (i8*)
                ],
                false,
            );
            self.module
                .add_function(out_fn_name, fn_type, Some(Linkage::External));
        }
        let out_fn = self
            .module
            .get_function(out_fn_name)
            .ok_or_else(|| format!("failed to declare '{}'", out_fn_name))?;

        // Marshal each argument to a TitrateValue and store into an array.
        let arg_count = arg_values.len();
        let array_ty = tv_ty.array_type(arg_count.max(1) as u32);
        let array_alloca = self
            .builder
            .build_alloca(array_ty, "native.args")
            .map_err(|e| format!("build_alloca native.args failed: {:?}", e))?;

        for (i, (val, ty)) in arg_values.iter().zip(arg_types.iter()).enumerate() {
            // Determine the correct marshal type. Container values ({i64, ptr}
            // structs) must marshal as "array" so the runtime decodes them as
            // arrays, not strings. Other struct values (strings, arrays of
            // objects) marshal as "string". When the inferred type says "int"
            // but the value is a struct, marshal as "string" to avoid
            // corruption.
            let marshal_ty = match ty.name() {
                "array" | "ArrayList" | "HashMap" => ty.clone(),
                "string" => ty.clone(),
                _ if val.is_struct_value() => Type::simple("string"),
                "int" | "u8" | "u16" | "u32" | "bool" | "byte" | "short" | "long" | "u64"
                | "size" | "float" | "double" | "half" | "quad" | "char" | "void" => ty.clone(),
                _ => Type::simple("string"),
            };
            let tv =
                native_bridge::marshal_to_titrate(self.context, &self.builder, *val, &marshal_ty)?;
            // Index with [0, i]: a single index on an array type would stride by
            // the whole array size (e.g. i=1 -> offset 48 for a 24-byte element),
            // corrupting every arg beyond the first. Two indices select element i.
            let elem_ptr = unsafe {
                self.builder.build_in_bounds_gep(
                    array_ty,
                    array_alloca,
                    &[
                        self.context.i32_type().const_int(0, false),
                        self.context.i32_type().const_int(i as u64, false),
                    ],
                    &format!("native.arg.{}", i),
                )
            }
            .map_err(|e| format!("build_in_bounds_gep native arg {} failed: {:?}", i, e))?;
            self.builder
                .build_store(elem_ptr, tv)
                .map_err(|e| format!("build_store native arg {} failed: {:?}", i, e))?;
        }

        // Allocate a TitrateValue on the stack for the return value.
        let out_alloca = self
            .builder
            .build_alloca(tv_ty, "native.out")
            .map_err(|e| format!("build_alloca native.out failed: {:?}", e))?;

        // Create the native name string as a global constant.
        let name_global = self
            .builder
            .build_global_string_ptr(native_name, "native.name")
            .map_err(|e| format!("build_global_string_ptr failed: {:?}", e))?;
        let name_ptr = name_global.as_pointer_value();
        let name_len_val = usize_type.const_int(native_name.len() as u64, false);

        // Bitcast typed pointers to i8* to match the C function signature.
        let array_ptr_i8 = self
            .builder
            .build_bit_cast(array_alloca, i8_ptr, "native.args.i8")
            .map_err(|e| format!("build_bit_cast native.args failed: {:?}", e))?;
        let out_ptr_i8 = self
            .builder
            .build_bit_cast(out_alloca, i8_ptr, "native.out.i8")
            .map_err(|e| format!("build_bit_cast native.out failed: {:?}", e))?;

        // Call titrate_native_call_out(name_ptr, name_len, args, arg_count, out).
        let count_val = usize_type.const_int(arg_count as u64, false);
        self.builder
            .build_call(
                out_fn,
                &[
                    name_ptr.into(),
                    name_len_val.into(),
                    array_ptr_i8.into(),
                    count_val.into(),
                    out_ptr_i8.into(),
                ],
                "native.call",
            )
            .map_err(|e| format!("build_call titrate_native_call_out failed: {:?}", e))?;

        // Unmarshal the result from the out pointer.
        let return_ty = native_bridge::infer_native_return_type(native_name);
        if return_ty.name() == "void" {
            return Ok(self.context.i32_type().const_int(0, false).into());
        }

        // Load the TitrateValue from the out alloca.
        let tv_result = self
            .builder
            .build_load(tv_ty, out_alloca, "native.result")
            .map_err(|e| format!("build_load native result failed: {:?}", e))?
            .into_struct_value();

        native_bridge::unmarshal_from_titrate(self.context, &self.builder, tv_result, &return_ty)
    }

    /// Convert a `StringValue` to a `BasicValueEnum` struct value by storing
    /// it into a temporary alloca and loading the struct.
    pub(super) fn string_value_to_basic(&self, sv: StringValue<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let string_ty = llvm_types::string_type(self.context).into_struct_type();
        let alloca = self
            .builder
            .build_alloca(string_ty, "str.tmp")
            .map_err(|e| format!("build_alloca str.tmp failed: {:?}", e))?;
        let len_ptr = self
            .builder
            .build_struct_gep(string_ty, alloca, 0, "str.len.ptr")
            .map_err(|e| format!("build_struct_gep 0 failed: {:?}", e))?;
        let ptr_ptr = self
            .builder
            .build_struct_gep(string_ty, alloca, 1, "str.ptr.ptr")
            .map_err(|e| format!("build_struct_gep 1 failed: {:?}", e))?;
        self.builder
            .build_store(len_ptr, sv.len)
            .map_err(|e| format!("build_store len failed: {:?}", e))?;
        self.builder
            .build_store(ptr_ptr, sv.ptr)
            .map_err(|e| format!("build_store ptr failed: {:?}", e))?;
        self.builder
            .build_load(string_ty, alloca, "str.val")
            .map_err(|e| format!("build_load str.val failed: {:?}", e))
    }

    /// Extract a `StringValue` from a `BasicValueEnum` that is a string struct.
    pub(super) fn basic_to_string_value(&mut self, v: BasicValueEnum<'ctx>) -> Result<StringValue<'ctx>, String> {
        match v {
            BasicValueEnum::StructValue(sv) => {
                let len = self
                    .builder
                    .build_extract_value(sv, 0, "sv.len")
                    .map_err(|e| format!("build_extract_value 0 failed: {:?}", e))?
                    .into_int_value();
                let ptr = self
                    .builder
                    .build_extract_value(sv, 1, "sv.ptr")
                    .map_err(|e| format!("build_extract_value 1 failed: {:?}", e))?
                    .into_pointer_value();
                self.null_aware_string(StringValue { len, ptr })
            }
            _ => Err(format!("expected string struct, got {:?}", v)),
        }
    }

    /// Render a possibly-null string slot: `{0, null}` (a null degraded by
    /// struct unmarshal) prints as "null" like the VM. Genuine empty strings
    /// keep a non-null buffer and pass through unchanged.
    pub(super) fn null_aware_string(
        &mut self,
        sv: StringValue<'ctx>,
    ) -> Result<StringValue<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let zero = i64_ty.const_int(0, false);
        let len_is_zero = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                sv.len,
                zero,
                "null.len.zero",
            )
            .map_err(|e| format!("build_int_compare null.len failed: {:?}", e))?;
        let ptr_as_int = self
            .builder
            .build_ptr_to_int(sv.ptr, i64_ty, "null.ptr.int")
            .map_err(|e| format!("build_ptr_to_int null.ptr failed: {:?}", e))?;
        let ptr_is_null = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                ptr_as_int,
                zero,
                "null.ptr.zero",
            )
            .map_err(|e| format!("build_int_compare null.ptr failed: {:?}", e))?;
        let is_null = self
            .builder
            .build_and(len_is_zero, ptr_is_null, "null.is")
            .map_err(|e| format!("build_and null.is failed: {:?}", e))?;
        let null_str = self.make_string_global("null");
        let len = self
            .builder
            .build_select(
                is_null,
                i64_ty.const_int(4, false),
                sv.len,
                "null.len",
            )
            .map_err(|e| format!("build_select null.len failed: {:?}", e))?
            .into_int_value();
        let ptr = self
            .builder
            .build_select(is_null, null_str.ptr, sv.ptr, "null.ptr")
            .map_err(|e| format!("build_select null.ptr failed: {:?}", e))?
            .into_pointer_value();
        Ok(StringValue { len, ptr })
    }

    /// Compile a string expression to a `StringValue`.
    pub(super) fn compile_string_expr(&mut self, expr: &Expr) -> Result<StringValue<'ctx>, String> {
        match expr {
            Expr::Literal(Literal::String(s), _) => Ok(self.make_string_global(s)),
            Expr::Identifier(name, _) => {
                let var = self
                    .locals
                    .get(name)
                    .ok_or_else(|| format!("codegen: unknown string variable '{}'", name))?;
                // The local stores a { i64, i8* } struct.
                let string_ty = llvm_types::string_type(self.context).into_struct_type();
                let struct_val = self
                    .builder
                    .build_load(string_ty, var.ptr, &format!("{}.val", name))
                    .map_err(|e| format!("build_load string failed: {:?}", e))?
                    .into_struct_value();
                let len = self
                    .builder
                    .build_extract_value(struct_val, 0, &format!("{}.len", name))
                    .map_err(|e| format!("build_extract_value len failed: {:?}", e))?
                    .into_int_value();
                let ptr = self
                    .builder
                    .build_extract_value(struct_val, 1, &format!("{}.ptr", name))
                    .map_err(|e| format!("build_extract_value ptr failed: {:?}", e))?
                    .into_pointer_value();
                self.null_aware_string(StringValue { len, ptr })
            }
            Expr::Binary(left, Operator::Add, right, _) => {
                // String concatenation: compile both sides as strings.
                // We don't gate on infer_expr_type because native function calls
                // (e.g. String_substring) return "unknown" from infer but are strings.
                let l = self.compile_string_expr(left)?;
                let r = self.compile_string_expr(right)?;
                self.build_string_concat(l, r)
            }
            _ => {
                // Try general compile_expr and convert.
                let v = self.compile_expr(expr)?;
                if v.is_struct_value() && self.is_string_struct(&v) {
                    self.basic_to_string_value(v)
                } else {
                    // Non-string value: convert to string using native toString.
                    let ty = self.infer_expr_type(expr);
                    let sv = self.compile_native_call("toString", &[v], &[ty])?;
                    self.basic_to_string_value(sv)
                }
            }
        }
    }

    /// Emit a call to `titrate_string_concat` and return the resulting
    /// `(len, ptr)` pair.
    pub(super) fn build_string_concat(
        &self,
        a: StringValue<'ctx>,
        b: StringValue<'ctx>,
    ) -> Result<StringValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());

        // Allocate space for the output length.
        let out_len_alloca = self
            .builder
            .build_alloca(i64_type, "concat.out_len")
            .map_err(|e| format!("build_alloca failed: {:?}", e))?;

        let concat_fn = self.get_function("titrate_string_concat");
        // Bitcast all pointers to i8* to match the function signature.
        let a_ptr = if a.ptr.get_type() == i8_ptr {
            a.ptr
        } else {
            self.builder
                .build_bit_cast(a.ptr, i8_ptr, "a.ptr.cast")
                .map_err(|e| format!("build_bit_cast a.ptr failed: {:?}", e))?
                .into_pointer_value()
        };
        let b_ptr = if b.ptr.get_type() == i8_ptr {
            b.ptr
        } else {
            self.builder
                .build_bit_cast(b.ptr, i8_ptr, "b.ptr.cast")
                .map_err(|e| format!("build_bit_cast b.ptr failed: {:?}", e))?
                .into_pointer_value()
        };
        // Bitcast out_len_alloca (i64*) to i8* to match the function signature.
        let out_len_ptr = self
            .builder
            .build_bit_cast(out_len_alloca, i8_ptr, "concat.out.len.ptr")
            .map_err(|e| format!("build_bit_cast concat.out_len failed: {:?}", e))?;
        let call_value = self
            .builder
            .build_call(
                concat_fn,
                &[
                    a.len.into(),
                    a_ptr.into(),
                    b.len.into(),
                    b_ptr.into(),
                    out_len_ptr.into(),
                ],
                "concat.result",
            )
            .map_err(|e| format!("build_call concat failed: {:?}", e))?;

        let result_ptr = match call_value.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
            inkwell::values::ValueKind::Instruction(_) => {
                return Err("titrate_string_concat did not return a value".to_string());
            }
        };

        let out_len = self
            .builder
            .build_load(i64_type, out_len_alloca, "concat.len")
            .map_err(|e| format!("build_load out_len failed: {:?}", e))?
            .into_int_value();

        // Cast the i8* result to the right pointer type if needed.
        let ptr = if result_ptr.get_type() == i8_ptr {
            result_ptr
        } else {
            self.builder
                .build_bit_cast(result_ptr, i8_ptr, "concat.ptr.cast")
                .map_err(|e| format!("build_bit_cast failed: {:?}", e))?
                .into_pointer_value()
        };

        Ok(StringValue { len: out_len, ptr })
    }

    /// Emit a call to `titrate_println(len, ptr)`.
    pub(super) fn build_println_string(&self, s: StringValue<'ctx>) -> Result<(), String> {
        let println_fn = self.get_function("titrate_println");
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        // Bitcast the data pointer to i8* to match the function signature.
        let ptr = if s.ptr.get_type() == i8_ptr {
            s.ptr
        } else {
            self.builder
                .build_bit_cast(s.ptr, i8_ptr, "println.ptr")
                .map_err(|e| format!("build_bit_cast println.ptr failed: {:?}", e))?
                .into_pointer_value()
        };
        self.builder
            .build_call(println_fn, &[s.len.into(), ptr.into()], "println")
            .map_err(|e| format!("build_call println failed: {:?}", e))?;
        Ok(())
    }

    /// Emit a call to the appropriate println helper for a primitive value.
    pub(super) fn build_println_primitive(&self, v: BasicValueEnum<'ctx>, ty: &Type) -> Result<(), String> {
        let name = ty.name();
        // A `{ i64, ptr }` struct is a string; print it even if type
        // inference called it an int (e.g. a mis-inferred native result).
        if v.is_struct_value() {
            let sv = v.into_struct_value();
            let len_val = self
                .builder
                .build_extract_value(sv, 0, "str.len")
                .map_err(|e| format!("extract str.len failed: {:?}", e))?;
            let ptr_val = self
                .builder
                .build_extract_value(sv, 1, "str.ptr")
                .map_err(|e| format!("extract str.ptr failed: {:?}", e))?;
            let s = StringValue {
                len: len_val.into_int_value(),
                ptr: ptr_val.into_pointer_value(),
            };
            return self.build_println_string(s);
        }
        // Handle "unknown" type by checking the LLVM value type.
        if name == "unknown" || name == "any" || name == "void" {
            if v.is_int_value() {
                // A call result whose Titrate type was not inferred. Print
                // from the actual LLVM value type instead of the toString
                // bridge, which mis-marshals numeric values (the runtime
                // would read the payload as a string and emit garbage).
                let int_val = v.into_int_value();
                if int_val.get_type().get_bit_width() == 1 {
                    let i32_type = self.context.i32_type();
                    let vv = self
                        .builder
                        .build_int_z_extend(int_val, i32_type, "zext")
                        .map_err(|e| format!("build_int_z_extend bool failed: {:?}", e))?;
                    let f = self.get_function("titrate_println_bool");
                    self.builder
                        .build_call(f, &[vv.into()], "println.b")
                        .map_err(|e| format!("build_call println_bool failed: {:?}", e))?;
                } else {
                    let i64_type = self.context.i64_type();
                    let vv = self
                        .builder
                        .build_int_z_extend(int_val, i64_type, "zext")
                        .map_err(|e| format!("build_int_z_extend println failed: {:?}", e))?;
                    let f = self.get_function("titrate_println_int");
                    self.builder
                        .build_call(f, &[vv.into()], "println.i")
                        .map_err(|e| format!("build_call println_int failed: {:?}", e))?;
                }
                return Ok(());
            }
            if v.is_float_value() {
                let f64_type = self.context.f64_type();
                let vv = self
                    .builder
                    .build_float_ext(v.into_float_value(), f64_type, "ext")
                    .map_err(|e| format!("build_float_ext println failed: {:?}", e))?;
                let f = self.get_function("titrate_println_double");
                self.builder
                    .build_call(f, &[vv.into()], "println.d")
                    .map_err(|e| format!("build_call println_double failed: {:?}", e))?;
                return Ok(());
            }
            // Fall back to the toString native bridge for other value kinds.
            let sv = self.compile_native_call("toString", &[v], std::slice::from_ref(ty))?;
            let sv_val = sv.into_struct_value();
            let len_val = self
                .builder
                .build_extract_value(sv_val, 0, "str.len")
                .map_err(|e| format!("extract str.len failed: {:?}", e))?;
            let ptr_val = self
                .builder
                .build_extract_value(sv_val, 1, "str.ptr")
                .map_err(|e| format!("extract str.ptr failed: {:?}", e))?;
            let s = StringValue {
                len: len_val.into_int_value(),
                ptr: ptr_val.into_pointer_value(),
            };
            return self.build_println_string(s);
        }
        match name {
            "float" | "double" | "half" | "quad" => {
                let f64_type = self.context.f64_type();
                let v = if v.get_type() == f64_type.into() {
                    v.into_float_value()
                } else {
                    self.builder
                        .build_float_ext(v.into_float_value(), f64_type, "ext")
                        .map_err(|e| format!("build_float_ext failed: {:?}", e))?
                };
                let f = self.get_function("titrate_println_double");
                self.builder
                    .build_call(f, &[v.into()], "println.d")
                    .map_err(|e| format!("build_call println_double failed: {:?}", e))?;
            }
            "bool" => {
                let i32_type = self.context.i32_type();
                let v = self
                    .builder
                    .build_int_z_extend(v.into_int_value(), i32_type, "zext")
                    .map_err(|e| format!("build_int_z_extend failed: {:?}", e))?;
                let f = self.get_function("titrate_println_bool");
                self.builder
                    .build_call(f, &[v.into()], "println.b")
                    .map_err(|e| format!("build_call println_bool failed: {:?}", e))?;
            }
            "char" => {
                let f = self.get_function("titrate_println_char");
                self.builder
                    .build_call(f, &[v.into()], "println.c")
                    .map_err(|e| format!("build_call println_char failed: {:?}", e))?;
            }
            // All other integer types: extend to i64 and call titrate_println_int.
            _ if llvm_types::is_integer(ty) => {
                let i64_type = self.context.i64_type();
                let v = if v.get_type() == i64_type.into() {
                    v.into_int_value()
                } else if llvm_types::integer_bit_width(ty) < Some(64) {
                    self.builder
                        .build_int_s_extend(v.into_int_value(), i64_type, "sext")
                        .map_err(|e| format!("build_int_s_extend failed: {:?}", e))?
                } else {
                    v.into_int_value()
                };
                let f = self.get_function("titrate_println_int");
                self.builder
                    .build_call(f, &[v.into()], "println.i")
                    .map_err(|e| format!("build_call println_int failed: {:?}", e))?;
            }
            _ => {
                return Err(format!("codegen: cannot println value of type {}", ty));
            }
        }
        Ok(())
    }

    /// Emit a call to `titrate_print(len, ptr)`.
    pub(super) fn build_print_string(&self, s: StringValue<'ctx>) -> Result<(), String> {
        let print_fn = self.get_function("titrate_print");
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let ptr = if s.ptr.get_type() == i8_ptr {
            s.ptr
        } else {
            self.builder
                .build_bit_cast(s.ptr, i8_ptr, "print.ptr")
                .map_err(|e| format!("build_bit_cast print.ptr failed: {:?}", e))?
                .into_pointer_value()
        };
        self.builder
            .build_call(print_fn, &[s.len.into(), ptr.into()], "print")
            .map_err(|e| format!("build_call print failed: {:?}", e))?;
        Ok(())
    }

    /// Emit a call to the appropriate print helper for a primitive value.
    pub(super) fn build_print_primitive(&self, v: BasicValueEnum<'ctx>, ty: &Type) -> Result<(), String> {
        let name = ty.name();
        // Handle "unknown" type by checking the LLVM value type.
        if name == "unknown" || name == "any" || name == "void" {
            if v.is_struct_value() {
                let sv = v.into_struct_value();
                let len_val = self
                    .builder
                    .build_extract_value(sv, 0, "str.len")
                    .map_err(|e| format!("extract str.len failed: {:?}", e))?;
                let ptr_val = self
                    .builder
                    .build_extract_value(sv, 1, "str.ptr")
                    .map_err(|e| format!("extract str.ptr failed: {:?}", e))?;
                let s = StringValue {
                    len: len_val.into_int_value(),
                    ptr: ptr_val.into_pointer_value(),
                };
                return self.build_print_string(s);
            }
            if v.is_int_value() {
                // A call result whose Titrate type was not inferred. Print
                // from the actual LLVM value type instead of the toString
                // bridge, which mis-marshals numeric values.
                let int_val = v.into_int_value();
                if int_val.get_type().get_bit_width() == 1 {
                    let i32_type = self.context.i32_type();
                    let vv = self
                        .builder
                        .build_int_z_extend(int_val, i32_type, "zext")
                        .map_err(|e| format!("build_int_z_extend bool failed: {:?}", e))?;
                    let f = self.get_function("titrate_print_bool");
                    self.builder
                        .build_call(f, &[vv.into()], "print.b")
                        .map_err(|e| format!("build_call print_bool failed: {:?}", e))?;
                } else {
                    let i64_type = self.context.i64_type();
                    let vv = self
                        .builder
                        .build_int_z_extend(int_val, i64_type, "zext")
                        .map_err(|e| format!("build_int_z_extend print failed: {:?}", e))?;
                    let f = self.get_function("titrate_print_int");
                    self.builder
                        .build_call(f, &[vv.into()], "print.i")
                        .map_err(|e| format!("build_call print_int failed: {:?}", e))?;
                }
                return Ok(());
            }
            if v.is_float_value() {
                let f64_type = self.context.f64_type();
                let vv = self
                    .builder
                    .build_float_ext(v.into_float_value(), f64_type, "ext")
                    .map_err(|e| format!("build_float_ext print failed: {:?}", e))?;
                let f = self.get_function("titrate_print_double");
                self.builder
                    .build_call(f, &[vv.into()], "print.d")
                    .map_err(|e| format!("build_call print_double failed: {:?}", e))?;
                return Ok(());
            }
            let sv = self.compile_native_call("toString", &[v], std::slice::from_ref(ty))?;
            let sv_val = sv.into_struct_value();
            let len_val = self
                .builder
                .build_extract_value(sv_val, 0, "str.len")
                .map_err(|e| format!("extract str.len failed: {:?}", e))?;
            let ptr_val = self
                .builder
                .build_extract_value(sv_val, 1, "str.ptr")
                .map_err(|e| format!("extract str.ptr failed: {:?}", e))?;
            let s = StringValue {
                len: len_val.into_int_value(),
                ptr: ptr_val.into_pointer_value(),
            };
            return self.build_print_string(s);
        }
        match name {
            "float" | "double" | "half" | "quad" => {
                let f64_type = self.context.f64_type();
                let v = if v.get_type() == f64_type.into() {
                    v.into_float_value()
                } else {
                    self.builder
                        .build_float_ext(v.into_float_value(), f64_type, "ext")
                        .map_err(|e| format!("build_float_ext failed: {:?}", e))?
                };
                let f = self.get_function("titrate_print_double");
                self.builder
                    .build_call(f, &[v.into()], "print.d")
                    .map_err(|e| format!("build_call print_double failed: {:?}", e))?;
            }
            "bool" => {
                let i32_type = self.context.i32_type();
                let v = self
                    .builder
                    .build_int_z_extend(v.into_int_value(), i32_type, "zext")
                    .map_err(|e| format!("build_int_z_extend failed: {:?}", e))?;
                let f = self.get_function("titrate_print_bool");
                self.builder
                    .build_call(f, &[v.into()], "print.b")
                    .map_err(|e| format!("build_call print_bool failed: {:?}", e))?;
            }
            "char" => {
                let f = self.get_function("titrate_print_char");
                self.builder
                    .build_call(f, &[v.into()], "print.c")
                    .map_err(|e| format!("build_call print_char failed: {:?}", e))?;
            }
            _ if llvm_types::is_integer(ty) => {
                let i64_type = self.context.i64_type();
                let v = if v.get_type() == i64_type.into() {
                    v.into_int_value()
                } else if llvm_types::integer_bit_width(ty) < Some(64) {
                    self.builder
                        .build_int_s_extend(v.into_int_value(), i64_type, "sext")
                        .map_err(|e| format!("build_int_s_extend failed: {:?}", e))?
                } else {
                    v.into_int_value()
                };
                let f = self.get_function("titrate_print_int");
                self.builder
                    .build_call(f, &[v.into()], "print.i")
                    .map_err(|e| format!("build_call print_int failed: {:?}", e))?;
            }
            _ => {
                return Err(format!("codegen: cannot print value of type {}", ty));
            }
        }
        Ok(())
    }
}
