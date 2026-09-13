// Array builtins

use super::types as llvm_types;
use super::LlvmBackend;
use inkwell::module::Linkage;
use crate::ast::{ClassMember, Expr, Type};
use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::BasicValueEnum;

impl<'ctx> LlvmBackend<'ctx> {
    /// Compile `array.length()` - extracts the i64 length from a TitrateArray struct and truncates to i32.
    pub(super) fn compile_array_length(&mut self, array_expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let arr_val = self.compile_expr(array_expr)?;
        if arr_val.is_struct_value() {
            let sv = arr_val.into_struct_value();
            let len_val = self
                .builder
                .build_extract_value(sv, 0, "arr.len")
                .map_err(|e| format!("extract arr.len failed: {:?}", e))?;
            let i32_val = self
                .builder
                .build_int_truncate(
                    len_val.into_int_value(),
                    self.context.i32_type(),
                    "arr.len.i32",
                )
                .map_err(|e| format!("int_truncate failed: {:?}", e))?;
            return Ok(i32_val.into());
        }
        Err("codegen: array_length called on non-array value".to_string())
    }

    /// Determine the element type name of an array expression (e.g. the `T`
    /// in `ArrayList<T>`), for locals and `obj.field` accesses.
    pub(super) fn array_element_type_name(&self, array_expr: &Expr) -> Option<String> {
        match array_expr {
            Expr::Identifier(name, _) => self
                .locals
                .get(name)
                .and_then(|v| v.full_type.as_ref())
                .and_then(|t| match t {
                    Type::Named { name: _, params } if params.len() == 1 => {
                        Some(params[0].name().to_string())
                    }
                    _ => None,
                }),
            Expr::MemberAccess(obj, field, _) => {
                let obj_ty = self.infer_expr_type(obj);
                let class_name = obj_ty.name();
                let field_ty = self.class_decls.get(class_name).and_then(|cd| {
                    cd.members.iter().find_map(|m| match m {
                        ClassMember::Field(f) if f.name == *field => Some(f.typ.clone()),
                        _ => None,
                    })
                });
                match field_ty {
                    Some(Type::Named { name: _, params }) if params.len() == 1 => {
                        Some(params[0].name().to_string())
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Compile `array.get(index)` - reads an element from a TitrateArray
    /// by calling the appropriate titrate_array_get_* function based on element type.
    pub(super) fn compile_array_get_string(
        &mut self,
        array_expr: &Expr,
        index_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Determine the element type from the array variable's full_type.
        let element_type_name = self.array_element_type_name(array_expr);

        // Choose the right native function based on element type.
        let (fn_name, return_is_struct): (&str, bool) = match element_type_name.as_deref() {
            Some("int") | Some("u32") | Some("byte") | Some("short") | Some("u8") | Some("u16") => {
                ("titrate_array_get_int", false)
            }
            Some("long") | Some("u64") | Some("size") => ("titrate_array_get_long", false),
            Some("double") | Some("float") | Some("half") | Some("quad") => {
                ("titrate_array_get_double", false)
            }
            _ => ("titrate_array_get_string", true),
        };
        let arr_val = self.compile_expr(array_expr)?;
        let idx_val = self.compile_expr(index_expr)?;

        if !arr_val.is_struct_value() {
            return Err("codegen: array_get_string called on non-array value".to_string());
        }

        let sv = arr_val.into_struct_value();
        let len_val = self
            .builder
            .build_extract_value(sv, 0, "arr.len")
            .map_err(|e| format!("extract arr.len failed: {:?}", e))?;
        let data_ptr = self
            .builder
            .build_extract_value(sv, 1, "arr.data")
            .map_err(|e| format!("extract arr.data failed: {:?}", e))?;

        let idx_i64 = if idx_val.is_int_value() {
            let idx_int = idx_val.into_int_value();
            if idx_int.get_type().get_bit_width() < 64 {
                self.builder
                    .build_int_z_extend(idx_int, self.context.i64_type(), "idx.ext")
                    .map_err(|e| format!("int_z_extend failed: {:?}", e))?
            } else {
                idx_int
            }
        } else {
            return Err("ArrayList.get: index must be integer".to_string());
        };

        let i64_ty = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let arr_c_abi_ty = self
            .context
            .struct_type(&[i64_ty.into(), i8_ptr.into()], false);

        let arr_struct_val = self
            .builder
            .build_insert_value(
                self.builder
                    .build_insert_value(
                        arr_c_abi_ty.const_zero(),
                        len_val.into_int_value(),
                        0,
                        "arr.len",
                    )
                    .map_err(|e| format!("insert_value failed: {:?}", e))?,
                data_ptr.into_pointer_value(),
                1,
                "arr.data",
            )
            .map_err(|e| format!("insert_value failed: {:?}", e))?;

        let arr_struct = match arr_struct_val {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv,
            _ => return Err("expected struct value from insert_value".to_string()),
        };

        if return_is_struct {
            // titrate_array_get_string(arr, index, out): the string comes back
            // through `out` (struct returns disagree across the FFI).
            let string_ty = llvm_types::string_type(self.context).into_struct_type();
            let void_ty = self.context.void_type();
            let fn_type =
                void_ty.fn_type(&[i8_ptr.into(), i64_ty.into(), i8_ptr.into()], false);
            let fn_val = self.module.get_function(fn_name).unwrap_or_else(|| {
                self.module
                    .add_function(fn_name, fn_type, Some(Linkage::External))
            });
            let arr_ptr = self.store_array_arg_to_ptr(arr_struct)?;
            let out_slot = self
                .builder
                .build_alloca(string_ty, "arr.get.out")
                .map_err(|e| format!("build_alloca arr.get.out failed: {:?}", e))?;
            self.builder
                .build_call(
                    fn_val,
                    &[arr_ptr.into(), idx_i64.into(), out_slot.into()],
                    "array_get_string",
                )
                .map_err(|e| format!("build_call {} failed: {:?}", fn_name, e))?;
            let result_val = self
                .builder
                .build_load(string_ty, out_slot, "arr.get.val")
                .map_err(|e| format!("build_load arr.get.val failed: {:?}", e))?;
            if result_val.get_type() == string_ty.into() {
                Ok(result_val)
            } else {
                self.builder
                    .build_bit_cast(result_val, string_ty, "arr.get.cast")
                    .map_err(|e| format!("bit_cast failed: {:?}", e))
            }
        } else {
            // titrate_array_get_int/long/double(*const TitrateArray, i64) -> scalar
            let ret_ty: BasicTypeEnum<'ctx> = if fn_name == "titrate_array_get_double" {
                self.context.f64_type().into()
            } else {
                i64_ty.into()
            };
            let fn_type = ret_ty.fn_type(&[i8_ptr.into(), i64_ty.into()], false);
            let fn_val = self.module.get_function(fn_name).unwrap_or_else(|| {
                self.module
                    .add_function(fn_name, fn_type, Some(Linkage::External))
            });
            let arr_ptr = self.store_array_arg_to_ptr(arr_struct)?;
            let result = self
                .builder
                .build_call(
                    fn_val,
                    &[arr_ptr.into(), idx_i64.into()],
                    "array_get_val",
                )
                .map_err(|e| format!("build_call {} failed: {:?}", fn_name, e))?;
            match result.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(v) => Ok(v),
                _ => Err("array_get did not return a value".to_string()),
            }
        }
    }

    /// Store a `{ i64, ptr }` TitrateArray struct into a stack slot and return
    /// its address. The runtime array accessors take the array by pointer to
    /// avoid the Win64 by-value struct-passing ABI ambiguity.
    pub(super) fn store_array_arg_to_ptr(
        &mut self,
        arr_struct: inkwell::values::StructValue<'ctx>,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let i8_ptr = self.context.ptr_type(AddressSpace::default());
        let arr_ty = self
            .context
            .struct_type(&[i64_ty.into(), i8_ptr.into()], false);
        let slot = self
            .builder
            .build_alloca(arr_ty, "arr.slot")
            .map_err(|e| format!("build_alloca arr.slot failed: {:?}", e))?;
        self.builder
            .build_store(slot, arr_struct)
            .map_err(|e| format!("build_store arr.slot failed: {:?}", e))?;
        Ok(slot)
    }
}
