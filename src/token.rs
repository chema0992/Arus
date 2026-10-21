#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Main,
    Script,
    Fn,
    Let,
    Return,

    Ident(String),
    Integer(i64),
    StringLiteral(String),

    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    Colon,

    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,

    EOF,
}
