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
fn optional_of_and_is_present() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn main(): void {
    let present = of(42);
    let absent = empty();
    io::println(Boolean.toString(present._present));
    io::println(Boolean.toString(absent._present));
    io::println(Integer.toString(present._value));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\n42");
}

#[test]
fn optional_is_empty() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn main(): void {
    let present = of(42);
    let absent = empty();
    io::println(Boolean.toString(!present._present));
    io::println(Boolean.toString(!absent._present));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "false\ntrue");
}

#[test]
fn optional_get() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn get(opt: Optional): int {
    if (opt._present) {
        return opt._value;
    }
    return -1;
}
public fn main(): void {
    let present = of(42);
    io::println(Integer.toString(get(present)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42");
}

#[test]
fn optional_or_else() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn orElse(opt: Optional, defaultValue: int): int {
    if (opt._present) {
        return opt._value;
    }
    return defaultValue;
}
public fn main(): void {
    let present = of(42);
    let absent = empty();
    io::println(Integer.toString(orElse(present, 99)));
    io::println(Integer.toString(orElse(absent, 99)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\n99");
}

#[test]
fn optional_map() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn map(opt: Optional): Optional {
    if (opt._present) {
        return of(opt._value * 2);
    }
    return empty();
}
public fn main(): void {
    let present = of(21);
    let absent = empty();
    let mapped1 = map(present);
    let mapped2 = map(absent);
    io::println(Integer.toString(mapped1._value));
    io::println(Boolean.toString(mapped1._present));
    io::println(Boolean.toString(mapped2._present));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\ntrue\nfalse");
}

#[test]
fn optional_filter() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn filterEven(opt: Optional): Optional {
    if (opt._present && (opt._value % 2) == 0) {
        return opt;
    }
    return empty();
}
public fn main(): void {
    let present = of(42);
    let filtered1 = filterEven(present);
    let filtered2 = filterEven(of(7));
    io::println(Boolean.toString(filtered1._present));
    io::println(Integer.toString(filtered1._value));
    io::println(Boolean.toString(filtered2._present));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\n42\nfalse");
}

#[test]
fn optional_if_present() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn ifPresentPrint(opt: Optional): void {
    if (opt._present) {
        io::println(Integer.toString(opt._value));
    }
}
public fn main(): void {
    let present = of(10);
    let absent = empty();
    ifPresentPrint(present);
    ifPresentPrint(absent);
    io::println("done");
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "10\ndone");
}

#[test]
fn optional_of_nullable() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn ofNullable(value: int, isNull: bool): Optional {
    if (isNull) {
        return empty();
    }
    return of(value);
}
public fn main(): void {
    let present = ofNullable(42, false);
    let absent = ofNullable(0, true);
    io::println(Boolean.toString(present._present));
    io::println(Integer.toString(present._value));
    io::println(Boolean.toString(absent._present));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\n42\nfalse");
}

#[test]
fn optional_or_else_get() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn orElseWithDefault(opt: Optional, defaultValue: int): int {
    if (opt._present) {
        return opt._value;
    }
    return defaultValue;
}
public fn main(): void {
    let present = of(42);
    let absent = empty();
    io::println(Integer.toString(orElseWithDefault(present, 999)));
    io::println(Integer.toString(orElseWithDefault(absent, 999)));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "42\n999");
}

#[test]
fn optional_to_string() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn optToString(opt: Optional): string {
    if (opt._present) {
        return "Optional(" + Integer.toString(opt._value) + ")";
    }
    return "Optional.empty";
}
public fn main(): void {
    let present = of(42);
    let absent = empty();
    io::println(optToString(present));
    io::println(optToString(absent));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "Optional(42)\nOptional.empty");
}

#[test]
fn optional_chained_operations() {
    let src = r#"
public class Optional {
    public var _value: int;
    public var _present: bool;
    public fn init() {
        this._present = false;
    }
}
public fn of(value: int): Optional {
    let opt = new Optional();
    opt._value = value;
    opt._present = true;
    return opt;
}
public fn empty(): Optional {
    let opt = new Optional();
    opt._present = false;
    return opt;
}
public fn mapDouble(opt: Optional): Optional {
    if (opt._present) {
        return of(opt._value * 2);
    }
    return empty();
}
public fn orElse(opt: Optional, defaultValue: int): int {
    if (opt._present) {
        return opt._value;
    }
    return defaultValue;
}
public fn main(): void {
    let result1 = orElse(mapDouble(of(5)), 0);
    let result2 = orElse(mapDouble(empty()), 0);
    io::println(Integer.toString(result1));
    io::println(Integer.toString(result2));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "10\n0");
}
