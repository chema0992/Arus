# Arus

<img width="1280" height="640" alt="logo2" src="https://github.com/user-attachments/assets/6cfaa956-3daf-484c-80a3-35a3a3223af1" />

![Rust](https://shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Apache](https://shields.io/badge/License-Apache_2.0-green.svg?style=for-the-badge)
![WIP](https://shields.io/badge/Status-WIP-yellow?style=for-the-badge&logo=git)

> **작성은 쉽게, 성능은 강력하게.**

[Arus](https://arus-lang.pages.dev)는 읽고 쓰기 쉬운 문법과 네이티브 코드 성능을 목표로 하는 시스템 프로그래밍 언어입니다.
독자적인 중간 표현 **AIR(Arus Intermediate Representation)** 와 권한(permission) 기반 포인터 모델을 핵심 설계로 삼습니다.

> [!WARNING]
> **Arus는 v1.0 이전의 초기 개발 단계입니다.**
> 아래 내용 중 **「현재 지원」** 은 구현된 동작을, **「설계 중」** 은 방향만 정해진 기능을 나타냅니다.
> 문법과 의미는 개발 과정에서 바뀔 수 있습니다.

## 설계 목표

- **쉬운 작성:** 불필요한 구문 장식을 줄이고, 읽기 쉬운 코드를 지향합니다.
- **네이티브 컴파일:** LLVM을 통해 네이티브 머신 코드를 생성합니다. GC 없이 런타임 의존을 최소화합니다.
- **명시적인 메모리 권한:** 포인터의 읽기·쓰기 권한을 타입에 표현하고, 컴파일 단계에서 잘못된 접근을 검사합니다. *(설계 중)*
- **독립적인 중간 표현:** AIR를 프론트엔드와 백엔드 사이의 경계로 둡니다.
- **점진적인 셀프호스팅:** 초기 컴파일러는 Rust로 작성하고, 언어가 성숙한 뒤 Arus로 재작성하는 것을 장기 목표로 합니다.

## 현재 지원하는 기능

대략 다음이 동작합니다.

| 영역 | 내용 |
|------|------|
| 프로그램 | `script`, `main` + `fn main()` |
| 타입 | `i8` `i16` `i32`, `u8` `u16` `u32`, `bool`, `str` |
| 변수 | `let` / `mut`, 타입 명시·추론, 대입 |
| 연산 | 산술, 비트, 비교, 논리, 연산자 우선순위 |
| 제어 | `if` / `else if` / `else`, `while`, `for` + `0..n` (반열린 구간) |
| 함수 | 인자, `->` 반환 타입, `return` |
| 입출력 | `print`, `println` (정수·문자열·불리언 등) |

회귀·문법 확인용 예제는 [`examples/`](examples/) 를 참고하세요.

## 컴파일 구조

```text
Arus Source
    ↓
Lexer / Parser
    ↓
AST
    ↓
AIR          ← 타입 등 언어 차원 검사
    ↓
LLVM IR
    ↓
Native Machine Code
```

- **AST:** 구문 구조를 표현합니다.
- **AIR:** 단순하고 명시적인 연산으로 프로그램을 나타내고, LLVM IR 이전에 언어 규칙을 검사합니다.
- **LLVM:** 최적화와 네이티브 코드 생성을 담당합니다.

AIR는 LLVM IR을 복제하는 단계가 아닙니다. **Arus의 의미를 유지하면서 분석하기 쉬운 중간 표현**을 제공하는 것이 목적입니다.

### AIR 예시 (방향)

```text
%1 = add 100 60
%2 = mul %1 2
%3 = sub %2 10
return %3
```

명령 집합, 타입 표기, 블록·함수 문법은 아직 확정되지 않았습니다.

## 실행 방법

```sh
cargo run -- path/to/file.arus
```

장기적으로는 다음 CLI 형태를 목표로 합니다. 현재 구현과 다를 수 있습니다.

```sh
arus run test.arus
arus build main.arus
```

## 프로그램 구조

### `script`

main 함수를 진입점으로 쓰지 않고, 파일 최상위 문장부터 하나하나씩 파싱합니다.

```arus
script

println("Hello, Arus!")
let value: i32 = 10
println(value)
return 0
```

### `main`

명시적인 진입 함수 `main`을 사용합니다.

```arus
main

fn main() {
    println("Hello, Arus!")
}
```

## 문법 (현재 구현 기준)

### 변수와 타입

```arus
let count: i32 = 10
mut total: i32 = 20
total = total + 1

let inferred = 30
let flag: bool = true
let text: str = "Arus"
```

- `let`은 불변 바인딩, `mut`은 가변 바인딩입니다.
- 타입을 생략하면 타입 추론을 사용합니다.
- 세미콜론은 필수가 아닙니다.

### 연산

```arus
let sum = 10 + 5
let shl = 1 << 4
let ok = 10 < 20 && true
```

산술, 비트, 비교, 논리 연산과 일반적인 우선순위를 지원합니다.

### 조건과 반복

```arus
if x > 20 {
    println("large")
} else if x > 10 {
    println("medium")
} else {
    println("small")
}

mut i: i32 = 0
while i < 3 {
    println(i)
    i = i + 1
}

for j in 0..5 {
    println(j)  // 0, 1, 2, 3, 4
}
```

- 조건식에 괄호를 요구하지 않습니다. 조건은 bool 타입이어야 합니다.
- `for a..b` 는 **a 이상 b 미만** 입니다.

### 함수

```arus
fn add(a: i32, b: i32) -> i32 {
    return a + b
}

let result: i32 = add(10, 20)
println(result)
```

반환은 `return`으로 명시합니다.

### 출력

```arus
print("no newline")
println("with newline")
println(42)
```

## 설계 중

아래는 방향만 정해졌거나 문서용 예시입니다. **현재 컴파일러에서 동작한다고 보장하지 않습니다.**

### 권한 기반 포인터

| 표기 | 의미 |
|------|------|
| `T r*` | 읽기 권한 |
| `T w*` | 쓰기 권한 |
| `T rw*` | 읽기 및 쓰기 권한 |

```arus
let value: i32 = 5
let pointer: i32 r* = &value
let copy = *pointer
// *pointer = 10  → 컴파일 오류 예정 (읽기 전용 포인터로 쓰기)
```

`let` / `mut` 가변성과 포인터 권한은 별개로 다룰 예정입니다. 별칭, 수명, 해제 규칙은 추후 정의합니다.

### 배열

```arus
let fixed: [i32; 3] = [10, 20, 30]
// mut values: [i32] = [1, 2, 3]
// values.push(4)
```

`[T; N]` 은 고정 크기, `[T]` 는 동적 배열을 검토 중입니다.

### 그 외

문자열 보간(`f"..."`), `struct` / `enum` / `match`, `input()` 등은 문법 초안 단계이며 구현 범위에 포함되기 전입니다.

## 로드맵

| 상태 | 항목 |
|------|------|
| 진행 | 기본 문법, 타입, 제어 흐름, 함수, AST → AIR → LLVM |
| 예정 | 배열, 포인터 권한 모델, 진단 메시지 개선 |
| 예정 | CLI (`run` / `build`), 예제·테스트 확충 |
| 장기 | 인터프리터 모드, 셀프호스팅 |

## 프로젝트 상태

실험적인 초기 단계입니다. 문법, 의미, 빌드 방법은 예고 없이 바뀔 수 있습니다.
피드백과 이슈는 저장소에서 환영합니다.

공식 사이트: [arus-lang.pages.dev](https://arus-lang.pages.dev)

## 라이선스

[LICENSE](LICENSE) 문서를 참고하세요.

