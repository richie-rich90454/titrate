/// AST node types for the Titrate language.
/// All desugaring is complete before the AST is returned from the parser.

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

/// Type representation in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named { name: String, params: Vec<Type> },
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
        }
    }

    pub fn params(&self) -> &[Type] {
        match self {
            Type::Named { params, .. } => params,
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
        }
    }
}

/// Reference kind for borrow expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum RefKind {
    Immutable,
    Mutable,
}

/// Binary operator.
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, BitShl, BitShr,
}

/// Unary operator.
#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}

/// Literal values.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Null,
}

/// Pattern for switch/case matching.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Literal),
    Wildcard,
    Constructor { name: String, bindings: Vec<String> },
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub typ: Type,
}

/// Variable declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub name: String,
    pub typ: Option<Type>,
    pub init: Option<Expr>,
    pub mutable: bool,
    pub span: Span,
}

/// Import declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: Vec<String>,
    pub span: Span,
}

/// Function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub access: Access,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub sugar: bool,
    pub span: Span,
}

/// Method signature (for interfaces).
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
}

/// Method declaration (for classes).
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub access: Access,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub span: Span,
}

/// Field declaration (for classes).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub access: Access,
    pub name: String,
    pub typ: Type,
    pub init: Option<Expr>,
    pub span: Span,
}

/// Class member.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field(FieldDecl),
    Method(MethodDecl),
    Constructor(MethodDecl),
}

/// Class declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub parent: Option<Type>,
    pub ifaces: Vec<Type>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

/// Interface declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub parents: Vec<Type>,
    pub methods: Vec<MethodSig>,
    pub span: Span,
}

/// Enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Param>,
}

/// Enum declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<Variant>,
    pub span: Span,
}

/// Top-level declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Function(FnDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Enum(EnumDecl),
    VarDecl(VarDecl),
    ConstDecl(VarDecl),
}

/// Block of statements.
pub type Block = Vec<Stmt>;

/// If statement.
#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Block,
    pub else_branch: Option<Block>,
    pub span: Span,
}

/// While statement.
#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Block,
    pub span: Span,
}

/// For statement.
#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub var: String,
    pub iterable: Expr,
    pub body: Block,
    pub span: Span,
}

/// Switch case.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub pattern: Pattern,
    pub body: Block,
}

/// Switch statement.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStmt {
    pub expr: Expr,
    pub cases: Vec<Case>,
    pub default: Option<Block>,
    pub span: Span,
}

/// Statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Block(Block),
    Expr(Expr),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Return(Option<Expr>),
    Break,
    Continue,
    Switch(SwitchStmt),
    VarDecl(VarDecl),
    ConstDecl(VarDecl),
}

/// Expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal, Span),
    Identifier(String, Span),
    Binary(Box<Expr>, Operator, Box<Expr>, Span),
    Unary(UnOp, Box<Expr>, Span),
    Call(Box<Expr>, Vec<Expr>, Span),
    MemberAccess(Box<Expr>, String, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    New(Type, Vec<Expr>, Span),
    This(Span),
    Super(Span),
    OwnedDeref(Box<Expr>, Span),
    RegionAlloc(Type, Box<Expr>, Span),
    RefExpr(Box<Expr>, RefKind, Span),
    UnsafeBlock(Block, Span),
    ErrorPropagation(Box<Expr>, Span),
    Cast(Box<Expr>, Type, Span),
    StaticCall { class_name: String, method: String, args: Vec<Expr>, span: Span },
    Assign(Box<Expr>, Box<Expr>, Span),
}

/// Complete program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<Import>,
    pub declarations: Vec<Declaration>,
}
