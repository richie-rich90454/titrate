/// Type-related AST nodes for the Titrate language.

use std::fmt;

/// Source location attached to every AST node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub fn new(line: u32, column: u32) -> Self {
        Span { line, column }
    }

    pub fn unknown() -> Self {
        Span { line: 0, column: 0 }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Access level for declarations.
#[derive(Debug, Clone, PartialEq)]
pub enum Access {
    Public,
    Private,
}

impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Access::Public => write!(f, "public"),
            Access::Private => write!(f, "private"),
        }
    }
}

/// Type parameter with optional interface constraint.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
    pub constraint: Option<Type>,
}

/// Type representation in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named { name: String, params: Vec<Type> },
    Ref(Box<Type>),       // &T
    MutRef(Box<Type>),    // &mut T
    Tuple(Vec<Type>),     // (T1, T2, ...)
}

impl Type {
    pub fn simple(name: &str) -> Type {
        Type::Named { name: name.to_string(), params: vec![] }
    }

    pub fn generic(name: &str, params: Vec<Type>) -> Type {
        Type::Named { name: name.to_string(), params }
    }

    pub fn name(&self) -> &str {
        match self {
            Type::Named { name, .. } => name,
            Type::Ref(inner) => inner.name(),
            Type::MutRef(inner) => inner.name(),
            Type::Tuple(_) => "tuple",
        }
    }

    pub fn params(&self) -> &[Type] {
        match self {
            Type::Named { params, .. } => params,
            Type::Ref(inner) => inner.params(),
            Type::MutRef(inner) => inner.params(),
            Type::Tuple(_) => &[],
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Named { name, params } => {
                write!(f, "{}", name)?;
                if !params.is_empty() {
                    write!(f, "<")?;
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", p)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Ref(inner) => write!(f, "&{}", inner),
            Type::MutRef(inner) => write!(f, "&mut {}", inner),
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Reference kind for borrow expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum RefKind {
    Immutable,
    Mutable,
}
