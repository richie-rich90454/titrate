use super::*;


// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

pub(super) const INTEGER_TYPES: &[&str] = &[
    "byte", "short", "int", "long", "vast", "uvast", "u8", "u16", "u32", "u64", "size",
];

pub(super) const FLOAT_TYPES: &[&str] = &["float", "double", "half", "quad"];

pub(super) fn is_integer_type(t: &ast::Type) -> bool {
    INTEGER_TYPES.contains(&t.name())
}

pub(super) fn is_float_type(t: &ast::Type) -> bool {
    FLOAT_TYPES.contains(&t.name())
}

pub(super) fn is_numeric_type(t: &ast::Type) -> bool {
    is_integer_type(t) || is_float_type(t)
}

pub(super) fn is_bool_type(t: &ast::Type) -> bool {
    t.name() == "bool"
}

pub(super) fn is_string_type(t: &ast::Type) -> bool {
    t.name() == "string"
}

pub(super) fn is_owned_type(t: &ast::Type) -> bool {
    t.name() == "Owned"
}

pub(super) fn is_result_type(t: &ast::Type) -> bool {
    t.name() == "Result"
}

pub(super) fn is_ref_type(t: &ast::Type) -> bool {
    matches!(t, ast::Type::Ref(_) | ast::Type::MutRef(_))
}

pub(super) fn is_void_type(t: &ast::Type) -> bool {
    t.name() == "void"
}

pub(super) fn is_unknown_type(t: &ast::Type) -> bool {
    t.name() == "unknown" || t.name() == "any"
}

/// Map an AST operator to its operator method name (e.g. Add 閳?"operator+").
pub(super) fn operator_method_name(op: &ast::Operator) -> String {
    match op {
        ast::Operator::Add => "operator+".to_string(),
        ast::Operator::Sub => "operator-".to_string(),
        ast::Operator::Mul => "operator*".to_string(),
        ast::Operator::Div => "operator/".to_string(),
        ast::Operator::Mod => "operator%".to_string(),
        ast::Operator::Eq => "operator==".to_string(),
        ast::Operator::Ne => "operator!=".to_string(),
        ast::Operator::Lt => "operator<".to_string(),
        ast::Operator::Gt => "operator>".to_string(),
        ast::Operator::Le => "operator<=".to_string(),
        ast::Operator::Ge => "operator>=".to_string(),
        ast::Operator::BitAnd => "operator&".to_string(),
        ast::Operator::BitOr => "operator|".to_string(),
        ast::Operator::BitXor => "operator^".to_string(),
        ast::Operator::BitShl => "operator<<".to_string(),
        ast::Operator::BitShr => "operator>>".to_string(),
        ast::Operator::And | ast::Operator::Or => String::new(), // not overloadable
    }
}

/// Check if the given type is a class that has an operator overload method for the given operator.
pub(super) fn class_has_operator_method(left_type: &ast::Type, op: &ast::Operator, scope: &Rc<RefCell<Scope>>) -> bool {
    let method_name = operator_method_name(op);
    if method_name.is_empty() {
        return false;
    }
    if let Some(Symbol::Class(class_decl)) = scope.borrow().lookup(left_type.name()) {
        for member in &class_decl.members {
            if let ast::ClassMember::Method(m) = member {
                if m.name == method_name {
                    return true;
                }
            }
        }
    }
    false
}

/// Determine the type of a literal.
pub(super) fn literal_type(lit: &ast::Literal) -> ast::Type {
    match lit {
        ast::Literal::Int(_) => ast::Type::simple("int"),
        ast::Literal::Float(_) => ast::Type::simple("double"),
        ast::Literal::Bool(_) => ast::Type::simple("bool"),
        ast::Literal::Char(_) => ast::Type::simple("char"),
        ast::Literal::String(_) => ast::Type::simple("string"),
        ast::Literal::Null => ast::Type::simple("void"),
    }
}

/// Check if `source` can be assigned to `target`.
pub(super) fn is_assignable(source: &ast::Type, target: &ast::Type) -> bool {
    // Same type is always assignable.
    if source == target {
        return true;
    }
    // "any" or "unknown" can be assigned to anything (from builtins).
    if is_unknown_type(source) {
        return true;
    }
    // &mut T can be assigned to &T (immutable ref is a supertype of mutable ref)
    if let (ast::Type::MutRef(src_inner), ast::Type::Ref(tgt_inner)) = (source, target) {
        return is_assignable(src_inner, tgt_inner);
    }
    // &T can be assigned to &T, &mut T to &mut T (same inner type)
    if let (ast::Type::Ref(src_inner), ast::Type::Ref(tgt_inner)) = (source, target) {
        return is_assignable(src_inner, tgt_inner);
    }
    if let (ast::Type::MutRef(src_inner), ast::Type::MutRef(tgt_inner)) = (source, target) {
        return is_assignable(src_inner, tgt_inner);
    }
    // Integer literal (int) can be assigned to any integer type.
    if source.name() == "int" && is_integer_type(target) {
        return true;
    }
    // Float literal (double) can be assigned to any float type.
    if source.name() == "double" && is_float_type(target) {
        return true;
    }
    // null can be assigned to Owned types and reference types.
    if source.name() == "void" && target.name() == "Owned" {
        return true;
    }
    // Owned<T> can be assigned to Owned<T>.
    if source.name() == "Owned" && target.name() == "Owned" {
        if source.params().len() == target.params().len() {
            return source.params() == target.params();
        }
    }
    // new T(...) returns T, which can be assigned to Owned<T>.
    // This handles `let x: Owned<int> = new int(5)`.
    if target.name() == "Owned" {
        if let Some(inner) = target.params().first() {
            if is_assignable(source, inner) {
                return true;
            }
        }
    }
    // Result<any, any> can be assigned to any Result<T, E>.
    if source.name() == "Result" && target.name() == "Result" {
        if source.params().iter().any(|p| is_unknown_type(p)) {
            return true;
        }
    }
    // For the Alpha, we are relaxed: any type name mismatch that isn't caught
    // by the above rules is a type error.
    false
}

/// Map a primitive type name to its static class name for toString desugaring.
pub(super) fn static_class_for_primitive(t: &ast::Type) -> Option<String> {
    match t.name() {
        "int" => Some("Integer".to_string()),
        "long" => Some("Long".to_string()),
        "short" => Some("Short".to_string()),
        "byte" => Some("Byte".to_string()),
        "vast" => Some("Vast".to_string()),
        "uvast" => Some("Uvast".to_string()),
        "u8" => Some("U8".to_string()),
        "u16" => Some("U16".to_string()),
        "u32" => Some("U32".to_string()),
        "u64" => Some("U64".to_string()),
        "size" => Some("Size".to_string()),
        "float" => Some("Float".to_string()),
        "double" => Some("Double".to_string()),
        "half" => Some("Half".to_string()),
        "quad" => Some("Quad".to_string()),
        "bool" => Some("Boolean".to_string()),
        "char" => Some("Char".to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ExhaustiveMode
// ---------------------------------------------------------------------------

/// Controls how non-exhaustive pattern matches are reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExhaustiveMode {
    /// Emit a warning for non-exhaustive matches (default).
    #[default]
    Warning,
    /// Emit an error for non-exhaustive matches.
    Error,
}
