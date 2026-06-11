// Type inference helpers and typed opcode emission

use crate::ast;
use super::super::opcodes::{CastTarget, OpCode};
use super::{Compiler, InferredType};

impl Compiler {
    // -----------------------------------------------------------------------
    // Type inference helpers
    // -----------------------------------------------------------------------

    pub(super) fn infer_expr_type(&self, expr: &ast::Expr) -> InferredType {
        match expr {
            ast::Expr::Literal(lit, _) => self.infer_literal_type(lit),
            ast::Expr::Identifier(name, _) => self.infer_identifier_type(name),
            ast::Expr::Binary(left, op, right, _) => {
                let lt = self.infer_expr_type(left);
                let rt = self.infer_expr_type(right);
                match op {
                    ast::Operator::Add
                    | ast::Operator::Sub
                    | ast::Operator::Mul
                    | ast::Operator::Div
                    | ast::Operator::Mod => {
                        // If either side is String, it's string concatenation
                        if lt == InferredType::String || rt == InferredType::String {
                            InferredType::String
                        } else {
                            self.wider_type(lt, rt)
                        }
                    }
                    ast::Operator::Eq
                    | ast::Operator::Ne
                    | ast::Operator::Lt
                    | ast::Operator::Gt
                    | ast::Operator::Le
                    | ast::Operator::Ge => InferredType::Bool,
                    ast::Operator::And | ast::Operator::Or => InferredType::Bool,
                    ast::Operator::BitAnd
                    | ast::Operator::BitOr
                    | ast::Operator::BitXor
                    | ast::Operator::BitShl
                    | ast::Operator::BitShr => self.wider_type(lt, rt),
                }
            }
            ast::Expr::Unary(op, operand, _) => {
                let ot = self.infer_expr_type(operand);
                match op {
                    ast::UnOp::Neg => ot,
                    ast::UnOp::Not => InferredType::Bool,
                    ast::UnOp::BitNot => ot,
                }
            }
            ast::Expr::Call(callee, _args, _) => {
                // Check for toString calls on builtin objects
                if let ast::Expr::MemberAccess(_, method, _) = callee.as_ref() {
                    if method == "toString" {
                        return InferredType::String;
                    }
                }
                if let ast::Expr::Identifier(name, _) = callee.as_ref() {
                    if name == "Ok" || name == "Err" {
                        return InferredType::Unknown; // Result type
                    }
                }
                InferredType::Unknown
            }
            ast::Expr::MemberAccess(_, _, _) => InferredType::Unknown,
            ast::Expr::Index(_, _, _) => InferredType::Unknown,
            ast::Expr::New(_, _, _) => InferredType::Class,
            ast::Expr::This(_) => InferredType::Class,
            ast::Expr::Super(_) => InferredType::Unknown,
            ast::Expr::OwnedDeref(inner, _) => self.infer_expr_type(inner),
            ast::Expr::RegionAlloc(_, _, _) => InferredType::Unknown,
            ast::Expr::RefExpr(_, _, _) => InferredType::Unknown,
            ast::Expr::UnsafeBlock(_, _) => InferredType::Unknown,
            ast::Expr::ErrorPropagation(_, _) => InferredType::Unknown,
            ast::Expr::Cast(_, target_type, _) => self.type_to_inferred(target_type),
            ast::Expr::StaticCall { method, .. } => {
                // toString always returns String
                if method == "toString" {
                    InferredType::String
                } else if method == "parseInt" {
                    InferredType::Unknown // Result type
                } else {
                    InferredType::Unknown
                }
            }
            ast::Expr::Assign(_, _, _) => InferredType::Unknown,
            ast::Expr::Ternary { .. } => InferredType::Unknown,
            ast::Expr::Unit(_) => InferredType::Void,
            ast::Expr::Tuple(_, _) => InferredType::Unknown,
            ast::Expr::Closure { .. } => InferredType::Unknown,
            ast::Expr::Range(..) | ast::Expr::RangeInclusive(..) => InferredType::Unknown,
        }
    }

    pub(super) fn infer_literal_type(&self, lit: &ast::Literal) -> InferredType {
        match lit {
            ast::Literal::Int(_) => InferredType::I64,
            ast::Literal::Float(_) => InferredType::F64,
            ast::Literal::Bool(_) => InferredType::Bool,
            ast::Literal::Char(_) => InferredType::Char,
            ast::Literal::String(_) => InferredType::String,
            ast::Literal::Null => InferredType::Null,
        }
    }

    pub(super) fn infer_identifier_type(&self, name: &str) -> InferredType {
        // Check if it's a local variable with a known type.
        for local in self.locals.iter().rev() {
            if local.name == name {
                // We don't track types on locals yet, so default to Unknown.
                return InferredType::Unknown;
            }
        }
        InferredType::Unknown
    }

    pub(super) fn wider_type(&self, a: InferredType, b: InferredType) -> InferredType {
        if a == b {
            return a;
        }
        // If either side is String, the result is String (concatenation).
        if a == InferredType::String || b == InferredType::String {
            return InferredType::String;
        }
        // Promote to the wider type.
        match (a, b) {
            (InferredType::F64, _) | (_, InferredType::F64) => InferredType::F64,
            (InferredType::F32, _) | (_, InferredType::F32) => InferredType::F32,
            (InferredType::I64, _) | (_, InferredType::I64) => InferredType::I64,
            (InferredType::I32, _) | (_, InferredType::I32) => InferredType::I32,
            (InferredType::I16, _) | (_, InferredType::I16) => InferredType::I16,
            _ => InferredType::I64, // default
        }
    }

    pub(super) fn type_to_inferred(&self, typ: &ast::Type) -> InferredType {
        match typ {
            ast::Type::Ref(inner) | ast::Type::MutRef(inner) => self.type_to_inferred(inner),
            ast::Type::Tuple(_) => InferredType::Unknown,
            ast::Type::Named { .. } => match typ.name() {
                "byte" => InferredType::I8,
                "short" => InferredType::I16,
                "int" => InferredType::I32,
                "long" => InferredType::I64,
                "vast" => InferredType::I128,
                "uvast" => InferredType::U128,
                "float" => InferredType::F32,
                "double" => InferredType::F64,
                "bool" => InferredType::Bool,
                "char" => InferredType::Char,
                "string" | "String" => InferredType::String,
                _ => InferredType::Unknown,
            },
        }
    }

    pub(super) fn type_to_cast_target(&self, typ: &ast::Type) -> CastTarget {
        match typ {
            ast::Type::Ref(inner) | ast::Type::MutRef(inner) => self.type_to_cast_target(inner),
            ast::Type::Tuple(_) => CastTarget::Long, // safe default
            ast::Type::Named { .. } => match typ.name() {
                "byte" => CastTarget::Byte,
                "short" => CastTarget::Short,
                "int" => CastTarget::Int,
                "long" => CastTarget::Long,
                "vast" => CastTarget::Vast,
                "uvast" => CastTarget::Uvast,
                "float" => CastTarget::Float,
                "double" => CastTarget::Double,
                "half" => CastTarget::Half,
                "quad" => CastTarget::Quad,
                "char" => CastTarget::Char,
                "string" | "String" => CastTarget::String,
                "bool" => CastTarget::Bool,
                _ => CastTarget::Long, // safe default
            },
        }
    }

    pub(super) fn is_builtin_object(&self, name: &str) -> bool {
        matches!(
            name,
            "io"
                | "Integer"
                | "Double"
                | "Float"
                | "Long"
                | "Byte"
                | "Short"
                | "Half"
                | "Quad"
                | "Vast"
                | "Uvast"
                | "Boolean"
                | "Char"
                | "String_"
                | "String"
                | "ArrayList"
                | "HashMap"
                | "File"
                | "malloc"
                | "free"
        )
    }

    // -----------------------------------------------------------------------
    // Typed opcode emission helpers
    // -----------------------------------------------------------------------

    pub(super) fn emit_add_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::ADD_I32,
                InferredType::F32 => OpCode::ADD_F32,
                InferredType::F64 => OpCode::ADD_F64,
                _ => OpCode::ADD_I64, // default for I64, I128, U128, Unknown
            },
            line,
        );
    }

    pub(super) fn emit_sub_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SUB_I32,
                InferredType::F32 => OpCode::SUB_F32,
                InferredType::F64 => OpCode::SUB_F64,
                _ => OpCode::SUB_I64,
            },
            line,
        );
    }

    pub(super) fn emit_mul_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::MUL_I32,
                InferredType::F32 => OpCode::MUL_F32,
                InferredType::F64 => OpCode::MUL_F64,
                _ => OpCode::MUL_I64,
            },
            line,
        );
    }

    pub(super) fn emit_div_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::DIV_I32,
                InferredType::F32 => OpCode::DIV_F32,
                InferredType::F64 => OpCode::DIV_F64,
                _ => OpCode::DIV_I64,
            },
            line,
        );
    }

    pub(super) fn emit_mod_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::MOD_I32,
                InferredType::F32 => OpCode::MOD_F32,
                InferredType::F64 => OpCode::MOD_F64,
                _ => OpCode::MOD_I64,
            },
            line,
        );
    }

    pub(super) fn emit_neg_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::NEG_I32,
                InferredType::F32 => OpCode::NEG_F32,
                InferredType::F64 => OpCode::NEG_F64,
                _ => OpCode::NEG_I64,
            },
            line,
        );
    }

    pub(super) fn emit_bitand_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITAND_I32,
                _ => OpCode::BITAND_I64,
            },
            line,
        );
    }

    pub(super) fn emit_bitor_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITOR_I32,
                _ => OpCode::BITOR_I64,
            },
            line,
        );
    }

    pub(super) fn emit_bitxor_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITXOR_I32,
                _ => OpCode::BITXOR_I64,
            },
            line,
        );
    }

    pub(super) fn emit_shl_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SHL_I32,
                _ => OpCode::SHL_I64,
            },
            line,
        );
    }

    pub(super) fn emit_shr_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::SHR_I32,
                _ => OpCode::SHR_I64,
            },
            line,
        );
    }

    pub(super) fn emit_bitnot_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::BITNOT_I32,
                _ => OpCode::BITNOT_I64,
            },
            line,
        );
    }

    pub(super) fn emit_eq_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::EQ_I32,
                InferredType::F32 => OpCode::EQ_F32,
                InferredType::F64 => OpCode::EQ_F64,
                InferredType::Bool => OpCode::EQ_BOOL,
                InferredType::Char => OpCode::EQ_CHAR,
                InferredType::String => OpCode::EQ_STRING,
                _ => OpCode::EQ_I64,
            },
            line,
        );
    }

    pub(super) fn emit_ne_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::NE_I32,
                InferredType::F32 => OpCode::NE_F32,
                InferredType::F64 => OpCode::NE_F64,
                _ => OpCode::NE_I64,
            },
            line,
        );
    }

    pub(super) fn emit_lt_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::LT_I32,
                InferredType::F32 => OpCode::LT_F32,
                InferredType::F64 => OpCode::LT_F64,
                _ => OpCode::LT_I64,
            },
            line,
        );
    }

    pub(super) fn emit_gt_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::GT_I32,
                InferredType::F32 => OpCode::GT_F32,
                InferredType::F64 => OpCode::GT_F64,
                _ => OpCode::GT_I64,
            },
            line,
        );
    }

    pub(super) fn emit_le_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::LE_I32,
                InferredType::F32 => OpCode::LE_F32,
                InferredType::F64 => OpCode::LE_F64,
                _ => OpCode::LE_I64,
            },
            line,
        );
    }

    pub(super) fn emit_ge_opcode(&mut self, ty: InferredType, line: u32) {
        self.emit_opcode(
            match ty {
                InferredType::I32 => OpCode::GE_I32,
                InferredType::F32 => OpCode::GE_F32,
                InferredType::F64 => OpCode::GE_F64,
                _ => OpCode::GE_I64,
            },
            line,
        );
    }
}
