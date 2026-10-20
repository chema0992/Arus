mod token;
mod lexer;
mod ast;
mod parser;
mod air;

use lexer::Lexer;
use parser::Parser;
use crate::air::AirPrinter;
use air::{AirAnalyzer, AirProgram};

fn main() {
    let source = r#"
main

fn main() {
    let x: i32 = (10 + 10) * 3
    let y = x + (10 * 2) / 3
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let program = parser.parse();

    let air = AirProgram::from_ast(&program);
    AirAnalyzer::analyze(&air);
}