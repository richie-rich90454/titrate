//! Advanced numeric lowering helpers for the WASM backend.
//!
//! The base TIR lowerer covers the primitive arithmetic and bitwise operators
//! plus the `math_*` host imports. This module fills the remaining numeric
//! surface with two families of operations that need no new host import:
//!
//! - [`NativeNumericOp`] maps a method name to a single native WebAssembly
//!   instruction (count-leading-zero, population count, rotations, fused
//!   rounding variants, `min`/`max`, `copysign`, ...).
//! - [`DerivedNumericOp`] maps a method name to a short expression built from
//!   the existing `math_exp` / `math_ln` imports (`sinh`, `cosh`, `tanh`,
//!   `log1p`, `expm1`, `sign`).
//!
//! Both emitters write into a caller-owned instruction buffer so they can be
//! used by any emission path without depending on private backend state.
//! Names outside these families return `None`; callers must report that as an
//! explicit unmapped-construct error instead of guessing an implementation.

use wasm_encoder::{BlockType, Instruction, Ieee64};

use super::runtime::import_index;

/// A numeric operation expressible as one native WebAssembly instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNumericOp {
    Clz,
    Ctz,
    PopCnt,
    Rotl,
    Rotr,
    Abs,
    Sqrt,
    Floor,
    Ceil,
    Trunc,
    Nearest,
    Min,
    Max,
    CopySign,
}

/// A numeric operation expressed with existing host imports and arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedNumericOp {
    Sinh,
    Cosh,
    Tanh,
    Log1p,
    Expm1,
    Sign,
}

/// The lowering chosen for a numeric method name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericPlan {
    Native(NativeNumericOp),
    Derived(DerivedNumericOp),
    /// A host import that already exists in the runtime contract.
    Host(&'static str),
}

fn method_of(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// Resolve a numeric method name. Unqualified and `Math`, `MathAdvanced`,
/// `Integer` and `Long` prefixes are all accepted; the method name is what
/// decides the plan.
pub fn plan(name: &str) -> Option<NumericPlan> {
    Some(match method_of(name) {
        "clz" | "leadingZeros" => NumericPlan::Native(NativeNumericOp::Clz),
        "ctz" | "trailingZeros" => NumericPlan::Native(NativeNumericOp::Ctz),
        "popcnt" | "bitCount" => NumericPlan::Native(NativeNumericOp::PopCnt),
        "rotl" => NumericPlan::Native(NativeNumericOp::Rotl),
        "rotr" => NumericPlan::Native(NativeNumericOp::Rotr),
        "abs" | "fabs" => NumericPlan::Native(NativeNumericOp::Abs),
        "sqrt" => NumericPlan::Native(NativeNumericOp::Sqrt),
        "floor" => NumericPlan::Native(NativeNumericOp::Floor),
        "ceil" => NumericPlan::Native(NativeNumericOp::Ceil),
        "trunc" => NumericPlan::Native(NativeNumericOp::Trunc),
        "nearest" => NumericPlan::Native(NativeNumericOp::Nearest),
        "min" => NumericPlan::Native(NativeNumericOp::Min),
        "max" => NumericPlan::Native(NativeNumericOp::Max),
        "copysign" => NumericPlan::Native(NativeNumericOp::CopySign),
        "sinh" => NumericPlan::Derived(DerivedNumericOp::Sinh),
        "cosh" => NumericPlan::Derived(DerivedNumericOp::Cosh),
        "tanh" => NumericPlan::Derived(DerivedNumericOp::Tanh),
        "log1p" => NumericPlan::Derived(DerivedNumericOp::Log1p),
        "expm1" => NumericPlan::Derived(DerivedNumericOp::Expm1),
        "sign" | "signum" => NumericPlan::Derived(DerivedNumericOp::Sign),
        "pow" => NumericPlan::Host("math_pow"),
        "hypot" => NumericPlan::Host("math_hypot"),
        "atan2" => NumericPlan::Host("math_atan2"),
        _ => return None,
    })
}

/// Emit a single native numeric instruction. Operands are already on the
/// stack in WebAssembly order; `rotl`/`rotr` and `min`/`max`/`copysign` take
/// two operands, the rest take one.
pub fn emit_native(op: NativeNumericOp, out: &mut Vec<Instruction<'static>>) {
    use Instruction::*;
    out.push(match op {
        NativeNumericOp::Clz => I64Clz,
        NativeNumericOp::Ctz => I64Ctz,
        NativeNumericOp::PopCnt => I64Popcnt,
        NativeNumericOp::Rotl => I64Rotl,
        NativeNumericOp::Rotr => I64Rotr,
        NativeNumericOp::Abs => F64Abs,
        NativeNumericOp::Sqrt => F64Sqrt,
        NativeNumericOp::Floor => F64Floor,
        NativeNumericOp::Ceil => F64Ceil,
        NativeNumericOp::Trunc => F64Trunc,
        NativeNumericOp::Nearest => F64Nearest,
        NativeNumericOp::Min => F64Min,
        NativeNumericOp::Max => F64Max,
        NativeNumericOp::CopySign => F64Copysign,
    });
}

fn f64_const(out: &mut Vec<Instruction<'static>>, value: f64) {
    out.push(Instruction::F64Const(Ieee64::new(value.to_bits())));
}

fn host_call(out: &mut Vec<Instruction<'static>>, name: &str) -> Result<(), String> {
    let index = import_index(name)
        .ok_or_else(|| format!("wasm numeric: missing host import '{}'", name))?;
    out.push(Instruction::Call(index));
    Ok(())
}

/// Emit a derived numeric operation. The single `f64` operand must be on the
/// stack; `scratch` is a writable `f64` local used to duplicate it where the
/// definition needs the operand twice.
pub fn emit_derived(
    op: DerivedNumericOp,
    out: &mut Vec<Instruction<'static>>,
    scratch: u32,
) -> Result<(), String> {
    use Instruction::*;
    match op {
        DerivedNumericOp::Log1p => {
            f64_const(out, 1.0);
            out.push(F64Add);
            host_call(out, "math_ln")?;
        }
        DerivedNumericOp::Expm1 => {
            host_call(out, "math_exp")?;
            f64_const(out, 1.0);
            out.push(F64Sub);
        }
        DerivedNumericOp::Sinh | DerivedNumericOp::Cosh => {
            out.push(LocalTee(scratch));
            host_call(out, "math_exp")?;
            out.push(LocalGet(scratch));
            out.push(F64Neg);
            host_call(out, "math_exp")?;
            out.push(if op == DerivedNumericOp::Sinh { F64Sub } else { F64Add });
            f64_const(out, 2.0);
            out.push(F64Div);
        }
        DerivedNumericOp::Tanh => {
            out.push(LocalTee(scratch));
            f64_const(out, 2.0);
            out.push(F64Mul);
            host_call(out, "math_exp")?;
            out.push(LocalTee(scratch));
            f64_const(out, 1.0);
            out.push(F64Sub);
            out.push(LocalGet(scratch));
            f64_const(out, 1.0);
            out.push(F64Add);
            out.push(F64Div);
        }
        DerivedNumericOp::Sign => {
            out.push(LocalTee(scratch));
            f64_const(out, 0.0);
            out.push(F64Gt);
            out.push(If(BlockType::Result(wasm_encoder::ValType::F64)));
            f64_const(out, 1.0);
            out.push(Else);
            out.push(LocalGet(scratch));
            f64_const(out, 0.0);
            out.push(F64Lt);
            out.push(If(BlockType::Result(wasm_encoder::ValType::F64)));
            f64_const(out, -1.0);
            out.push(Else);
            f64_const(out, 0.0);
            out.push(End);
            out.push(End);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_native_and_derived_names() {
        assert_eq!(plan("Integer.bitCount"), Some(NumericPlan::Native(NativeNumericOp::PopCnt)));
        assert_eq!(plan("clz"), Some(NumericPlan::Native(NativeNumericOp::Clz)));
        assert_eq!(plan("MathAdvanced.copysign"), Some(NumericPlan::Native(NativeNumericOp::CopySign)));
        assert_eq!(plan("Math.log1p"), Some(NumericPlan::Derived(DerivedNumericOp::Log1p)));
        assert_eq!(plan("Math.sign"), Some(NumericPlan::Derived(DerivedNumericOp::Sign)));
        assert_eq!(plan("Math.pow"), Some(NumericPlan::Host("math_pow")));
        assert_eq!(plan("Math.unknown"), None);
    }

    #[test]
    fn native_ops_emit_one_instruction() {
        let ops = [
            NativeNumericOp::Clz,
            NativeNumericOp::Ctz,
            NativeNumericOp::PopCnt,
            NativeNumericOp::Rotl,
            NativeNumericOp::Rotr,
            NativeNumericOp::Abs,
            NativeNumericOp::Sqrt,
            NativeNumericOp::Floor,
            NativeNumericOp::Ceil,
            NativeNumericOp::Trunc,
            NativeNumericOp::Nearest,
            NativeNumericOp::Min,
            NativeNumericOp::Max,
            NativeNumericOp::CopySign,
        ];
        for op in ops {
            let mut out = Vec::new();
            emit_native(op, &mut out);
            assert_eq!(out.len(), 1, "{:?}", op);
        }
    }

    #[test]
    fn derived_ops_emit_valid_length_instruction_runs() {
        let ops = [
            DerivedNumericOp::Sinh,
            DerivedNumericOp::Cosh,
            DerivedNumericOp::Tanh,
            DerivedNumericOp::Log1p,
            DerivedNumericOp::Expm1,
            DerivedNumericOp::Sign,
        ];
        for op in ops {
            let mut out = Vec::new();
            emit_derived(op, &mut out, 7).expect("host imports exist");
            assert!(out.len() >= 3, "{:?}", op);
        }
    }
}
