
use crate::ast::*;
use std::collections::HashMap;

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

    If {
        condition: AirValue,
        then_body: Vec<AirInstruction>,
        else_body: Option<Vec<AirInstruction>>,
    },
}

#[derive(Debug, Clone)]
pub enum AirBinaryOp {
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
}

#[derive(Debug, Clone)]
pub enum AirValue {
    Integer(i64),
    Bool(bool),
    String(String),
    Variable(String),
    Temporary(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AirType {
    I32,
    Str,
    Bool,
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
        self.next_temp = 1;

        for stmt in &function.body {
            self.generate_stmt(stmt);
        }

        AirFunction {
            name: function.name.clone(),
            instructions: self.instructions.clone(),
        }
    }

    fn generate_block(&mut self, statements: &[Stmt]) -> Vec<AirInstruction> {
        let outer_instructions = std::mem::take(&mut self.instructions);

        for stmt in statements {
            self.generate_stmt(stmt);
        }

        std::mem::replace(&mut self.instructions, outer_instructions)
    }

    fn generate_if_block(
        &mut self,
        condition: &Expr,
        then_body: &[Stmt],
        else_body: Option<&ElseBody>,
    ) -> Vec<AirInstruction> {
        let outer_instructions = std::mem::take(&mut self.instructions);

        let condition = self.generate_expr(condition);
        let then_body = self.generate_block(then_body);

        let else_body = else_body.map(|body| self.generate_else_body(body));

        self.instructions.push(AirInstruction::If {
            condition,
            then_body,
            else_body,
        });

        std::mem::replace(&mut self.instructions, outer_instructions)
    }

    fn generate_else_body(&mut self, else_body: &ElseBody) -> Vec<AirInstruction> {
        match else_body {
            ElseBody::Else { body } => self.generate_block(body),

            ElseBody::ElseIf {
                condition,
                body,
                next,
            } => self.generate_if_block(condition, body, next.as_deref()),
        }
    }

    fn generate_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let value = self.generate_expr(value);

                let ty = ty.as_ref().map(|ty| match ty {
                    Type::I32 => AirType::I32,
                    Type::Str => AirType::Str,
                    Type::Bool => AirType::Bool,
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

            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                let instructions = self.generate_if_block(
                    condition,
                    then_body,
                    else_body.as_ref(),
                );

                self.instructions.extend(instructions);
            }
        }
    }

    fn generate_expr(&mut self, expr: &Expr) -> AirValue {
        match expr {
            Expr::Integer(value) => AirValue::Integer(*value),
            Expr::Bool(value) => AirValue::Bool(*value),
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
                    BinaryOp::Equal => AirBinaryOp::Equal,
                    BinaryOp::NotEqual => AirBinaryOp::NotEqual,
                    BinaryOp::Less => AirBinaryOp::Less,
                    BinaryOp::Greater => AirBinaryOp::Greater,
                    BinaryOp::LessEqual => AirBinaryOp::LessEqual,
                    BinaryOp::GreaterEqual => AirBinaryOp::GreaterEqual,
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
        let mut output = String::from("module {\n");

        for function in &program.functions {
            output.push_str(&format!("    {} {{\n", function.name));
            output.push_str(&Self::print_instructions(
                &function.instructions,
                8,
            ));
            output.push_str("    }\n");
        }

        output.push_str("}\n");
        output
    }

    fn print_instructions(
        instructions: &[AirInstruction],
        indent: usize,
    ) -> String {
        let mut output = String::new();

        for instruction in instructions {
            output.push_str(&Self::print_instruction(instruction, indent));
            output.push('\n');
        }

        output
    }

    fn print_instruction(instruction: &AirInstruction, indent: usize) -> String {
        let prefix = " ".repeat(indent);

        match instruction {
            AirInstruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                format!(
                    "{prefix}%{destination} = {} {} {}",
                    Self::operator_name(operator),
                    Self::print_value(left),
                    Self::print_value(right),
                )
            }

            AirInstruction::Let { name, ty, value } => match ty {
                Some(ty) => format!(
                    "{prefix}let {name}: {} = {}",
                    Self::type_name(ty),
                    Self::print_value(value),
                ),

                None => format!(
                    "{prefix}let {name} = {}",
                    Self::print_value(value),
                ),
            },

            AirInstruction::Return { value } => {
                format!("{prefix}return {}", Self::print_value(value))
            }

            AirInstruction::Call { function, args } => {
                let args = args
                    .iter()
                    .map(Self::print_value)
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{prefix}call {function}({args})")
            }

            AirInstruction::If {
                condition,
                then_body,
                else_body,
            } => {
                let mut output = format!(
                    "{prefix}if {} {{\n",
                    Self::print_value(condition),
                );

                output.push_str(&Self::print_instructions(
                    then_body,
                    indent + 4,
                ));

                output.push_str(&format!("{prefix}}}"));

                if let Some(else_body) = else_body {
                    output.push_str(" else {\n");

                    output.push_str(&Self::print_instructions(
                        else_body,
                        indent + 4,
                    ));

                    output.push_str(&format!("{prefix}}}"));
                }

                output
            }
        }
    }

    fn operator_name(operator: &AirBinaryOp) -> &'static str {
        match operator {
            AirBinaryOp::Add => "add",
            AirBinaryOp::Sub => "sub",
            AirBinaryOp::Mul => "mul",
            AirBinaryOp::Div => "div",
            AirBinaryOp::Equal => "eq",
            AirBinaryOp::NotEqual => "ne",
            AirBinaryOp::Less => "lt",
            AirBinaryOp::Greater => "gt",
            AirBinaryOp::LessEqual => "le",
            AirBinaryOp::GreaterEqual => "ge",
        }
    }

    fn print_value(value: &AirValue) -> String {
        match value {
            AirValue::Integer(value) => value.to_string(),
            AirValue::Bool(value) => value.to_string(),
            AirValue::String(value) => format!("{value:?}"),
            AirValue::Variable(name) => name.clone(),
            AirValue::Temporary(temp) => format!("%{temp}"),
        }
    }

    fn type_name(ty: &AirType) -> String {
        match ty {
            AirType::I32 => "i32".to_string(),
            AirType::Str => "str".to_string(),
            AirType::Bool => "bool".to_string(),
            AirType::Named(name) => name.clone(),
        }
    }
}

pub struct AirAnalyzer;

impl AirAnalyzer {
    pub fn analyze(program: &AirProgram) {
        for function in &program.functions {
            let mut variables = HashMap::new();
            let mut temporaries = HashMap::new();

            Self::check_instructions(
                &function.instructions,
                &mut variables,
                &mut temporaries,
            );
        }
    }

    fn check_instructions(
        instructions: &[AirInstruction],
        variables: &mut HashMap<String, AirType>,
        temporaries: &mut HashMap<u32, AirType>,
    ) {
        for instruction in instructions {
            match instruction {
                AirInstruction::Binary {
                    destination,
                    operator,
                    left,
                    right,
                } => {
                    let left_type = Self::value_type(left, variables, temporaries);
                    let right_type = Self::value_type(right, variables, temporaries);

                    let result_type = match operator {
                        AirBinaryOp::Add
                        | AirBinaryOp::Sub
                        | AirBinaryOp::Mul
                        | AirBinaryOp::Div => {
                            if left_type != AirType::I32
                                || right_type != AirType::I32
                            {
                                panic!(
                                    "AIR ERR: 산술 연산에는 i32 피연산자가 필요합니다."
                                );
                            }

                            AirType::I32
                        }

                        AirBinaryOp::Equal | AirBinaryOp::NotEqual => {
                            if left_type != right_type {
                                panic!(
                                    "AIR ERR: 비교 연산의 양쪽 타입이 다릅니다."
                                );
                            }

                            match left_type {
                                AirType::I32 | AirType::Bool => AirType::Bool,
                                _ => panic!(
                                    "AIR ERR: ==와 !=는 현재 i32와 bool만 지원합니다."
                                ),
                            }
                        }

                        AirBinaryOp::Less
                        | AirBinaryOp::Greater
                        | AirBinaryOp::LessEqual
                        | AirBinaryOp::GreaterEqual => {
                            if left_type != AirType::I32
                                || right_type != AirType::I32
                            {
                                panic!(
                                    "AIR ERR: 크기 비교에는 i32 피연산자가 필요합니다."
                                );
                            }

                            AirType::Bool
                        }
                    };

                    temporaries.insert(*destination, result_type);
                    println!("AIR CHECK: %{destination} OK");
                }

                AirInstruction::Let { name, ty, value } => {
                    let actual_type =
                        Self::value_type(value, variables, temporaries);

                    let variable_type = match ty {
                        Some(declared_type) => {
                            if *declared_type != actual_type {
                                panic!(
                                    "AIR ERR: 변수 '{name}'의 선언 타입과 값의 타입이 다릅니다."
                                );
                            }

                            declared_type.clone()
                        }

                        None => actual_type,
                    };

                    variables.insert(name.clone(), variable_type);
                    println!("AIR CHECK: let {name} OK");
                }

                AirInstruction::Return { value } => {
                    Self::value_type(value, variables, temporaries);
                    println!("AIR CHECK: return OK");
                }

                AirInstruction::Call { function, args } => {
                    if function != "print" && function != "println" {
                        panic!("AIR ERR: 지원하지 않는 함수: {function}");
                    }

                    if args.len() != 1 {
                        panic!("AIR ERR: 함수 인자는 하나여야 합니다.");
                    }

                    for arg in args {
                        Self::value_type(arg, variables, temporaries);
                    }

                    println!("AIR CHECK: call {function} OK");
                }

                AirInstruction::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let condition_type =
                        Self::value_type(condition, variables, temporaries);

                    if condition_type != AirType::Bool {
                        panic!(
                            "AIR ERR: if 조건식에는 bool 타입이 필요합니다."
                        );
                    }

                    let mut then_variables = variables.clone();
                    let mut then_temporaries = temporaries.clone();

                    Self::check_instructions(
                        then_body,
                        &mut then_variables,
                        &mut then_temporaries,
                    );

                    if let Some(else_body) = else_body {
                        let mut else_variables = variables.clone();
                        let mut else_temporaries = temporaries.clone();

                        Self::check_instructions(
                            else_body,
                            &mut else_variables,
                            &mut else_temporaries,
                        );
                    }

                    println!("AIR CHECK: if OK");
                }
            }
        }
    }

    fn value_type(
        value: &AirValue,
        variables: &HashMap<String, AirType>,
        temporaries: &HashMap<u32, AirType>,
    ) -> AirType {
        match value {
            AirValue::Integer(_) => AirType::I32,
            AirValue::Bool(_) => AirType::Bool,
            AirValue::String(_) => AirType::Str,

            AirValue::Variable(name) => variables
                .get(name)
                .cloned()
                .unwrap_or_else(|| {
                    panic!("AIR ERR: 정의되지 않은 변수: {name}")
                }),

            AirValue::Temporary(temp) => temporaries
                .get(temp)
                .cloned()
                .unwrap_or_else(|| {
                    panic!("AIR ERR: 정의되지 않은 임시값: %{temp}")
                }),
        }
    }
}

