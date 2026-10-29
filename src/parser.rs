use crate::ast::*;
use crate::token::Token;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn parse_assignment(&mut self) -> Stmt {
        let name = match self.peek() {
            Token::Ident(name) => name.clone(),
            _ => unreachable!(),
        };

        self.advance(); // 변수 이름
        self.advance(); // =

        let value = self.parse_expression();

        Stmt::Assign { name, value }
    }
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, position: 0 }
    }

    pub fn parse(&mut self) -> Program {
        let mode = match self.advance() {
            Token::Main => ProgramMode::Main,
            Token::Script => ProgramMode::Script,
            _ => panic!("Parser ERR: 프로그램 모드가 필요합니다."),
        };

        let mut functions = Vec::new();
        if mode == ProgramMode::Main {
            while *self.peek() != Token::EOF {
                if *self.peek() == Token::Semicolon {
                    self.advance();
                    continue;
                }
                functions.push(self.parse_function());
            }
        } else {
            // script 모드에서는 함수 선언 외의 최상위 문장을 순서대로 실행한다.
            // 기존 AST/LLVM 파이프라인을 재사용하도록 최상위 문장을 합성 main에 담는다.
            let mut script_body = Vec::new();
            while *self.peek() != Token::EOF {
                if *self.peek() == Token::Semicolon {
                    self.advance();
                    continue;
                }
                if *self.peek() == Token::Fn {
                    let function = self.parse_function();
                    if function.name == "main" {
                        panic!("Parser ERR: script 모드에서는 fn main()을 직접 선언하지 마세요. 최상위 문장이 자동으로 main에 들어갑니다.");
                    }
                    functions.push(function);
                } else {
                    script_body.push(self.parse_statement());
                }
            }

            functions.push(Function {
                name: "main".to_string(),
                params: Vec::new(),
                return_type: Some(Type::I32),
                body: script_body,
            });
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
            _ => panic!("Parser ERR: 함수 이름 뒤에 '('가 필요합니다."),
        }

        let mut params = Vec::new();
        if *self.peek() != Token::RightParen {
            loop {
                let param_name = match self.advance() {
                    Token::Ident(name) => name,
                    _ => panic!("Parser ERR: 매개변수 이름이 필요합니다."),
                };

                match self.advance() {
                    Token::Colon => {}
                    _ => panic!("Parser ERR: 매개변수 이름 뒤에 ':'가 필요합니다."),
                }

                let ty = self.parse_type();
                params.push(Parameter { name: param_name, ty });

                if *self.peek() == Token::Comma {
                    self.advance();
                    if *self.peek() == Token::RightParen {
                        panic!("Parser ERR: 매개변수 뒤에 불필요한 ','가 있습니다.");
                    }
                } else {
                    break;
                }
            }
        }

        match self.advance() {
            Token::RightParen => {}
            _ => panic!("Parser ERR: 매개변수 목록을 닫는 ')'가 필요합니다."),
        }

        let return_type = if *self.peek() == Token::Arrow {
            self.advance();
            Some(self.parse_type())
        } else {
            None
        };

        let body = self.parse_block();
        Function { name, params, return_type, body }
    }

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
                Token::Semicolon => {
                    self.advance();
                }
                Token::EOF => panic!("Parser ERR: 블록의 닫는 '}}'가 없습니다."),
                _ => body.push(self.parse_statement()),
            }
        }
        body
    }

    fn parse_statement(&mut self) -> Stmt {
        if matches!(self.peek(), Token::Ident(_))
            && self.tokens.get(self.position + 1) == Some(&Token::Equal)
        {
            return self.parse_assignment();
        }
        match self.peek() {
            Token::Let => self.parse_let(false),
            Token::Mut => self.parse_let(true),
            Token::Return => self.parse_return(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Ident(_) => self.parse_call_statement(),
            _ => panic!("Parser ERR: 알 수 없는 문장입니다: {:?}", self.peek()),
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

        Stmt::If { condition, then_body, else_body }
    }

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
            ElseBody::ElseIf { condition, body, next }
        } else if *self.peek() == Token::LeftBrace {
            ElseBody::Else { body: self.parse_block() }
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
        Stmt::While { condition, body }
    }

    fn parse_for(&mut self) -> Stmt {
        match self.advance() {
            Token::For => {}
            _ => unreachable!(),
        }

        let name = match self.advance() {
            Token::Ident(name) => name,
            _ => panic!("Parser ERR: for 뒤에 반복 변수 이름이 필요합니다."),
        };

        match self.advance() {
            Token::In => {}
            _ => panic!("Parser ERR: for 변수 뒤에 'in'이 필요합니다."),
        }

        let start = self.parse_expression();
        match self.advance() {
            Token::DotDot => {}
            _ => panic!("Parser ERR: for 범위는 시작..끝 형식이어야 합니다."),
        }
        let end = self.parse_expression();
        let body = self.parse_block();

        Stmt::For { name, start, end, body }
    }

    fn parse_call_statement(&mut self) -> Stmt {
        let name = match self.advance() {
            Token::Ident(name) => name,
            _ => panic!("Parser ERR: 함수 이름이 필요합니다."),
        };
        match self.advance() {
            Token::LeftParen => {}
            _ => panic!("Parser ERR: 함수 호출에 '('가 필요합니다."),
        }
        let args = self.parse_call_arguments();
        Stmt::Call { name, args }
    }

    // '('를 이미 소비한 상태에서 인자 목록을 파싱한다.
    fn parse_call_arguments(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        if *self.peek() != Token::RightParen {
            loop {
                args.push(self.parse_expression());
                if *self.peek() == Token::Comma {
                    self.advance();
                    if *self.peek() == Token::RightParen {
                        panic!("Parser ERR: 함수 인자 뒤에 불필요한 ','가 있습니다.");
                    }
                } else {
                    break;
                }
            }
        }

        match self.advance() {
            Token::RightParen => args,
            _ => panic!("Parser ERR: 함수 호출 인자 목록에 ')'가 필요합니다."),
        }
    }

    fn parse_array_literal(&mut self) -> Expr {
        let mut elements = Vec::new();

        if *self.peek() != Token::RightBracket {
            loop {
                elements.push(self.parse_expression());

                if *self.peek() == Token::Comma {
                    self.advance();

                    if *self.peek() == Token::RightBracket {
                        panic!("Parser ERR: 배열 원소 뒤에 불필요한 ','가 있습니다.");
                    }
                } else {
                    break;
                }
            }
        }

        match self.advance() {
            Token::RightBracket => {}
            _ => panic!("Parser ERR: 배열 리터럴에 ']'가 필요합니다."),
        }

        Expr::Array(elements)
    }

    fn parse_let(&mut self, mutable: bool) -> Stmt {
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
        Stmt::Let {
            name,
            ty,
            value,
            mutable,
        }
    }

    fn parse_return(&mut self) -> Stmt {
        self.advance();
        let value = self.parse_expression();
        Stmt::Return(value)
    }

    fn parse_type(&mut self) -> Type {
        match self.advance() {
            Token::LeftBracket => {
                let element_type = self.parse_type();

                match self.advance() {
                    Token::Semicolon => {}
                    _ => panic!("Parser ERR: 배열 타입에는 ';'가 필요합니다."),
                }

                let size = match self.advance() {
                    Token::Integer(value) => {
                        if value < 0 {
                            panic!("Parser ERR: 배열 크기는 음수일 수 없습니다.");
                        }

                        value as usize
                    }
                    _ => panic!("Parser ERR: 배열 크기는 정수여야 합니다."),
                };

                match self.advance() {
                    Token::RightBracket => {}
                    _ => panic!("Parser ERR: 배열 타입에는 ']'가 필요합니다."),
                }

                Type::Array {
                    element_type: Box::new(element_type),
                    size,
                }
            }
            Token::Ident(name) if name == "i8" => Type::I8,
            Token::Ident(name) if name == "i16" => Type::I16,
            Token::Ident(name) if name == "i32" => Type::I32,
            Token::Ident(name) if name == "u8" => Type::U8,
            Token::Ident(name) if name == "u16" => Type::U16,
            Token::Ident(name) if name == "u32" => Type::U32,
            Token::Ident(name) if name == "str" => Type::Str,
            Token::Ident(name) if name == "bool" => Type::Bool,
            Token::Ident(name) => Type::Named(name),
            _ => panic!("Parser ERR: 타입이 필요합니다."),
        }
    }

    fn parse_expression(&mut self) -> Expr {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Expr {
        let mut expr = self.parse_logical_and();
        loop {
            if *self.peek() != Token::OrOr { break; }
            self.advance();
            let right = self.parse_logical_and();
            expr = Expr::Binary {
                left: Box::new(expr), operator: BinaryOp::Or, right: Box::new(right),
            };
        }
        expr
    }

    fn parse_logical_and(&mut self) -> Expr {
        let mut expr = self.parse_bit_or();
        loop {
            if *self.peek() != Token::AndAnd { break; }
            self.advance();
            let right = self.parse_bit_or();
            expr = Expr::Binary {
                left: Box::new(expr), operator: BinaryOp::And, right: Box::new(right),
            };
        }
        expr
    }

    fn parse_bit_or(&mut self) -> Expr {
        let mut expr = self.parse_bit_xor();

        while *self.peek() == Token::Pipe {
            self.advance();
            let right = self.parse_bit_xor();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::BitOr,
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_bit_xor(&mut self) -> Expr {
        // Equality is intentionally lower precedence than bitwise AND,
        // so `i & 1 == 0` parses as `(i & 1) == 0`.
        let mut expr = self.parse_equality();

        while *self.peek() == Token::Caret {
            self.advance();
            let right = self.parse_equality();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::BitXor,
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_bit_and(&mut self) -> Expr {
        let mut expr = self.parse_comparison();

        while *self.peek() == Token::Ampersand {
            self.advance();
            let right = self.parse_comparison();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::BitAnd,
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_equality(&mut self) -> Expr {
        let mut expr = self.parse_bit_and();

        loop {
            let operator = match self.peek() {
                Token::EqualEqual => BinaryOp::Equal,
                Token::BangEqual => BinaryOp::NotEqual,
                _ => break,
            };

            self.advance();
            let right = self.parse_bit_and();

            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        expr
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut expr = self.parse_shift();
        loop {
            let operator = match self.peek() {
                Token::Less => BinaryOp::Less,
                Token::Greater => BinaryOp::Greater,
                Token::LessEqual => BinaryOp::LessEqual,
                Token::GreaterEqual => BinaryOp::GreaterEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) };
        }
        expr
    }

    fn parse_shift(&mut self) -> Expr {
        let mut expr = self.parse_additive();

        loop {
            let operator = match self.peek() {
                Token::ShiftLeft => BinaryOp::ShiftLeft,
                Token::ShiftRight => BinaryOp::ShiftRight,
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
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) };
        }
        expr
    }

    fn parse_term(&mut self) -> Expr {
        let mut expr = self.parse_unary();
        loop {
            let operator = match self.peek() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) };
        }
        expr
    }

    fn parse_unary(&mut self) -> Expr {
        if *self.peek() == Token::Bang {
            self.advance();
            let operand = self.parse_unary();
            return Expr::Unary { operator: UnaryOp::Not, operand: Box::new(operand) };
        }
        self.parse_factor()
    }

    fn parse_factor(&mut self) -> Expr {
        match self.advance() {
            Token::Integer(value) => Expr::Integer(value),
            Token::StringLiteral(value) => Expr::String(value),
            Token::True => Expr::Bool(true),
            Token::False => Expr::Bool(false),

            Token::LeftBracket => self.parse_array_literal(),

            Token::Ident(name) => {
                if *self.peek() == Token::LeftParen {
                    self.advance();
                    let args = self.parse_call_arguments();
                    Expr::Call { name, args }
                } else {
                    Expr::Ident(name)
                }
            }

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
