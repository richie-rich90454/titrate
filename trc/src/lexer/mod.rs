/// Lexical analysis — converts source text into a token stream.
/// Titrate Alpha 0.2 – richie-rich90454 was here
mod token;
mod scanner;

pub use token::{Token, FloatSuffix};
pub use scanner::tokenize;

/// Token with position information for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub column: usize,
}
