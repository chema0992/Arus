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
}

#[derive(Debug)]
pub enum Expr {
    Integer(i64),
    Ident(String),

    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },
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
    Named(String),
}
