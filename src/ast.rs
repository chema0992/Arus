#[derive(Debug)]
pub struct Program {
    pub mode: ProgramMode,
    pub functions: Vec<Function>,
}

#[derive(Debug)]
pub enum ProgramMode {
    Main,
    Script,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<Type>,
        value: Expr,
    },
    Return(Expr),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<ElseBody>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
}

#[derive(Debug)]
pub enum Expr {
    Integer(i64),
    String(String),
    Bool(bool),
    Ident(String),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    // Reserved AST shape for future f"...{expression}..." support.
    #[allow(dead_code)]
    InterpolatedString {
        parts: Vec<StringPart>,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
    },
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum StringPart {
    Text(String),
    Expr(Box<Expr>),
}

#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitXor,
    BitOr,
}

#[derive(Debug)]
pub enum UnaryOp {
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    Str,
    Bool,
    Named(String),
}

#[derive(Debug)]
pub enum ElseBody {
    ElseIf {
        condition: Expr,
        body: Vec<Stmt>,
        next: Option<Box<ElseBody>>,
    },
    Else {
        body: Vec<Stmt>,
    },
}
