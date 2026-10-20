use crate::token::Token;

pub struct Lexer {
    source: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            position: 0,
        }
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.position;

        while self.position < self.source.len()
            && (self.source[self.position].is_ascii_alphanumeric()
                || self.source[self.position] == '_')
        {
            self.position += 1;
        }

        let text: String = self.source[start..self.position]
            .iter()
            .collect();

        match text.as_str() {
            "main" => Token::Main,
            "script" => Token::Script,
            "fn" => Token::Fn,
            "let" => Token::Let,
            "return" => Token::Return,
            _ => Token::Ident(text),
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.position;

        while self.position < self.source.len()
            && self.source[self.position].is_ascii_digit()
        {
            self.position += 1;
        }

        let text: String = self.source[start..self.position]
            .iter()
            .collect();

        Token::Integer(text.parse().unwrap())
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while self.position < self.source.len() {
            let c = self.source[self.position];

            // 공백
            if c.is_whitespace() {
                self.position += 1;
                continue;
            }

            // 식별자
            if c.is_ascii_alphabetic() || c == '_' {
                tokens.push(self.read_identifier());
                continue;
            }

            // 숫자
            if c.is_ascii_digit() {
                tokens.push(self.read_number());
                continue;
            }

            // 연산자 / 괄호
            let token = match c {
                '+' => Token::Plus,
                '-' => Token::Minus,
                '*' => Token::Star,
                '/' => Token::Slash,
                '=' => Token::Equal,

                ':' => Token::Colon,

                '(' => Token::LeftParen,
                ')' => Token::RightParen,
                '{' => Token::LeftBrace,
                '}' => Token::RightBrace,

                _ => {
                // 알 수 없는 문자를 발견하면 그 즉시 에러 메시지를 뿜고 종료!
                panic!("ERR: 알 수 없는 문자 발견 -> '{}'", c);
            }

            };

            tokens.push(token);
            self.position += 1;
        }
        tokens.push(Token::EOF);
        tokens
    }

}