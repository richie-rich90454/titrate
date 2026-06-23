use trc::lexer;
use trc::parser;
use trc::bytecode;

fn run_source(source: &str) -> Result<Vec<String>, String> {
    let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;
    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile(&ast)?;
    let mut vm = bytecode::Vm::new();
    vm.load_program(compiled);
    vm.run()?;
    Ok(vm.output)
}

fn assert_output(output: &[String], expected: &str) {
    let actual: String = output.join("\n");
    assert_eq!(actual, expected.trim_end().replace("\r\n", "\n"));
}

#[test]
fn result_ok_creation() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn main(): void {
    let good = ok(42);
    let bad = err("failed");
    io::println(Boolean.toString(good._isOk));
    io::println(Boolean.toString(bad._isOk));
    io::println(Integer.toString(good._okValue));
    io::println(bad._errValue);
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\n42\nfailed");
}

#[test]
fn result_is_ok_is_err() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn main(): void {
    let good = ok(42);
    let bad = err("failed");
    io::println(Boolean.toString(good._isOk));
    io::println(Boolean.toString(!good._isOk));
    io::println(Boolean.toString(bad._isOk));
    io::println(Boolean.toString(!bad._isOk));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\nfalse\ntrue");
}

#[test]
fn result_unwrap() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn unwrap(r: Result): int {
    if (r._isOk) {
        return r._okValue;
    }
    return -1;
}
public fn main(): void {
    let good = ok(42);
    io::println(Integer.toString(unwrap(good)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42");
}

#[test]
fn result_unwrap_or() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn unwrapOr(r: Result, defaultValue: int): int {
    if (r._isOk) {
        return r._okValue;
    }
    return defaultValue;
}
public fn main(): void {
    let good = ok(42);
    let bad = err("failed");
    io::println(Integer.toString(unwrapOr(good, 0)));
    io::println(Integer.toString(unwrapOr(bad, 99)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\n99");
}

#[test]
fn result_unwrap_err() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn unwrapErr(r: Result): string {
    if (!r._isOk) {
        return r._errValue;
    }
    return "";
}
public fn main(): void {
    let bad = err("something went wrong");
    io::println(unwrapErr(bad));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "something went wrong");
}

#[test]
fn result_map() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn mapDouble(r: Result): Result {
    if (r._isOk) {
        return ok(r._okValue * 2);
    }
    return r;
}
public fn main(): void {
    let good = ok(21);
    let bad = err("failed");
    let mapped1 = mapDouble(good);
    let mapped2 = mapDouble(bad);
    io::println(Integer.toString(mapped1._okValue));
    io::println(Boolean.toString(mapped1._isOk));
    io::println(Boolean.toString(mapped2._isOk));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\ntrue\nfalse");
}

#[test]
fn result_map_err() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn mapErrUpper(r: Result): Result {
    if (!r._isOk) {
        let upper: string = String_toUpperCase(r._errValue);
        return err(upper);
    }
    return r;
}
public fn main(): void {
    let bad = err("failed");
    let mapped = mapErrUpper(bad);
    io::println(mapped._errValue);
    io::println(Boolean.toString(mapped._isOk));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "FAILED\nfalse");
}

#[test]
fn result_and_then() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn andThenDouble(r: Result): Result {
    if (r._isOk) {
        if (r._okValue > 0) {
            return ok(r._okValue * 2);
        }
        return err("non-positive");
    }
    return r;
}
public fn main(): void {
    let good = ok(21);
    let bad = ok(-5);
    let result1 = andThenDouble(good);
    let result2 = andThenDouble(bad);
    io::println(Integer.toString(result1._okValue));
    io::println(Boolean.toString(result2._isOk));
    io::println(result2._errValue);
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\nfalse\nnon-positive");
}

#[test]
fn result_or_else() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn orElseDefault(r: Result): Result {
    if (!r._isOk) {
        return ok(0);
    }
    return r;
}
public fn main(): void {
    let good = ok(42);
    let bad = err("failed");
    let result1 = orElseDefault(good);
    let result2 = orElseDefault(bad);
    io::println(Integer.toString(result1._okValue));
    io::println(Integer.toString(result2._okValue));
    io::println(Boolean.toString(result2._isOk));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\n0\ntrue");
}

#[test]
fn result_chained_operations() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn unwrapOr(r: Result, defaultValue: int): int {
    if (r._isOk) {
        return r._okValue;
    }
    return defaultValue;
}
public fn mapDouble(r: Result): Result {
    if (r._isOk) {
        return ok(r._okValue * 2);
    }
    return r;
}
public fn main(): void {
    let good = ok(10);
    let bad = err("failed");
    let result1 = unwrapOr(mapDouble(good), 0);
    let result2 = unwrapOr(mapDouble(bad), 0);
    io::println(Integer.toString(result1));
    io::println(Integer.toString(result2));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "20\n0");
}

#[test]
fn result_error_propagation() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn divide(a: int, b: int): Result {
    if (b == 0) {
        return err("division by zero");
    }
    return ok(a / b);
}
public fn main(): void {
    let good = divide(10, 2);
    let bad = divide(10, 0);
    io::println(Boolean.toString(good._isOk));
    io::println(Integer.toString(good._okValue));
    io::println(Boolean.toString(bad._isOk));
    io::println(bad._errValue);
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\n5\nfalse\ndivision by zero");
}

#[test]
fn result_to_string() {
    let src = r#"
public class Result {
    public var _okValue: int;
    public var _errValue: string;
    public var _isOk: bool;
    public fn init() {
        this._isOk = false;
    }
}
public fn ok(value: int): Result {
    let r = new Result();
    r._okValue = value;
    r._isOk = true;
    return r;
}
public fn err(msg: string): Result {
    let r = new Result();
    r._errValue = msg;
    r._isOk = false;
    return r;
}
public fn resultToString(r: Result): string {
    if (r._isOk) {
        return "Ok(" + Integer.toString(r._okValue) + ")";
    }
    return "Err(" + (r._errValue as string) + ")";
}
public fn main(): void {
    let good = ok(42);
    let bad = err("failed");
    io::println(resultToString(good));
    io::println(resultToString(bad));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "Ok(42)\nErr(failed)");
}
