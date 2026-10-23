
use crate::air::*;
use std::collections::HashMap;

#[derive(Clone)]
struct GeneratedValue {
    operand: String,
    ty: AirType,
}

pub struct LlvmGenerator {
    globals: String,
    output: String,
    variables: HashMap<String, GeneratedValue>,
    string_pool: HashMap<String, String>,
    temporary_types: HashMap<u32, AirType>,
    next_string: u32,
    next_call: u32,
    next_if: u32,
    next_bool_string: u32,
    current_terminated: bool,
}

impl LlvmGenerator {
    pub fn new() -> Self {
        Self {
            globals: String::from(concat!(
                "@.fmt.print.i32 = private unnamed_addr constant [3 x i8] c\"%d\\00\", align 1\n",
                "@.fmt.println.i32 = private unnamed_addr constant [4 x i8] c\"%d\\0A\\00\", align 1\n",
                "@.fmt.print.str = private unnamed_addr constant [3 x i8] c\"%s\\00\", align 1\n",
                "@.fmt.println.str = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1\n",
                "@.fmt.print.bool = private unnamed_addr constant [3 x i8] c\"%s\\00\", align 1\n",
                "@.fmt.println.bool = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1\n",
                "@.bool.true = private unnamed_addr constant [5 x i8] c\"true\\00\", align 1\n",
                "@.bool.false = private unnamed_addr constant [6 x i8] c\"false\\00\", align 1\n",
                "\n",
                "declare i32 @printf(ptr, ...)\n",
                "\n"
            )),
            output: String::new(),
            variables: HashMap::new(),
            string_pool: HashMap::new(),
            temporary_types: HashMap::new(),
            next_string: 0,
            next_call: 1,
            next_if: 1,
            next_bool_string: 1,
            current_terminated: false,
        }
    }

    pub fn generate(program: &AirProgram) -> String {
        let mut generator = Self::new();

        for function in &program.functions {
            generator.generate_function(function);
        }

        format!("{}{}", generator.globals, generator.output)
    }

    fn generate_function(&mut self, function: &AirFunction) {
        self.variables.clear();
        self.temporary_types.clear();
        self.next_call = 1;
        self.next_if = 1;
        self.next_bool_string = 1;
        self.current_terminated = false;

        self.output
            .push_str(&format!("define i32 @{}() {{\n", function.name));

        self.output.push_str("entry:\n");

        self.generate_instructions(&function.instructions);

        if !self.current_terminated {
            self.output.push_str("    ret i32 0\n");
            self.current_terminated = true;
        }

        self.output.push_str("}\n\n");
    }

    fn generate_instructions(&mut self, instructions: &[AirInstruction]) {
        for instruction in instructions {
            if self.current_terminated {
                break;
            }

            self.generate_instruction(instruction);
        }
    }

    fn generate_instruction(&mut self, instruction: &AirInstruction) {
        match instruction {
            AirInstruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let left = self.value(left);
                let right = self.value(right);

                let result_type;

                match operator {
                    AirBinaryOp::Add
                    | AirBinaryOp::Sub
                    | AirBinaryOp::Mul
                    | AirBinaryOp::Div => {
                        if left.ty != AirType::I32
                            || right.ty != AirType::I32
                        {
                            panic!(
                                "LLVM ERR: 산술 연산에는 i32 값이 필요합니다."
                            );
                        }

                        let op = match operator {
                            AirBinaryOp::Add => "add",
                            AirBinaryOp::Sub => "sub",
                            AirBinaryOp::Mul => "mul",
                            AirBinaryOp::Div => "sdiv",
                            _ => unreachable!(),
                        };

                        self.output.push_str(&format!(
                            "    %t{} = {} i32 {}, {}\n",
                            destination,
                            op,
                            left.operand,
                            right.operand
                        ));

                        result_type = AirType::I32;
                    }

                    AirBinaryOp::Equal
                    | AirBinaryOp::NotEqual
                    | AirBinaryOp::Less
                    | AirBinaryOp::Greater
                    | AirBinaryOp::LessEqual
                    | AirBinaryOp::GreaterEqual => {
                        let (predicate, llvm_type) = match operator {
                            AirBinaryOp::Equal | AirBinaryOp::NotEqual => {
                                if left.ty != right.ty {
                                    panic!(
                                        "LLVM ERR: 비교 연산의 양쪽 타입이 다릅니다."
                                    );
                                }

                                if !matches!(
                                    &left.ty,
                                    AirType::I32 | AirType::Bool
                                ) {
                                    panic!(
                                        "LLVM ERR: 현재 ==와 !=는 i32와 bool만 지원합니다."
                                    );
                                }

                                let predicate = match operator {
                                    AirBinaryOp::Equal => "eq",
                                    AirBinaryOp::NotEqual => "ne",
                                    _ => unreachable!(),
                                };

                                (predicate, Self::llvm_type(&left.ty))
                            }

                            AirBinaryOp::Less => {
                                Self::check_integer_comparison(
                                    &left.ty,
                                    &right.ty,
                                );
                                ("slt", "i32")
                            }

                            AirBinaryOp::Greater => {
                                Self::check_integer_comparison(
                                    &left.ty,
                                    &right.ty,
                                );
                                ("sgt", "i32")
                            }

                            AirBinaryOp::LessEqual => {
                                Self::check_integer_comparison(
                                    &left.ty,
                                    &right.ty,
                                );
                                ("sle", "i32")
                            }

                            AirBinaryOp::GreaterEqual => {
                                Self::check_integer_comparison(
                                    &left.ty,
                                    &right.ty,
                                );
                                ("sge", "i32")
                            }

                            _ => unreachable!(),
                        };

                        self.output.push_str(&format!(
                            "    %t{} = icmp {} {} {}, {}\n",
                            destination,
                            predicate,
                            llvm_type,
                            left.operand,
                            right.operand
                        ));

                        result_type = AirType::Bool;
                    }
                }

                self.temporary_types.insert(*destination, result_type);
            }

            AirInstruction::Let { name, ty, value } => {
                let value = self.value(value);
                let declared_type = ty.clone().unwrap_or_else(|| value.ty.clone());

                if declared_type != value.ty {
                    panic!(
                        "LLVM ERR: 변수 '{}'의 선언 타입과 초기값 타입이 다릅니다.",
                        name
                    );
                }

                self.variables.insert(name.clone(), value);
            }

            AirInstruction::Return { value } => {
                let value = self.value(value);

                if value.ty != AirType::I32 {
                    panic!("LLVM ERR: 현재 return은 i32 값만 지원합니다.");
                }

                self.output
                    .push_str(&format!("    ret i32 {}\n", value.operand));

                self.current_terminated = true;
            }

            AirInstruction::Call { function, args } => {
                if function != "print" && function != "println" {
                    panic!("LLVM ERR: 지원하지 않는 출력 함수: {function}");
                }

                if args.len() != 1 {
                    panic!("LLVM ERR: 함수 인자는 하나여야 합니다.");
                }

                let value = self.value(&args[0]);

                let format_string = match (function.as_str(), &value.ty) {
                    ("print", AirType::I32) => "@.fmt.print.i32",
                    ("println", AirType::I32) => "@.fmt.println.i32",
                    ("print", AirType::Str) => "@.fmt.print.str",
                    ("println", AirType::Str) => "@.fmt.println.str",
                    ("print", AirType::Bool) => "@.fmt.print.bool",
                    ("println", AirType::Bool) => "@.fmt.println.bool",

                    (_, AirType::Named(name)) => {
                        panic!("LLVM ERR: 출력할 수 없는 타입입니다: {name}");
                    }

                    _ => unreachable!(),
                };

                let (llvm_type, operand) = match &value.ty {
                    AirType::I32 => ("i32", value.operand.clone()),
                    AirType::Str => ("ptr", value.operand.clone()),

                    AirType::Bool => {
                        ("ptr", self.bool_string_pointer(&value.operand))
                    }

                    AirType::Named(name) => {
                        panic!("LLVM ERR: 출력할 수 없는 타입입니다: {name}");
                    }
                };

                let call_id = self.next_call;
                self.next_call += 1;

                self.output.push_str(&format!(
                    "    %call{} = call i32 (ptr, ...) @printf(ptr {}, {} {})\n",
                    call_id,
                    format_string,
                    llvm_type,
                    operand
                ));
            }

            AirInstruction::If {
                condition,
                then_body,
                else_body,
            } => {
                self.generate_if(condition, then_body, else_body.as_deref());
            }
        }
    }

    fn generate_if(
        &mut self,
        condition: &AirValue,
        then_body: &[AirInstruction],
        else_body: Option<&[AirInstruction]>,
    ) {
        let condition = self.value(condition);

        if condition.ty != AirType::Bool {
            panic!("LLVM ERR: if 조건식에는 bool 타입이 필요합니다.");
        }

        let if_id = self.next_if;
        self.next_if += 1;

        let then_label = format!("if.then.{if_id}");
        let else_label = format!("if.else.{if_id}");
        let end_label = format!("if.end.{if_id}");

        self.output.push_str(&format!(
            "    br i1 {}, label %{}, label %{}\n",
            condition.operand,
            then_label,
            else_label
        ));

        self.current_terminated = true;

        // 바깥쪽 변수는 각 분기에서 읽을 수 있지만,
        // 분기 안에서 선언된 변수는 분기 밖으로 유출되지 않는다.
        let outer_variables = self.variables.clone();

        self.output.push_str(&format!("{then_label}:\n"));
        self.current_terminated = false;
        self.variables = outer_variables.clone();

        self.generate_instructions(then_body);

        if !self.current_terminated {
            self.output
                .push_str(&format!("    br label %{end_label}\n"));
            self.current_terminated = true;
        }

        self.output.push_str(&format!("{else_label}:\n"));
        self.current_terminated = false;
        self.variables = outer_variables.clone();

        if let Some(else_body) = else_body {
            self.generate_instructions(else_body);
        }

        if !self.current_terminated {
            self.output
                .push_str(&format!("    br label %{end_label}\n"));
            self.current_terminated = true;
        }

        self.output.push_str(&format!("{end_label}:\n"));

        self.variables = outer_variables;
        self.current_terminated = false;
    }

    fn value(&mut self, value: &AirValue) -> GeneratedValue {
        match value {
            AirValue::Integer(value) => GeneratedValue {
                operand: value.to_string(),
                ty: AirType::I32,
            },

            AirValue::Bool(value) => GeneratedValue {
                operand: value.to_string(),
                ty: AirType::Bool,
            },

            AirValue::String(value) => GeneratedValue {
                operand: self.string_pointer(value),
                ty: AirType::Str,
            },

            AirValue::Temporary(temp) => {
                let ty = self
                    .temporary_types
                    .get(temp)
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!(
                            "LLVM ERR: 임시값 %{temp}의 타입 정보를 찾을 수 없습니다."
                        )
                    });

                GeneratedValue {
                    operand: format!("%t{temp}"),
                    ty,
                }
            }

            AirValue::Variable(name) => self
                .variables
                .get(name)
                .cloned()
                .unwrap_or_else(|| {
                    panic!("LLVM ERR: 변수를 찾을 수 없습니다: {name}")
                }),
        }
    }

    fn llvm_type(ty: &AirType) -> &'static str {
        match ty {
            AirType::I32 => "i32",
            AirType::Bool => "i1",
            AirType::Str => "ptr",
            AirType::Named(name) => {
                panic!("LLVM ERR: 지원하지 않는 타입: {name}");
            }
        }
    }

    fn check_integer_comparison(left: &AirType, right: &AirType) {
        if *left != AirType::I32 || *right != AirType::I32 {
            panic!("LLVM ERR: 크기 비교에는 i32 값이 필요합니다.");
        }
    }

    fn bool_string_pointer(&mut self, condition: &str) -> String {
        let name = format!("%boolstr{}", self.next_bool_string);
        self.next_bool_string += 1;

        self.output.push_str(&format!(
            "    {name} = select i1 {condition}, ptr @.bool.true, ptr @.bool.false\n"
        ));

        name
    }

    fn string_pointer(&mut self, value: &str) -> String {
        let global_name = if let Some(name) = self.string_pool.get(value) {
            name.clone()
        } else {
            let name = format!(".str{}", self.next_string);
            self.next_string += 1;

            let byte_length = value.as_bytes().len() + 1;
            let escaped = Self::escape_llvm_string(value);

            self.globals.push_str(&format!(
                "@{} = private unnamed_addr constant [{} x i8] c\"{}\", align 1\n",
                name,
                byte_length,
                escaped
            ));

            self.string_pool.insert(value.to_string(), name.clone());
            name
        };

        let byte_length = value.as_bytes().len() + 1;

        format!(
            "getelementptr inbounds ([{} x i8], ptr @{}, i64 0, i64 0)",
            byte_length,
            global_name
        )
    }

    fn escape_llvm_string(value: &str) -> String {
        let mut escaped = String::new();

        for byte in value.as_bytes() {
            match *byte {
                b'"' => escaped.push_str("\\22"),
                b'\\' => escaped.push_str("\\5C"),
                0x20..=0x7E => escaped.push(*byte as char),
                byte => escaped.push_str(&format!("\\{byte:02X}")),
            }
        }

        // printf는 NUL 종료 문자열을 사용한다.
        escaped.push_str("\\00");
        escaped
    }
}

