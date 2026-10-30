
use crate::air::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct GeneratedValue {
    operand: String,
    ty: AirType,
}

pub struct LlvmGenerator {
    globals: String,
    output: String,
    variables: HashMap<String, GeneratedValue>,
    active_mutable_slots: HashMap<String, String>,
    mutable_types: HashMap<String, AirType>,
    slot_by_instruction: HashMap<usize, String>,
    string_pool: HashMap<String, String>,
    temporary_types: HashMap<u32, AirType>,
    next_string: u32,
    next_slot: u32,
    next_array: u32,
    next_load: u32,
    next_promoted: u32,
    next_call: u32,
    next_for: u32,
    next_if: u32,
    next_while: u32,
    next_logical: u32,
    current_block: String,
    next_bool_string: u32,
    function_signatures: HashMap<String, (Vec<AirType>, AirType)>,
    current_return_type: AirType,
    current_terminated: bool,
}

impl LlvmGenerator {
    pub fn new() -> Self {
        Self {
            globals: String::from(concat!(
                "@.fmt.print.i32 = private unnamed_addr constant [3 x i8] c\"%d\\00\", align 1\n",
                "@.fmt.println.i32 = private unnamed_addr constant [4 x i8] c\"%d\\0A\\00\", align 1\n",
                "@.fmt.print.u32 = private unnamed_addr constant [3 x i8] c\"%u\\00\", align 1\n",
                "@.fmt.println.u32 = private unnamed_addr constant [4 x i8] c\"%u\\0A\\00\", align 1\n",
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
            active_mutable_slots: HashMap::new(),
            mutable_types: HashMap::new(),
            slot_by_instruction: HashMap::new(),
            string_pool: HashMap::new(),
            temporary_types: HashMap::new(),
            next_string: 0,
            next_slot: 0,
            next_array: 0,
            next_load: 0,
            next_promoted: 0,
            next_call: 1,
            next_for: 1,
            next_if: 1,
            next_while: 1,
            next_logical: 1,
            current_block: String::new(),
            next_bool_string: 1,
            function_signatures: HashMap::new(),
            current_return_type: AirType::Void,
            current_terminated: false,
        }
    }

    pub fn generate(program: &AirProgram) -> String {
        let mut generator = Self::new();
        generator.function_signatures = program.functions.iter().map(|function| {
            (
                function.name.clone(),
                (
                    function.params.iter().map(|param| param.ty.clone()).collect(),
                    function.return_type.clone(),
                ),
            )
        }).collect();

        for function in &program.functions {
            generator.generate_function(function);
        }

        format!("{}{}", generator.globals, generator.output)
    }

    fn infer_value_type(value: &AirValue) -> Option<AirType> {
        match value {
            AirValue::Integer(_) => Some(AirType::I32),
            AirValue::TypedInteger(_, ty) => Some(ty.clone()),
            AirValue::Bool(_) => Some(AirType::Bool),
            AirValue::String(_) => Some(AirType::Str),
            AirValue::Array(elements) => {
                let first_type = Self::infer_value_type(elements.first()?)?;
                if elements.iter().all(|element| Self::infer_value_type(element).as_ref() == Some(&first_type)) {
                    Some(AirType::Array {
                        element_type: Box::new(first_type),
                        size: elements.len(),
                    })
                } else {
                    None
                }
            }
            AirValue::Variable(_) | AirValue::Temporary(_) => None,
        }
    }

    fn store_array_elements(
        &mut self,
        slot: &str,
        array_type: &AirType,
        elements: &[AirValue],
        array_id: u32,
    ) {
        let (element_type, size) = match array_type {
            AirType::Array { element_type, size } => (element_type.as_ref(), *size),
            _ => panic!("LLVM ERR: 배열 초기화에 배열 타입이 필요합니다."),
        };
        if elements.len() != size {
            panic!(
                "LLVM ERR: 배열 초기화 길이가 다릅니다. 기대: {}, 실제: {}",
                size, elements.len()
            );
        }

        let llvm_array_type = Self::llvm_type(array_type);
        let llvm_element_type = Self::llvm_type(element_type);
        for (index, element) in elements.iter().enumerate() {
            let element_ptr = format!("%arr{array_id}.elem{index}");
            self.output.push_str(&format!(
                "    {element_ptr} = getelementptr inbounds {llvm_array_type}, ptr {slot}, i32 0, i32 {index}\n"
            ));

            if matches!(element_type, AirType::Array { .. }) {
                if let AirValue::Array(nested_elements) = element {
                    let nested_id = self.next_array;
                    self.next_array += 1;
                    self.store_array_elements(&element_ptr, element_type, nested_elements, nested_id);
                } else {
                    let generated = self.value(element);
                    if &generated.ty != element_type {
                        panic!("LLVM ERR: 중첩 배열 원소 타입이 선언과 다릅니다.");
                    }
                    self.output.push_str(&format!(
                        "    store {llvm_element_type} {}, ptr {element_ptr}\n", generated.operand
                    ));
                }
            } else {
                let mut generated = self.value(element);
                if matches!(element, AirValue::Integer(_)) && Self::is_integer_type(element_type) {
                    generated.ty = element_type.clone();
                }
                if generated.ty != *element_type {
                    panic!(
                        "LLVM ERR: 배열 원소 타입이 선언과 다릅니다. 기대: {:?}, 실제: {:?}",
                        element_type, generated.ty
                    );
                }
                self.output.push_str(&format!(
                    "    store {llvm_element_type} {}, ptr {element_ptr}\n", generated.operand
                ));
            }
        }
    }

    fn collect_addressed_variables(
        instructions: &[AirInstruction],
        addressed: &mut HashSet<String>,
    ) {
        for instruction in instructions {
            match instruction {
                AirInstruction::AddressOf { variable, .. } => {
                    addressed.insert(variable.clone());
                }
                AirInstruction::If { then_body, else_body, .. } => {
                    Self::collect_addressed_variables(then_body, addressed);
                    if let Some(body) = else_body {
                        Self::collect_addressed_variables(body, addressed);
                    }
                }
                AirInstruction::While { condition_instructions, body, .. } => {
                    Self::collect_addressed_variables(condition_instructions, addressed);
                    Self::collect_addressed_variables(body, addressed);
                }
                AirInstruction::For { body, .. } => {
                    Self::collect_addressed_variables(body, addressed);
                }
                AirInstruction::Logical { left_instructions, right_instructions, .. } => {
                    Self::collect_addressed_variables(left_instructions, addressed);
                    Self::collect_addressed_variables(right_instructions, addressed);
                }
                _ => {}
            }
        }
    }

    fn collect_mutable_slots(
        instructions: &[AirInstruction],
        addressed: &HashSet<String>,
        slot_by_instruction: &mut HashMap<usize, String>,
        allocas: &mut Vec<String>,
        next_slot: &mut u32,
    ) {
        for instruction in instructions {
            let needs_slot = match instruction {
                AirInstruction::For { .. } => true,
                AirInstruction::Let { name, mutable, .. } => *mutable || addressed.contains(name),
                _ => false,
            };
            if needs_slot {
                let key = instruction as *const AirInstruction as usize;
                let slot = format!("%slot{}", *next_slot);
                *next_slot += 1;
                slot_by_instruction.insert(key, slot.clone());
                let declared_type = match instruction {
                    AirInstruction::Let { ty, value, .. } => {
                        ty.clone().or_else(|| Self::infer_value_type(value))
                    }
                    _ => None,
                };
                // Address-taken locals use their declared pointee type. Existing
                // mutable scalar slots keep the previous roomy i64 allocation.
                let address_taken = match instruction {
                    AirInstruction::Let { name, .. } => addressed.contains(name),
                    _ => false,
                };
                let slot_type = if address_taken {
                    declared_type
                        .as_ref()
                        .map(Self::llvm_type)
                        .unwrap_or_else(|| "i64".to_string())
                } else {
                    match declared_type.as_ref() {
                        Some(array_type @ AirType::Array { .. }) => Self::llvm_type(array_type),
                        _ => "i64".to_string(),
                    }
                };
                allocas.push(format!("    {slot} = alloca {slot_type}, align 8\n"));
            }

            match instruction {
                AirInstruction::If { then_body, else_body, .. } => {
                    Self::collect_mutable_slots(then_body, addressed, slot_by_instruction, allocas, next_slot);
                    if let Some(else_body) = else_body {
                        Self::collect_mutable_slots(else_body, addressed, slot_by_instruction, allocas, next_slot);
                    }
                }
                AirInstruction::While { condition_instructions, body, .. } => {
                    Self::collect_mutable_slots(condition_instructions, addressed, slot_by_instruction, allocas, next_slot);
                    Self::collect_mutable_slots(body, addressed, slot_by_instruction, allocas, next_slot);
                }
                AirInstruction::For { body, .. } => {
                    Self::collect_mutable_slots(body, addressed, slot_by_instruction, allocas, next_slot);
                }
                AirInstruction::Logical { left_instructions, right_instructions, .. } => {
                    Self::collect_mutable_slots(left_instructions, addressed, slot_by_instruction, allocas, next_slot);
                    Self::collect_mutable_slots(right_instructions, addressed, slot_by_instruction, allocas, next_slot);
                }
                _ => {}
            }
        }
    }

    fn is_integer_type(ty: &AirType) -> bool {
        matches!(ty, AirType::I8 | AirType::I16 | AirType::I32 | AirType::U8 | AirType::U16 | AirType::U32)
    }

    fn is_signed_integer_type(ty: &AirType) -> bool {
        matches!(ty, AirType::I8 | AirType::I16 | AirType::I32)
    }

    fn generate_function(&mut self, function: &AirFunction) {
        self.variables.clear();
        self.active_mutable_slots.clear();
        self.mutable_types.clear();
        self.slot_by_instruction.clear();
        self.temporary_types.clear();
        self.next_slot = 0;
        self.next_array = 0;
        self.next_load = 0;
        self.next_promoted = 0;
        self.next_call = 1;
        self.next_if = 1;
        self.next_while = 1;
        self.next_for = 1;
        self.next_logical = 1;
        self.next_bool_string = 1;
        self.current_terminated = false;
        self.current_return_type = function.return_type.clone();

        let mut addressed_variables = HashSet::new();
        Self::collect_addressed_variables(&function.instructions, &mut addressed_variables);

        let mut allocas = Vec::new();
        Self::collect_mutable_slots(
            &function.instructions,
            &addressed_variables,
            &mut self.slot_by_instruction,
            &mut allocas,
            &mut self.next_slot,
        );

        let params = function.params.iter()
            .enumerate()
            .map(|(index, param)| format!("{} %arg{}", Self::llvm_type(&param.ty), index))
            .collect::<Vec<_>>()
            .join(", ");
        self.output.push_str(&format!(
            "define {} @{}({}) {{\n",
            Self::llvm_type(&function.return_type),
            function.name,
            params,
        ));

        self.output.push_str("entry:\n");
        self.current_block = "entry".to_string();
        for alloca in allocas {
            self.output.push_str(&alloca);
        }

        for (index, param) in function.params.iter().enumerate() {
            self.variables.insert(param.name.clone(), GeneratedValue {
                operand: format!("%arg{}", index),
                ty: param.ty.clone(),
            });
        }

        self.generate_instructions(&function.instructions);

        if !self.current_terminated {
            let (llvm_type, default_value) = match &function.return_type {
                AirType::I8 => ("i8", "0"),
                AirType::I16 => ("i16", "0"),
                AirType::I32 => ("i32", "0"),
                AirType::U8 => ("i8", "0"),
                AirType::U16 => ("i16", "0"),
                AirType::U32 => ("i32", "0"),
                AirType::Bool => ("i1", "false"),
                AirType::Str => ("ptr", "null"),
                AirType::Void => ("void", ""),
                AirType::Array { .. } => panic!("LLVM ERR: 배열 반환형은 아직 지원하지 않습니다."),
                AirType::Pointer { .. } => ("ptr", "null"),
                AirType::Named(name) => panic!("LLVM ERR: 지원하지 않는 반환 타입: {name}"),
            };
            if function.return_type == AirType::Void {
                self.output.push_str("    ret void\n");
            } else {
                self.output.push_str(&format!("    ret {llvm_type} {default_value}\n"));
            }
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
            AirInstruction::Binary { destination, operator, left, right } => {
                let mut left_value = self.value(left);
                let mut right_value = self.value(right);

                // 정수 리터럴은 반대편 피연산자의 정수 타입을 따른다.
                if matches!(left, AirValue::Integer(_)) && Self::is_integer_type(&right_value.ty) {
                    left_value.ty = right_value.ty.clone();
                } else if matches!(right, AirValue::Integer(_)) && Self::is_integer_type(&left_value.ty) {
                    right_value.ty = left_value.ty.clone();
                }

                let result_type = match operator {
                    AirBinaryOp::Add | AirBinaryOp::Sub | AirBinaryOp::Mul | AirBinaryOp::Div
                    | AirBinaryOp::ShiftLeft | AirBinaryOp::ShiftRight | AirBinaryOp::BitAnd
                    | AirBinaryOp::BitXor | AirBinaryOp::BitOr => {
                        if !Self::is_integer_type(&left_value.ty) || left_value.ty != right_value.ty {
                            panic!("LLVM ERR: 산술/비트 연산에는 같은 정수 타입이 필요합니다.");
                        }
                        let llvm_type = Self::llvm_type(&left_value.ty);
                        let op = match operator {
                            AirBinaryOp::Add => "add",
                            AirBinaryOp::Sub => "sub",
                            AirBinaryOp::Mul => "mul",
                            AirBinaryOp::Div => if Self::is_signed_integer_type(&left_value.ty) { "sdiv" } else { "udiv" },
                            AirBinaryOp::ShiftLeft => "shl",
                            AirBinaryOp::ShiftRight => if Self::is_signed_integer_type(&left_value.ty) { "ashr" } else { "lshr" },
                            AirBinaryOp::BitAnd => "and",
                            AirBinaryOp::BitXor => "xor",
                            AirBinaryOp::BitOr => "or",
                            _ => unreachable!(),
                        };
                        self.output.push_str(&format!(
                            "    %t{} = {} {} {}, {}\n",
                            destination, op, llvm_type, left_value.operand, right_value.operand
                        ));
                        left_value.ty.clone()
                    }

                    AirBinaryOp::Equal | AirBinaryOp::NotEqual
                    | AirBinaryOp::Less | AirBinaryOp::Greater
                    | AirBinaryOp::LessEqual | AirBinaryOp::GreaterEqual => {
                        if left_value.ty != right_value.ty {
                            panic!("LLVM ERR: 비교 연산의 양쪽 타입이 다릅니다.");
                        }
                        let predicate = match operator {
                            AirBinaryOp::Equal => "eq",
                            AirBinaryOp::NotEqual => "ne",
                            AirBinaryOp::Less => {
                                if !Self::is_integer_type(&left_value.ty) { panic!("LLVM ERR: 크기 비교는 정수 타입이 필요합니다."); }
                                if Self::is_signed_integer_type(&left_value.ty) { "slt" } else { "ult" }
                            }
                            AirBinaryOp::Greater => {
                                if !Self::is_integer_type(&left_value.ty) { panic!("LLVM ERR: 크기 비교는 정수 타입이 필요합니다."); }
                                if Self::is_signed_integer_type(&left_value.ty) { "sgt" } else { "ugt" }
                            }
                            AirBinaryOp::LessEqual => {
                                if !Self::is_integer_type(&left_value.ty) { panic!("LLVM ERR: 크기 비교는 정수 타입이 필요합니다."); }
                                if Self::is_signed_integer_type(&left_value.ty) { "sle" } else { "ule" }
                            }
                            AirBinaryOp::GreaterEqual => {
                                if !Self::is_integer_type(&left_value.ty) { panic!("LLVM ERR: 크기 비교는 정수 타입이 필요합니다."); }
                                if Self::is_signed_integer_type(&left_value.ty) { "sge" } else { "uge" }
                            }
                            _ => unreachable!(),
                        };
                        if !Self::is_integer_type(&left_value.ty) && left_value.ty != AirType::Bool {
                            panic!("LLVM ERR: ==와 !=는 정수와 bool만 지원합니다.");
                        }
                        self.output.push_str(&format!(
                            "    %t{} = icmp {} {} {}, {}\n",
                            destination, predicate, Self::llvm_type(&left_value.ty), left_value.operand, right_value.operand
                        ));
                        AirType::Bool
                    }
                };

                self.temporary_types.insert(*destination, result_type);
            }

            AirInstruction::Unary {
                destination,
                operator,
                operand,
            } => {
                let operand = self.value(operand);

                match operator {
                    AirUnaryOp::Not => {
                        if operand.ty != AirType::Bool {
                            panic!(
                                "LLVM ERR: '!' 연산에는 bool 값이 필요합니다."
                            );
                        }

                        self.output.push_str(&format!(
                            "    %t{} = xor i1 {}, true\n",
                            destination,
                            operand.operand,
                        ));

                        self.temporary_types.insert(
                            *destination,
                            AirType::Bool,
                        );
                    }
                }
            }

            AirInstruction::AddressOf { destination, variable, pointer_type } => {
                let slot = self.active_mutable_slots.get(variable).cloned().unwrap_or_else(|| {
                    panic!("LLVM ERR: 주소를 가져올 변수 '{}'의 저장 공간이 없습니다.", variable)
                });
                let element_type = match pointer_type {
                    AirType::Pointer { element_type, .. } => element_type.as_ref(),
                    _ => panic!("LLVM ERR: address_of 결과 타입이 포인터가 아닙니다."),
                };
                let llvm_element_type = Self::llvm_type(element_type);
                self.output.push_str(&format!(
                    "    %t{destination} = getelementptr inbounds {llvm_element_type}, ptr {slot}, i32 0\n"
                ));
                self.temporary_types.insert(*destination, pointer_type.clone());
            }

            AirInstruction::PointerLoad { destination, pointer, pointer_type } => {
                let pointer_value = self.value(pointer);
                if &pointer_value.ty != pointer_type {
                    panic!("LLVM ERR: 역참조 포인터 타입이 일치하지 않습니다.");
                }
                let element_type = match pointer_type {
                    AirType::Pointer { element_type, .. } => element_type.as_ref(),
                    _ => panic!("LLVM ERR: load 대상이 포인터가 아닙니다."),
                };
                let llvm_element_type = Self::llvm_type(element_type);
                self.output.push_str(&format!(
                    "    %t{destination} = load {llvm_element_type}, ptr {}\n",
                    pointer_value.operand
                ));
                self.temporary_types.insert(*destination, (*element_type).clone());
            }

            AirInstruction::PointerStore { pointer, value, pointer_type } => {
                let pointer_value = self.value(pointer);
                if &pointer_value.ty != pointer_type {
                    panic!("LLVM ERR: store 대상 포인터 타입이 일치하지 않습니다.");
                }
                let element_type = match pointer_type {
                    AirType::Pointer { element_type, .. } => element_type.as_ref(),
                    _ => panic!("LLVM ERR: store 대상이 포인터가 아닙니다."),
                };
                let mut generated = self.value(value);
                if matches!(value, AirValue::Integer(_)) && Self::is_integer_type(element_type) {
                    generated.ty = element_type.clone();
                }
                if generated.ty != *element_type {
                    panic!("LLVM ERR: 포인터에 대입하는 값의 타입이 다릅니다.");
                }
                let llvm_element_type = Self::llvm_type(element_type);
                self.output.push_str(&format!(
                    "    store {llvm_element_type} {}, ptr {}\n",
                    generated.operand, pointer_value.operand
                ));
            }

            AirInstruction::Logical {
                destination,
                operator,
                left_instructions,
                left,
                right_instructions,
                right,
            } => {
                self.generate_logical(
                    *destination,
                    operator,
                    left_instructions,
                    left,
                    right_instructions,
                    right,
                );
            }

            AirInstruction::Let { name, ty, value, mutable } => {
                // Array literals are lowered here into a real stack object plus per-element stores.
                // They are not represented as a scalar LLVM operand.
                let precomputed = if matches!(ty, Some(AirType::Array { .. })) || matches!(value, AirValue::Array(_)) {
                    None
                } else {
                    Some(self.value(value))
                };
                let declared_type = ty.clone()
                    .or_else(|| precomputed.as_ref().map(|generated| generated.ty.clone()))
                    .or_else(|| Self::infer_value_type(value))
                    .unwrap_or_else(|| panic!("LLVM ERR: 변수 '{}'의 타입을 추론할 수 없습니다.", name));

                if let AirType::Array { .. } = &declared_type {
                    let array_id = self.next_array;
                    self.next_array += 1;
                    let key = instruction as *const AirInstruction as usize;
                    let slot = if *mutable || self.slot_by_instruction.contains_key(&key) {
                        self.slot_by_instruction.get(&key).cloned().unwrap_or_else(|| {
                            panic!("LLVM ERR: 배열 변수 '{}'의 저장 공간이 없습니다.", name)
                        })
                    } else {
                        let slot = format!("%array{array_id}");
                        let llvm_array_type = Self::llvm_type(&declared_type);
                        self.output.push_str(&format!(
                            "    {slot} = alloca {llvm_array_type}, align 8\n"
                        ));
                        slot
                    };

                    if let AirValue::Array(elements) = value {
                        self.store_array_elements(&slot, &declared_type, elements, array_id);
                    } else {
                        let generated = precomputed.unwrap_or_else(|| self.value(value));
                        if generated.ty != declared_type {
                            panic!(
                                "LLVM ERR: 변수 '{}'의 배열 초기값 타입이 다릅니다. 선언: {:?}, 실제: {:?}",
                                name, declared_type, generated.ty
                            );
                        }
                        let llvm_array_type = Self::llvm_type(&declared_type);
                        self.output.push_str(&format!(
                            "    store {llvm_array_type} {}, ptr {slot}\n", generated.operand
                        ));
                    }

                    if *mutable || self.slot_by_instruction.contains_key(&key) {
                        self.variables.remove(name);
                        self.active_mutable_slots.insert(name.clone(), slot);
                        self.mutable_types.insert(name.clone(), declared_type);
                    } else {
                        self.active_mutable_slots.remove(name);
                        self.mutable_types.remove(name);
                        self.variables.insert(name.clone(), GeneratedValue {
                            operand: slot,
                            ty: declared_type,
                        });
                    }
                } else {
                    let mut generated = precomputed.unwrap_or_else(|| self.value(value));
                    if matches!(value, AirValue::Integer(_)) && Self::is_integer_type(&declared_type) {
                        generated.ty = declared_type.clone();
                    }
                    if declared_type != generated.ty {
                        panic!(
                            "LLVM ERR: 변수 '{}'의 선언 타입과 초기값 타입이 다릅니다. 선언: {:?}, 실제: {:?}",
                            name, declared_type, generated.ty
                        );
                    }

                    let key = instruction as *const AirInstruction as usize;
                    if *mutable || self.slot_by_instruction.contains_key(&key) {
                        let slot = self.slot_by_instruction.get(&key).cloned().unwrap_or_else(|| {
                            panic!("LLVM ERR: 변수 '{}'의 저장 공간이 없습니다.", name)
                        });
                        self.variables.remove(name);
                        self.active_mutable_slots.insert(name.clone(), slot.clone());
                        self.mutable_types.insert(name.clone(), declared_type.clone());
                        self.output.push_str(&format!(
                            "    store {} {}, ptr {}\n",
                            Self::llvm_type(&declared_type), generated.operand, slot
                        ));
                    } else {
                        self.active_mutable_slots.remove(name);
                        self.mutable_types.remove(name);
                        self.variables.insert(name.clone(), generated);
                    }
                }
            }

            AirInstruction::Assign { name, value } => {
                let slot = self.active_mutable_slots.get(name).cloned().unwrap_or_else(|| {
                    panic!("LLVM ERR: 불변이거나 선언되지 않은 변수 '{}'에 대입했습니다.", name)
                });
                let expected_type = self.mutable_types.get(name).cloned().unwrap_or_else(|| {
                    panic!("LLVM ERR: 가변 변수 '{}'의 타입 정보가 없습니다.", name)
                });
                if matches!(expected_type, AirType::Array { .. }) {
                    if let AirValue::Array(elements) = value {
                        let array_id = self.next_array;
                        self.next_array += 1;
                        self.store_array_elements(&slot, &expected_type, elements, array_id);
                    } else {
                        let generated = self.value(value);
                        if generated.ty != expected_type {
                            panic!("LLVM ERR: 변수 '{}'에 대입하는 배열 타입이 다릅니다.", name);
                        }
                        let llvm_array_type = Self::llvm_type(&expected_type);
                        self.output.push_str(&format!(
                            "    store {llvm_array_type} {}, ptr {slot}\n", generated.operand
                        ));
                    }
                } else {
                    let mut generated = self.value(value);
                    if matches!(value, AirValue::Integer(_)) && Self::is_integer_type(&expected_type) {
                        generated.ty = expected_type.clone();
                    }
                    if generated.ty != expected_type {
                        panic!("LLVM ERR: 변수 '{}'에 대입하는 값의 타입이 다릅니다.", name);
                    }
                    self.output.push_str(&format!(
                        "    store {} {}, ptr {}\n",
                        Self::llvm_type(&expected_type), generated.operand, slot
                    ));
                }
            }

            AirInstruction::Return { value } => {
                let mut generated = self.value(value);
                if matches!(value, AirValue::Integer(_)) && Self::is_integer_type(&self.current_return_type) {
                    generated.ty = self.current_return_type.clone();
                }
                if generated.ty != self.current_return_type {
                    panic!("LLVM ERR: 반환 값의 타입이 함수 반환형과 다릅니다.");
                }
                self.output.push_str(&format!(
                    "    ret {} {}\n",
                    Self::llvm_type(&generated.ty), generated.operand
                ));
                self.current_terminated = true;
            }

            AirInstruction::Call { destination, function, args } => {
                if function == "print" || function == "println" {
                    if destination.is_some() {
                        panic!("LLVM ERR: print/println은 반환값이 있는 식으로 사용할 수 없습니다.");
                    }
                    if args.len() != 1 {
                        panic!("LLVM ERR: 함수 인자는 하나여야 합니다.");
                    }

                    let value = self.value(&args[0]);
                    let format_string = match (function.as_str(), &value.ty) {
                        ("print", AirType::I8 | AirType::I16 | AirType::I32) => "@.fmt.print.i32",
                        ("println", AirType::I8 | AirType::I16 | AirType::I32) => "@.fmt.println.i32",
                        ("print", AirType::U8 | AirType::U16 | AirType::U32) => "@.fmt.print.u32",
                        ("println", AirType::U8 | AirType::U16 | AirType::U32) => "@.fmt.println.u32",

                        ("print", AirType::Str) => "@.fmt.print.str",
                        ("println", AirType::Str) => "@.fmt.println.str",

                        ("print", AirType::Bool) => "@.fmt.print.bool",
                        ("println", AirType::Bool) => "@.fmt.println.bool",

                        (_, AirType::Named(name)) => {
                            panic!(
                                "LLVM ERR: 출력할 수 없는 타입입니다: {name}"
                            );
                        }

                        (_, AirType::Void) => {
                            panic!("LLVM ERR: void 값은 출력할 수 없습니다.");
                        }

                        _ => {
                            panic!(
                                "LLVM ERR: 지원하지 않는 출력 함수 또는 타입입니다: {}",
                                function
                            );
                        }
                    };

                    let (llvm_type, operand) = match &value.ty {
                        AirType::I8 => ("i32", self.promote_integer(&value.operand, "i8", true)),
                        AirType::I16 => ("i32", self.promote_integer(&value.operand, "i16", true)),
                        AirType::I32 | AirType::U32 => ("i32", value.operand.clone()),
                        AirType::U8 => ("i32", self.promote_integer(&value.operand, "i8", false)),
                        AirType::U16 => ("i32", self.promote_integer(&value.operand, "i16", false)),
                        AirType::Str => ("ptr", value.operand.clone()),
                        AirType::Bool => ("ptr", self.bool_string_pointer(&value.operand)),
                        AirType::Array { .. } => panic!("LLVM ERR: 배열 전체를 print/println으로 출력할 수 없습니다."),
                        AirType::Void => panic!("LLVM ERR: void 값은 출력할 수 없습니다."),
                        AirType::Pointer { .. } => panic!("LLVM ERR: 포인터 자체는 출력할 수 없습니다."),
                        AirType::Named(name) => panic!("LLVM ERR: 출력할 수 없는 타입입니다: {name}"),
                    };

                    let call_id = self.next_call;
                    self.next_call += 1;
                    self.output.push_str(&format!(
                        "    %call{} = call i32 (ptr, ...) @printf(ptr {}, {} {})\n",
                        call_id, format_string, llvm_type, operand
                    ));
                } else {
                    let (param_types, return_type) = self.function_signatures
                        .get(function)
                        .cloned()
                        .unwrap_or_else(|| panic!("LLVM ERR: 선언되지 않은 함수: {function}"));

                    if args.len() != param_types.len() {
                        panic!("LLVM ERR: 함수 '{function}'의 인자 개수가 맞지 않습니다.");
                    }

                    let mut generated_args = Vec::with_capacity(args.len());
                    for (arg, expected_type) in args.iter().zip(param_types.iter()) {
                        let mut value = self.value(arg);
                        if matches!(arg, AirValue::Integer(_)) && Self::is_integer_type(expected_type) {
                            value.ty = expected_type.clone();
                        }
                        if &value.ty != expected_type {
                            panic!("LLVM ERR: 함수 '{function}'의 인자 타입이 맞지 않습니다.");
                        }
                        generated_args.push(format!("{} {}", Self::llvm_type(&value.ty), value.operand));
                    }
                    let args_text = generated_args.join(", ");
                    let llvm_return_type = Self::llvm_type(&return_type);

                    if let Some(destination) = destination {
                        if return_type == AirType::Void {
                            panic!("LLVM ERR: 반환값이 없는 함수 '{function}'를 식 안에서 사용할 수 없습니다.");
                        }
                        self.output.push_str(&format!(
                            "    %t{} = call {} @{}({})\n",
                            destination, llvm_return_type, function, args_text
                        ));
                        self.temporary_types.insert(*destination, return_type);
                    } else {
                        self.output.push_str(&format!(
                            "    call {} @{}({})\n",
                            llvm_return_type, function, args_text
                        ));
                    }
                }
            }

            AirInstruction::If {
                condition,
                then_body,
                else_body,
            } => {
                self.generate_if(condition, then_body, else_body.as_deref());
            }

            AirInstruction::While {
                condition_instructions,
                condition,
                body,
            } => {
                self.generate_while(condition_instructions, condition, body);
            }

            AirInstruction::For { name, start, end, body } => {
                self.generate_for(instruction, name, start, end, body);
            }
        }
    }

    fn generate_if(&mut self, condition: &AirValue, then_body: &[AirInstruction], else_body: Option<&[AirInstruction]>) {
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
        let outer_mutable_slots = self.active_mutable_slots.clone();
        let outer_mutable_types = self.mutable_types.clone();

        self.output.push_str(&format!("{then_label}:\n"));
        self.current_block = then_label.clone();
        self.current_terminated = false;
        self.variables = outer_variables.clone();
        self.active_mutable_slots = outer_mutable_slots.clone();
        self.mutable_types = outer_mutable_types.clone();

        self.generate_instructions(then_body);

        if !self.current_terminated {
            self.output
                .push_str(&format!("    br label %{end_label}\n"));
            self.current_terminated = true;
        }

        self.output.push_str(&format!("{else_label}:\n"));
        self.current_block = else_label.clone();
        self.current_terminated = false;
        self.variables = outer_variables.clone();
        self.active_mutable_slots = outer_mutable_slots.clone();
        self.mutable_types = outer_mutable_types.clone();

        if let Some(else_body) = else_body {
            self.generate_instructions(else_body);
        }

        if !self.current_terminated {
            self.output
                .push_str(&format!("    br label %{end_label}\n"));
            self.current_terminated = true;
        }

        self.output.push_str(&format!("{end_label}:\n"));
        self.current_block = end_label.clone();

        self.variables = outer_variables;
        self.active_mutable_slots = outer_mutable_slots;
        self.mutable_types = outer_mutable_types;
        self.current_terminated = false;
    }

    fn generate_logical(
        &mut self,
        destination: u32,
        operator: &AirLogicalOp,
        left_instructions: &[AirInstruction],
        left: &AirValue,
        right_instructions: &[AirInstruction],
        right: &AirValue,
    ) {
        // 왼쪽 식은 먼저 평가한다.
        self.generate_instructions(left_instructions);

        if self.current_terminated {
            panic!(
                "LLVM ERR: 논리 연산의 왼쪽 식이 비정상적으로 종료되었습니다."
            );
        }

        let left_value = self.value(left);

        if left_value.ty != AirType::Bool {
            panic!(
                "LLVM ERR: 논리 연산의 왼쪽 값에는 bool 타입이 필요합니다."
            );
        }

        let left_block = self.current_block.clone();

        let logical_id = self.next_logical;
        self.next_logical += 1;

        let right_label =
            format!("logical.right.{logical_id}");

        let end_label =
            format!("logical.end.{logical_id}");

        let (true_target, false_target, short_value) =
            match operator {
                AirLogicalOp::And => {
                    (&right_label, &end_label, "false")
                }

                AirLogicalOp::Or => {
                    (&end_label, &right_label, "true")
                }
            };

        // AND: false이면 오른쪽 식을 건너뛴다.
        // OR: true이면 오른쪽 식을 건너뛴다.
        self.output.push_str(&format!(
            "    br i1 {}, label %{}, label %{}\n",
            left_value.operand,
            true_target,
            false_target,
        ));

        self.current_terminated = true;

        // 오른쪽 식을 평가하는 블록
        self.output.push_str(
            &format!("{right_label}:\n")
        );

        self.current_block = right_label.clone();
        self.current_terminated = false;

        self.generate_instructions(right_instructions);

        if self.current_terminated {
            panic!(
                "LLVM ERR: 논리 연산의 오른쪽 식이 비정상적으로 종료되었습니다."
            );
        }

        let right_value = self.value(right);

        if right_value.ty != AirType::Bool {
            panic!(
                "LLVM ERR: 논리 연산의 오른쪽 값에는 bool 타입이 필요합니다."
            );
        }

        let right_block = self.current_block.clone();

        self.output.push_str(&format!(
            "    br label %{end_label}\n"
        ));

        self.current_terminated = true;

        // 두 경로가 합쳐지는 블록
        self.output.push_str(
            &format!("{end_label}:\n")
        );

        self.current_block = end_label;
        self.current_terminated = false;

        // 단락 평가 경로의 값과 오른쪽 식의 값을 합친다.
        self.output.push_str(&format!(
            "    %t{} = phi i1 [ {}, %{} ], [ {}, %{} ]\n",
            destination,
            short_value,
            left_block,
            right_value.operand,
            right_block,
        ));

        self.temporary_types.insert(
            destination,
            AirType::Bool,
        );
    }

    fn generate_for(
        &mut self,
        instruction: &AirInstruction,
        name: &str,
        start: &AirValue,
        end: &AirValue,
        body: &[AirInstruction],
    ) {
        let key = instruction as *const AirInstruction as usize;
        let slot = self.slot_by_instruction.get(&key).cloned().unwrap_or_else(|| {
            panic!("LLVM ERR: for 반복 변수의 저장 공간이 없습니다.")
        });

        // 범위 시작과 끝은 반복문에 진입하기 전에 한 번만 평가한다.
        let mut start_value = self.value(start);
        let mut end_value = self.value(end);
        if matches!(start, AirValue::Integer(_)) && Self::is_integer_type(&end_value.ty) {
            start_value.ty = end_value.ty.clone();
        } else if matches!(end, AirValue::Integer(_)) && Self::is_integer_type(&start_value.ty) {
            end_value.ty = start_value.ty.clone();
        }
        if !Self::is_integer_type(&start_value.ty) || start_value.ty != end_value.ty {
            panic!("LLVM ERR: for 범위는 같은 정수 타입이어야 합니다.");
        }
        let index_type = start_value.ty.clone();

        let outer_variables = self.variables.clone();
        let outer_mutable_slots = self.active_mutable_slots.clone();
        let outer_mutable_types = self.mutable_types.clone();

        self.variables.remove(name);
        self.active_mutable_slots.insert(name.to_string(), slot.clone());
        self.mutable_types.insert(name.to_string(), index_type.clone());
        self.output.push_str(&format!(
            "    store {} {}, ptr {}\n",
            Self::llvm_type(&index_type), start_value.operand, slot
        ));

        let for_id = self.next_for;
        self.next_for += 1;
        let condition_label = format!("for.condition.{for_id}");
        let body_label = format!("for.body.{for_id}");
        let end_label = format!("for.end.{for_id}");

        self.output.push_str(&format!("    br label %{condition_label}\n"));
        self.current_terminated = true;

        self.output.push_str(&format!("{condition_label}:\n"));
        self.current_block = condition_label.clone();
        self.current_terminated = false;
        let index = self.value(&AirValue::Variable(name.to_string()));
        let predicate = if Self::is_signed_integer_type(&index_type) { "slt" } else { "ult" };
        let condition_name = format!("%for.cond.{for_id}");
        self.output.push_str(&format!(
            "    {condition_name} = icmp {predicate} {} {}, {}\n",
            Self::llvm_type(&index_type), index.operand, end_value.operand
        ));
        self.output.push_str(&format!(
            "    br i1 {condition_name}, label %{body_label}, label %{end_label}\n"
        ));
        self.current_terminated = true;

        self.output.push_str(&format!("{body_label}:\n"));
        self.current_block = body_label;
        self.current_terminated = false;
        let loop_variables = self.variables.clone();
        let loop_mutable_slots = self.active_mutable_slots.clone();
        let loop_mutable_types = self.mutable_types.clone();
        self.generate_instructions(body);
        self.variables = loop_variables;
        self.active_mutable_slots = loop_mutable_slots;
        self.mutable_types = loop_mutable_types;

        if !self.current_terminated {
            let index = self.value(&AirValue::Variable(name.to_string()));
            let next_value = format!("%for.next.{for_id}");
            self.output.push_str(&format!(
                "    {next_value} = add {} {}, 1\n",
                Self::llvm_type(&index_type), index.operand
            ));
            self.output.push_str(&format!(
                "    store {} {next_value}, ptr {slot}\n",
                Self::llvm_type(&index_type)
            ));
            self.output.push_str(&format!("    br label %{condition_label}\n"));
            self.current_terminated = true;
        }

        self.output.push_str(&format!("{end_label}:\n"));
        self.current_block = end_label;
        self.variables = outer_variables;
        self.active_mutable_slots = outer_mutable_slots;
        self.mutable_types = outer_mutable_types;
        self.current_terminated = false;
    }

    fn generate_while(
        &mut self,
        condition_instructions: &[AirInstruction],
        condition: &AirValue,
        body: &[AirInstruction],
    ) {
        let while_id = self.next_while;
        self.next_while += 1;

        let condition_label = format!("while.condition.{while_id}");
        let body_label = format!("while.body.{while_id}");
        let end_label = format!("while.end.{while_id}");

        self.output.push_str(&format!("    br label %{condition_label}\n"));
        self.current_terminated = true;

        let outer_variables = self.variables.clone();
        let outer_mutable_slots = self.active_mutable_slots.clone();
        let outer_mutable_types = self.mutable_types.clone();

        self.output.push_str(&format!("{condition_label}:\n"));
        self.current_block = condition_label.clone();
        self.current_terminated = false;
        self.variables = outer_variables.clone();
        self.active_mutable_slots = outer_mutable_slots.clone();
        self.mutable_types = outer_mutable_types.clone();
        self.generate_instructions(condition_instructions);

        let condition = self.value(condition);
        if condition.ty != AirType::Bool {
            panic!("LLVM ERR: while 조건식에는 bool 타입이 필요합니다.");
        }
        self.output.push_str(&format!(
            "    br i1 {}, label %{}, label %{}\n",
            condition.operand, body_label, end_label,
        ));
        self.current_terminated = true;

        self.output.push_str(&format!("{body_label}:\n"));
        self.current_block = body_label.clone();
        self.current_terminated = false;
        self.variables = outer_variables.clone();
        self.active_mutable_slots = outer_mutable_slots.clone();
        self.mutable_types = outer_mutable_types.clone();
        self.generate_instructions(body);

        if !self.current_terminated {
            self.output.push_str(&format!("    br label %{condition_label}\n"));
            self.current_terminated = true;
        }

        self.output.push_str(&format!("{end_label}:\n"));
        self.current_block = end_label;
        self.variables = outer_variables;
        self.active_mutable_slots = outer_mutable_slots;
        self.mutable_types = outer_mutable_types;
        self.current_terminated = false;
    }

    fn value(&mut self, value: &AirValue) -> GeneratedValue {
        match value {
            AirValue::Integer(value) => GeneratedValue {
                operand: value.to_string(),
                ty: AirType::I32,
            },

            AirValue::TypedInteger(value, ty) => GeneratedValue {
                operand: value.to_string(),
                ty: ty.clone(),
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

            AirValue::Variable(name) => {
                if let Some(slot) = self.active_mutable_slots.get(name).cloned() {
                    let ty = self.mutable_types.get(name).cloned().unwrap_or_else(|| {
                        panic!("LLVM ERR: 가변 변수 '{name}'의 타입을 찾을 수 없습니다.")
                    });
                    let load = format!("%load{}", self.next_load);
                    self.next_load += 1;
                    self.output.push_str(&format!(
                        "    {load} = load {}, ptr {slot}\n",
                        Self::llvm_type(&ty)
                    ));
                    GeneratedValue { operand: load, ty }
                } else {
                    let generated = self.variables.get(name).cloned().unwrap_or_else(|| {
                        panic!("LLVM ERR: 변수를 찾을 수 없습니다: {name}")
                    });
                    if matches!(generated.ty, AirType::Array { .. }) {
                        let load = format!("%load{}", self.next_load);
                        self.next_load += 1;
                        let llvm_array_type = Self::llvm_type(&generated.ty);
                        self.output.push_str(&format!(
                            "    {load} = load {llvm_array_type}, ptr {}\n", generated.operand
                        ));
                        GeneratedValue { operand: load, ty: generated.ty }
                    } else {
                        generated
                    }
                }
            },

            AirValue::Array(_) => {
                panic!("LLVM ERR: 배열 리터럴은 배열 변수 초기화 또는 대입에서 처리해야 합니다.")
            },
        }
    }

    fn llvm_type(ty: &AirType) -> String {
        match ty {
            AirType::I8 | AirType::U8 => "i8".to_string(),
            AirType::I16 | AirType::U16 => "i16".to_string(),
            AirType::I32 | AirType::U32 => "i32".to_string(),
            AirType::Bool => "i1".to_string(),
            AirType::Str => "ptr".to_string(),
            AirType::Void => "void".to_string(),
            AirType::Array { element_type, size } => {
                format!("[{} x {}]", size, Self::llvm_type(element_type))
            }
            AirType::Pointer { .. } => "ptr".to_string(),
            AirType::Named(name) => panic!("LLVM ERR: 지원하지 않는 타입: {name}"),
        }
    }

    fn promote_integer(&mut self, operand: &str, source_type: &str, signed: bool) -> String {
        let name = format!("%promoted{}", self.next_promoted);
        self.next_promoted += 1;
        let extension = if signed { "sext" } else { "zext" };
        self.output.push_str(&format!(
            "    {name} = {extension} {source_type} {operand} to i32\n"
        ));
        name
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

