use crate::ast::*;

#[derive(Debug)]
pub struct AirProgram {
    pub mode: AirMode,
    pub functions: Vec<AirFunction>,
}

#[derive(Debug)]
pub enum AirMode {
    Main,
    Script,
}

#[derive(Debug)]
pub struct AirFunction {
    pub name: String,
    pub instructions: Vec<AirInstruction>,
}

#[derive(Debug, Clone)]
pub enum AirInstruction {
    Binary {
        destination: u32,
        operator: AirBinaryOp,
        left: AirValue,
        right: AirValue,
    },

    Let {
        name: String,
        ty: Option<AirType>,
        value: AirValue,
    },

    Return {
        value: AirValue,
    },

    Call {
        function: String,
        args: Vec<AirValue>,
    },
}

#[derive(Debug, Clone)]
pub enum AirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub enum AirValue {
    Integer(i64),
    String(String),
    Variable(String),
    Temporary(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AirType {
    I32,
    Str,
    Named(String),
}

impl AirProgram {
    pub fn from_ast(program: &Program) -> Self {
        let mut generator = AirGenerator::new();
        generator.generate_program(program)
    }
}

#[derive(Debug)]
pub struct AirGenerator {
    next_temp: u32,
    instructions: Vec<AirInstruction>,
}

impl AirGenerator {
    pub fn new() -> Self {
        Self {
            next_temp: 1,
            instructions: Vec::new(),
        }
    }

    fn generate_program(&mut self, program: &Program) -> AirProgram {
        let mode = match program.mode {
            ProgramMode::Main => AirMode::Main,
            ProgramMode::Script => AirMode::Script,
        };

        let mut functions = Vec::new();

        for function in &program.functions {
            functions.push(self.generate_function(function));
        }

        AirProgram { mode, functions }
    }

    fn generate_function(&mut self, function: &Function) -> AirFunction {
        self.instructions.clear();

        for stmt in &function.body {
            self.generate_stmt(stmt);
        }

        AirFunction {
            name: function.name.clone(),
            instructions: self.instructions.clone(),
        }
    }

    fn generate_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let value = self.generate_expr(value);

                let ty = ty.as_ref().map(|ty| match ty {
                    Type::I32 => AirType::I32,
                    Type::Str => AirType::Str,
                    Type::Named(name) => AirType::Named(name.clone()),
                });

                self.instructions.push(AirInstruction::Let {
                    name: name.clone(),
                    ty,
                    value,
                });
            }

            Stmt::Return(value) => {
                let value = self.generate_expr(value);
                self.instructions.push(AirInstruction::Return { value });
            }

            Stmt::Call { name, args } => {
                let args = args
                    .iter()
                    .map(|arg| self.generate_expr(arg))
                    .collect();

                self.instructions.push(AirInstruction::Call {
                    function: name.clone(),
                    args,
                });
            }
        }
    }

    fn generate_expr(&mut self, expr: &Expr) -> AirValue {
        match expr {
            Expr::Integer(value) => AirValue::Integer(*value),
            Expr::String(value) => AirValue::String(value.clone()),
            Expr::Ident(name) => AirValue::Variable(name.clone()),
            Expr::InterpolatedString { .. } => {
                panic!("AIR ERR: f-string은 아직 구현되지 않았습니다.");
            }

            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.generate_expr(left);
                let right = self.generate_expr(right);

                let destination = self.next_temp;
                self.next_temp += 1;

                let operator = match operator {
                    BinaryOp::Add => AirBinaryOp::Add,
                    BinaryOp::Sub => AirBinaryOp::Sub,
                    BinaryOp::Mul => AirBinaryOp::Mul,
                    BinaryOp::Div => AirBinaryOp::Div,
                };

                self.instructions.push(AirInstruction::Binary {
                    destination,
                    operator,
                    left,
                    right,
                });

                AirValue::Temporary(destination)
            }
        }
    }
}

pub struct AirPrinter;

impl AirPrinter {
    pub fn print(program: &AirProgram) -> String {
        let mut output = String::new();
        output.push_str("module {\n");

        for function in &program.functions {
            output.push_str(&format!("    {} {{\n", function.name));

            for instruction in &function.instructions {
                output.push_str("        ");
                output.push_str(&Self::print_instruction(instruction));
                output.push('\n');
            }

            output.push_str("    }\n");
        }

        output.push_str("}\n");
        output
    }

    fn print_instruction(_instruction: &AirInstruction) -> String {
        todo!()
    }
}

pub struct AirAnalyzer;

impl AirAnalyzer {
    pub fn analyze(program: &AirProgram) {
        for function in &program.functions {
            for instruction in &function.instructions {
                Self::check_instruction(instruction);
            }
        }
    }

    fn check_instruction(instruction: &AirInstruction) {
        match instruction {
            AirInstruction::Binary {
                destination,
                left,
                right,
                ..
            } => {
                Self::check_value(left);
                Self::check_value(right);
                println!("AIR CHECK: %{destination} OK");
            }

            AirInstruction::Let { value, .. } => {
                Self::check_value(value);
                println!("AIR CHECK: let OK");
            }

            AirInstruction::Return { value } => {
                Self::check_value(value);
                println!("AIR CHECK: return OK");
            }

            AirInstruction::Call { function, args } => {
                if function != "print" && function != "println" {
                    panic!("AIR ERR: 지원하지 않는 함수: {}", function);
                }

                if args.len() != 1 {
                    panic!("AIR ERR: 함수 인자는 하나여야 합니다.");
                }

                for arg in args {
                    Self::check_value(arg);
                }

                println!("AIR CHECK: call {} OK", function);
            }
        }
    }

    fn check_value(value: &AirValue) {
        match value {
            AirValue::Integer(_) => {}
            AirValue::String(_) => {}
            AirValue::Variable(_) => {}
            AirValue::Temporary(_) => {}
        }
    }
}
