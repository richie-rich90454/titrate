//! Semantic analyzer for the Titrate language.
//! Every drop matters — richie-rich90454, 2026
//!
//! Performs symbol resolution, type checking, ownership analysis,
//! error-propagation validation, and toString desugaring.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use crate::ast;

mod errors;
mod scope;
mod types;
mod registration;
mod stmts;
mod exprs;
mod closures;
mod inference;
#[cfg(test)]
mod tests;

// Re-export public types
pub use errors::{CompileError, Suggestion};
pub use scope::{Scope, Symbol, VarState};
pub use types::ExhaustiveMode;

// Re-export for tests
#[cfg(test)]
pub(crate) use errors::levenshtein;

// ---------------------------------------------------------------------------
// Analyzer
// ---------------------------------------------------------------------------

pub(super) struct Analyzer {
    pub(super) errors: Vec<CompileError>,
    pub(super) warnings: Vec<String>,
    /// Ownership state per variable in the current function scope.
    /// Keyed by a scope-depth-qualified name to handle shadowing.
    pub(super) var_states: HashMap<String, VarState>,
    /// Track which variables are locals in the current function (for borrow-checking).
    pub(super) local_vars: Vec<String>,
    /// Current function return type (for return-checking and ?-operator).
    pub(super) current_return_type: Option<ast::Type>,
    /// Current function name (for better error messages).
    pub(super) current_fn_name: Option<String>,
    /// Whether we are inside an unsafe block.
    pub(super) in_unsafe: bool,
    /// How to report non-exhaustive pattern matches.
    pub(super) exhaustive_mode: ExhaustiveMode,
    /// Track variables that have been used (read) in the current function.
    pub(super) used_vars: HashSet<String>,
    /// Track whether the current position is after a return/break/continue
    /// (for unreachable code detection).
    pub(super) after_terminator: bool,
}

impl Analyzer {
    #[allow(dead_code)]
    pub(super) fn new() -> Self {
        Analyzer {
            errors: Vec::new(),
            warnings: Vec::new(),
            var_states: HashMap::new(),
            local_vars: Vec::new(),
            current_return_type: None,
            current_fn_name: None,
            in_unsafe: false,
            exhaustive_mode: ExhaustiveMode::default(),
            used_vars: HashSet::new(),
            after_terminator: false,
        }
    }

    pub(super) fn with_exhaustive_mode(mode: ExhaustiveMode) -> Self {
        Analyzer {
            errors: Vec::new(),
            warnings: Vec::new(),
            var_states: HashMap::new(),
            local_vars: Vec::new(),
            current_return_type: None,
            current_fn_name: None,
            in_unsafe: false,
            exhaustive_mode: mode,
            used_vars: HashSet::new(),
            after_terminator: false,
        }
    }

    pub(super) fn error(&mut self, err: impl Into<CompileError>) {
        self.errors.push(err.into());
    }

    pub(super) fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze a Titrate program, performing symbol resolution, type checking,
/// ownership analysis, and toString desugaring.
///
/// Returns the (possibly modified) program on success, or a vector of
/// compile errors describing all semantic errors found.
pub fn analyze(program: &ast::Program) -> Result<ast::Program, Vec<String>> {
    analyze_with_mode(program, ExhaustiveMode::default())
}

/// Analyze with an explicit exhaustiveness mode.
/// Returns `Ok(program)` if there are no errors (warnings are reported
/// but do not cause a failure), or `Err(errors)` otherwise.
pub fn analyze_with_mode(program: &ast::Program, mode: ExhaustiveMode) -> Result<ast::Program, Vec<String>> {
    analyze_with_mode_and_warnings(program, mode).map(|(prog, _)| prog)
}

/// Analyze with an explicit exhaustiveness mode, returning warnings alongside the result.
pub fn analyze_with_mode_and_warnings(program: &ast::Program, mode: ExhaustiveMode) -> Result<(ast::Program, Vec<String>), Vec<String>> {
    let mut program = program.clone();
    let mut analyzer = Analyzer::with_exhaustive_mode(mode);
    analyzer.analyze_program(&mut program);
    let warnings = analyzer.warnings.clone();
    if analyzer.errors.is_empty() {
        Ok((program, warnings))
    } else {
        Err(analyzer.errors.iter().map(|e| e.to_string()).collect())
    }
}
