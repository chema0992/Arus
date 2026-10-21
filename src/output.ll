@.fmt.print.i32 = private unnamed_addr constant [3 x i8] c"%d\00", align 1
    @.fmt.println.i32 = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1

    declare i32 @printf(ptr, ...)

    define i32 @main() {
entry:
    %call1 = call i32 (ptr, ...) @printf(ptr @.fmt.print.i32, i32 123)
    %call2 = call i32 (ptr, ...) @printf(ptr @.fmt.println.i32, i32 456)
    %t1 = add i32 10, 10
    %t2 = mul i32 %t1, 3
    %call3 = call i32 (ptr, ...) @printf(ptr @.fmt.println.i32, i32 %t2)
    %call4 = call i32 (ptr, ...) @printf(ptr @.fmt.println.i32, i32 10)
    ret i32 10
}

