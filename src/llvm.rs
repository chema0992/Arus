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
    next_string: u32,
    next_call: u32,
}

impl LlvmGenerator {
    pub fn new() -> Self {
        Self {
            globals: String::from(
                "@.fmt.print.i32 = private unnamed_addr constant [3 x i8] c\"%d\\00\", align 1\n\
@.fmt.println.i32 = private unnamed_addr constant [4 x i8] c\"%d\\0A\\00\", align 1\n\
@.fmt.print.str = private unnamed_addr constant [3 x i8] c\"%s\\00\", align 1\n\
@.fmt.println.str = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1\n\
\n\
declare i32 @printf(ptr, ...)\n\n",
            ),
            output: String::new(),
            variables: HashMap::new(),
            string_pool: HashMap::new(),
            next_string: 0,
            next_call: 1,
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
        self.next_call = 1;

        self.output
            .push_str(&format!("define i32 @{}() {{\n", function.name));
        self.output.push_str("entry:\n");

        for instruction in &function.instructions {
            self.generate_instruction(instruction);
        }

        if !matches!(
            function.instructions.last(),
            Some(AirInstruction::Return { .. })
        ) {
            self.output.push_str("    ret i32 0\n");
        }

        self.output.push_str("}\n\n");
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

                if left.ty != AirType::I32 || right.ty != AirType::I32 {
                    panic!("LLVM ERR: 산술 연산은 현재 i32 값만 지원합니다.");
                }

                let op = match operator {
                    AirBinaryOp::Add => "add",
                    AirBinaryOp::Sub => "sub",
                    AirBinaryOp::Mul => "mul",
                    AirBinaryOp::Div => "sdiv",
                };

                self.output.push_str(&format!(
                    "    %t{} = {} i32 {}, {}\n",
                    destination, op, left.operand, right.operand
                ));
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
            }

            AirInstruction::Call { function, args } => {
                if args.len() != 1 {
                    panic!("LLVM ERR: 함수 인자는 하나여야 합니다.");
                }

                let value = self.value(&args[0]);

                let format_string = match (function.as_str(), &value.ty) {
                    ("print", AirType::I32) => "@.fmt.print.i32",
                    ("println", AirType::I32) => "@.fmt.println.i32",
                    ("print", AirType::Str) => "@.fmt.print.str",
                    ("println", AirType::Str) => "@.fmt.println.str",
                    (_, AirType::Named(name)) => {
                        panic!("LLVM ERR: 출력할 수 없는 타입입니다: {name}");
                    }
                    (other, _) => panic!("LLVM ERR: 지원하지 않는 출력 함수입니다: {other}"),
                };

                let llvm_type = match value.ty {
                    AirType::I32 => "i32",
                    AirType::Str => "ptr",
                    AirType::Named(_) => unreachable!(),
                };

                let call_id = self.next_call;
                self.next_call += 1;

                self.output.push_str(&format!(
                    "    %call{} = call i32 (ptr, ...) @printf(ptr {}, {} {})\n",
                    call_id, format_string, llvm_type, value.operand
                ));
            }
        }
    }

    fn value(&mut self, value: &AirValue) -> GeneratedValue {
        match value {
            AirValue::Integer(value) => GeneratedValue {
                operand: value.to_string(),
                ty: AirType::I32,
            },

            AirValue::String(value) => GeneratedValue {
                operand: self.string_pointer(value),
                ty: AirType::Str,
            },

            AirValue::Temporary(temp) => GeneratedValue {
                operand: format!("%t{temp}"),
                ty: AirType::I32,
            },

            AirValue::Variable(name) => self
                .variables
                .get(name)
                .cloned()
                .unwrap_or_else(|| panic!("LLVM ERR: 변수를 찾을 수 없습니다: {name}")),
        }
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
                name, byte_length, escaped
            ));

            self.string_pool.insert(value.to_string(), name.clone());
            name
        };

        let byte_length = value.as_bytes().len() + 1;
        format!(
            "getelementptr inbounds ([{} x i8], ptr @{}, i64 0, i64 0)",
            byte_length, global_name
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

        // printf expects a NUL-terminated C string.
        escaped.push_str("\\00");
        escaped
    }
}
