#[derive(Debug)]
pub struct Program {
    pub mode: ProgramMode,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Let { name: String, ty: Option<Type>, value: Expr, mutable: bool },
    Return(Expr),
    Call { name: String, args: Vec<Expr> },
    If { condition: Expr, then_body: Vec<Stmt>, else_body: Option<ElseBody> },
    While { condition: Expr, body: Vec<Stmt> },
    For { name: String, start: Expr, end: Expr, body: Vec<Stmt> },
    Assign { name: String, value: Expr },
    AssignDeref { pointer: Expr, value: Expr },
}

#[derive(Debug)]
pub enum Expr {
    Integer(i64),
    String(String),
    Bool(bool),
    Ident(String),
    AddressOf(Box<Expr>),
    Dereference(Box<Expr>),
    Array(Vec<Expr>),
    Call { name: String, args: Vec<Expr> },
    #[allow(dead_code)]
    InterpolatedString { parts: Vec<StringPart> },
    Binary { left: Box<Expr>, operator: BinaryOp, right: Box<Expr> },
    Unary { operator: UnaryOp, operand: Box<Expr> },
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum StringPart {
    Text(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add, Sub, Mul, Div,
    Equal, NotEqual, Less, Greater, LessEqual, GreaterEqual,
    And, Or,
    ShiftLeft, ShiftRight, BitAnd, BitXor, BitOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp { Not }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerPermission {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I8, I16, I32, U8, U16, U32,
    Str, Bool,
    Named(String),
    Pointer {
        element_type: Box<Type>,
        permission: PointerPermission,
    },
    Array {
        element_type: Box<Type>,
        size: usize,
    },
}

#[derive(Debug)]
pub enum ElseBody {
    ElseIf { condition: Expr, body: Vec<Stmt>, next: Option<Box<ElseBody>> },
    Else { body: Vec<Stmt> },
}
