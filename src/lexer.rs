use crate::token::Token;

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            input: source.chars().collect(),
            position: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            if self.position >= self.input.len() {
                tokens.push(Token::EOF);
                break;
            }

            let ch = self.input[self.position];

            let token = match ch {
                '+' => {
                    self.position += 1;
                    Token::Plus
                }
                '-' => {
                    self.position += 1;
                    Token::Minus
                }
                '*' => {
                    self.position += 1;
                    Token::Star
                }
                '/' => {
                    self.position += 1;
                    Token::Slash
                }

                '=' => {
                    if self.input.get(self.position + 1) == Some(&'=') {
                        self.position += 2;
                        Token::EqualEqual
                    } else {
                        self.position += 1;
                        Token::Equal
                    }
                }

                '!' => {
                    if self.input.get(self.position + 1) == Some(&'=') {
                        self.position += 2;
                        Token::BangEqual
                    } else {
                        self.position += 1;
                        Token::Bang
                    }
                }

                '&' => {
                    if self.input.get(self.position + 1) == Some(&'&') {
                        self.position += 2;
                        Token::AndAnd
                    } else {
                        panic!("Lexer ERR: 단독 '&'는 지원하지 않습니다.");
                    }
                }

                '|' => {
                    if self.input.get(self.position + 1) == Some(&'|') {
                        self.position += 2;
                        Token::OrOr
                    } else {
                        panic!("Lexer ERR: 단독 '|'는 지원하지 않습니다.");
                    }
                }

                '<' => {
                    if self.input.get(self.position + 1) == Some(&'=') {
                        self.position += 2;
                        Token::LessEqual
                    } else {
                        self.position += 1;
                        Token::Less
                    }
                }

                '>' => {
                    if self.input.get(self.position + 1) == Some(&'=') {
                        self.position += 2;
                        Token::GreaterEqual
                    } else {
                        self.position += 1;
                        Token::Greater
                    }
                }

                ':' => {
                    self.position += 1;
                    Token::Colon
                }
                '(' => {
                    self.position += 1;
                    Token::LeftParen
                }
                ')' => {
                    self.position += 1;
                    Token::RightParen
                }
                '{' => {
                    self.position += 1;
                    Token::LeftBrace
                }
                '}' => {
                    self.position += 1;
                    Token::RightBrace
                }

                '"' => Token::StringLiteral(self.read_string()),

                c if c.is_ascii_digit() => {
                    Token::Integer(self.read_number())
                }

                c if c.is_ascii_alphabetic() || c == '_' => {
                    self.read_identifier()
                }

                other => {
                    panic!("Lexer ERR: 알 수 없는 문자: {other}")
                }
            };

            tokens.push(token);
        }

        tokens
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len()
            && self.input[self.position].is_whitespace()
        {
            self.position += 1;
        }
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.position;

        while self.position < self.input.len() {
            let ch = self.input[self.position];

            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.position += 1;
            } else {
                break;
            }
        }

        let name: String = self.input[start..self.position]
            .iter()
            .collect();

        match name.as_str() {
            "main" => Token::Main,
            "script" => Token::Script,
            "fn" => Token::Fn,
            "let" => Token::Let,
            "return" => Token::Return,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "true" => Token::True,
            "false" => Token::False,
            _ => Token::Ident(name),
        }
    }

    fn read_number(&mut self) -> i64 {
        let start = self.position;

        while self.position < self.input.len()
            && self.input[self.position].is_ascii_digit()
        {
            self.position += 1;
        }

        let number: String = self.input[start..self.position]
            .iter()
            .collect();

        number
            .parse::<i64>()
            .unwrap_or_else(|_| {
                panic!("Lexer ERR: 정수 범위를 벗어났습니다: {number}")
            })
    }

    fn read_string(&mut self) -> String {
        // 여는 큰따옴표를 건너뜀.
        self.position += 1;

        let mut value = String::new();

        while self.position < self.input.len() {
            let ch = self.input[self.position];
            self.position += 1;

            match ch {
                '"' => return value,

                '\\' => {
                    if self.position >= self.input.len() {
                        panic!(
                            "Lexer ERR: 문자열 끝에 불완전한 이스케이프가 있습니다."
                        );
                    }

                    let escaped = self.input[self.position];
                    self.position += 1;

                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        'r' => value.push('\r'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        other => {
                            panic!(
                                "Lexer ERR: 지원하지 않는 이스케이프: \\{other}"
                            );
                        }
                    }
                }

                '\n' | '\r' => {
                    panic!(
                        "Lexer ERR: 문자열 안의 실제 줄바꿈은 지원하지 않습니다. \\n을 사용하세요."
                    );
                }

                other => value.push(other),
            }
        }

        panic!("Lexer ERR: 문자열의 닫는 따옴표가 없습니다.")
    }
}
