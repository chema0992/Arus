#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Main,
    Script,
    Fn,
    Let,
    Return,

    Ident(String),
    Integer(i64),

    Plus,
    Minus,
    Star,
    Slash,

    Colon,

    Equal,

    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,

    EOF,
}
