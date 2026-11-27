//! Data types for the shared Titrate intermediate representation (TIR).
//!
//! This module is deliberately standalone: no backend consumes it yet.

use std::collections::HashMap;

use crate::ast::{Literal, Operator, RefKind, Span, UnOp};

macro_rules! index_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u16);

        impl $name {
            pub const fn new(raw: u16) -> Self {
                Self(raw)
            }

            pub const fn as_u16(self) -> u16 {
                self.0
            }
        }
    };
}

index_id!(NameId);
index_id!(FunctionId);
index_id!(ClassId);
index_id!(VTableId);

/// Bidirectional mapping from strings to stable, compact name handles.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameInterner {
    names: Vec<String>,
    indices: HashMap<String, u16>,
}

impl NameInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, name: &str) -> Result<NameId, String> {
        if let Some(&raw) = self.indices.get(name) {
            return Ok(NameId::new(raw));
        }
        let raw = u16::try_from(self.names.len()).map_err(|_| {
            format!("name interner overflow: more than {} distinct names", u16::MAX as usize + 1)
        })?;
        self.names.push(name.to_string());
        self.indices.insert(name.to_string(), raw);
        Ok(NameId::new(raw))
    }

    pub fn get(&self, id: NameId) -> Option<&str> {
        self.names.get(id.as_u16() as usize).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Pointer width assumed by the foundation layout. Backends re-check ABI sizes.
pub const POINTER_SIZE: u32 = 8;

pub fn align_up(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    Void,
    Bool,
    Byte,
    Short,
    Int,
    Long,
    Vast,
    UVast,
    Float,
    Double,
    Half,
    Quad,
    Char,
    String,
    Size,
    U8,
    U16,
    U32,
    U64,
}

impl Primitive {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "void" => Self::Void,
            "bool" => Self::Bool,
            "byte" => Self::Byte,
            "short" => Self::Short,
            "int" => Self::Int,
            "long" => Self::Long,
            "vast" => Self::Vast,
            "uvast" => Self::UVast,
            "float" => Self::Float,
            "double" => Self::Double,
            "half" => Self::Half,
            "quad" => Self::Quad,
            "char" => Self::Char,
            "string" => Self::String,
            "size" => Self::Size,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            _ => return None,
        })
    }

    pub const fn size_bytes(self) -> u32 {
        match self {
            Self::Void => 0,
            Self::Bool | Self::Byte | Self::U8 => 1,
            Self::Short | Self::U16 | Self::Half => 2,
            Self::Int | Self::U32 | Self::Float | Self::Char => 4,
            Self::Long | Self::U64 | Self::Double | Self::Size => 8,
            Self::Vast | Self::UVast | Self::String => POINTER_SIZE,
            Self::Quad => 16,
        }
    }

    pub const fn alignment(self) -> u32 {
        let size = self.size_bytes();
        if size == 0 {
            1
        } else {
            size
        }
    }
}

/// A resolved type. Names are interned; structure mirrors `ast::Type`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TirType {
    Named { name: NameId, params: Vec<TirType> },
    Ref(Box<TirType>),
    MutRef(Box<TirType>),
    Tuple(Vec<TirType>),
}

impl TirType {
    pub fn size_align(&self, names: &NameInterner) -> Result<(u32, u32), String> {
        match self {
            Self::Named { name, .. } => {
                let raw = names.get(*name).ok_or_else(|| {
                    format!("invalid name id {} in TIR type", name.as_u16())
                })?;
                match Primitive::from_name(raw) {
                    Some(primitive) => Ok((primitive.size_bytes(), primitive.alignment())),
                    None => Ok((POINTER_SIZE, POINTER_SIZE)),
                }
            }
            Self::Ref(_) | Self::MutRef(_) => Ok((POINTER_SIZE, POINTER_SIZE)),
            Self::Tuple(items) => {
                let mut size = 0u32;
                let mut alignment = 1u32;
                for item in items {
                    let (item_size, item_align) = item.size_align(names)?;
                    size = align_up(size, item_align) + item_size;
                    alignment = alignment.max(item_align);
                }
                Ok((align_up(size, alignment), alignment))
            }
        }
    }
}

/// Effect flags attached to a lowered function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Capabilities(u16);

impl Capabilities {
    pub const NONE: Self = Self(0);
    pub const IO: Self = Self(1 << 0);
    pub const FILE_SYSTEM: Self = Self(1 << 1);
    pub const NETWORK: Self = Self(1 << 2);
    pub const TIME: Self = Self(1 << 3);
    pub const RANDOM: Self = Self(1 << 4);
    pub const NATIVE: Self = Self(1 << 5);
    pub const MUTATION: Self = Self(1 << 6);

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn is_pure(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Vectorization preference carried on loops and on their enclosing function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SimdHint {
    #[default]
    None,
    Allow,
    Require,
}

/// A call site's resolved destination. Names remain interned so backends can
/// compare targets without string work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallTarget {
    /// A function defined in this program.
    Direct(FunctionId),
    /// Dispatch through a shared vtable slot.
    Virtual { vtable: VTableId, slot: u16 },
    /// A VM/module native, named `module.method`.
    Native { name: NameId },
    /// A name supplied by another module; resolved after linking.
    External { name: NameId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TirParam {
    pub name: NameId,
    pub typ: TirType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TirExpr {
    Literal(Literal, Span),
    Var(NameId, Span),
    This(Span),
    Unit(Span),
    Unary { op: UnOp, operand: Box<TirExpr>, span: Span },
    Binary { op: Operator, left: Box<TirExpr>, right: Box<TirExpr>, span: Span },
    Assign { target: Box<TirExpr>, value: Box<TirExpr>, span: Span },
    Cast { value: Box<TirExpr>, typ: TirType, span: Span },
    Is { value: Box<TirExpr>, typ: TirType, span: Span },
    Ref { value: Box<TirExpr>, kind: RefKind, span: Span },
    Call { target: CallTarget, type_args: Vec<TirType>, args: Vec<TirExpr>, span: Span },
    Field { object: Box<TirExpr>, field: NameId, slot: Option<u16>, span: Span },
    New { class: NameId, args: Vec<TirExpr>, span: Span },
    Tuple { items: Vec<TirExpr>, span: Span },
    Ternary { condition: Box<TirExpr>, then_expr: Box<TirExpr>, else_expr: Box<TirExpr>, span: Span },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TirStmt {
    Block(Vec<TirStmt>),
    Expr(TirExpr),
    Let {
        name: NameId,
        typ: Option<TirType>,
        init: Option<TirExpr>,
        mutable: bool,
        span: Span,
    },
    Return(Option<TirExpr>),
    Break,
    Continue,
    If {
        cond: TirExpr,
        then_branch: Vec<TirStmt>,
        else_branch: Option<Vec<TirStmt>>,
        span: Span,
    },
    While {
        cond: TirExpr,
        body: Vec<TirStmt>,
        simd: SimdHint,
        span: Span,
    },
    For {
        var: NameId,
        iterable: TirExpr,
        body: Vec<TirStmt>,
        simd: SimdHint,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TirFunction {
    pub id: FunctionId,
    pub name: NameId,
    pub type_params: Vec<NameId>,
    pub params: Vec<TirParam>,
    pub return_type: Option<TirType>,
    pub body: Vec<TirStmt>,
    pub capability: Capabilities,
    pub simd: SimdHint,
    pub is_public: bool,
}

/// A top-level `var`/`const` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TirGlobal {
    pub name: NameId,
    pub typ: Option<TirType>,
    pub init: Option<TirExpr>,
    pub is_const: bool,
    pub is_public: bool,
}

/// One field in a shared class layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSlot {
    pub name: NameId,
    pub typ: TirType,
    pub slot: u16,
    pub offset: u32,
}

/// One entry in a shared vtable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VTableSlot {
    pub method: NameId,
    pub target: FunctionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VTable {
    pub class: NameId,
    pub entries: Vec<VTableSlot>,
}

/// Instance layout shared by every use of a class. Slot 0 is the vtable
/// pointer; fields follow it. Sizes use natural alignment up to the field's
/// own size, with heap values treated as pointers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassLayout {
    pub name: NameId,
    pub parent: Option<ClassId>,
    pub fields: Vec<FieldSlot>,
    pub total_slots: u16,
    pub size_bytes: u32,
    pub vtable: VTableId,
}

/// Complete lowered program. All ids index the vectors here.
#[derive(Debug, Clone, PartialEq)]
pub struct TirProgram {
    pub names: NameInterner,
    pub functions: Vec<TirFunction>,
    pub globals: Vec<TirGlobal>,
    pub classes: Vec<ClassLayout>,
    pub vtables: Vec<VTable>,
}

impl TirProgram {
    pub fn function(&self, id: FunctionId) -> &TirFunction {
        &self.functions[id.as_u16() as usize]
    }

    pub fn class(&self, id: ClassId) -> &ClassLayout {
        &self.classes[id.as_u16() as usize]
    }

    pub fn vtable(&self, id: VTableId) -> &VTable {
        &self.vtables[id.as_u16() as usize]
    }

    pub fn name(&self, id: NameId) -> Option<&str> {
        self.names.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interner_is_stable_and_distinct() {
        let mut names = NameInterner::new();
        let a = names.intern("area").unwrap();
        let b = names.intern("radius").unwrap();
        assert_eq!(a, names.intern("area").unwrap());
        assert_ne!(a, b);
        assert_eq!(names.get(a), Some("area"));
        assert_eq!(names.get(b), Some("radius"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn interner_reports_overflow() {
        let mut names = NameInterner::new();
        for value in 0..=u16::MAX {
            names.intern(&value.to_string()).unwrap();
        }
        assert!(names.intern("overflow").is_err());
    }

    #[test]
    fn primitive_sizes_and_alignment() {
        let mut names = NameInterner::new();
        let int = TirType::Named { name: names.intern("int").unwrap(), params: vec![] };
        let double = TirType::Named { name: names.intern("double").unwrap(), params: vec![] };
        assert_eq!(int.size_align(&names).unwrap(), (4, 4));
        assert_eq!(double.size_align(&names).unwrap(), (8, 8));
        let pair = TirType::Tuple(vec![int, double]);
        assert_eq!(pair.size_align(&names).unwrap(), (16, 8));
    }
}
