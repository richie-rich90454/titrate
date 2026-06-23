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
fn string_length_basic() {
    let src = r#"
public fn main(): void {
    io::println(Integer.toString(String_length("hello")));
    io::println(Integer.toString(String_length("")));
    io::println(Integer.toString(String_length("Titrate")));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "5\n0\n7");
}

#[test]
fn string_char_at() {
    let src = r#"
public fn main(): void {
    io::println(String_charAt("hello", 0 as int));
    io::println(String_charAt("hello", 4 as int));
    io::println(String_charAt("abc", 1 as int));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "h\no\nb");
}

#[test]
fn string_substring() {
    let src = r#"
public fn substring(s: string, start: int, end: int): string {
    var result: string = "";
    var i: int = start;
    while (i < end && i < String_length(s)) {
        result = result + (String_charAt(s, i as int) as string);
        i = i + 1;
    }
    return result;
}
public fn main(): void {
    io::println(substring("hello world", 0, 5));
    io::println(substring("hello world", 6, 11));
    io::println(substring("abcdef", 2, 4));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "hello\nworld\ncd");
}

#[test]
fn string_case_conversion() {
    let src = r#"
public fn main(): void {
    io::println(String_toUpperCase("hello world"));
    io::println(String_toLowerCase("HELLO WORLD"));
    io::println(String_toUpperCase("MixedCase"));
    io::println(String_toLowerCase("MixedCase"));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "HELLO WORLD\nhello world\nMIXEDCASE\nmixedcase");
}

#[test]
fn string_trim() {
    let src = r#"
public fn main(): void {
    io::println(String_trim("  hello  "));
    io::println(String_trim("\t\nhi\r\n"));
    io::println(String_trim("no_spaces"));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "hello\nhi\nno_spaces");
}

#[test]
fn string_replace() {
    let src = r#"
public fn main(): void {
    io::println(String_replace("a-b-c", "-", "+"));
    io::println(String_replace("hello world", "world", "Titrate"));
    io::println(String_replace("aaa", "a", "bb"));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "a+b+c\nhello Titrate\nbbbbbb");
}

#[test]
fn string_starts_ends_with() {
    let src = r#"
public fn main(): void {
    io::println(Boolean.toString(String_startsWith("hello world", "hello")));
    io::println(Boolean.toString(String_startsWith("hello world", "world")));
    io::println(Boolean.toString(String_endsWith("hello world", "world")));
    io::println(Boolean.toString(String_endsWith("hello world", "hello")));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\ntrue\nfalse");
}

#[test]
fn string_pad() {
    let src = r#"
public fn main(): void {
    io::println(String_padLeft("42", 5, '0'));
    io::println(String_padRight("42", 5, '*'));
    io::println(String_padLeft("hi", 6, ' '));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "00042\n42***\n    hi");
}

#[test]
fn string_split() {
    let src = r#"
public fn indexOf(s: string, sub: string): int {
    let sLen: int = String_length(s);
    let subLen: int = String_length(sub);
    if (subLen == 0) { return 0; }
    if (subLen > sLen) { return -1; }
    var i: int = 0;
    while (i <= sLen - subLen) {
        var match: bool = true;
        var j: int = 0;
        while (j < subLen) {
            if ((String_charAt(s, (i + j) as int) as string) != (String_charAt(sub, j as int) as string)) {
                match = false;
            }
            j = j + 1;
        }
        if (match) { return i; }
        i = i + 1;
    }
    return -1;
}
public fn substring(s: string, start: int, end: int): string {
    var result: string = "";
    var i: int = start;
    while (i < end && i < String_length(s)) {
        result = result + (String_charAt(s, i as int) as string);
        i = i + 1;
    }
    return result;
}
public fn main(): void {
    let s = "a,b,c";
    let delim = ",";
    var count: int = 1;
    var start: int = 0;
    var idx: int = indexOf(s, delim);
    while (idx >= 0) {
        io::println(substring(s, start, idx));
        start = idx + String_length(delim);
        let remaining: string = substring(s, start, String_length(s));
        idx = indexOf(remaining, delim);
        if (idx >= 0) { idx = idx + start; }
        count = count + 1;
    }
    io::println(substring(s, start, String_length(s)));
    io::println(Integer.toString(count));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "a\nb\nc\n3");
}

#[test]
fn string_from_char_code() {
    let src = r#"
public fn main(): void {
    io::println(String_fromCharCode(65));
    io::println(String_fromCharCode(97));
    io::println(String_fromCharCode(48));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "A\na\n0");
}

#[test]
fn string_concat() {
    let src = r#"
public fn main(): void {
    io::println("Hello, " + "World!");
    io::println("foo" + "bar" + "baz");
    io::println("num: " + Integer.toString(42));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "Hello, World!\nfoobarbaz\nnum: 42");
}

#[test]
fn string_index_of() {
    let src = r#"
public fn indexOf(s: string, sub: string): int {
    let sLen: int = String_length(s);
    let subLen: int = String_length(sub);
    if (subLen == 0) { return 0; }
    if (subLen > sLen) { return -1; }
    var i: int = 0;
    while (i <= sLen - subLen) {
        var match: bool = true;
        var j: int = 0;
        while (j < subLen) {
            if ((String_charAt(s, (i + j) as int) as string) != (String_charAt(sub, j as int) as string)) {
                match = false;
            }
            j = j + 1;
        }
        if (match) { return i; }
        i = i + 1;
    }
    return -1;
}
public fn main(): void {
    io::println(Integer.toString(indexOf("hello world", "world")));
    io::println(Integer.toString(indexOf("hello world", "o")));
    io::println(Integer.toString(indexOf("hello world", "xyz")));
    io::println(Integer.toString(indexOf("aaa", "a")));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "6\n4\n-1\n0");
}

#[test]
fn string_contains() {
    let src = r#"
public fn indexOf(s: string, sub: string): int {
    let sLen: int = String_length(s);
    let subLen: int = String_length(sub);
    if (subLen == 0) { return 0; }
    if (subLen > sLen) { return -1; }
    var i: int = 0;
    while (i <= sLen - subLen) {
        var match: bool = true;
        var j: int = 0;
        while (j < subLen) {
            if ((String_charAt(s, (i + j) as int) as string) != (String_charAt(sub, j as int) as string)) {
                match = false;
            }
            j = j + 1;
        }
        if (match) { return i; }
        i = i + 1;
    }
    return -1;
}
public fn main(): void {
    io::println(Boolean.toString(indexOf("hello world", "world") >= 0));
    io::println(Boolean.toString(indexOf("hello world", "xyz") >= 0));
    io::println(Boolean.toString(indexOf("abcdef", "cde") >= 0));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\ntrue");
}

#[test]
fn string_repeat() {
    let src = r#"
public fn repeat(s: string, count: int): string {
    var result: string = "";
    var i: int = 0;
    while (i < count) {
        result = result + (s as string);
        i = i + 1;
    }
    return result;
}
public fn main(): void {
    io::println(repeat("ab", 3));
    io::println(repeat("x", 5));
    io::println(Integer.toString(String_length(repeat("", 10))));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "ababab\nxxxxx\n0");
}

#[test]
fn string_reverse() {
    let src = r#"
public fn reverse(s: string): string {
    var result: string = "";
    var i: int = String_length(s) - 1;
    while (i >= 0) {
        result = result + (String_charAt(s, i as int) as string);
        i = i - 1;
    }
    return result;
}
public fn main(): void {
    io::println(reverse("hello"));
    io::println(reverse("abc"));
    io::println(Integer.toString(String_length(reverse(""))));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "olleh\ncba\n0");
}

#[test]
fn string_is_empty_is_blank() {
    let src = r#"
public fn isEmpty(s: string): bool {
    return String_length(s) == 0;
}
public fn isBlank(s: string): bool {
    let len: int = String_length(s);
    var i: int = 0;
    while (i < len) {
        let c: string = String_charAt(s, i as int) as string;
        if (c != " " && c != "\t" && c != "\n" && c != "\r") {
            return false;
        }
        i = i + 1;
    }
    return true;
}
public fn main(): void {
    io::println(Boolean.toString(isEmpty("")));
    io::println(Boolean.toString(isEmpty("x")));
    io::println(Boolean.toString(isBlank("   ")));
    io::println(Boolean.toString(isBlank("  \t\n ")));
    io::println(Boolean.toString(isBlank("  x  ")));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "true\nfalse\ntrue\ntrue\nfalse");
}

#[test]
fn string_compare_to() {
    let src = r#"
public fn compareTo(a: string, b: string): int {
    var minLen: int = String_length(a);
    if (String_length(b) < minLen) { minLen = String_length(b); }
    var i: int = 0;
    while (i < minLen) {
        let ca: int = String_charAt(a, i as int) as int;
        let cb: int = String_charAt(b, i as int) as int;
        if (ca < cb) { return -1; }
        if (ca > cb) { return 1; }
        i = i + 1;
    }
    if (String_length(a) < String_length(b)) { return -1; }
    if (String_length(a) > String_length(b)) { return 1; }
    return 0;
}
public fn main(): void {
    io::println(Integer.toString(compareTo("apple", "apple")));
    io::println(Integer.toString(compareTo("apple", "banana")));
    io::println(Integer.toString(compareTo("banana", "apple")));
    io::println(Integer.toString(compareTo("ab", "abc")));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "0\n-1\n1\n-1");
}

#[test]
fn string_format() {
    let src = r#"
public fn format(template: string, name: string, age: string): string {
    var result: string = "";
    var argIdx: int = 0;
    var i: int = 0;
    while (i < String_length(template)) {
        if (i + 1 < String_length(template) && (String_charAt(template, i as int) as string) == "{" && (String_charAt(template, (i + 1) as int) as string) == "}") {
            if (argIdx == 0) {
                result = result + (name as string);
                argIdx = 1;
            } else {
                result = result + (age as string);
                argIdx = 2;
            }
            i = i + 2;
        } else {
            result = result + (String_charAt(template, i as int) as string);
            i = i + 1;
        }
    }
    return result;
}
public fn main(): void {
    io::println(format("Name: {}, Age: {}", "Alice", "30"));
    io::println(format("Hello {}!", "World", ""));
}
"#;
    let output = run_source(src).expect("should run");
    assert_output(&output, "Name: Alice, Age: 30\nHello World!");
}
