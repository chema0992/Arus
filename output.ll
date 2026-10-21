@.fmt.print.i32 = private unnamed_addr constant [3 x i8] c"%d\00", align 1
@.fmt.println.i32 = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@.fmt.print.str = private unnamed_addr constant [3 x i8] c"%s\00", align 1
@.fmt.println.str = private unnamed_addr constant [4 x i8] c"%s\0A\00", align 1

declare i32 @printf(ptr, ...)

@.str0 = private unnamed_addr constant [8 x i8] c"Hello, \00", align 1
@.str1 = private unnamed_addr constant [6 x i8] c"Arus!\00", align 1
@.str2 = private unnamed_addr constant [19 x i8] c"\EC\95\88\EB\85\95, \EC\95\84\EB\A3\A8\EC\8A\A4!\00", align 1
@.str3 = private unnamed_addr constant [21 x i8] c"\ED\83\80\EC\9E\85 \EC\B6\94\EB\A1\A0\EB\8F\84 \EB\90\A8\00", align 1
define i32 @main() {
entry:
    %call1 = call i32 (ptr, ...) @printf(ptr @.fmt.print.str, ptr getelementptr inbounds ([8 x i8], ptr @.str0, i64 0, i64 0))
    %call2 = call i32 (ptr, ...) @printf(ptr @.fmt.println.str, ptr getelementptr inbounds ([6 x i8], ptr @.str1, i64 0, i64 0))
    %call3 = call i32 (ptr, ...) @printf(ptr @.fmt.println.str, ptr getelementptr inbounds ([19 x i8], ptr @.str2, i64 0, i64 0))
    %call4 = call i32 (ptr, ...) @printf(ptr @.fmt.println.str, ptr getelementptr inbounds ([21 x i8], ptr @.str3, i64 0, i64 0))
    %call5 = call i32 (ptr, ...) @printf(ptr @.fmt.println.i32, i32 123)
    ret i32 0
}

