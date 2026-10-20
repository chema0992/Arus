use crate::ast::*;
use crate::token::Token;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Program {
        let mode = match self.advance() {
            Token::Main => ProgramMode::Main,
            Token::Script => ProgramMode::Script,
            _ => panic!("Parser ERR: 프로그램 모드가 필요합니다."),
        };

        let mut functions = Vec::new();

        while *self.peek() != Token::EOF {
            functions.push(self.parse_function());
        }

        Program {
            mode,
            functions,
        }
    }

    fn parse_function(&mut self) -> Function {
        match self.advance() {
            Token::Fn => {}
            _ => panic!("ERR: 함수 선언은 fn으로 시작해야 합니다."),
        }

        // main은 Lexer에서 Token::Main으로 처리됨
        let name = match self.advance() {
            Token::Ident(name) => name,
            Token::Main => "main".to_string(),
            _ => panic!("ERR: 함수 이름이 필요합니다."),
        };

        match self.advance() {
            Token::LeftParen => {}
            _ => panic!("ERR: '('가 필요합니다."),
        }

        match self.advance() {
            Token::RightParen => {}
            _ => panic!("ERR: ')'가 필요합니다."),
        }

        match self.advance() {
            Token::LeftBrace => {}
            _ => panic!("ERR: 함수 본문을 여는 '{{'가 필요합니다."),
        }

        // 함수 본문 파싱
        let mut body = Vec::new();

        while *self.peek() != Token::RightBrace {
            if *self.peek() == Token::EOF {
                panic!("ERR: 함수 본문이 닫히지 않았습니다.");
            }

            body.push(self.parse_statement());
        }

        self.advance(); // }

        Function {
            name,
            body,
        }
    }

    fn parse_statement(&mut self) -> Stmt {
        match self.peek() {
            Token::Let => self.parse_let(),
            _ => panic!("ERR: 알 수 없는 문장"),
        }
    }

    fn parse_let(&mut self) -> Stmt {
        self.advance(); // let

        let name = match self.advance() {
            Token::Ident(name) => name,
            _ => panic!("ERR: 변수 이름이 필요합니다."),
        };

        let ty = if *self.peek() == Token::Colon {
            self.advance(); // :
            Some(self.parse_type())
        } else {
            None
        };

        match self.advance() {
            Token::Equal => {}
            _ => panic!("ERR: '='가 필요합니다."),
        }

        let value = self.parse_expression();

        Stmt::Let {
            name,
            ty,
            value,
        }
    }

    fn parse_type(&mut self) -> Type {
        match self.advance() {
            Token::Ident(name) if name == "i32" => Type::I32,
            Token::Ident(name) => Type::Named(name),
            _ => panic!("ERR: 타입이 필요합니다."),
        }
    }

    fn parse_expression(&mut self) -> Expr {
        let mut expr = self.parse_term();

        loop {
            let op = match self.peek() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance();

            let right = self.parse_term();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_term(&mut self) -> Expr {
        let mut expr = self.parse_factor();

        loop {
            let op = match self.peek() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };

            self.advance();

            let right = self.parse_factor();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_factor(&mut self) -> Expr {
        match self.advance() {
            Token::Integer(value) => Expr::Integer(value),

            Token::Ident(name) => Expr::Ident(name),

            Token::LeftParen => {
                let expr = self.parse_expression();

                match self.advance() {
                    Token::RightParen => expr,
                    _ => panic!("ERR: ')'가 필요합니다."),
                }
            }

            _ => panic!("ERR: 식이 필요합니다."),
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();

        // EOF 다음으로 이동하지 않음
        if token != Token::EOF {
            self.position += 1;
        }

        token
    }
}