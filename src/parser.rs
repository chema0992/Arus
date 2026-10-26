
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

        Program { mode, functions }
    }

    fn parse_function(&mut self) -> Function {
        match self.advance() {
            Token::Fn => {}
            _ => panic!("Parser ERR: 함수 선언은 fn으로 시작해야 합니다."),
        }

        let name = match self.advance() {
            Token::Ident(name) => name,
            Token::Main => "main".to_string(),
            _ => panic!("Parser ERR: 함수 이름이 필요합니다."),
        };

        match self.advance() {
            Token::LeftParen => {}
            _ => panic!("Parser ERR: '('가 필요합니다."),
        }

        match self.advance() {
            Token::RightParen => {}
            _ => panic!("Parser ERR: ')'가 필요합니다."),
        }

        let body = self.parse_block();

        Function { name, body }
    }

    // { ... } 블록을 파싱한다.
    fn parse_block(&mut self) -> Vec<Stmt> {
        match self.advance() {
            Token::LeftBrace => {}
            _ => panic!("Parser ERR: 블록을 여는 '{{'가 필요합니다."),
        }

        let mut body = Vec::new();

        loop {
            match self.peek() {
                Token::RightBrace => {
                    self.advance();
                    break;
                }

                Token::EOF => {
                    panic!("Parser ERR: 블록의 닫는 '}}'가 없습니다.");
                }

                _ => body.push(self.parse_statement()),
            }
        }

        body
    }

    fn parse_statement(&mut self) -> Stmt {
        match self.peek() {
            Token::Let => self.parse_let(),
            Token::Return => self.parse_return(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::Ident(_) => self.parse_call_statement(),

            _ => panic!(
                "Parser ERR: 알 수 없는 문장입니다: {:?}",
                self.peek()
            ),
        }
    }

    // if 조건식 { ... } [else if ...] [else { ... }]
    fn parse_if(&mut self) -> Stmt {
        match self.advance() {
            Token::If => {}
            _ => unreachable!(),
        }

        let condition = self.parse_expression();
        let then_body = self.parse_block();

        let else_body = if *self.peek() == Token::Else {
            self.advance();
            Some(self.parse_else_body())
        } else {
            None
        };

        Stmt::If {
            condition,
            then_body,
            else_body,
        }
    }

    // else if의 연결 구조를 ElseBody AST로 만든다.
    fn parse_else_body(&mut self) -> ElseBody {
        if *self.peek() == Token::If {
            self.advance();

            let condition = self.parse_expression();
            let body = self.parse_block();

            let next = if *self.peek() == Token::Else {
                self.advance();
                Some(Box::new(self.parse_else_body()))
            } else {
                None
            };

            ElseBody::ElseIf {
                condition,
                body,
                next,
            }
        } else if *self.peek() == Token::LeftBrace {
            ElseBody::Else {
                body: self.parse_block(),
            }
        } else {
            panic!(
                "Parser ERR: else 뒤에는 if 또는 '{{'가 필요합니다. 현재 토큰: {:?}",
                self.peek()
            );
        }
    }

    fn parse_while(&mut self) -> Stmt {
        match self.advance() {
            Token::While => {}
            _ => unreachable!(),
        }

        let condition = self.parse_expression();
        let body = self.parse_block();

        Stmt::While {
            condition,
            body,
        }
    }

    fn parse_call_statement(&mut self) -> Stmt {
        let name = match self.advance() {
            Token::Ident(name) => name,
            _ => panic!("Parser ERR: 함수 이름이 필요합니다."),
        };

        match self.advance() {
            Token::LeftParen => {}
            _ => panic!("Parser ERR: '('가 필요합니다."),
        }

        let mut args = Vec::new();

        if *self.peek() != Token::RightParen {
            args.push(self.parse_expression());
        }

        match self.advance() {
            Token::RightParen => {}
            _ => panic!("Parser ERR: ')'가 필요합니다."),
        }

        Stmt::Call { name, args }
    }

    fn parse_let(&mut self) -> Stmt {
        self.advance();

        let name = match self.advance() {
            Token::Ident(name) => name,
            _ => panic!("Parser ERR: 변수 이름이 필요합니다."),
        };

        let ty = if *self.peek() == Token::Colon {
            self.advance();
            Some(self.parse_type())
        } else {
            None
        };

        match self.advance() {
            Token::Equal => {}
            _ => panic!("Parser ERR: '='가 필요합니다."),
        }

        let value = self.parse_expression();

        Stmt::Let { name, ty, value }
    }

    fn parse_return(&mut self) -> Stmt {
        self.advance();

        let value = self.parse_expression();

        Stmt::Return(value)
    }

    fn parse_type(&mut self) -> Type {
        match self.advance() {
            Token::Ident(name) if name == "i32" => Type::I32,
            Token::Ident(name) if name == "str" => Type::Str,
            Token::Ident(name) if name == "bool" => Type::Bool,
            Token::Ident(name) => Type::Named(name),
            _ => panic!("Parser ERR: 타입이 필요합니다."),
        }
    }

    // 비교 연산자: == != < > <= >=
    fn parse_expression(&mut self) -> Expr {
        let mut expr = self.parse_additive();

        loop {
            let operator = match self.peek() {
                Token::EqualEqual => BinaryOp::Equal,
                Token::BangEqual => BinaryOp::NotEqual,
                Token::Less => BinaryOp::Less,
                Token::Greater => BinaryOp::Greater,
                Token::LessEqual => BinaryOp::LessEqual,
                Token::GreaterEqual => BinaryOp::GreaterEqual,
                _ => break,
            };

            self.advance();

            let right = self.parse_additive();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    // 덧셈과 뺄셈
    fn parse_additive(&mut self) -> Expr {
        let mut expr = self.parse_term();

        loop {
            let operator = match self.peek() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance();

            let right = self.parse_term();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    // 곱셈과 나눗셈
    fn parse_term(&mut self) -> Expr {
        let mut expr = self.parse_factor();

        loop {
            let operator = match self.peek() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };

            self.advance();

            let right = self.parse_factor();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_factor(&mut self) -> Expr {
        match self.advance() {
            Token::Integer(value) => Expr::Integer(value),
            Token::StringLiteral(value) => Expr::String(value),
            Token::True => Expr::Bool(true),
            Token::False => Expr::Bool(false),
            Token::Ident(name) => Expr::Ident(name),

            Token::LeftParen => {
                let expr = self.parse_expression();

                match self.advance() {
                    Token::RightParen => expr,
                    _ => panic!("Parser ERR: ')'가 필요합니다."),
                }
            }

            _ => panic!("Parser ERR: 식이 필요합니다."),
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();

        if token != Token::EOF {
            self.position += 1;
        }

        token
    }
}

