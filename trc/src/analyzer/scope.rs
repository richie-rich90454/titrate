use super::*;

// ---------------------------------------------------------------------------
// Symbol types
// ---------------------------------------------------------------------------

/// Ownership state of a variable at a program point.
#[derive(Debug, Clone, PartialEq)]
pub enum VarState {
    Live,
    Moved,
    BorrowedImmutable,
    BorrowedMutable,
}

/// A resolved symbol in scope.
#[derive(Debug, Clone)]
pub enum Symbol {
    Variable {
        typ: ast::Type,
        mutable: bool,
    },
    Function(ast::FnDecl),
    Class(ast::ClassDecl),
    Interface(ast::InterfaceDecl),
    Enum(ast::EnumDecl),
    Variant {
        enum_name: String,
        variant_name: String,
        fields: Vec<ast::Param>,
    },
}

// ---------------------------------------------------------------------------
// Scope
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Scope {
    parent: Option<Rc<RefCell<Scope>>>,
    pub(super) symbols: HashMap<String, Symbol>,
}

impl Scope {
    pub(super) fn new(parent: Option<Rc<RefCell<Scope>>>) -> Self {
        Scope {
            parent,
            symbols: HashMap::new(),
        }
    }

    pub(super) fn define(&mut self, name: String, symbol: Symbol) {
        self.symbols.insert(name, symbol);
    }

    pub(super) fn lookup(&self, name: &str) -> Option<Symbol> {
        if let Some(sym) = self.symbols.get(name) {
            return Some(sym.clone());
        }
        if let Some(ref p) = self.parent {
            return p.borrow().lookup(name);
        }
        None
    }

    /// Collect all symbol names visible in this scope (including parent scopes).
    pub(super) fn all_names(&self) -> Vec<String> {
        let mut names: HashSet<String> = self.symbols.keys().cloned().collect();
        if let Some(ref p) = self.parent {
            for name in p.borrow().all_names() {
                names.insert(name);
            }
        }
        names.into_iter().collect()
    }
}
