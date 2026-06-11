/// Token types and related definitions for the Titrate lexer.

use std::fmt;

/// Suffix for float literals with explicit width.
#[derive(Debug, Clone, PartialEq)]
pub enum FloatSuffix {
    Half,
    Quad,
}

/// Every token the Titrate lexer can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Access modifiers
    Public,
    Private,

    // Declaration keywords
    Fn,
    Class,
    Interface,
    Enum,
    Extends,
    Implements,
    Let,
    Var,
    Const,

    // Control flow keywords
    Do,
    If,
    Else,
    While,
    For,
    Return,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    With,

    // Literal keywords
    True,
    False,
    Null,

    // Object-oriented keywords
    New,
    This,
    Super,

    // Result type keywords
    Result,
    Ok,
    Err,

    // Ownership keywords
    Owned,
    Region,
    Unsafe,

    // Type operation keywords
    As,
    Is,
    Type,

    // Module keywords
    Import,
    Module,
    Where,

    // Primitive type keywords
    Void,
    Bool,
    Byte,
    Short,
    Int,
    Long,
    Vast,
    Uvast,
    Float,
    Double,
    Half,
    Quad,
    Char,
    String,
    Size,

    // Unsigned type keywords
    U8,
    U16,
    U32,
    U64,

    // Operators and punctuation
    Plus,
    PlusPlus,
    Minus,
    MinusMinus,
    Star,
    Slash,
    Percent,
    Equals,
    EqualEqual,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    AndAnd,
    OrOr,
    Not,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    LeftShift,
    RightShift,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,
    AmpersandEqual,
    PipeEqual,
    CaretEqual,
    LeftShiftEqual,
    RightShiftEqual,
    ColonColon,
    Arrow,
    FatArrow,
    Question,
    Dot,
    DotDot,
    DotDotEq,
    Comma,
    Semicolon,
    Colon,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    RefMut, // &mut

    // Literals
    IntLiteral(i64),
    FloatLiteral { value: f64, suffix: Option<FloatSuffix> },
    StringLiteral(String),
    RawStringLiteral(String),
    CharLiteral(char),
    ByteLiteral(u8),
    BoolLiteral(bool),
    NullLiteral,

    // Identifier
    Identifier(String),

    // Error token for unrecognised input
    Error(String),

    // End of file
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Public => write!(f, "public"),
            Token::Private => write!(f, "private"),
            Token::Fn => write!(f, "fn"),
            Token::Class => write!(f, "class"),
            Token::Interface => write!(f, "interface"),
            Token::Enum => write!(f, "enum"),
            Token::Extends => write!(f, "extends"),
            Token::Implements => write!(f, "implements"),
            Token::Let => write!(f, "let"),
            Token::Var => write!(f, "var"),
            Token::Const => write!(f, "const"),
            Token::Do => write!(f, "do"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::Return => write!(f, "return"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Switch => write!(f, "switch"),
            Token::Case => write!(f, "case"),
            Token::Default => write!(f, "default"),
            Token::With => write!(f, "with"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Null => write!(f, "null"),
            Token::New => write!(f, "new"),
            Token::This => write!(f, "this"),
            Token::Super => write!(f, "super"),
            Token::Result => write!(f, "Result"),
            Token::Ok => write!(f, "Ok"),
            Token::Err => write!(f, "Err"),
            Token::Owned => write!(f, "Owned"),
            Token::Region => write!(f, "region"),
            Token::Unsafe => write!(f, "unsafe"),
            Token::As => write!(f, "as"),
            Token::Is => write!(f, "is"),
            Token::Type => write!(f, "type"),
            Token::Import => write!(f, "import"),
            Token::Module => write!(f, "module"),
            Token::Where => write!(f, "where"),
            Token::Void => write!(f, "void"),
            Token::Bool => write!(f, "bool"),
            Token::Byte => write!(f, "byte"),
            Token::Short => write!(f, "short"),
            Token::Int => write!(f, "int"),
            Token::Long => write!(f, "long"),
            Token::Vast => write!(f, "vast"),
            Token::Uvast => write!(f, "uvast"),
            Token::Float => write!(f, "float"),
            Token::Double => write!(f, "double"),
            Token::Half => write!(f, "half"),
            Token::Quad => write!(f, "quad"),
            Token::Char => write!(f, "char"),
            Token::String => write!(f, "string"),
            Token::Size => write!(f, "size"),
            Token::U8 => write!(f, "u8"),
            Token::U16 => write!(f, "u16"),
            Token::U32 => write!(f, "u32"),
            Token::U64 => write!(f, "u64"),
            Token::Plus => write!(f, "+"),
            Token::PlusPlus => write!(f, "++"),
            Token::Minus => write!(f, "-"),
            Token::MinusMinus => write!(f, "--"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Equals => write!(f, "="),
            Token::EqualEqual => write!(f, "=="),
            Token::NotEqual => write!(f, "!="),
            Token::Less => write!(f, "<"),
            Token::Greater => write!(f, ">"),
            Token::LessEqual => write!(f, "<="),
            Token::GreaterEqual => write!(f, ">="),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::Not => write!(f, "!"),
            Token::Ampersand => write!(f, "&"),
            Token::Pipe => write!(f, "|"),
            Token::Caret => write!(f, "^"),
            Token::Tilde => write!(f, "~"),
            Token::LeftShift => write!(f, "<<"),
            Token::RightShift => write!(f, ">>"),
            Token::ColonColon => write!(f, "::"),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::Question => write!(f, "?"),
            Token::Dot => write!(f, "."),
            Token::DotDot => write!(f, ".."),
            Token::DotDotEq => write!(f, "..="),
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            Token::RefMut => write!(f, "&mut"),
            Token::PlusEqual => write!(f, "+="),
            Token::MinusEqual => write!(f, "-="),
            Token::StarEqual => write!(f, "*="),
            Token::SlashEqual => write!(f, "/="),
            Token::PercentEqual => write!(f, "%="),
            Token::AmpersandEqual => write!(f, "&="),
            Token::PipeEqual => write!(f, "|="),
            Token::CaretEqual => write!(f, "^="),
            Token::LeftShiftEqual => write!(f, "<<="),
            Token::RightShiftEqual => write!(f, ">>="),
            Token::IntLiteral(v) => write!(f, "{}", v),
            Token::FloatLiteral { value, suffix } => {
                write!(f, "{}", value)?;
                match suffix {
                    Some(FloatSuffix::Half) => write!(f, "h"),
                    Some(FloatSuffix::Quad) => write!(f, "q"),
                    None => Ok(()),
                }
            }
            Token::StringLiteral(s) => write!(f, "\"{}\"", s),
            Token::RawStringLiteral(s) => write!(f, "r\"{}\"", s),
            Token::CharLiteral(c) => write!(f, "'{}'", c),
            Token::ByteLiteral(v) => write!(f, "b'{}'", v),
            Token::BoolLiteral(b) => write!(f, "{}", b),
            Token::NullLiteral => write!(f, "null"),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::Error(s) => write!(f, "ERROR({})", s),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

/// Try to consume an operator suffix after the identifier "operator".
/// Returns the operator string (e.g. "+", "==", "<=") if the next
/// characters form a valid overloadable operator, or None otherwise.
/// Does NOT consume any characters if the operator is not valid.
pub(super) fn try_consume_operator(chars: &mut std::iter::Peekable<std::str::Chars>, column: &mut usize) -> Option<String> {
    // Peek at the next character without consuming
    let next = *chars.peek()?;

    // Determine the operator string by peeking ahead
    let op_str = match next {
        '+' | '-' | '*' | '/' | '%' => next.to_string(),
        '=' => {
            // Only "==" is a valid operator, not "="
            if chars.clone().nth(1) == Some('=') {
                "==".to_string()
            } else {
                return None; // "operator=" is not valid
            }
        }
        '!' => {
            // Only "!=" is a valid operator, not "!"
            if chars.clone().nth(1) == Some('=') {
                "!=".to_string()
            } else {
                return None;
            }
        }
        '<' => {
            if chars.clone().nth(1) == Some('=') {
                "<=".to_string()
            } else if chars.clone().nth(1) == Some('<') {
                "<<".to_string()
            } else {
                "<".to_string()
            }
        }
        '>' => {
            if chars.clone().nth(1) == Some('=') {
                ">=".to_string()
            } else if chars.clone().nth(1) == Some('>') {
                ">>".to_string()
            } else {
                ">".to_string()
            }
        }
        '&' => "&".to_string(),
        '|' => "|".to_string(),
        '^' => "^".to_string(),
        _ => return None,
    };

    // Now consume the characters that make up the operator
    for _ in 0..op_str.len() {
        chars.next();
        *column += 1;
    }

    Some(op_str)
}
