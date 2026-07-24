//! Type mapping from Titrate AST types to LLVM types.
//!
//! This module implements the mapping described in Section 3.2 of the
//! Titrate LLVM backend specification. The central entry point is
//! [`llvm_type`], which converts an [`crate::ast::Type`] into the
//! corresponding inkwell `BasicTypeEnum`.
//!
//! Mapping summary:
//!
//! | Titrate       | LLVM                       |
//! |---------------|----------------------------|
//! | `void`        | `void` (handled specially) |
//! | `bool`        | `i1`                       |
//! | `byte`        | `i8`                       |
//! | `short`       | `i16`                      |
//! | `int`         | `i32`                      |
//! | `long`        | `i64`                      |
//! | `vast`        | `i128`                     |
//! | `uvast`       | `i128` (unsigned semantic) |
//! | `float`       | `f32`                      |
//! | `double`      | `f64`                      |
//! | `half`        | `f16` (half)               |
//! | `quad`        | `fp128`                    |
//! | `char`        | `i32`                      |
//! | `string`      | `{ i64, i8* }`             |
//! | `size`        | `i64`                      |
//! | `u8`/`u16`/...| `i8`/`i16`/...             |
//! | `Owned<T>`    | `T*`                       |
//! | `&T`/`&mut T` | `T*`                       |
//! | `array<T>`    | `{ i64, T* }`              |
//! | `Result<T,E>` | `{ i32, i8* }`             |
//! | class         | `{ vtable*, fields... }`*  |
//! | interface     | `{ object*, vtable* }`*    |
//! | enum          | `{ i32 tag, [payload] }`*  |
//! | tuple         | anonymous struct           |
//!
//! Types marked `*` are not yet fully implemented; for now heap-allocated
//! user types (classes, interfaces, enums, and generic containers such as
//! `ArrayList<T>` / `HashMap<K,V>`) are represented as opaque `i8*`
//! pointers, which is sufficient for Phase 1 because we only emit calls
//! into the runtime and never touch their fields directly.

use inkwell::context::Context;
use inkwell::types::BasicTypeEnum;
use inkwell::AddressSpace;

use crate::ast::Type;

/// LLVM representation of a Titrate `string`: a struct `{ i64, i8* }`
/// holding the byte length and a pointer to a UTF-8 buffer.
pub const STRING_TYPE_NAME: &str = "string";

/// Return the LLVM struct type used to represent Titrate strings:
/// `{ i64, i8* }`.
pub fn string_type<'ctx>(context: &'ctx Context) -> BasicTypeEnum<'ctx> {
    let i64_type = context.i64_type();
    let i8_ptr = context.ptr_type(AddressSpace::default());
    context
        .struct_type(&[i64_type.into(), i8_ptr.into()], false)
        .into()
}

/// Return the LLVM type used to represent Titrate `array<T>`:
/// `{ i64, T* }` (length + elements pointer).
pub fn array_type<'ctx>(
    context: &'ctx Context,
    element_ty: BasicTypeEnum<'ctx>,
) -> BasicTypeEnum<'ctx> {
    let i64_type = context.i64_type();
    let elem_ptr = context.ptr_type(AddressSpace::default());
    let _ = element_ty; // element type is informational only; we use opaque ptr
    context
        .struct_type(&[i64_type.into(), elem_ptr.into()], false)
        .into()
}

/// Return the LLVM struct type used to represent Titrate `Result<T, E>`:
/// `{ i32, i8* }` where:
///   - field 0: tag (0 = Ok, 1 = Err)
///   - field 1: heap-allocated payload pointer
pub fn result_type<'ctx>(context: &'ctx Context) -> BasicTypeEnum<'ctx> {
    let i32_type = context.i32_type();
    let i8_ptr = context.ptr_type(AddressSpace::default());
    context
        .struct_type(&[i32_type.into(), i8_ptr.into()], false)
        .into()
}

/// Return true if the given Titrate type is `Result<T, E>`.
pub fn is_result(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if name == "Result")
}

/// Return the LLVM type used to represent Titrate `Owned<T>`, `&T`, and
/// `&mut T`: a pointer to the inner type.
pub fn pointer_to<'ctx>(
    _ty: BasicTypeEnum<'ctx>,
    context: &'ctx Context,
) -> BasicTypeEnum<'ctx> {
    // In LLVM 15+, all pointers are opaque `ptr`, so we ignore the inner
    // type and just return an opaque pointer.
    context.ptr_type(AddressSpace::default()).into()
}

/// Convert a Titrate AST type into the corresponding LLVM `BasicTypeEnum`.
///
/// For `void`, use [`llvm_type_or_void`] instead, since `void` is not a
/// `BasicType`. This function returns an error for `void`.
pub fn llvm_type<'ctx>(
    context: &'ctx Context,
    ty: &Type,
) -> Result<BasicTypeEnum<'ctx>, String> {
    match ty {
        Type::Ref(inner) | Type::MutRef(inner) => {
            // References are pointers to the inner type. For convenience we
            // always lower to `i8*` when the inner type is not a primitive
            // (so that `&ArrayList<string>` etc. compile), and to `T*`
            // otherwise.
            let inner = llvm_type(context, inner).unwrap_or_else(|_| {
                context.ptr_type(AddressSpace::default()).into()
            });
            Ok(pointer_to(inner, context))
        }
        Type::Tuple(elements) => {
            let mut fields = Vec::with_capacity(elements.len());
            for e in elements {
                fields.push(llvm_type(context, e)?);
            }
            Ok(context
                .struct_type(&fields, false)
                .into())
        }
        Type::Named { name, params } => llvm_named_type(context, name, params),
    }
}

/// Like [`llvm_type`] but returns `None` for `void` instead of erroring.
/// Useful when a type might be void (e.g. function return types).
pub fn llvm_type_or_void<'ctx>(
    context: &'ctx Context,
    ty: Option<&Type>,
) -> Result<Option<BasicTypeEnum<'ctx>>, String> {
    match ty {
        None => Ok(None),
        Some(t) if is_void(t) => Ok(None),
        Some(t) => llvm_type(context, t).map(Some),
    }
}

/// Return true if the given Titrate type is `void`.
pub fn is_void(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if name == "void")
}

/// Return true if the given Titrate type is `string`.
pub fn is_string(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if name == STRING_TYPE_NAME)
}

/// Return true if the given Titrate type is `bool`.
pub fn is_bool(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if name == "bool")
}

/// Return true if the type is one of the Titrate integer types
/// (`byte`, `short`, `int`, `long`, `vast`, `uvast`, `size`, `u8`..`u64`).
pub fn is_integer(ty: &Type) -> bool {
    INTEGER_TYPES.contains(&ty.name())
}

/// Return true if the type is one of the Titrate float types
/// (`float`, `double`, `half`, `quad`).
pub fn is_float(ty: &Type) -> bool {
    FLOAT_TYPES.contains(&ty.name())
}

/// Return true if the type is numeric (integer or float).
pub fn is_numeric(ty: &Type) -> bool {
    is_integer(ty) || is_float(ty)
}

/// Return true if the type is `char`.
pub fn is_char(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if name == "char")
}

/// Return true if the type is `Owned<T>`.
pub fn is_owned(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if name == "Owned")
}

/// Return true if the type is a reference (`&T` or `&mut T`).
pub fn is_ref(ty: &Type) -> bool {
    matches!(ty, Type::Ref(_) | Type::MutRef(_))
}

/// Return true if the type is a tuple.
pub fn is_tuple(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(_))
}

/// Return true if the type is `array<T>` (the built-in array type).
pub fn is_array(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if name == "array")
}

/// Return true if the type is a heap-allocated user type (class,
/// interface, enum) or a generic container that we currently treat as an
/// opaque pointer.
pub fn is_opaque_pointer_type(ty: &Type) -> bool {
    let name = ty.name();
    !is_primitive_name(name)
        && name != "void"
        && name != "Owned"
        && name != "Result"
        && !is_ref(ty)
        && !is_tuple(ty)
}

/// Return true if `name` corresponds to a primitive Titrate type that has
/// a direct LLVM representation (not a pointer).
pub fn is_primitive_name(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "byte"
            | "short"
            | "int"
            | "long"
            | "vast"
            | "uvast"
            | "float"
            | "double"
            | "half"
            | "quad"
            | "char"
            | "string"
            | "size"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
    )
}

/// Return the bit width of an integer type, or `None` for non-integer types.
pub fn integer_bit_width(ty: &Type) -> Option<u32> {
    let name = ty.name();
    match name {
        "bool" => Some(1),
        "byte" | "u8" => Some(8),
        "short" | "u16" => Some(16),
        "int" | "u32" | "char" => Some(32),
        "long" | "u64" | "size" => Some(64),
        "vast" | "uvast" => Some(128),
        _ => None,
    }
}

/// List of Titrate integer type names.
pub const INTEGER_TYPES: &[&str] = &[
    "byte", "short", "int", "long", "vast", "uvast", "u8", "u16", "u32", "u64", "size",
];

/// List of Titrate float type names.
pub const FLOAT_TYPES: &[&str] = &["float", "double", "half", "quad"];

/// Internal helper: map a named Titrate type to an LLVM type.
fn llvm_named_type<'ctx>(
    context: &'ctx Context,
    name: &str,
    params: &[Type],
) -> Result<BasicTypeEnum<'ctx>, String> {
    match name {
        "void" => Err("void is not a BasicType; use llvm_type_or_void".to_string()),
        "bool" => Ok(context.bool_type().into()),
        "byte" | "u8" => Ok(context.i8_type().into()),
        "short" | "u16" => Ok(context.i16_type().into()),
        "int" | "u32" | "char" => Ok(context.i32_type().into()),
        "long" | "u64" | "size" => Ok(context.i64_type().into()),
        "vast" | "uvast" => Ok(context.i128_type().into()),
        "float" => Ok(context.f32_type().into()),
        "double" => Ok(context.f64_type().into()),
        "half" => Ok(context.f16_type().into()),
        "quad" => Ok(context.f128_type().into()),
        "string" => Ok(string_type(context)),
        "array" => {
            let elem_ty = params
                .first()
                .ok_or_else(|| "array<T> requires a type parameter".to_string())?;
            let elem_llvm = llvm_type(context, elem_ty)?;
            Ok(array_type(context, elem_llvm))
        }
        "Owned" => {
            let inner = params
                .first()
                .ok_or_else(|| "Owned<T> requires a type parameter".to_string())?;
            let inner_llvm = llvm_type(context, inner)
                .unwrap_or_else(|_| context.ptr_type(AddressSpace::default()).into());
            Ok(pointer_to(inner_llvm, context))
        }
        "Result" => {
            // Result<T, E> is lowered to { i32, i8* } where:
            //   tag: 0 = Ok, 1 = Err
            //   payload: heap-allocated pointer to the Ok or Err value
            let _ = params; // type params are informational only for Phase 1
            Ok(result_type(context))
        }
        // Heap-allocated user types (classes, interfaces, enums) and
        // generic containers like HashMap<K,V>,
        // Optional<T> are represented as opaque `i8*` pointers for Phase 1.
        // ArrayList<T> and array<T> are represented as {i64, ptr} (TitrateArray).
        "ArrayList" => {
            let i64_ty = context.i64_type();
            let ptr_ty = context.ptr_type(AddressSpace::default());
            Ok(context.struct_type(&[i64_ty.into(), ptr_ty.into()], false).into())
        }
        _ => Ok(context.ptr_type(AddressSpace::default()).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Type;

    fn llvm_type_str(ty: &Type) -> String {
        let context = Context::create();
        let llvm = llvm_type(&context, ty).expect("type should map");
        llvm.print_to_string().to_string()
    }

    #[test]
    fn void_errors() {
        let context = Context::create();
        let ty = Type::simple("void");
        assert!(llvm_type(&context, &ty).is_err());
    }

    #[test]
    fn void_or_none() {
        let context = Context::create();
        let ty = Type::simple("void");
        assert_eq!(llvm_type_or_void(&context, Some(&ty)).unwrap(), None);
        assert_eq!(llvm_type_or_void(&context, None).unwrap(), None);
    }

    #[test]
    fn bool_maps_to_i1() {
        let s = llvm_type_str(&Type::simple("bool"));
        assert_eq!(s, "i1");
    }

    #[test]
    fn byte_and_u8_map_to_i8() {
        assert_eq!(llvm_type_str(&Type::simple("byte")), "i8");
        assert_eq!(llvm_type_str(&Type::simple("u8")), "i8");
    }

    #[test]
    fn short_and_u16_map_to_i16() {
        assert_eq!(llvm_type_str(&Type::simple("short")), "i16");
        assert_eq!(llvm_type_str(&Type::simple("u16")), "i16");
    }

    #[test]
    fn int_u32_char_map_to_i32() {
        assert_eq!(llvm_type_str(&Type::simple("int")), "i32");
        assert_eq!(llvm_type_str(&Type::simple("u32")), "i32");
        assert_eq!(llvm_type_str(&Type::simple("char")), "i32");
    }

    #[test]
    fn long_u64_size_map_to_i64() {
        assert_eq!(llvm_type_str(&Type::simple("long")), "i64");
        assert_eq!(llvm_type_str(&Type::simple("u64")), "i64");
        assert_eq!(llvm_type_str(&Type::simple("size")), "i64");
    }

    #[test]
    fn vast_and_uvast_map_to_i128() {
        assert_eq!(llvm_type_str(&Type::simple("vast")), "i128");
        assert_eq!(llvm_type_str(&Type::simple("uvast")), "i128");
    }

    #[test]
    fn float_maps_to_f32() {
        assert_eq!(llvm_type_str(&Type::simple("float")), "float");
    }

    #[test]
    fn double_maps_to_f64() {
        assert_eq!(llvm_type_str(&Type::simple("double")), "double");
    }

    #[test]
    fn half_maps_to_half() {
        assert_eq!(llvm_type_str(&Type::simple("half")), "half");
    }

    #[test]
    fn quad_maps_to_fp128() {
        assert_eq!(llvm_type_str(&Type::simple("quad")), "fp128");
    }

    #[test]
    fn string_maps_to_struct() {
        let s = llvm_type_str(&Type::simple("string"));
        assert!(s.contains("i64") && s.contains("ptr"), "got: {}", s);
    }

    #[test]
    fn array_maps_to_struct_with_len_and_ptr() {
        let ty = Type::generic("array", vec![Type::simple("int")]);
        let s = llvm_type_str(&ty);
        assert!(s.contains("i64"), "got: {}", s);
        assert!(s.contains("ptr"), "got: {}", s);
    }

    #[test]
    fn owned_maps_to_pointer() {
        let ty = Type::generic("Owned", vec![Type::simple("int")]);
        let s = llvm_type_str(&ty);
        assert!(s.starts_with("ptr"), "got: {}", s);
    }

    #[test]
    fn result_maps_to_struct() {
        let ty = Type::generic("Result", vec![Type::simple("int"), Type::simple("string")]);
        let s = llvm_type_str(&ty);
        assert!(s.contains("i32") && s.contains("ptr"), "got: {}", s);
    }

    #[test]
    fn ref_maps_to_pointer() {
        let ty = Type::Ref(Box::new(Type::simple("int")));
        let s = llvm_type_str(&ty);
        assert!(s.starts_with("ptr"), "got: {}", s);
    }

    #[test]
    fn mut_ref_maps_to_pointer() {
        let ty = Type::MutRef(Box::new(Type::simple("double")));
        let s = llvm_type_str(&ty);
        assert!(s.starts_with("ptr"), "got: {}", s);
    }

    #[test]
    fn tuple_maps_to_anon_struct() {
        let ty = Type::Tuple(vec![Type::simple("int"), Type::simple("double")]);
        let s = llvm_type_str(&ty);
        assert!(s.contains("i32"), "got: {}", s);
        assert!(s.contains("double"), "got: {}", s);
    }

    #[test]
    fn generic_container_is_opaque_pointer() {
        let ty = Type::generic("ArrayList", vec![Type::simple("string")]);
        let s = llvm_type_str(&ty);
        assert!(s.starts_with("ptr"), "got: {}", s);
    }

    #[test]
    fn user_class_is_opaque_pointer() {
        let ty = Type::simple("MyClass");
        let s = llvm_type_str(&ty);
        assert!(s.starts_with("ptr"), "got: {}", s);
    }

    #[test]
    fn integer_bit_widths() {
        assert_eq!(integer_bit_width(&Type::simple("bool")), Some(1));
        assert_eq!(integer_bit_width(&Type::simple("byte")), Some(8));
        assert_eq!(integer_bit_width(&Type::simple("short")), Some(16));
        assert_eq!(integer_bit_width(&Type::simple("int")), Some(32));
        assert_eq!(integer_bit_width(&Type::simple("long")), Some(64));
        assert_eq!(integer_bit_width(&Type::simple("vast")), Some(128));
        assert_eq!(integer_bit_width(&Type::simple("double")), None);
    }

    #[test]
    fn predicates_work() {
        assert!(is_void(&Type::simple("void")));
        assert!(is_string(&Type::simple("string")));
        assert!(is_bool(&Type::simple("bool")));
        assert!(is_integer(&Type::simple("int")));
        assert!(is_integer(&Type::simple("u8")));
        assert!(is_float(&Type::simple("double")));
        assert!(is_char(&Type::simple("char")));
        assert!(is_owned(&Type::generic("Owned", vec![Type::simple("int")])));
        assert!(is_ref(&Type::Ref(Box::new(Type::simple("int")))));
        assert!(is_ref(&Type::MutRef(Box::new(Type::simple("int")))));
        assert!(is_tuple(&Type::Tuple(vec![])));
        assert!(is_array(&Type::generic("array", vec![Type::simple("int")])));
        assert!(is_result(&Type::generic("Result", vec![Type::simple("int"), Type::simple("string")])));
        assert!(is_opaque_pointer_type(&Type::simple("MyClass")));
        assert!(!is_opaque_pointer_type(&Type::simple("int")));
        assert!(!is_opaque_pointer_type(&Type::generic("Result", vec![Type::simple("int"), Type::simple("string")])));
    }
}