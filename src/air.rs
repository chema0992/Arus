
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
    pub params: Vec<AirParameter>,
    pub return_type: AirType,
    pub instructions: Vec<AirInstruction>,
}

#[derive(Debug, Clone)]
pub struct AirParameter {
    pub name: String,
    pub ty: AirType,
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
        destination: Option<u32>,
        function: String,
        args: Vec<AirValue>,
    },

    If {
        condition: AirValue,
        then_body: Vec<AirInstruction>,
        else_body: Option<Vec<AirInstruction>>,
    },

    While {
        condition_instructions: Vec<AirInstruction>,
        condition: AirValue,
        body: Vec<AirInstruction>,
    },

    Unary {
        destination: u32,
        operator: AirUnaryOp,
        operand: AirValue,
    },

    Logical {
        destination: u32,
        operator: AirLogicalOp,
        left_instructions: Vec<AirInstruction>,
        left: AirValue,
        right_instructions: Vec<AirInstruction>,
        right: AirValue,
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
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitXor,
    BitOr,
}

#[derive(Debug, Clone)]
pub enum AirUnaryOp {
    Not,
}

#[derive(Debug, Clone)]
pub enum AirLogicalOp {
    And,
    Or,
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
    Void,
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

        let params = function.params.iter().map(|param| AirParameter {
            name: param.name.clone(),
            ty: Self::convert_type(&param.ty),
        }).collect();

        // 생략된 반환형은 일반 함수에서는 void로, main에서는 i32로 취급한다.
        // 따라서 사용자가 제시한 fn main() { return x } 문법도 지원한다.
        let return_type = function.return_type.as_ref().map(Self::convert_type)
            .unwrap_or_else(|| {
                if function.name == "main" { AirType::I32 } else { AirType::Void }
            });

        AirFunction {
            name: function.name.clone(),
            params,
            return_type,
            instructions: self.instructions.clone(),
        }
    }

    fn convert_type(ty: &Type) -> AirType {
        match ty {
            Type::I32 => AirType::I32,
            Type::Str => AirType::Str,
            Type::Bool => AirType::Bool,
            Type::Named(name) => AirType::Named(name.clone()),
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

        fn generate_while_block(&mut self, condition: &Expr, body: &[Stmt]) -> Vec<AirInstruction> {
            let outer_instructions = std::mem::take(&mut self.instructions);

            // 조건식을 계산하며 생성된 AIR 명령을 별도로 저장한다.
            let condition = self.generate_expr(condition);
            let condition_instructions =
                std::mem::take(&mut self.instructions);

            // 반복문 본문을 별도의 AIR 명령 목록으로 생성한다.
            let body = self.generate_block(body);

            self.instructions.push(AirInstruction::While {
                condition_instructions,
                condition,
                body,
            });

            std::mem::replace(
                &mut self.instructions,
                outer_instructions,
            )
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
                    destination: None,
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

            Stmt::While {
                condition,
                body,
            } => {
                let instructions = self.generate_while_block(condition, body);

                self.instructions.extend(instructions);
            }
        }
    }

    fn generate_expr_isolated(
        &mut self,
        expr: &Expr,
    ) -> (Vec<AirInstruction>, AirValue) {
        let outer_instructions =
            std::mem::take(&mut self.instructions);

        let value = self.generate_expr(expr);

        let generated_instructions =
            std::mem::replace(
                &mut self.instructions,
                outer_instructions,
            );

        (generated_instructions, value)
    }

    fn generate_logical_expr(
        &mut self,
        left: &Expr,
        operator: AirLogicalOp,
        right: &Expr,
    ) -> AirValue {
        let (left_instructions, left_value) =
            self.generate_expr_isolated(left);

        let (right_instructions, right_value) =
            self.generate_expr_isolated(right);

        let destination = self.next_temp;
        self.next_temp += 1;

        self.instructions.push(AirInstruction::Logical {
            destination,
            operator,
            left_instructions,
            left: left_value,
            right_instructions,
            right: right_value,
        });

        AirValue::Temporary(destination)
    }

    fn generate_expr(&mut self, expr: &Expr) -> AirValue {
        match expr {
            Expr::Integer(value) => AirValue::Integer(*value),
            Expr::Bool(value) => AirValue::Bool(*value),
            Expr::String(value) => AirValue::String(value.clone()),
            Expr::Ident(name) => AirValue::Variable(name.clone()),

            Expr::Call { name, args } => {
                let args = args.iter().map(|arg| self.generate_expr(arg)).collect();
                let destination = self.next_temp;
                self.next_temp += 1;
                self.instructions.push(AirInstruction::Call {
                    destination: Some(destination),
                    function: name.clone(),
                    args,
                });
                AirValue::Temporary(destination)
            }

            Expr::InterpolatedString { .. } => {
                panic!("AIR ERR: f-string은 아직 구현되지 않았습니다.");
            }

                        Expr::Unary {
                operator,
                operand,
            } => {
                let operand = self.generate_expr(operand);

                let destination = self.next_temp;
                self.next_temp += 1;

                let operator = match operator {
                    UnaryOp::Not => AirUnaryOp::Not,
                };

                self.instructions.push(AirInstruction::Unary {
                    destination,
                    operator,
                    operand,
                });

                AirValue::Temporary(destination)
            }

            Expr::Binary {
                left,
                operator: BinaryOp::And,
                right,
            } => {
                self.generate_logical_expr(
                    left,
                    AirLogicalOp::And,
                    right,
                )
            }

            Expr::Binary {
                left,
                operator: BinaryOp::Or,
                right,
            } => {
                self.generate_logical_expr(
                    left,
                    AirLogicalOp::Or,
                    right,
                )
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
                    BinaryOp::ShiftLeft => AirBinaryOp::ShiftLeft,
                    BinaryOp::ShiftRight => AirBinaryOp::ShiftRight,
                    BinaryOp::BitAnd => AirBinaryOp::BitAnd,
                    BinaryOp::BitXor => AirBinaryOp::BitXor,
                    BinaryOp::BitOr => AirBinaryOp::BitOr,
                    BinaryOp::And | BinaryOp::Or => {
                        unreachable!(
                            "논리 연산은 generate_logical_expr에서 처리해야 합니다."
                        );
                    }
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
            let params = function.params.iter()
                .map(|param| format!("{}: {}", param.name, Self::type_name(&param.ty)))
                .collect::<Vec<_>>().join(", ");
            output.push_str(&format!(
                "    fn {}({}) -> {} {{\n",
                function.name, params, Self::type_name(&function.return_type)
            ));
            output.push_str(&Self::print_instructions(&function.instructions, 8));
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

            AirInstruction::Unary {
                destination,
                operator,
                operand,
            } => {
                let operator = match operator {
                    AirUnaryOp::Not => "not",
                };

                format!(
                    "{prefix}%{destination} = {operator} {}",
                    Self::print_value(operand),
                )
            }

            AirInstruction::Logical {
                destination,
                operator,
                left_instructions,
                left,
                right_instructions,
                right,
            } => {
                let operator = match operator {
                    AirLogicalOp::And => "and",
                    AirLogicalOp::Or => "or",
                };

                let mut output = format!(
                    "{prefix}%{destination} = logical_{operator} {} {}\n",
                    Self::print_value(left),
                    Self::print_value(right),
                );

                output.push_str(
                    &format!("{prefix}    left {{\n")
                );

                output.push_str(
                    &Self::print_instructions(
                        left_instructions,
                        indent + 8,
                    )
                );

                output.push_str(
                    &format!("{prefix}    }}\n")
                );

                output.push_str(
                    &format!("{prefix}    right {{\n")
                );

                output.push_str(
                    &Self::print_instructions(
                        right_instructions,
                        indent + 8,
                    )
                );

                output.push_str(
                    &format!("{prefix}    }}")
                );

                output
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

            AirInstruction::Call { destination, function, args } => {
                let args = args.iter().map(Self::print_value)
                    .collect::<Vec<_>>().join(", ");
                match destination {
                    Some(destination) => format!("{prefix}%{destination} = call {function}({args})"),
                    None => format!("{prefix}call {function}({args})"),
                }
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

            AirInstruction::While {
                condition_instructions,
                condition,
                body,
            } => {
                let mut output =
                    format!("{prefix}while {{\n");

                output.push_str(
                    &format!("{prefix}    condition {{\n")
                );

                output.push_str(
                    &Self::print_instructions(
                        condition_instructions,
                        indent + 8,
                    )
                );

                output.push_str(
                    &format!(
                        "{prefix}        branch_if {}\n",
                        Self::print_value(condition),
                    )
                );

                output.push_str(
                    &format!("{prefix}    }}\n")
                );

                output.push_str(
                    &format!("{prefix}    body {{\n")
                );

                output.push_str(
                    &Self::print_instructions(body, indent + 8)
                );

                output.push_str(
                    &format!("{prefix}    }}\n")
                );

                output.push_str(
                    &format!("{prefix}}}")
                );

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
            AirBinaryOp::ShiftLeft => "shl",
            AirBinaryOp::ShiftRight => "ashr",
            AirBinaryOp::BitAnd => "and",
            AirBinaryOp::BitXor => "xor",
            AirBinaryOp::BitOr => "or",
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
            AirType::Void => "void".to_string(),
            AirType::Named(name) => name.clone(),
        }
    }
}

pub struct AirAnalyzer;

impl AirAnalyzer {
    // AirTypeChecker가 실제 의미/타입 검사를 수행한다.
    // Analyzer는 이후 메모리/권한 분석이 추가될 자리를 유지하고,
    // 현재는 각 함수가 검사 파이프라인을 통과했음을 표시한다.
    pub fn analyze(program: &AirProgram) {
        for function in &program.functions {
            println!(
                "AIR ANALYZER: fn {}({} params) -> {:?}",
                function.name,
                function.params.len(),
                function.return_type
            );
        }
        println!("AIR ANALYZER: 완료");
    }
}

pub struct AirTypeChecker;

type FunctionSignatures = HashMap<String, (Vec<AirType>, AirType)>;

impl AirTypeChecker {
    pub fn check(program: &AirProgram) -> Result<(), String> {
        let mut signatures = FunctionSignatures::new();

        // 모든 시그니처를 먼저 등록하므로 함수의 선언 순서와 재귀 호출에 의존하지 않는다.
        for function in &program.functions {
            if function.name == "print" || function.name == "println" {
                return Err(format!(
                    "AIR Type ERR: '{}'는 내장 함수 이름이므로 재정의할 수 없습니다.",
                    function.name
                ));
            }

            let signature = (
                function.params.iter().map(|param| param.ty.clone()).collect(),
                function.return_type.clone(),
            );

            if signatures.insert(function.name.clone(), signature).is_some() {
                return Err(format!(
                    "AIR Type ERR: 함수 '{}'가 중복 선언되었습니다.",
                    function.name
                ));
            }
        }

        for function in &program.functions {
            if function.name == "main" {
                if !function.params.is_empty() {
                    return Err("AIR Type ERR: main 함수는 매개변수를 받을 수 없습니다.".to_string());
                }
                if function.return_type != AirType::I32 {
                    return Err("AIR Type ERR: main 함수의 반환형은 i32여야 합니다.".to_string());
                }
            }

            for param in &function.params {
                if matches!(&param.ty, AirType::Named(_) | AirType::Void) {
                    return Err(format!(
                        "AIR Type ERR: 매개변수 '{}'의 타입 {:?}은 아직 지원되지 않습니다.",
                        param.name, param.ty
                    ));
                }
            }
            if matches!(&function.return_type, AirType::Named(_)) {
                return Err(format!(
                    "AIR Type ERR: 함수 '{}'의 반환형 {:?}은 아직 지원되지 않습니다.",
                    function.name, function.return_type
                ));
            }

            let mut variables = HashMap::<String, AirType>::new();
            let mut temporaries = HashMap::<u32, AirType>::new();

            for param in &function.params {
                if variables.insert(param.name.clone(), param.ty.clone()).is_some() {
                    return Err(format!(
                        "함수 '{}': AIR Type ERR: 매개변수 '{}'가 중복되었습니다.",
                        function.name, param.name
                    ));
                }
            }

            Self::check_instructions(
                &function.instructions,
                &mut variables,
                &mut temporaries,
                &signatures,
                &function.return_type,
            )
            .map_err(|error| format!("함수 '{}': {}", function.name, error))?;
        }

        Ok(())
    }

    fn value_type(
        value: &AirValue,
        variables: &HashMap<String, AirType>,
        temporaries: &HashMap<u32, AirType>,
    ) -> Result<AirType, String> {
        match value {
            AirValue::Integer(_) => Ok(AirType::I32),
            AirValue::Bool(_) => Ok(AirType::Bool),
            AirValue::String(_) => Ok(AirType::Str),
            AirValue::Variable(name) => variables.get(name).cloned().ok_or_else(|| {
                format!("AIR Type ERR: 선언되지 않은 변수 '{}'", name)
            }),
            AirValue::Temporary(id) => temporaries.get(id).cloned().ok_or_else(|| {
                format!("AIR Type ERR: 타입을 알 수 없는 임시값 %{}", id)
            }),
        }
    }

    fn check_instructions(
        instructions: &[AirInstruction],
        variables: &mut HashMap<String, AirType>,
        temporaries: &mut HashMap<u32, AirType>,
        signatures: &FunctionSignatures,
        return_type: &AirType,
    ) -> Result<(), String> {
        for instruction in instructions {
            match instruction {
                AirInstruction::Binary { destination, operator, left, right } => {
                    let left_type = Self::value_type(left, variables, temporaries)?;
                    let right_type = Self::value_type(right, variables, temporaries)?;

                    let result_type = match operator {
                        AirBinaryOp::Add | AirBinaryOp::Sub | AirBinaryOp::Mul | AirBinaryOp::Div | AirBinaryOp::ShiftLeft | AirBinaryOp::ShiftRight | AirBinaryOp::BitAnd | AirBinaryOp::BitXor | AirBinaryOp::BitOr=> {
                            if left_type != AirType::I32 || right_type != AirType::I32 {
                                return Err(format!(
                                    "AIR Type ERR: {:?} 연산은 i32가 필요합니다. 왼쪽: {:?}, 오른쪽: {:?}",
                                    operator, left_type, right_type
                                ));
                            }
                            AirType::I32
                        }
                        AirBinaryOp::Less | AirBinaryOp::Greater | AirBinaryOp::LessEqual | AirBinaryOp::GreaterEqual => {
                            if left_type != AirType::I32 || right_type != AirType::I32 {
                                return Err(format!(
                                    "AIR Type ERR: 크기 비교는 i32가 필요합니다. 왼쪽: {:?}, 오른쪽: {:?}",
                                    left_type, right_type
                                ));
                            }
                            AirType::Bool
                        }
                        AirBinaryOp::Equal | AirBinaryOp::NotEqual => {
                            if left_type != right_type {
                                return Err(format!(
                                    "AIR Type ERR: {:?} 연산의 양쪽 타입이 다릅니다. 왼쪽: {:?}, 오른쪽: {:?}",
                                    operator, left_type, right_type
                                ));
                            }
                            match left_type {
                                AirType::I32 | AirType::Bool => AirType::Bool,
                                _ => return Err(format!(
                                    "AIR Type ERR: ==와 !=는 현재 i32와 bool만 지원합니다. 실제 타입: {:?}",
                                    left_type
                                )),
                            }
                        }
                        
                    };
                    temporaries.insert(*destination, result_type);
                }

                AirInstruction::Unary { destination, operator, operand } => {
                    let operand_type = Self::value_type(operand, variables, temporaries)?;
                    match operator {
                        AirUnaryOp::Not if operand_type != AirType::Bool => {
                            return Err(format!(
                                "AIR Type ERR: ! 연산은 bool이 필요합니다. 실제 타입: {:?}",
                                operand_type
                            ));
                        }
                        AirUnaryOp::Not => {}
                    }
                    temporaries.insert(*destination, AirType::Bool);
                }

                AirInstruction::Logical {
                    destination, operator, left_instructions, left, right_instructions, right,
                } => {
                    let mut left_variables = variables.clone();
                    let mut left_temporaries = temporaries.clone();
                    Self::check_instructions(
                        left_instructions, &mut left_variables, &mut left_temporaries,
                        signatures, return_type,
                    )?;
                    let left_type = Self::value_type(left, &left_variables, &left_temporaries)?;
                    if left_type != AirType::Bool {
                        return Err(format!(
                            "AIR Type ERR: {:?} 연산의 왼쪽 값은 bool이어야 합니다. 실제 타입: {:?}",
                            operator, left_type
                        ));
                    }

                    let mut right_variables = variables.clone();
                    let mut right_temporaries = temporaries.clone();
                    Self::check_instructions(
                        right_instructions, &mut right_variables, &mut right_temporaries,
                        signatures, return_type,
                    )?;
                    let right_type = Self::value_type(right, &right_variables, &right_temporaries)?;
                    if right_type != AirType::Bool {
                        return Err(format!(
                            "AIR Type ERR: {:?} 연산의 오른쪽 값은 bool이어야 합니다. 실제 타입: {:?}",
                            operator, right_type
                        ));
                    }
                    temporaries.insert(*destination, AirType::Bool);
                }

                AirInstruction::Let { name, ty, value } => {
                    let actual_type = Self::value_type(value, variables, temporaries)?;
                    if actual_type == AirType::Void {
                        return Err(format!(
                            "AIR Type ERR: void 함수 호출 결과를 변수 '{}'에 저장할 수 없습니다.", name
                        ));
                    }
                    let variable_type = if let Some(expected_type) = ty {
                        if expected_type != &actual_type {
                            return Err(format!(
                                "AIR Type ERR: 변수 '{}'의 타입이 일치하지 않습니다. 선언: {:?}, 실제: {:?}",
                                name, expected_type, actual_type
                            ));
                        }
                        expected_type.clone()
                    } else {
                        actual_type
                    };
                    variables.insert(name.clone(), variable_type);
                }

                AirInstruction::Return { value } => {
                    let actual_type = Self::value_type(value, variables, temporaries)?;
                    if *return_type == AirType::Void {
                        return Err(
                            "AIR Type ERR: 반환형이 생략된 일반 함수(void)에서는 값을 반환할 수 없습니다."
                                .to_string(),
                        );
                    }
                    if actual_type != *return_type {
                        return Err(format!(
                            "AIR Type ERR: 반환 타입이 일치하지 않습니다. 선언: {:?}, 실제: {:?}",
                            return_type, actual_type
                        ));
                    }
                }

                AirInstruction::Call { destination, function, args } => {
                    let arg_types = args.iter()
                        .map(|arg| Self::value_type(arg, variables, temporaries))
                        .collect::<Result<Vec<_>, _>>()?;

                    if function == "print" || function == "println" {
                        if destination.is_some() {
                            return Err(format!(
                                "AIR Type ERR: 내장 함수 '{}'는 식의 값으로 사용할 수 없습니다.", function
                            ));
                        }
                        if args.len() != 1 {
                            return Err(format!(
                                "AIR Type ERR: '{}'에는 인자 하나가 필요합니다.", function
                            ));
                        }
                        if arg_types[0] != AirType::I32
                            && arg_types[0] != AirType::Str
                            && arg_types[0] != AirType::Bool
                        {
                            return Err(format!(
                                "AIR Type ERR: '{}'에서 지원하지 않는 인자 타입입니다: {:?}",
                                function, arg_types[0]
                            ));
                        }
                    } else {
                        let (param_types, result_type) = signatures.get(function).ok_or_else(|| {
                            format!("AIR Type ERR: 선언되지 않은 함수 '{}'", function)
                        })?;

                        if arg_types.len() != param_types.len() {
                            return Err(format!(
                                "AIR Type ERR: 함수 '{}'는 인자 {}개를 받지만 {}개가 전달되었습니다.",
                                function, param_types.len(), arg_types.len()
                            ));
                        }
                        for (index, (actual, expected)) in arg_types.iter().zip(param_types).enumerate() {
                            if actual != expected {
                                return Err(format!(
                                    "AIR Type ERR: 함수 '{}'의 {}번째 인자 타입이 다릅니다. 기대: {:?}, 실제: {:?}",
                                    function, index + 1, expected, actual
                                ));
                            }
                        }

                        if let Some(destination) = destination {
                            if *result_type == AirType::Void {
                                return Err(format!(
                                    "AIR Type ERR: 반환값이 없는 함수 '{}'를 식 안에서 사용할 수 없습니다.",
                                    function
                                ));
                            }
                            temporaries.insert(*destination, result_type.clone());
                        }
                    }
                }

                AirInstruction::If { condition, then_body, else_body } => {
                    let condition_type = Self::value_type(condition, variables, temporaries)?;
                    if condition_type != AirType::Bool {
                        return Err(format!(
                            "AIR Type ERR: if 조건은 bool이어야 합니다. 실제 타입: {:?}", condition_type
                        ));
                    }
                    let mut then_variables = variables.clone();
                    let mut then_temporaries = temporaries.clone();
                    Self::check_instructions(
                        then_body, &mut then_variables, &mut then_temporaries,
                        signatures, return_type,
                    )?;
                    if let Some(else_body) = else_body {
                        let mut else_variables = variables.clone();
                        let mut else_temporaries = temporaries.clone();
                        Self::check_instructions(
                            else_body, &mut else_variables, &mut else_temporaries,
                            signatures, return_type,
                        )?;
                    }
                }

                AirInstruction::While { condition_instructions, condition, body } => {
                    let mut condition_variables = variables.clone();
                    let mut condition_temporaries = temporaries.clone();
                    Self::check_instructions(
                        condition_instructions, &mut condition_variables, &mut condition_temporaries,
                        signatures, return_type,
                    )?;
                    let condition_type = Self::value_type(
                        condition, &condition_variables, &condition_temporaries,
                    )?;
                    if condition_type != AirType::Bool {
                        return Err(format!(
                            "AIR Type ERR: while 조건은 bool이어야 합니다. 실제 타입: {:?}", condition_type
                        ));
                    }
                    let mut body_variables = variables.clone();
                    let mut body_temporaries = temporaries.clone();
                    Self::check_instructions(
                        body, &mut body_variables, &mut body_temporaries,
                        signatures, return_type,
                    )?;
                }
            }
        }
        Ok(())
    }
}
