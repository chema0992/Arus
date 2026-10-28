
use crate::ast::*;
use std::collections::{HashMap, HashSet};

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
        mutable: bool,
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

    For {
        name: String,
        start: AirValue,
        end: AirValue,
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

    Assign {
        name: String,
        value: AirValue,
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
    TypedInteger(i64, AirType),
    Bool(bool),
    String(String),
    Variable(String),
    Temporary(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AirType {
    I8,
    I16,
    I32,
    U8,
    U16,
    U32,
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
    variable_types: HashMap<String, AirType>,
    function_signatures: HashMap<String, (Vec<AirType>, AirType)>,
    current_return_type: AirType,
}

impl AirGenerator {
    pub fn new() -> Self {
        Self {
            next_temp: 1,
            instructions: Vec::new(),
            variable_types: HashMap::new(),
            function_signatures: HashMap::new(),
            current_return_type: AirType::Void,
        }
    }

    fn generate_program(&mut self, program: &Program) -> AirProgram {
        let mode = match program.mode {
            ProgramMode::Main => AirMode::Main,
            ProgramMode::Script => AirMode::Script,
        };

        self.function_signatures = program.functions.iter().map(|function| {
            let params = function.params.iter().map(|param| Self::convert_type(&param.ty)).collect();
            let return_type = function.return_type.as_ref().map(Self::convert_type)
                .unwrap_or_else(|| if function.name == "main" { AirType::I32 } else { AirType::Void });
            (function.name.clone(), (params, return_type))
        }).collect();

        let mut functions = Vec::new();

        for function in &program.functions {
            functions.push(self.generate_function(function));
        }

        AirProgram { mode, functions }
    }

    fn generate_function(&mut self, function: &Function) -> AirFunction {
        self.instructions.clear();
        self.next_temp = 1;
        self.variable_types.clear();

        let params: Vec<AirParameter> = function.params.iter().map(|param| AirParameter {
            name: param.name.clone(),
            ty: Self::convert_type(&param.ty),
        }).collect();

        let return_type = function.return_type.as_ref().map(Self::convert_type)
            .unwrap_or_else(|| if function.name == "main" { AirType::I32 } else { AirType::Void });
        self.current_return_type = return_type.clone();
        for param in &params {
            self.variable_types.insert(param.name.clone(), param.ty.clone());
        }

        for stmt in &function.body {
            self.generate_stmt(stmt);
        }

        AirFunction {
            name: function.name.clone(),
            params,
            return_type,
            instructions: self.instructions.clone(),
        }
    }

    fn convert_type(ty: &Type) -> AirType {
        match ty {
            Type::I8 => AirType::I8,
            Type::I16 => AirType::I16,
            Type::I32 => AirType::I32,
            Type::U8 => AirType::U8,
            Type::U16 => AirType::U16,
            Type::U32 => AirType::U32,
            Type::Str => AirType::Str,
            Type::Bool => AirType::Bool,
            Type::Named(name) => AirType::Named(name.clone()),
        }
    }

    fn generate_block(&mut self, statements: &[Stmt]) -> Vec<AirInstruction> {
        let outer_instructions = std::mem::take(&mut self.instructions);
        let outer_variable_types = self.variable_types.clone();

        for stmt in statements {
            self.generate_stmt(stmt);
        }

        self.variable_types = outer_variable_types;
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
            Stmt::Let { name, ty, value, mutable } => {
                let ty = ty.as_ref().map(Self::convert_type);
                let inferred_type = ty.clone().or_else(|| self.infer_expr_type(value)).unwrap_or(AirType::I32);
                let value = self.generate_expr_expected(value, Some(&inferred_type));
                self.variable_types.insert(name.clone(), inferred_type);

                self.instructions.push(AirInstruction::Let {
                    name: name.clone(),
                    ty,
                    value,
                    mutable: *mutable,
                });
            }

            Stmt::Assign { name, value } => {
                let expected_type = self.variable_types.get(name).cloned();
                let value = self.generate_expr_expected(value, expected_type.as_ref());
                self.instructions.push(AirInstruction::Assign {
                    name: name.clone(),
                    value,
                });
            }

            Stmt::Return(value) => {
                let expected_type = if self.current_return_type == AirType::Void {
                    None
                } else {
                    Some(self.current_return_type.clone())
                };
                let value = self.generate_expr_expected(value, expected_type.as_ref());
                self.instructions.push(AirInstruction::Return { value });
            }

            Stmt::Call { name, args } => {
                let expected_params = self.function_signatures.get(name).map(|signature| signature.0.clone());
                let args = args.iter().enumerate().map(|(index, arg)| {
                    self.generate_expr_expected(arg, expected_params.as_ref().and_then(|types| types.get(index)))
                }).collect();

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

            Stmt::For { name, start, end, body } => {
                // 시작/끝 표현식은 반복문에 진입하기 전에 한 번만 생성한다.
                // 한쪽이 숫자 리터럴이면 반대쪽의 정수 타입을 따른다.
                let outer_variable_types = self.variable_types.clone();
                let start_type = self.infer_expr_type(start);
                let end_type = self.infer_expr_type(end);
                let index_type = match (start_type, end_type) {
                    (Some(_), Some(end_type)) if matches!(start, Expr::Integer(_)) => end_type,
                    (Some(start_type), Some(_)) if matches!(end, Expr::Integer(_)) => start_type,
                    (Some(start_type), _) => start_type,
                    (_, Some(end_type)) => end_type,
                    (None, None) => AirType::I32,
                };
                let start_value = self.generate_expr_expected(start, Some(&index_type));
                let end_value = self.generate_expr_expected(end, Some(&index_type));
                self.variable_types.insert(name.clone(), index_type);
                let body = self.generate_block(body);
                self.variable_types = outer_variable_types;
                self.instructions.push(AirInstruction::For {
                    name: name.clone(),
                    start: start_value,
                    end: end_value,
                    body,
                });
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

    fn is_integer_type(ty: &AirType) -> bool {
        matches!(ty, AirType::I8 | AirType::I16 | AirType::I32 | AirType::U8 | AirType::U16 | AirType::U32)
    }

    fn infer_expr_type(&self, expr: &Expr) -> Option<AirType> {
        match expr {
            Expr::Integer(_) => Some(AirType::I32),
            Expr::Bool(_) => Some(AirType::Bool),
            Expr::String(_) => Some(AirType::Str),
            Expr::Ident(name) => self.variable_types.get(name).cloned(),
            Expr::Call { name, .. } => self.function_signatures.get(name).map(|signature| signature.1.clone()),
            Expr::InterpolatedString { .. } => Some(AirType::Str),
            Expr::Unary { operator: UnaryOp::Not, .. } => Some(AirType::Bool),
            Expr::Binary { left, operator, right } => {
                match operator {
                    BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::Greater
                    | BinaryOp::LessEqual | BinaryOp::GreaterEqual | BinaryOp::And | BinaryOp::Or => Some(AirType::Bool),
                    _ => {
                        let left_type = self.infer_expr_type(left);
                        let right_type = self.infer_expr_type(right);
                        if matches!(left.as_ref(), Expr::Integer(_)) {
                            right_type.or(left_type)
                        } else if matches!(right.as_ref(), Expr::Integer(_)) {
                            left_type.or(right_type)
                        } else if left_type == right_type {
                            left_type
                        } else {
                            left_type.or(right_type)
                        }
                    }
                }
            }
        }
    }

    fn generate_expr(&mut self, expr: &Expr) -> AirValue {
        self.generate_expr_expected(expr, None)
    }

    fn generate_expr_expected(&mut self, expr: &Expr, expected: Option<&AirType>) -> AirValue {
        match expr {
            Expr::Integer(value) => {
                if let Some(ty) = expected.filter(|ty| Self::is_integer_type(ty)) {
                    AirValue::TypedInteger(*value, ty.clone())
                } else {
                    AirValue::Integer(*value)
                }
            }
            Expr::Bool(value) => AirValue::Bool(*value),
            Expr::String(value) => AirValue::String(value.clone()),
            Expr::Ident(name) => AirValue::Variable(name.clone()),

            Expr::Call { name, args } => {
                let expected_params = self.function_signatures.get(name).map(|signature| signature.0.clone());
                let args: Vec<AirValue> = args.iter().enumerate().map(|(index, arg)| {
                    self.generate_expr_expected(arg, expected_params.as_ref().and_then(|types| types.get(index)))
                }).collect();
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

            Expr::Unary { operator, operand } => {
                let bool_type = AirType::Bool;
                let operand = self.generate_expr_expected(operand, Some(&bool_type));
                let destination = self.next_temp;
                self.next_temp += 1;
                let operator = match operator {
                    UnaryOp::Not => AirUnaryOp::Not,
                };
                self.instructions.push(AirInstruction::Unary { destination, operator, operand });
                AirValue::Temporary(destination)
            }

            Expr::Binary { left, operator: BinaryOp::And, right } => {
                self.generate_logical_expr(left, AirLogicalOp::And, right)
            }

            Expr::Binary { left, operator: BinaryOp::Or, right } => {
                self.generate_logical_expr(left, AirLogicalOp::Or, right)
            }

            Expr::Binary { left, operator, right } => {
                let operator_is_numeric = matches!(operator,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
                    | BinaryOp::ShiftLeft | BinaryOp::ShiftRight | BinaryOp::BitAnd
                    | BinaryOp::BitXor | BinaryOp::BitOr
                );
                let preferred_type = if operator_is_numeric && expected.is_some_and(Self::is_integer_type) {
                    expected.cloned()
                } else {
                    let left_type = if matches!(left.as_ref(), Expr::Integer(_)) {
                        None
                    } else {
                        self.infer_expr_type(left).filter(Self::is_integer_type)
                    };
                    let right_type = if matches!(right.as_ref(), Expr::Integer(_)) {
                        None
                    } else {
                        self.infer_expr_type(right).filter(Self::is_integer_type)
                    };
                    left_type.or(right_type)
                };

                let left_value = self.generate_expr_expected(left, preferred_type.as_ref());
                let right_value = self.generate_expr_expected(right, preferred_type.as_ref());
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
                    BinaryOp::And | BinaryOp::Or => unreachable!("논리 연산은 generate_logical_expr에서 처리해야 합니다."),
                };
                self.instructions.push(AirInstruction::Binary {
                    destination,
                    operator,
                    left: left_value,
                    right: right_value,
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

            AirInstruction::Let { name, ty, value, mutable } => {
                let keyword = if *mutable { "mut" } else { "let" };
                match ty {
                    Some(ty) => format!(
                        "{prefix}{keyword} {name}: {} = {}",
                        Self::type_name(ty), Self::print_value(value),
                    ),
                    None => format!(
                        "{prefix}{keyword} {name} = {}",
                        Self::print_value(value),
                    ),
                }
            },

            AirInstruction::Assign { name, value } => {
                format!("{prefix}{name} = {}", Self::print_value(value))
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

            AirInstruction::For { name, start, end, body } => {
                let mut output = format!(
                    "{prefix}for {name} in {}..{} {{\n",
                    Self::print_value(start), Self::print_value(end),
                );
                output.push_str(&Self::print_instructions(body, indent + 4));
                output.push_str(&format!("{prefix}}}"));
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
            AirValue::Integer(value) | AirValue::TypedInteger(value, _) => value.to_string(),
            AirValue::Bool(value) => value.to_string(),
            AirValue::String(value) => format!("{value:?}"),
            AirValue::Variable(name) => name.clone(),
            AirValue::Temporary(temp) => format!("%{temp}"),
        }
    }

    fn type_name(ty: &AirType) -> String {
        match ty {
            AirType::I8 => "i8".to_string(),
            AirType::I16 => "i16".to_string(),
            AirType::I32 => "i32".to_string(),
            AirType::U8 => "u8".to_string(),
            AirType::U16 => "u16".to_string(),
            AirType::U32 => "u32".to_string(),
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

        if !signatures.contains_key("main") {
            return Err("AIR Type ERR: 진입점 main 함수가 없습니다.".to_string());
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
                if !Self::is_supported_value_type(&param.ty) {
                    return Err(format!(
                        "AIR Type ERR: 매개변수 '{}'의 타입 {:?}은 아직 지원되지 않습니다.",
                        param.name, param.ty
                    ));
                }
            }
            if function.return_type != AirType::Void && !Self::is_supported_value_type(&function.return_type) {
                return Err(format!(
                    "AIR Type ERR: 함수 '{}'의 반환형 {:?}은 아직 지원되지 않습니다.",
                    function.name, function.return_type
                ));
            }

            let mut variables = HashMap::<String, AirType>::new();
            let mut mutable_variables = HashSet::<String>::new();
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
                &mut mutable_variables,
                &mut temporaries,
                &signatures,
                &function.return_type,
            )
            .map_err(|error| format!("함수 '{}': {}", function.name, error))?;
        }

        Ok(())
    }

    fn is_integer_type(ty: &AirType) -> bool {
        matches!(ty, AirType::I8 | AirType::I16 | AirType::I32 | AirType::U8 | AirType::U16 | AirType::U32)
    }

    fn is_signed_integer_type(ty: &AirType) -> bool {
        matches!(ty, AirType::I8 | AirType::I16 | AirType::I32)
    }

    fn is_supported_value_type(ty: &AirType) -> bool {
        Self::is_integer_type(ty) || matches!(ty, AirType::Str | AirType::Bool)
    }

    fn integer_fits(value: i64, ty: &AirType) -> bool {
        match ty {
            AirType::I8 => (-128..=127).contains(&value),
            AirType::I16 => (i16::MIN as i64..=i16::MAX as i64).contains(&value),
            AirType::I32 => (i32::MIN as i64..=i32::MAX as i64).contains(&value),
            AirType::U8 => (0..=u8::MAX as i64).contains(&value),
            AirType::U16 => (0..=u16::MAX as i64).contains(&value),
            AirType::U32 => (0..=u32::MAX as i64).contains(&value),
            _ => false,
        }
    }

    fn value_type(
        value: &AirValue,
        variables: &HashMap<String, AirType>,
        temporaries: &HashMap<u32, AirType>,
    ) -> Result<AirType, String> {
        match value {
            AirValue::Integer(value) => {
                if Self::integer_fits(*value, &AirType::I32) {
                    Ok(AirType::I32)
                } else {
                    Err(format!("AIR Type ERR: 정수 리터럴 {}은 i32 범위를 벗어났습니다.", value))
                }
            }
            AirValue::TypedInteger(value, ty) => {
                if Self::is_integer_type(ty) && Self::integer_fits(*value, ty) {
                    Ok(ty.clone())
                } else {
                    Err(format!("AIR Type ERR: 정수 리터럴 {}이 {:?} 범위를 벗어났습니다.", value, ty))
                }
            }
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

    fn value_type_in_context(
        value: &AirValue,
        expected: &AirType,
        variables: &HashMap<String, AirType>,
        temporaries: &HashMap<u32, AirType>,
    ) -> Result<AirType, String> {
        if let AirValue::Integer(number) = value {
            if Self::is_integer_type(expected) {
                if Self::integer_fits(*number, expected) {
                    return Ok(expected.clone());
                }
                return Err(format!(
                    "AIR Type ERR: 정수 리터럴 {}은 {:?} 범위를 벗어났습니다.", number, expected
                ));
            }
        }
        Self::value_type(value, variables, temporaries)
    }

    fn compatible_binary_types(
        left: &AirValue,
        right: &AirValue,
        variables: &HashMap<String, AirType>,
        temporaries: &HashMap<u32, AirType>,
    ) -> Result<(AirType, AirType), String> {
        let mut left_type = Self::value_type(left, variables, temporaries)?;
        let mut right_type = Self::value_type(right, variables, temporaries)?;

        if matches!(left, AirValue::Integer(_)) && Self::is_integer_type(&right_type) {
            left_type = Self::value_type_in_context(left, &right_type, variables, temporaries)?;
        } else if matches!(right, AirValue::Integer(_)) && Self::is_integer_type(&left_type) {
            right_type = Self::value_type_in_context(right, &left_type, variables, temporaries)?;
        }

        Ok((left_type, right_type))
    }

    fn check_instructions(
        instructions: &[AirInstruction],
        variables: &mut HashMap<String, AirType>,
        mutable_variables: &mut HashSet<String>,
        temporaries: &mut HashMap<u32, AirType>,
        signatures: &FunctionSignatures,
        return_type: &AirType,
    ) -> Result<(), String> {
        for instruction in instructions {
            match instruction {
                AirInstruction::Binary { destination, operator, left, right } => {
                    let (left_type, right_type) = Self::compatible_binary_types(left, right, variables, temporaries)?;
                    let result_type = match operator {
                        AirBinaryOp::Add | AirBinaryOp::Sub | AirBinaryOp::Mul | AirBinaryOp::Div
                        | AirBinaryOp::ShiftLeft | AirBinaryOp::ShiftRight | AirBinaryOp::BitAnd
                        | AirBinaryOp::BitXor | AirBinaryOp::BitOr => {
                            if !Self::is_integer_type(&left_type) || left_type != right_type {
                                return Err(format!(
                                    "AIR Type ERR: {:?} 연산은 같은 정수 타입이 필요합니다. 왼쪽: {:?}, 오른쪽: {:?}",
                                    operator, left_type, right_type
                                ));
                            }
                            left_type
                        }
                        AirBinaryOp::Less | AirBinaryOp::Greater | AirBinaryOp::LessEqual | AirBinaryOp::GreaterEqual => {
                            if !Self::is_integer_type(&left_type) || left_type != right_type {
                                return Err(format!(
                                    "AIR Type ERR: 크기 비교는 같은 정수 타입이 필요합니다. 왼쪽: {:?}, 오른쪽: {:?}",
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
                            if Self::is_integer_type(&left_type) || left_type == AirType::Bool {
                                AirType::Bool
                            } else {
                                return Err(format!(
                                    "AIR Type ERR: ==와 !=는 정수와 bool만 지원합니다. 실제 타입: {:?}",
                                    left_type
                                ));
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
                                "AIR Type ERR: ! 연산은 bool이 필요합니다. 실제 타입: {:?}", operand_type
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
                    let mut left_mutable = mutable_variables.clone();
                    let mut left_temporaries = temporaries.clone();
                    Self::check_instructions(
                        left_instructions, &mut left_variables, &mut left_mutable, &mut left_temporaries,
                        signatures, return_type,
                    )?;
                    let left_type = Self::value_type(left, &left_variables, &left_temporaries)?;
                    if left_type != AirType::Bool {
                        return Err(format!(
                            "AIR Type ERR: {:?} 연산의 왼쪽 값은 bool이어야 합니다. 실제 타입: {:?}", operator, left_type
                        ));
                    }

                    let mut right_variables = variables.clone();
                    let mut right_mutable = mutable_variables.clone();
                    let mut right_temporaries = temporaries.clone();
                    Self::check_instructions(
                        right_instructions, &mut right_variables, &mut right_mutable, &mut right_temporaries,
                        signatures, return_type,
                    )?;
                    let right_type = Self::value_type(right, &right_variables, &right_temporaries)?;
                    if right_type != AirType::Bool {
                        return Err(format!(
                            "AIR Type ERR: {:?} 연산의 오른쪽 값은 bool이어야 합니다. 실제 타입: {:?}", operator, right_type
                        ));
                    }
                    temporaries.insert(*destination, AirType::Bool);
                }

                AirInstruction::Let { name, ty, value, mutable } => {
                    let actual_type = if let Some(expected) = ty {
                        Self::value_type_in_context(value, expected, variables, temporaries)?
                    } else {
                        Self::value_type(value, variables, temporaries)?
                    };
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
                    if *mutable {
                        mutable_variables.insert(name.clone());
                    } else {
                        mutable_variables.remove(name);
                    }
                }

                AirInstruction::Assign { name, value } => {
                    let expected_type = variables.get(name).cloned().ok_or_else(|| {
                        format!("AIR Type ERR: 선언되지 않은 변수 '{}'에 대입했습니다.", name)
                    })?;
                    if !mutable_variables.contains(name) {
                        return Err(format!(
                            "AIR Type ERR: 불변 변수 '{}'에는 재대입할 수 없습니다. 'mut'를 사용하세요.", name
                        ));
                    }
                    let actual_type = Self::value_type_in_context(value, &expected_type, variables, temporaries)?;
                    if actual_type != expected_type {
                        return Err(format!(
                            "AIR Type ERR: 변수 '{}'에 대입하는 값의 타입이 다릅니다. 기대: {:?}, 실제: {:?}",
                            name, expected_type, actual_type
                        ));
                    }
                }

                AirInstruction::Return { value } => {
                    let actual_type = Self::value_type_in_context(value, return_type, variables, temporaries)?;
                    if *return_type == AirType::Void {
                        return Err("AIR Type ERR: 반환형이 생략된 일반 함수(void)에서는 값을 반환할 수 없습니다.".to_string());
                    }
                    if actual_type != *return_type {
                        return Err(format!(
                            "AIR Type ERR: 반환 타입이 일치하지 않습니다. 선언: {:?}, 실제: {:?}", return_type, actual_type
                        ));
                    }
                }

                AirInstruction::Call { destination, function, args } => {
                    let raw_arg_types = args.iter()
                        .map(|arg| Self::value_type(arg, variables, temporaries))
                        .collect::<Result<Vec<_>, _>>()?;

                    if function == "print" || function == "println" {
                        if destination.is_some() {
                            return Err(format!("AIR Type ERR: 내장 함수 '{}'는 식의 값으로 사용할 수 없습니다.", function));
                        }
                        if args.len() != 1 {
                            return Err(format!("AIR Type ERR: '{}'에는 인자 하나가 필요합니다.", function));
                        }
                        if !Self::is_supported_value_type(&raw_arg_types[0]) {
                            return Err(format!(
                                "AIR Type ERR: '{}'에서 지원하지 않는 인자 타입입니다: {:?}", function, raw_arg_types[0]
                            ));
                        }
                    } else {
                        let (param_types, result_type) = signatures.get(function).ok_or_else(|| {
                            format!("AIR Type ERR: 선언되지 않은 함수 '{}'.", function)
                        })?;
                        if args.len() != param_types.len() {
                            return Err(format!(
                                "AIR Type ERR: 함수 '{}'는 인자 {}개를 받지만 {}개가 전달되었습니다.",
                                function, param_types.len(), args.len()
                            ));
                        }
                        for (index, (arg, expected)) in args.iter().zip(param_types).enumerate() {
                            let actual = Self::value_type_in_context(arg, expected, variables, temporaries)?;
                            if actual != *expected {
                                return Err(format!(
                                    "AIR Type ERR: 함수 '{}'의 {}번째 인자 타입이 다릅니다. 기대: {:?}, 실제: {:?}",
                                    function, index + 1, expected, actual
                                ));
                            }
                        }
                        if let Some(destination) = destination {
                            if *result_type == AirType::Void {
                                return Err(format!(
                                    "AIR Type ERR: 반환값이 없는 함수 '{}'를 식 안에서 사용할 수 없습니다.", function
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
                    let mut then_mutable = mutable_variables.clone();
                    let mut then_temporaries = temporaries.clone();
                    Self::check_instructions(
                        then_body, &mut then_variables, &mut then_mutable, &mut then_temporaries,
                        signatures, return_type,
                    )?;
                    if let Some(else_body) = else_body {
                        let mut else_variables = variables.clone();
                        let mut else_mutable = mutable_variables.clone();
                        let mut else_temporaries = temporaries.clone();
                        Self::check_instructions(
                            else_body, &mut else_variables, &mut else_mutable, &mut else_temporaries,
                            signatures, return_type,
                        )?;
                    }
                }

                AirInstruction::While { condition_instructions, condition, body } => {
                    let mut condition_variables = variables.clone();
                    let mut condition_mutable = mutable_variables.clone();
                    let mut condition_temporaries = temporaries.clone();
                    Self::check_instructions(
                        condition_instructions, &mut condition_variables, &mut condition_mutable,
                        &mut condition_temporaries, signatures, return_type,
                    )?;
                    let condition_type = Self::value_type(condition, &condition_variables, &condition_temporaries)?;
                    if condition_type != AirType::Bool {
                        return Err(format!(
                            "AIR Type ERR: while 조건은 bool이어야 합니다. 실제 타입: {:?}", condition_type
                        ));
                    }
                    let mut body_variables = variables.clone();
                    let mut body_mutable = mutable_variables.clone();
                    let mut body_temporaries = temporaries.clone();
                    Self::check_instructions(
                        body, &mut body_variables, &mut body_mutable, &mut body_temporaries,
                        signatures, return_type,
                    )?;
                }

                AirInstruction::For { name, start, end, body } => {
                    let start_type = Self::value_type(start, variables, temporaries)?;
                    let end_type = Self::value_type_in_context(end, &start_type, variables, temporaries)?;
                    let index_type = if Self::is_integer_type(&start_type) && start_type == end_type {
                        start_type
                    } else if matches!(start, AirValue::Integer(_)) {
                        let start_as_end = Self::value_type_in_context(start, &end_type, variables, temporaries)?;
                        if Self::is_integer_type(&end_type) && start_as_end == end_type { end_type.clone() }
                        else {
                            return Err(format!("AIR Type ERR: for 범위는 같은 정수 타입이어야 합니다. 시작: {:?}, 끝: {:?}", start_type, end_type));
                        }
                    } else {
                        return Err(format!("AIR Type ERR: for 범위는 같은 정수 타입이어야 합니다. 시작: {:?}, 끝: {:?}", start_type, end_type));
                    };

                    let mut loop_variables = variables.clone();
                    let mut loop_mutable = mutable_variables.clone();
                    let mut loop_temporaries = temporaries.clone();
                    loop_variables.insert(name.clone(), index_type);
                    loop_mutable.insert(name.clone());
                    Self::check_instructions(
                        body, &mut loop_variables, &mut loop_mutable, &mut loop_temporaries,
                        signatures, return_type,
                    )?;
                }
            }
        }
        Ok(())
    }
}
