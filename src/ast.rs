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
    pub body: Vec<Stmt>,
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
}

#[derive(Debug)]
pub enum Expr {
    Integer(i64),
    String(String),
    Ident(String),

    // Reserved AST shape for future f"...{expression}..." support.
    // The lexer/parser do not construct this variant yet.
    #[allow(dead_code)]
    InterpolatedString {
        parts: Vec<StringPart>,
    },

    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },
}

// A future f-string is represented as alternating text and parsed expressions.
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
}

#[derive(Debug)]
pub enum Type {
    I32,
    Str,
    Named(String),
}
