; ModuleID = 'titrate_main'
source_filename = "titrate_main"

@__titrate_exception = internal global ptr null

declare void @titrate_println(i64, ptr)

declare ptr @titrate_string_concat(i64, ptr, i64, ptr, ptr)

declare void @titrate_free(ptr)

declare ptr @titrate_malloc(i64)

declare void @titrate_println_int(i64)

declare void @titrate_println_double(double)

declare void @titrate_println_bool(i32)

declare void @titrate_println_char(i32)

define i32 @main() {
entry:
  %native.args = alloca [1 x { i32, i32, [16 x i8] }], align 8
  %tv.payload = alloca [16 x i8], align 1
  store i64 42, ptr %tv.payload, align 4
  %tv.payload.val = load [16 x i8], ptr %tv.payload, align 1
  %tv.payload.final = insertvalue { i32, i32, [16 x i8] } { i32 5, i32 0, [16 x i8] zeroinitializer }, [16 x i8] %tv.payload.val, 2
  %native.arg.0 = getelementptr inbounds [1 x { i32, i32, [16 x i8] }], ptr %native.args, i32 0
  store { i32, i32, [16 x i8] } %tv.payload.final, ptr %native.arg.0, align 4
  %native.call = call { i32, i32, [16 x i8] } @titrate_toString(ptr %native.args, i64 1)
  %tv.result.payload = extractvalue { i32, i32, [16 x i8] } %native.call, 2
  %tv.result.alloca = alloca [16 x i8], align 1
  store [16 x i8] %tv.result.payload, ptr %tv.result.alloca, align 1
  %tv.result.val = load { i64, ptr }, ptr %tv.result.alloca, align 8
  %sv.len = extractvalue { i64, ptr } %tv.result.val, 0
  %sv.ptr = extractvalue { i64, ptr } %tv.result.val, 1
  call void @titrate_println(i64 %sv.len, ptr %sv.ptr)
  ret i32 0
}

declare { i32, i32, [16 x i8] } @titrate_toString(ptr, i64)
