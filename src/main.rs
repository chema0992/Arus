use std::env;
use std::fs;
use std::process::Command;

mod air;
mod ast;
mod lexer;
mod llvm;
mod parser;
mod token;

use air::{AirAnalyzer, AirProgram};
use lexer::Lexer;
use llvm::LlvmGenerator;
use parser::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("사용법: arus <파일.arus>");
        return;
    }

    let filename = &args[1];

    let source = fs::read_to_string(filename)
        .expect("Arus 파일을 읽을 수 없습니다.");

    println!("=== SOURCE ===");
    println!("{}", source);

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    println!("=== TOKENS ===");
    for token in &tokens {
        println!("{:?}", token);
    }

    let mut parser = Parser::new(tokens);
    let ast = parser.parse();

    println!("=== AST ===");
    println!("{:#?}", ast);

    let air = AirProgram::from_ast(&ast);

    println!("=== AIR ===");
    println!("{:#?}", air);

    AirAnalyzer::analyze(&air);

    let llvm_ir = LlvmGenerator::generate(&air);

    fs::write("output.ll", &llvm_ir)
        .expect("LLVM IR 파일 저장 실패");

    println!("=== LLVM IR ===");
    println!("{}", llvm_ir);

    let status = Command::new("clang")
        .arg("output.ll")
        .arg("-o")
        .arg("arus_program")
        .status()
        .expect("clang 실행 실패");

    if !status.success() {
        panic!("LLVM 컴파일 또는 링크에 실패했습니다.");
    }

    println!("실행 파일 생성 완료: ./arus_program");
}