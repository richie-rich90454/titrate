/// Statement and expression AST nodes for the Titrate language.

use super::types::{Access, RefKind, Span, Type, TypeParam};

/// Binary operator.
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, BitShl, BitShr, BitUshr,
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
    pub glob: bool,
    pub span: Span,
}

/// Function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub access: Access,
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub sugar: bool,
    pub where_clause: Vec<TypeParam>,
    pub span: Span,
}

/// Method signature (for interfaces).
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Option<Block>, // Default method body (for interface default methods)
}

/// Method declaration (for classes).
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub access: Access,
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub where_clause: Vec<TypeParam>,
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
    pub type_params: Vec<TypeParam>,
    pub parent: Option<Type>,
    pub ifaces: Vec<Type>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

/// Interface declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>,
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
    pub type_params: Vec<TypeParam>,
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

/// Do-while statement.
#[derive(Debug, Clone, PartialEq)]
pub struct DoWhileStmt {
    pub body: Block,
    pub condition: Expr,
    pub span: Span,
}

/// While-let statement.
#[derive(Debug, Clone, PartialEq)]
pub struct WhileLetStmt {
    pub var_name: String,
    pub expr: Expr,
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

/// C-style for statement.
#[derive(Debug, Clone, PartialEq)]
pub struct CForStmt {
    pub init: Option<Box<Stmt>>,
    pub condition: Option<Expr>,
    pub increment: Option<Expr>,
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

/// With statement (context manager / RAII).
/// `with (resource) { body }` or `with (let f: T = expr) { body }`
/// The resource's `.close()` method is called automatically when the body exits.
#[derive(Debug, Clone, PartialEq)]
pub struct WithStmt {
    pub resource_expr: Expr,
    pub var_name: Option<String>,
    pub var_type: Option<Type>,
    pub body: Block,
    pub span: Span,
}

/// Statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Block(Block),
    Expr(Expr),
    If(IfStmt),
    While(WhileStmt),
    DoWhile(DoWhileStmt),
    WhileLet(WhileLetStmt),
    For(ForStmt),
    CFor(CForStmt),
    Return(Option<Expr>),
    Break,
    Continue,
    Switch(SwitchStmt),
    With(WithStmt),
    VarDecl(VarDecl),
    ConstDecl(VarDecl),
    TupleDestructure { names: Vec<String>, expr: Expr, mutable: bool, span: Span },
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
    Ternary { condition: Box<Expr>, then_expr: Box<Expr>, else_expr: Box<Expr>, span: Span },
    Range(Box<Expr>, Box<Expr>, Span),
    RangeInclusive(Box<Expr>, Box<Expr>, Span),
    Unit(Span),
    Tuple(Vec<Expr>, Span),
    Closure {
        params: Vec<(String, Type)>,
        return_type: Type,
        body: Block,
        expr: Option<Box<Expr>>,
        captured_vars: Vec<String>,
        span: Span,
    },
}

/// Complete program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<Import>,
    pub declarations: Vec<Declaration>,
}
