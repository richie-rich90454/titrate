use std::path::PathBuf;
use trc::lexer;
use trc::parser;
use trc::bytecode;
use trc::bytecode::value::Value;

/// Helper: compile and run a Titrate source string.
/// Uses `compile` (single-file, no module resolution) for self-contained programs.
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

/// Helper: compile and run a Titrate source string with module resolution.
/// Uses `compile_with_modules` so that `import tt::...` statements resolve
/// against the project's `lib/` directory.
fn run_source_with_modules(source: &str, root_dir: &str) -> Result<Vec<String>, String> {
    let tokens = lexer::tokenize(source).map_err(|e| format!("tokenize: {}", e))?;
    let ast = parser::parse(tokens).map_err(|e| format!("parse: {}", e))?;

    let mut compiler = bytecode::Compiler::new();
    let compiled = compiler.compile_with_modules(&ast, &PathBuf::from(root_dir))?;

    let mut vm = bytecode::Vm::new();
    vm.set_working_dir(PathBuf::from(root_dir));
    vm.load_program(compiled);
    vm.run()?;

    Ok(vm.output)
}

/// Join output lines and compare against an expected string.
fn assert_output(output: &[String], expected: &str) {
    let actual: String = output.join("\n");
    let expected_trimmed = expected.trim_end().replace("\r\n", "\n");
    assert_eq!(actual, expected_trimmed);
}

// ---------------------------------------------------------------------------
// 1. Chemistry test – Atom-like class with element data
// ---------------------------------------------------------------------------

#[test]
fn test_chemistry() {
    let source = r#"
public class Element {
    public String symbol;
    public int atomicNumber;
    public double mass;

    public Element(String sym, int num, double m) {
        this.symbol = sym;
        this.atomicNumber = num;
        this.mass = m;
    }
}

public void main() {
    let o: Element = new Element("O", 8, 15.999);
    let h: Element = new Element("H", 1, 1.008);

    io::println(o.symbol);
    io::println(Integer.toString(o.atomicNumber));
    io::println(Double.toString(o.mass));
    io::println(h.symbol);
    io::println(Integer.toString(h.atomicNumber));

    double waterMass = o.mass + 2.0 * h.mass;
    io::println(Double.toString(waterMass));
}
"#;
    let output = run_source(source).expect("chemistry test should succeed");
    assert_output(&output, "O\n8\n15.999\nH\n1\n18.015");
}

// ---------------------------------------------------------------------------
// 2. NDArray test – 2D array using flat ArrayList with index math
// ---------------------------------------------------------------------------

#[test]
fn test_ndarray() {
    let source = r#"
public class NDArray {
    public ArrayList<double> data;
    public int rows;
    public int cols;

    public NDArray(int r, int c) {
        this.rows = r;
        this.cols = c;
        this.data = new ArrayList<double>();
        int i = 0;
        while (i < r * c) {
            this.data.add(0.0);
            i = i + 1;
        }
    }

    public void set2D(int r, int c, double val) {
        this.data.set(r * this.cols + c, val);
    }

    public double get2D(int r, int c) {
        return this.data.get(r * this.cols + c);
    }

    public double sum() {
        double s = 0.0;
        int i = 0;
        while (i < this.data.size()) {
            s = s + this.data.get(i);
            i = i + 1;
        }
        return s;
    }
}

public void main() {
    let a: NDArray = new NDArray(2, 3);
    a.set2D(0, 0, 1.0); a.set2D(0, 1, 2.0); a.set2D(0, 2, 3.0);
    a.set2D(1, 0, 4.0); a.set2D(1, 1, 5.0); a.set2D(1, 2, 6.0);

    io::println(Double.toString(a.get2D(0, 0)));
    io::println(Double.toString(a.get2D(1, 2)));
    io::println(Double.toString(a.sum()));
}
"#;
    let output = run_source(source).expect("NDArray test should succeed");
    assert_output(&output, "1\n6\n21");
}

// ---------------------------------------------------------------------------
// 3. Matrix test – 2x2 matrix multiplication using flat array
// ---------------------------------------------------------------------------

#[test]
fn test_matrix() {
    let source = r#"
public class Matrix {
    public ArrayList<double> data;
    public int rows;
    public int cols;

    public Matrix(int r, int c) {
        this.rows = r;
        this.cols = c;
        this.data = new ArrayList<double>();
        int i = 0;
        while (i < r * c) {
            this.data.add(0.0);
            i = i + 1;
        }
    }

    public void set(int r, int c, double val) {
        this.data.set(r * this.cols + c, val);
    }

    public double get(int r, int c) {
        return this.data.get(r * this.cols + c);
    }
}

public void main() {
    let m1: Matrix = new Matrix(2, 2);
    m1.set(0, 0, 1.0); m1.set(0, 1, 2.0);
    m1.set(1, 0, 3.0); m1.set(1, 1, 4.0);

    let m2: Matrix = new Matrix(2, 2);
    m2.set(0, 0, 5.0); m2.set(0, 1, 6.0);
    m2.set(1, 0, 7.0); m2.set(1, 1, 8.0);

    // m1 * m2 = [19, 22; 43, 50]
    let m3: Matrix = new Matrix(2, 2);
    int i = 0;
    while (i < 2) {
        int j = 0;
        while (j < 2) {
            double sum = 0.0;
            int k = 0;
            while (k < 2) {
                sum = sum + m1.get(i, k) * m2.get(k, j);
                k = k + 1;
            }
            m3.set(i, j, sum);
            j = j + 1;
        }
        i = i + 1;
    }

    io::println(Double.toString(m3.get(0, 0)));
    io::println(Double.toString(m3.get(0, 1)));
    io::println(Double.toString(m3.get(1, 0)));
    io::println(Double.toString(m3.get(1, 1)));
}
"#;
    let output = run_source(source).expect("Matrix test should succeed");
    assert_output(&output, "19\n22\n43\n50");
}

// ---------------------------------------------------------------------------
// 4. JSON test – native Json_parse and Json_stringify
// ---------------------------------------------------------------------------

#[test]
fn test_json_parse() {
    let mut vm = bytecode::Vm::new();
    let json_str = Value::String(std::rc::Rc::new(r#"{"name":"Titrate","version":3}"#.to_string()));
    let result = vm.call_native_by_name("Json_parse", &[json_str]).expect("Json_parse should succeed");
    match result {
        Value::ClassInstance { class_name, .. } => {
            assert_eq!(class_name, "HashMap", "Json_parse should return a HashMap class instance");
        }
        other => panic!("Expected ClassInstance from Json_parse, got {:?}", other),
    }
}

#[test]
fn test_json_stringify() {
    let mut vm = bytecode::Vm::new();
    // First parse, then stringify
    let json_str = Value::String(std::rc::Rc::new(r#"{"key":"value"}"#.to_string()));
    let parsed = vm.call_native_by_name("Json_parse", &[json_str]).expect("Json_parse should succeed");
    let stringified = vm.call_native_by_name("Json_stringify", &[parsed]).expect("Json_stringify should succeed");
    match stringified {
        Value::String(s) => {
            assert!(s.contains("key"), "stringified JSON should contain 'key', got '{}'", s);
            assert!(s.contains("value"), "stringified JSON should contain 'value', got '{}'", s);
        }
        other => panic!("Expected String from Json_stringify, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 5. Regex test – native Regex_match, Regex_find, Regex_replace
// ---------------------------------------------------------------------------

#[test]
fn test_regex_match() {
    let mut vm = bytecode::Vm::new();
    let pattern = Value::String(std::rc::Rc::new(r#"\d+"#.to_string()));
    let text = Value::String(std::rc::Rc::new("123".to_string()));
    let result = vm.call_native_by_name("Regex_match", &[pattern, text]).expect("Regex_match should succeed");
    match result {
        Value::Bool(true) => {}
        other => panic!("Regex_match on '123' with \\d+ should be true, got {:?}", other),
    }
}

#[test]
fn test_regex_match_false() {
    let mut vm = bytecode::Vm::new();
    let pattern = Value::String(std::rc::Rc::new(r#"\d+"#.to_string()));
    let text = Value::String(std::rc::Rc::new("abc".to_string()));
    let result = vm.call_native_by_name("Regex_match", &[pattern, text]).expect("Regex_match should succeed");
    match result {
        Value::Bool(false) => {}
        other => panic!("Regex_match on 'abc' with \\d+ should be false, got {:?}", other),
    }
}

#[test]
fn test_regex_find() {
    let mut vm = bytecode::Vm::new();
    let pattern = Value::String(std::rc::Rc::new(r#"\d+"#.to_string()));
    let text = Value::String(std::rc::Rc::new("a42b15c".to_string()));
    let result = vm.call_native_by_name("Regex_find", &[pattern, text]).expect("Regex_find should succeed");
    // Regex_find returns matches - verify it found digits
    match result {
        Value::String(s) => {
            // Should contain the number matches (42 and 15, or individual digits)
            assert!(!s.is_empty(), "Regex_find should find matches, got empty string");
        }
        Value::Array { elements } => {
            assert!(!elements.is_empty(), "Regex_find should return non-empty results");
        }
        other => panic!("Expected String or Array from Regex_find, got {:?}", other),
    }
}

#[test]
fn test_regex_replace() {
    let mut vm = bytecode::Vm::new();
    let pattern = Value::String(std::rc::Rc::new(r#"\d+"#.to_string()));
    let text = Value::String(std::rc::Rc::new("a1b2c3".to_string()));
    let replacement = Value::String(std::rc::Rc::new("X".to_string()));
    let result = vm.call_native_by_name("Regex_replace", &[pattern, text, replacement]).expect("Regex_replace should succeed");
    match result {
        Value::String(s) => {
            assert_eq!(s.as_str(), "aXbXcX", "Regex_replace should replace all digits, got '{}'", s);
        }
        other => panic!("Expected String from Regex_replace, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 6. Fraction test – self-contained Fraction class
// ---------------------------------------------------------------------------

#[test]
fn test_fraction() {
    let source = r#"
public class Fraction {
    public int num;
    public int den;

    public Fraction(int n, int d) {
        this.num = n;
        this.den = d;
    }

    public Fraction add(Fraction other) {
        int n = this.num * other.den + other.num * this.den;
        int d = this.den * other.den;
        return new Fraction(n, d);
    }

    public Fraction mul(Fraction other) {
        return new Fraction(this.num * other.num, this.den * other.den);
    }

    public String toString() {
        return Integer.toString(this.num) + "/" + Integer.toString(this.den);
    }
}

public void main() {
    let a: Fraction = new Fraction(1, 3);
    let b: Fraction = new Fraction(1, 6);

    let sum: Fraction = a.add(b);
    io::println(sum.toString());

    let product: Fraction = a.mul(b);
    io::println(product.toString());
}
"#;
    let output = run_source(source).expect("Fraction test should succeed");
    // 1/3 + 1/6 = 9/18 (not reduced), 1/3 * 1/6 = 1/18
    assert_output(&output, "9/18\n1/18");
}

// ---------------------------------------------------------------------------
// 7. Complex test – self-contained Complex class
// ---------------------------------------------------------------------------

#[test]
fn test_complex() {
    let source = r#"
public class Complex {
    public double re;
    public double im;

    public Complex(double r, double i) {
        this.re = r;
        this.im = i;
    }

    public Complex add(Complex other) {
        return new Complex(this.re + other.re, this.im + other.im);
    }

    public Complex mul(Complex other) {
        double r = this.re * other.re - this.im * other.im;
        double i = this.re * other.im + this.im * other.re;
        return new Complex(r, i);
    }

    public String toString() {
        return Double.toString(this.re) + "+" + Double.toString(this.im) + "i";
    }
}

public void main() {
    let a: Complex = new Complex(3.0, 4.0);
    let b: Complex = new Complex(1.0, 2.0);

    let sum: Complex = a.add(b);
    io::println(sum.toString());

    let product: Complex = a.mul(b);
    io::println(product.toString());
}
"#;
    let output = run_source(source).expect("Complex test should succeed");
    // (3+4i) + (1+2i) = 4+6i
    // (3+4i) * (1+2i) = -5+10i
    // Double.toString(4.0) = "4", Double.toString(6.0) = "6"
    assert_output(&output, "4+6i\n-5+10i");
}

// ---------------------------------------------------------------------------
// 8. Decimal test – self-contained fixed-point Decimal class
// ---------------------------------------------------------------------------

#[test]
fn test_decimal() {
    let source = r#"
public class Decimal {
    public int value;  // scaled by 1000

    public Decimal(int v) {
        this.value = v;
    }

    public Decimal add(Decimal other) {
        return new Decimal(this.value + other.value);
    }

    public Decimal mul(Decimal other) {
        return new Decimal(this.value * other.value / 1000);
    }

    public int compareTo(Decimal other) {
        if (this.value < other.value) { return -1; }
        if (this.value > other.value) { return 1; }
        return 0;
    }

    public String toString() {
        int whole = this.value / 1000;
        int frac = this.value % 1000;
        if (frac < 0) { frac = -frac; }
        let fs: String = Integer.toString(frac);
        while (fs.length() < 3) { fs = "0" + fs; }
        // Remove trailing zeros
        while (fs.length() > 1 && fs.substring(fs.length() - 1, fs.length()) == "0") {
            fs = fs.substring(0, fs.length() - 1);
        }
        return Integer.toString(whole) + "." + fs;
    }
}

public void main() {
    // 1.50 = 1500, 2.25 = 2250
    let a: Decimal = new Decimal(1500);
    let b: Decimal = new Decimal(2250);

    let sum: Decimal = a.add(b);
    io::println(sum.toString());

    let product: Decimal = a.mul(b);
    io::println(product.toString());

    io::println(Integer.toString(a.compareTo(b)));
    io::println(Integer.toString(b.compareTo(a)));
    io::println(Integer.toString(a.compareTo(a)));
}
"#;
    let output = run_source(source).expect("Decimal test should succeed");
    // 1500 + 2250 = 3750 => 3.75
    // 1500 * 2250 / 1000 = 3375 => 3.375
    // 1500 < 2250 => -1, 2250 > 1500 => 1, 1500 == 1500 => 0
    assert_output(&output, "3.75\n3.375\n-1\n1\n0");
}

// ---------------------------------------------------------------------------
// 9. Statistics test – mean and stdev via ArrayList
// ---------------------------------------------------------------------------

#[test]
fn test_statistics() {
    let source = r#"
public void main() {
    let data: ArrayList<double> = new ArrayList<double>();
    data.add(2.0);
    data.add(4.0);
    data.add(4.0);
    data.add(4.0);
    data.add(5.0);
    data.add(5.0);
    data.add(7.0);
    data.add(9.0);

    // Mean
    double sum = 0.0;
    int i = 0;
    while (i < data.size()) {
        sum = sum + data.get(i);
        i = i + 1;
    }
    double mean = sum / data.size() as double;
    io::println(Double.toString(mean));

    // Variance
    double varSum = 0.0;
    i = 0;
    while (i < data.size()) {
        double diff = data.get(i) - mean;
        varSum = varSum + diff * diff;
        i = i + 1;
    }
    double variance = varSum / data.size() as double;
    // Newton's method sqrt
    double x = variance;
    int j = 0;
    while (j < 20) {
        x = 0.5 * (x + variance / x);
        j = j + 1;
    }
    io::println(Double.toString(x));
}
"#;
    let output = run_source(source).expect("Statistics test should succeed");
    // mean = 40/8 = 5.0
    let mean: f64 = output[0].parse().expect("mean should be a number");
    assert!((mean - 5.0).abs() < 1e-9, "mean should be 5.0, got {}", mean);
    // stdev should be positive
    let stdev: f64 = output[1].parse().expect("stdev should be a number");
    assert!(stdev > 0.0, "stdev should be positive, got {}", stdev);
}

// ---------------------------------------------------------------------------
// 10. Itertools test – combinations count
// ---------------------------------------------------------------------------

#[test]
fn test_itertools() {
    let source = r#"
fn factorial(n: int): int {
    int result = 1;
    int i = 2;
    while (i <= n) {
        result = result * i;
        i = i + 1;
    }
    return result;
}

fn combinations(n: int, k: int): int {
    return factorial(n) / (factorial(k) * factorial(n - k));
}

fn permutations(n: int, k: int): int {
    int result = 1;
    int i = 0;
    while (i < k) {
        result = result * (n - i);
        i = i + 1;
    }
    return result;
}

public void main() {
    // C(3,2) = 3
    io::println(Integer.toString(combinations(3, 2)));
    // P(3,2) = 6
    io::println(Integer.toString(permutations(3, 2)));
    // C(5,3) = 10
    io::println(Integer.toString(combinations(5, 3)));
}
"#;
    let output = run_source(source).expect("Itertools test should succeed");
    assert_output(&output, "3\n6\n10");
}

// ---------------------------------------------------------------------------
// 11. Algorithms test – bubble sort and linear search
// ---------------------------------------------------------------------------

#[test]
fn test_algorithms() {
    let source = r#"
fn bubbleSort(arr: ArrayList<int>): void {
    int n = arr.size();
    int i = 0;
    while (i < n - 1) {
        int j = 0;
        while (j < n - 1 - i) {
            if (arr.get(j) > arr.get(j + 1)) {
                int tmp = arr.get(j);
                arr.set(j, arr.get(j + 1));
                arr.set(j + 1, tmp);
            }
            j = j + 1;
        }
        i = i + 1;
    }
}

fn linearSearch(arr: ArrayList<int>, target: int): int {
    int i = 0;
    while (i < arr.size()) {
        if (arr.get(i) == target) { return i; }
        i = i + 1;
    }
    return -1;
}

public void main() {
    let nums: ArrayList<int> = new ArrayList<int>();
    nums.add(42);
    nums.add(7);
    nums.add(13);
    nums.add(3);
    nums.add(99);

    bubbleSort(nums);
    io::println(Integer.toString(nums.get(0)));
    io::println(Integer.toString(nums.get(4)));

    let idx: int = linearSearch(nums, 13);
    io::println(Integer.toString(idx));
}
"#;
    let output = run_source(source).expect("Algorithms test should succeed");
    // sorted: [3, 7, 13, 42, 99]
    assert_output(&output, "3\n99\n2");
}

// ---------------------------------------------------------------------------
// 12. Graph test – adjacency list via HashMap
// ---------------------------------------------------------------------------

#[test]
fn test_graph() {
    let source = r#"
public void main() {
    let adjA: ArrayList<string> = new ArrayList<string>();
    adjA.add("B"); adjA.add("C");

    let adjB: ArrayList<string> = new ArrayList<string>();
    adjB.add("C"); adjB.add("D");

    let adjC: ArrayList<string> = new ArrayList<string>();
    adjC.add("D");

    let adjD: ArrayList<string> = new ArrayList<string>();

    let graph: HashMap<string, ArrayList<string> > = new HashMap<string, ArrayList<string> >();
    graph.put("A", adjA);
    graph.put("B", adjB);
    graph.put("C", adjC);
    graph.put("D", adjD);

    // Check adjacency sizes
    io::println(Integer.toString(graph.get("A").size()));
    io::println(Integer.toString(graph.get("B").size()));
    io::println(Integer.toString(graph.get("C").size()));
    io::println(Integer.toString(graph.get("D").size()));

    // Check containsKey
    io::println(Boolean.toString(graph.containsKey("A")));
    io::println(Boolean.toString(graph.containsKey("Z")));
}
"#;
    let output = run_source(source).expect("Graph test should succeed");
    assert_output(&output, "2\n2\n1\n0\ntrue\nfalse");
}

// ---------------------------------------------------------------------------
// 13. Trie test – self-contained Trie class
// ---------------------------------------------------------------------------

#[test]
fn test_trie() {
    let source = r#"
public class TrieNode {
    public HashMap<string, TrieNode> children;
    public bool isEnd;

    public TrieNode() {
        this.children = new HashMap<string, TrieNode>();
        this.isEnd = false;
    }
}

public class Trie {
    public TrieNode root;

    public Trie() {
        this.root = new TrieNode();
    }

    public void insert(String word) {
        let nd: TrieNode = this.root;
        int i = 0;
        while (i < word.length()) {
            let ch: String = word.substring(i, i + 1);
            if (!nd.children.containsKey(ch)) {
                nd.children.put(ch, new TrieNode());
            }
            nd = nd.children.get(ch);
            i = i + 1;
        }
        nd.isEnd = true;
    }

    public bool search(String word) {
        let nd: TrieNode = this.root;
        int i = 0;
        while (i < word.length()) {
            let ch: String = word.substring(i, i + 1);
            if (!nd.children.containsKey(ch)) { return false; }
            nd = nd.children.get(ch);
            i = i + 1;
        }
        return nd.isEnd;
    }

    public bool startsWith(String prefix) {
        let nd: TrieNode = this.root;
        int i = 0;
        while (i < prefix.length()) {
            let ch: String = prefix.substring(i, i + 1);
            if (!nd.children.containsKey(ch)) { return false; }
            nd = nd.children.get(ch);
            i = i + 1;
        }
        return true;
    }
}

public void main() {
    let t: Trie = new Trie();
    t.insert("apple");
    t.insert("app");
    t.insert("application");

    io::println(Boolean.toString(t.search("app")));
    io::println(Boolean.toString(t.search("apple")));
    io::println(Boolean.toString(t.search("ap")));
    io::println(Boolean.toString(t.startsWith("ap")));
}
"#;
    let output = run_source(source).expect("Trie test should succeed");
    assert_output(&output, "true\ntrue\nfalse\ntrue");
}

// ---------------------------------------------------------------------------
// 14. Counter test – HashMap-based word counter
// ---------------------------------------------------------------------------

#[test]
fn test_counter() {
    let source = r#"
public class Counter {
    public HashMap<string, int> counts;

    public Counter() {
        this.counts = new HashMap<string, int>();
    }

    public void increment(String key) {
        if (this.counts.containsKey(key)) {
            this.counts.put(key, this.counts.get(key) + 1);
        } else {
            this.counts.put(key, 1);
        }
    }

    public int get(String key) {
        if (this.counts.containsKey(key)) {
            return this.counts.get(key);
        }
        return 0;
    }
}

public void main() {
    let c: Counter = new Counter();
    c.increment("apple");
    c.increment("banana");
    c.increment("apple");
    c.increment("apple");
    c.increment("banana");

    io::println(Integer.toString(c.get("apple")));
    io::println(Integer.toString(c.get("banana")));
    io::println(Integer.toString(c.get("cherry")));
}
"#;
    let output = run_source(source).expect("Counter test should succeed");
    assert_output(&output, "3\n2\n0");
}

// ---------------------------------------------------------------------------
// 15. PriorityQueue test – min-heap via sorted ArrayList (no insert)
// ---------------------------------------------------------------------------

#[test]
fn test_priority_queue() {
    let source = r#"
public void main() {
    let data: ArrayList<int> = new ArrayList<int>();
    // Insert sorted: 5, 1, 3, 2, 4
    // We'll just add them and sort manually (bubble sort)
    data.add(5);
    data.add(1);
    data.add(3);
    data.add(2);
    data.add(4);

    // Bubble sort ascending
    int n = data.size();
    int i = 0;
    while (i < n - 1) {
        int j = 0;
        while (j < n - 1 - i) {
            if (data.get(j) > data.get(j + 1)) {
                int tmp = data.get(j);
                data.set(j, data.get(j + 1));
                data.set(j + 1, tmp);
            }
            j = j + 1;
        }
        i = i + 1;
    }

    // Now data is [1, 2, 3, 4, 5] - pop from front
    io::println(Integer.toString(data.size()));
    io::println(Integer.toString(data.get(0)));
    io::println(Integer.toString(data.get(1)));
    io::println(Integer.toString(data.get(2)));
}
"#;
    let output = run_source(source).expect("PriorityQueue test should succeed");
    assert_output(&output, "5\n1\n2\n3");
}

// ---------------------------------------------------------------------------
// 16. Optimization & PDE module compilation test
//    Verifies that lib/tt/math/optimization/Optimization.tr and
//    lib/tt/math/pde/PDE.tr compile correctly with module resolution.
//    Uses compile_with_modules so imports are resolved against ../lib/.
// ---------------------------------------------------------------------------

#[test]
fn test_optimization_pde_modules_compile() {
    // The root_dir is the project root (parent of trc/), so that
    // `import tt::...` resolves to ../lib/tt/...
    let root_dir = "..";

    let source = r#"
import tt::math::optimization::Optimization;
import tt::math::pde::PDE;
import tt::util::ArrayList;
import tt::lang::Double;
import tt::lang::Integer;

public fn main(): void {
    // --- PDE boundary condition classes ---
    let dbc = new DirichletBC(100.0);
    io::println(Double.toString(dbc.value));

    let nbc = new NeumannBC(0.5);
    io::println(Double.toString(nbc.gradient));

    let rbc = new RobinBC(0.1, 0.9);
    io::println(Double.toString(rbc.alpha));
    io::println(Double.toString(rbc.beta));

    // --- Optimization: OptResult class ---
    let xs = new ArrayList<double>();
    xs.add(1.0);
    xs.add(2.0);
    let result = new OptResult(xs, 0.5, 10, true);
    io::println(Double.toString(result.value));
    io::println(Integer.toString(result.iterations));

    // --- PDE: BoundaryCondition class ---
    let bc = new BoundaryCondition("dirichlet", 42.0, 0.0);
    io::println(bc.kind);
    io::println(Double.toString(bc.value));
}
"#;

    let output = run_source_with_modules(source, root_dir)
        .expect("Optimization & PDE modules should compile and run");
    assert_output(
        &output,
        "100\n0.5\n0.1\n0.9\n0.5\n10\ndirichlet\n42",
    );
}

// ---------------------------------------------------------------------------
// 17. Finance module compilation & execution test
//    Verifies that lib/tt/finance/{OrderBook,MarketData,Indicators,Risk}.tr
//    compile correctly with module resolution and produce expected results.
// ---------------------------------------------------------------------------

#[test]
fn test_finance_modules_compile_and_run() {
    let root_dir = "..";

    let source = r#"
import tt::util::ArrayList;
import tt::lang::Double;
import tt::lang::Integer;
import tt::finance::OrderBook;
import tt::finance::MarketData;
import tt::finance::Indicators;
import tt::finance::Risk;

public fn main(): void {
    // --- OrderBook ---
    let book: OrderBook = new OrderBook();
    book.addBid(new Order("b1", "buy", 100.0, 10.0));
    book.addBid(new Order("b2", "buy", 101.0, 5.0));
    book.addAsk(new Order("a1", "sell", 103.0, 8.0));
    book.addAsk(new Order("a2", "sell", 102.0, 4.0));
    io::println(Double.toString(book.bestBid()));
    io::println(Double.toString(book.bestAsk()));
    io::println(Double.toString(book.spread()));
    io::println(Integer.toString(book.depth(2).size()));
    io::println(book.bbo());
    book.cancelOrder("b2");
    io::println(Double.toString(book.bestBid()));

    // --- MarketData ---
    let candle: OHLCV = new OHLCV(100.0, 105.0, 99.0, 104.0, 250.0, 1700000000);
    io::println(Double.toString(candle.close));
    io::println(Integer.toString(candle.timestamp));
    let tick: Tick = new Tick(104.5, 2.0, "buy", 1700000001);
    io::println(Double.toString(tick.size));
    let trade: Trade = new Trade(104.5, 2.0, "t1", 1700000002);
    io::println(trade.id);
    let quote: Quote = new Quote(104.0, 10.0, 104.5, 8.0, 1700000003);
    io::println(Double.toString(quote.spread()));

    // --- Indicators ---
    let data: ArrayList<double> = new ArrayList<double>();
    data.add(1.0); data.add(2.0); data.add(3.0); data.add(4.0); data.add(5.0);
    let s: ArrayList<double> = Indicators.sma(data, 3);
    io::println(Double.toString(s.get(0)));
    io::println(Integer.toString(s.size()));
    let e: ArrayList<double> = Indicators.ema(data, 3);
    io::println(Double.toString(e.get(0)));
    let w: ArrayList<double> = Indicators.ewma(data, 0.5);
    io::println(Double.toString(w.get(1)));
    let rsi: ArrayList<double> = Indicators.rsi(data, 3);
    io::println(Integer.toString(rsi.size()));
    let m: ArrayList<double> = Indicators.macd(data, 2, 3, 2);
    io::println(Integer.toString(m.size()));
    let bb: ArrayList<double> = Indicators.bollingerBands(data, 3, 2.0);
    io::println(Integer.toString(bb.size()));
    let vols: ArrayList<double> = new ArrayList<double>();
    vols.add(10.0); vols.add(20.0); vols.add(30.0);
    let v: double = Indicators.vwap(data, vols);
    io::println(Integer.toString((v > 2.0 && v < 3.0) ? 1 : 0));

    // --- Risk ---
    let rets: ArrayList<double> = new ArrayList<double>();
    rets.add(0.01); rets.add(-0.02); rets.add(0.03); rets.add(-0.01); rets.add(0.02);
    let vr: double = Risk.valueAtRisk(rets, 0.95);
    io::println(Integer.toString(vr > 0.0 ? 1 : 0));
    let sr: double = Risk.sharpeRatio(rets, 0.0);
    io::println(Integer.toString(sr > 0.0 ? 1 : 0));
    let equity: ArrayList<double> = new ArrayList<double>();
    equity.add(100.0); equity.add(110.0); equity.add(105.0); equity.add(120.0);
    let dd: double = Risk.maxDrawdown(equity);
    io::println(Integer.toString((dd > 0.04 && dd < 0.05) ? 1 : 0));
    let kc: double = Risk.kellyCriterion(0.6, 1.5);
    io::println(Integer.toString((kc > 0.3 && kc < 0.4) ? 1 : 0));
}
"#;

    let output = run_source_with_modules(source, root_dir)
        .expect("Finance modules should compile and run");
    assert_output(
        &output,
        "101\n102\n1\n2\nBBO: bid=101 ask=102\n100\n104\n1700000000\n2\nt1\n0.5\n2\n3\n1\n1.5\n2\n5\n3\n1\n1\n1\n1\n1",
    );
}

// ---------------------------------------------------------------------------
// 18. Stdlib-expansion aggregators – every self-contained run_all_*_tests()
//     added to lib/tt is invoked here so the harness actually exercises it.
//     Thin wrappers under tests/stdlib/src/tests/ do the same per file for
//     stdlib_runtest_all; this test covers the bytecode VM path in one pass.
// ---------------------------------------------------------------------------

#[test]
fn test_stdlib_expansion_aggregators() {
    let source = r#"
import tt::util::Pair;
import tt::util::Range;
import tt::util::Lazy;
import tt::util::StringView;
import tt::util::SortedList;
import tt::util::BitVector;
import tt::util::BloomFilter;
import tt::util::LRUCache;
import tt::util::MultiMap;
import tt::util::RefCount;
import tt::util::FunctionExt;
import tt::util::ResultExt;
import tt::lang::String;
import tt::math::Optimization;
import tt::math::Interpolation;
import tt::math::linalg::MatrixNorms;
import tt::math::ndarray::NDArrayLinalg;
import tt::math::ndarray::NDArrayIndexing;
import tt::math::ndarray::NDArrayStats;
import tt::math::ndarray::NDArrayUfuncs;
import tt::sigproc::Convolution;
import tt::sigproc::WindowFunctions;
import tt::ml::Activation;
import tt::ml::Loss;
import tt::ml::Tensor;
import tt::ml::Sequential;
import tt::ml::Linear;
import tt::ml::Regularization;
import tt::ml::Optim;
import tt::ml::Estimator;
import tt::ml::Metrics;
import tt::ml::DataLoader;
import tt::ml::Autograd;
import tt::graphics::Color;
import tt::graphics::Canvas;
import tt::graphics::Window;
import tt::graphics::Sprite;
import tt::graphics::Shapes;
import tt::graphics::Font;
import tt::graphics::Draw;
import tt::audio::AudioDevice;
import tt::audio::Synth;
import tt::net::Mime;
import tt::net::Url;
import tt::net::Uri;
import tt::async::Task;
import tt::sys::Watchdog;
import tt::sys::PathUtils;
import tt::concurrent::Executor;
import tt::logging::StructuredLogger;
import tt::serialization::MessagePack;
import tt::datastruct::AnyValue;
import tt::datastruct::ArenaMap;
import tt::datastruct::ArrayBlockingQueue;
import tt::datastruct::ArrayDeque;
import tt::datastruct::AtomicCounterMap;
import tt::datastruct::AvlTree;
import tt::datastruct::BTree;
import tt::datastruct::BTreeMap;
import tt::datastruct::BTreeSet;
import tt::datastruct::BitMatrix;
import tt::datastruct::BitsetView;
import tt::datastruct::BitString;
import tt::datastruct::BipartiteGraph;
import tt::datastruct::BlockPool;
import tt::datastruct::BlockingCollection;
import tt::datastruct::BloomFilterExt;
import tt::datastruct::BoundedQueue;
import tt::datastruct::BroadcastBuffer;
import tt::datastruct::ByteArray;
import tt::datastruct::BulkOps;
import tt::datastruct::CapabilityCollection;
import tt::datastruct::CharArray;
import tt::datastruct::CollectionTraits;
import tt::datastruct::Comparators;
import tt::datastruct::CompressedIntArray;
import tt::datastruct::ConcurrentBag;
import tt::datastruct::ConcurrentDictionary;
import tt::datastruct::ConcurrentHashMap;
import tt::datastruct::ConcurrentLinkedQueue;
import tt::datastruct::ConcurrentSkipListMap;
import tt::datastruct::ConcurrentSkipListSet;
import tt::datastruct::ConcurrentStack;
import tt::datastruct::Conversions;
import tt::datastruct::ConvexHullSet;
import tt::datastruct::CopyOnWriteArrayList;
import tt::datastruct::CopyOnWriteArraySet;
import tt::datastruct::CountMinSketch;
import tt::datastruct::CountingAllocator;
import tt::datastruct::CuckooFilter;
import tt::datastruct::DataDoc;
import tt::datastruct::DataTypes;
import tt::datastruct::DefaultDictList;
import tt::datastruct::DefaultDictSet;
import tt::datastruct::DelayQueue;
import tt::datastruct::DenseSet;
import tt::datastruct::DequeView;
import tt::datastruct::DictView;
import tt::datastruct::DQEntry;
import tt::datastruct::DirectedAcyclicGraph;
import tt::datastruct::DirectedGraph;
import tt::datastruct::DynamicBitset;
import tt::datastruct::EnumMap;
import tt::datastruct::EnumSet;
import tt::datastruct::ExpectedValue;
import tt::datastruct::ExpiringSet;
import tt::datastruct::FenwickTree;
import tt::datastruct::FlatMap;
import tt::datastruct::FlatSet;
import tt::datastruct::FreeList;
import tt::datastruct::FrozenRecord;
import tt::datastruct::GrowthPolicy;
import tt::datastruct::HashPolicy;
import tt::datastruct::Hashtable;
import tt::datastruct::HyperLogLog;
import tt::datastruct::HyperLogLogExt;
import tt::datastruct::IdentityHashMap;
import tt::datastruct::ImmutableDict;
import tt::datastruct::ImmutableList;
import tt::datastruct::ImmutableSet;
import tt::datastruct::IndexedSet;
import tt::datastruct::IntervalTree;
import tt::datastruct::IntrusiveList;
import tt::datastruct::IntrusiveSet;
import tt::datastruct::ItemsView;
import tt::datastruct::KDTree;
import tt::datastruct::KeysView;
import tt::datastruct::LazyMap;
import tt::datastruct::LazySequence;
import tt::datastruct::LBQNode;
import tt::datastruct::LHSNode;
import tt::datastruct::LifoQueue;
import tt::datastruct::LinkedBlockingQueue;
import tt::datastruct::LinkedHashMap;
import tt::datastruct::LinkedHashSet;
import tt::datastruct::LinkedTransferQueue;
import tt::datastruct::List;
import tt::datastruct::LruMap;
import tt::datastruct::LsmTree;
import tt::datastruct::LTQNode;
import tt::datastruct::Map;
import tt::datastruct::MappingProxy;
import tt::datastruct::MemoryView;
import tt::datastruct::MinHash;
import tt::datastruct::MonotonicBuffer;
import tt::datastruct::MultidimensionalArray;
import tt::datastruct::MultiIndex;
import tt::datastruct::MultiSet;
import tt::datastruct::NamedStruct;
import tt::datastruct::ObjectPool;
import tt::datastruct::Octree;
import tt::datastruct::OctreeIndex;
import tt::datastruct::OrderedMultiDict;
import tt::datastruct::PersistentTrie;
import tt::datastruct::PrefixMap;
import tt::datastruct::PriorityBlockingQueue;
import tt::datastruct::Properties;
import tt::datastruct::QuadTree;
import tt::datastruct::QuotientFilter;
import tt::datastruct::RadixTree;
import tt::datastruct::RangeMap;
import tt::datastruct::RangeTree;
import tt::datastruct::RangeView;
import tt::datastruct::RingBuffer;
import tt::datastruct::RingChannel;
import tt::datastruct::Rope;
import tt::datastruct::RopeBuilder;
import tt::datastruct::RTree;
import tt::datastruct::RunLengthList;
import tt::datastruct::SegmentTree;
import tt::datastruct::SemaphoreQueue;
import tt::datastruct::SentinelValue;
import tt::datastruct::SequencedMap;
import tt::datastruct::SequencedSet;
import tt::datastruct::SimpleNamespace;
import tt::datastruct::SimpleQueue;
import tt::datastruct::SkipList;
import tt::datastruct::SlotMap;
import tt::datastruct::SmallVector;
import tt::datastruct::SortedKeyList;
import tt::datastruct::SparseSet;
import tt::datastruct::SpatialHash;
import tt::datastruct::SplayTree;
import tt::datastruct::StaticArray;
import tt::datastruct::StreamBuffer;
import tt::datastruct::SuffixArray;
import tt::datastruct::SuffixNode;
import tt::datastruct::SuffixTrie;
import tt::datastruct::SynchronousQueue;
import tt::datastruct::Treap;
import tt::datastruct::TreeNode;
import tt::datastruct::TreeTraversal;
import tt::datastruct::TrieMap;
import tt::datastruct::TrieSet;
import tt::datastruct::TrieSetNode;
import tt::datastruct::TtlMap;
import tt::datastruct::TupleArray;
import tt::datastruct::TypeRegistry;
import tt::datastruct::UndirectedGraph;
import tt::datastruct::UnorderedMultiMap;
import tt::datastruct::UnorderedMultiSet;
import tt::datastruct::ValuesView;
import tt::datastruct::VariantVector;
import tt::datastruct::Vector;
import tt::datastruct::VoronoiCell;
import tt::datastruct::WeakHashMap;
import tt::datastruct::WeakKeyDictionary;
import tt::datastruct::WeakReferenceMap;
import tt::datastruct::WeakSet;
import tt::datastruct::WeakValueDictionary;
import tt::datastruct::WeightedGraph;
import tt::util::BitField;
import tt::util::BiMap;
import tt::util::TtlCache;
import tt::math::Noise;
import tt::math::Easing;
import tt::compression::RunLength;
import tt::encoding::Checksum;
import tt::statistics::Distributions;
import tt::text::CaseConvert;
import tt::net::MacAddress;
import tt::net::HttpStatus;
import tt::foundation::Cofree;
import tt::foundation::Either;
import tt::foundation::FreeMonad;
import tt::foundation::Monad;
import tt::foundation::NonEmpty;
import tt::foundation::Reader;
import tt::foundation::State;
import tt::foundation::Try;
import tt::foundation::Validated;
import tt::foundation::Writer;
import tt::datascience::Bayesian;
import tt::datascience::Clustering;
import tt::datascience::Correlation;
import tt::datascience::Dimensionality;
import tt::datascience::FeatureEngineering;
import tt::datascience::Hypothesis;
import tt::datascience::Regression;
import tt::datascience::Resampling;
import tt::datascience::SignalStats;
import tt::datascience::TimeSeries;
import tt::platform::AsyncRuntime;
import tt::platform::ConfigStore;
import tt::platform::Cron;
import tt::platform::Env;
import tt::platform::FsWatcher;
import tt::platform::ProcessPool;
import tt::platform::Registry;
import tt::platform::ResourcePool;
import tt::platform::Scheduler;
import tt::platform::SignalBus;
import tt::ml::Attention;
import tt::ml::Callback;
import tt::ml::Checkpoint;
import tt::ml::CrossValidation;
import tt::ml::DBSCAN;
import tt::ml::DecisionTree;
import tt::ml::EarlyStopping;
import tt::ml::Embedding;
import tt::ml::GradientBoosting;
import tt::ml::KMeans;
import tt::ml::LDA;
import tt::ml::LSTM;
import tt::ml::Metric;
import tt::ml::NaiveBayes;
import tt::ml::PCA;
import tt::ml::RandomForest;
import tt::ml::RNN;
import tt::ml::SVM;
import tt::ml::Tokenizer;
import tt::ml::Trainer;
import tt::ml::TransformerBlock;
import tt::graphics::Camera2D;
import tt::graphics::Gui;
import tt::graphics::InputAction;
import tt::graphics::Lighting;
import tt::graphics::Material;
import tt::graphics::Mesh3D;
import tt::graphics::ParticleSystem;
import tt::graphics::PostProcess;
import tt::graphics::Renderer2D;
import tt::graphics::Tileset;
import tt::audio::AudioStream;
import tt::audio::Envelope;
import tt::audio::Filter;
import tt::audio::Midi;
import tt::audio::Mixer;
import tt::audio::Reverb;
import tt::audio::Sequencer;
import tt::audio::SoundFont;
import tt::audio::Spectrogram;
import tt::audio::Voice;

public fn main(): void {
    Pair.run_all_pair_tests();
    Range.run_all_range_tests();
    Lazy.run_all_lazy_tests();
    StringView.run_all_stringview_tests();
    SortedList.run_all_sortedlist_tests();
    BitVector.run_all_bitvector_tests();
    BloomFilter.run_all_bloomfilter_tests();
    LRUCache.run_all_lrucache_tests();
    MultiMap.run_all_multimap_tests();
    RefCount.run_all_refcount_tests();
    FunctionExt.run_all_functionext_tests();
    ResultExt.run_all_resultext_tests();
    String.run_all_string_tests();
    Optimization.run_all_optimization_tests();
    Interpolation.run_all_interpolation_tests();
    MatrixNorms.run_all_matrix_norms_tests();
    NDArrayLinalg.run_all_ndarray_linalg_tests();
    NDArrayIndexing.run_all_ndarray_indexing_tests();
    NDArrayStats.run_all_ndarray_stats_tests();
    NDArrayUfuncs.run_all_ndarray_ufuncs_tests();
    Convolution.run_all_convolution_tests();
    WindowFunctions.run_all_window_functions_tests();
    Activation.run_all_activation_tests();
    Loss.run_all_loss_core_tests();
    Tensor.run_all_ndtensor_tests();
    Sequential.run_all_sequential_tests();
    Linear.run_all_linear_tests();
    Regularization.run_all_regularization_tests();
    Optim.run_all_sgd_tests();
    Estimator.run_all_estimator_tests();
    Metrics.run_all_metrics_tests();
    DataLoader.run_all_ndarray_dataloader_tests();
    Autograd.run_all_autograd_tests();
    Color.run_all_color_tests();
    Canvas.run_all_canvas_tests();
    Window.run_all_window_tests();
    Sprite.run_all_sprite_tests();
    Shapes.run_all_shapes_tests();
    Font.run_all_font_tests();
    Draw.run_all_draw_tests();
    AudioDevice.run_all_audio_device_tests();
    Synth.run_all_synth_tests();
    Mime.run_all_mime_tests();
    Url.run_all_url_tests();
    Uri.run_all_uri_tests();
    Task.run_all_task_tests();
    Watchdog.run_all_watchdog_tests();
    PathUtils.run_all_path_utils_tests();
    Executor.run_all_executor_tests();
    StructuredLogger.run_all_structured_logger_tests();
    MessagePack.run_all_messagepack_tests();
    AnyValue.run_all_anyvalue_tests();
    ArenaMap.run_all_arenamap_tests();
    ArrayBlockingQueue.run_all_arrayblockingqueue_tests();
    ArrayDeque.run_all_arraydeque_tests();
    AtomicCounterMap.run_all_atomiccountermap_tests();
    AvlTree.run_all_avltree_tests();
    BTree.run_all_btree_tests();
    BTreeMap.run_all_btreemap_tests();
    BTreeSet.run_all_btreeset_tests();
    BitMatrix.run_all_bitmatrix_tests();
    BitsetView.run_all_bitsetview_tests();
    BitString.run_all_bitstring_tests();
    BipartiteGraph.run_all_bipartitegraph_tests();
    BlockPool.run_all_blockpool_tests();
    BlockingCollection.run_all_blockingcollection_tests();
    BloomFilterExt.run_all_bloomfilterext_tests();
    BoundedQueue.run_all_boundedqueue_tests();
    BroadcastBuffer.run_all_broadcastbuffer_tests();
    ByteArray.run_all_bytearray_tests();
    BulkOps.run_all_bulkops_tests();
    CapabilityCollection.run_all_capabilitycollection_tests();
    CharArray.run_all_chararray_tests();
    CollectionTraits.run_all_collectiontraits_tests();
    Comparators.run_all_comparators_tests();
    CompressedIntArray.run_all_compressedintarray_tests();
    ConcurrentBag.run_all_concurrentbag_tests();
    ConcurrentDictionary.run_all_concurrentdictionary_tests();
    ConcurrentHashMap.run_all_concurrenthashmap_tests();
    ConcurrentLinkedQueue.run_all_concurrentlinkedqueue_tests();
    ConcurrentSkipListMap.run_all_concurrentskiplistmap_tests();
    ConcurrentSkipListSet.run_all_concurrentskiplistset_tests();
    ConcurrentStack.run_all_concurrentstack_tests();
    Conversions.run_all_conversions_tests();
    ConvexHullSet.run_all_convexhullset_tests();
    CopyOnWriteArrayList.run_all_copyonwritearraylist_tests();
    CopyOnWriteArraySet.run_all_copyonwritearrayset_tests();
    CountMinSketch.run_all_countminsketch_tests();
    CountingAllocator.run_all_countingallocator_tests();
    CuckooFilter.run_all_cuckoofilter_tests();
    DataDoc.run_all_datadoc_tests();
    DataTypes.run_all_datatypes_tests();
    DefaultDictList.run_all_defaultdictlist_tests();
    DefaultDictSet.run_all_defaultdictset_tests();
    DelayQueue.run_all_delayqueue_tests();
    DenseSet.run_all_denseset_tests();
    DequeView.run_all_dequeview_tests();
    DictView.run_all_dictview_tests();
    DQEntry.run_all_dqentry_tests();
    DirectedAcyclicGraph.run_all_directedacyclicgraph_tests();
    DirectedGraph.run_all_directedgraph_tests();
    DynamicBitset.run_all_dynamicbitset_tests();
    EnumMap.run_all_enummap_tests();
    EnumSet.run_all_enumset_tests();
    ExpectedValue.run_all_expectedvalue_tests();
    ExpiringSet.run_all_expiringset_tests();
    FenwickTree.run_all_fenwicktree_tests();
    FlatMap.run_all_flatmap_tests();
    FlatSet.run_all_flatset_tests();
    FreeList.run_all_freelist_tests();
    FrozenRecord.run_all_frozenrecord_tests();
    GrowthPolicy.run_all_growthpolicy_tests();
    HashPolicy.run_all_hashpolicy_tests();
    Hashtable.run_all_hashtable_tests();
    HyperLogLog.run_all_hyperloglog_tests();
    HyperLogLogExt.run_all_hyperloglogext_tests();
    IdentityHashMap.run_all_identityhashmap_tests();
    ImmutableDict.run_all_immutabledict_tests();
    ImmutableList.run_all_immutablelist_tests();
    ImmutableSet.run_all_immutableset_tests();
    IndexedSet.run_all_indexedset_tests();
    IntervalTree.run_all_intervaltree_tests();
    IntrusiveList.run_all_intrusivelist_tests();
    IntrusiveSet.run_all_intrusiveset_tests();
    ItemsView.run_all_itemsview_tests();
    KDTree.run_all_kdtree_tests();
    KeysView.run_all_keysview_tests();
    LazyMap.run_all_lazymap_tests();
    LazySequence.run_all_lazysequence_tests();
    LBQNode.run_all_lbqnode_tests();
    LHSNode.run_all_lhsnode_tests();
    LifoQueue.run_all_lifoqueue_tests();
    LinkedBlockingQueue.run_all_linkedblockingqueue_tests();
    LinkedHashMap.run_all_linkedhashmap_tests();
    LinkedHashSet.run_all_linkedhashset_tests();
    LinkedTransferQueue.run_all_linkedtransferqueue_tests();
    List.run_all_list_tests();
    LruMap.run_all_lrumap_tests();
    LsmTree.run_all_lsmtree_tests();
    LTQNode.run_all_ltqnode_tests();
    Map.run_all_map_tests();
    MappingProxy.run_all_mappingproxy_tests();
    MemoryView.run_all_memoryview_tests();
    MinHash.run_all_minhash_tests();
    MonotonicBuffer.run_all_monotonicbuffer_tests();
    MultidimensionalArray.run_all_multidimensionalarray_tests();
    MultiIndex.run_all_multiindex_tests();
    MultiSet.run_all_multiset_tests();
    NamedStruct.run_all_namedstruct_tests();
    ObjectPool.run_all_objectpool_tests();
    Octree.run_all_octree_tests();
    OctreeIndex.run_all_octreeindex_tests();
    OrderedMultiDict.run_all_orderedmultidict_tests();
    PersistentTrie.run_all_persistenttrie_tests();
    PrefixMap.run_all_prefixmap_tests();
    PriorityBlockingQueue.run_all_priorityblockingqueue_tests();
    Properties.run_all_properties_tests();
    QuadTree.run_all_quadtree_tests();
    QuotientFilter.run_all_quotientfilter_tests();
    RadixTree.run_all_radixtree_tests();
    RangeMap.run_all_rangemap_tests();
    RangeTree.run_all_rangetree_tests();
    RangeView.run_all_rangeview_tests();
    RingBuffer.run_all_ringbuffer_tests();
    RingChannel.run_all_ringchannel_tests();
    Rope.run_all_rope_tests();
    RopeBuilder.run_all_ropebuilder_tests();
    RTree.run_all_rtree_tests();
    RunLengthList.run_all_runlengthlist_tests();
    SegmentTree.run_all_segmenttree_tests();
    SemaphoreQueue.run_all_semaphorequeue_tests();
    SentinelValue.run_all_sentinelvalue_tests();
    SequencedMap.run_all_sequencedmap_tests();
    SequencedSet.run_all_sequencedset_tests();
    SimpleNamespace.run_all_simplenamespace_tests();
    SimpleQueue.run_all_simplequeue_tests();
    SkipList.run_all_skiplist_tests();
    SlotMap.run_all_slotmap_tests();
    SmallVector.run_all_smallvector_tests();
    SortedKeyList.run_all_sortedkeylist_tests();
    SparseSet.run_all_sparseset_tests();
    SpatialHash.run_all_spatialhash_tests();
    SplayTree.run_all_splaytree_tests();
    StaticArray.run_all_staticarray_tests();
    StreamBuffer.run_all_streambuffer_tests();
    SuffixArray.run_all_suffixarray_tests();
    SuffixNode.run_all_suffixnode_tests();
    SuffixTrie.run_all_suffixtrie_tests();
    SynchronousQueue.run_all_synchronousqueue_tests();
    Treap.run_all_treap_tests();
    TreeNode.run_all_treenode_tests();
    TreeTraversal.run_all_treetraversal_tests();
    TrieMap.run_all_triemap_tests();
    TrieSet.run_all_trieset_tests();
    TrieSetNode.run_all_triesetnode_tests();
    TtlMap.run_all_ttlmap_tests();
    TupleArray.run_all_tuplearray_tests();
    TypeRegistry.run_all_typeregistry_tests();
    UndirectedGraph.run_all_undirectedgraph_tests();
    UnorderedMultiMap.run_all_unorderedmultimap_tests();
    UnorderedMultiSet.run_all_unorderedmultiset_tests();
    ValuesView.run_all_valuesview_tests();
    VariantVector.run_all_variantvector_tests();
    Vector.run_all_vector_tests();
    VoronoiCell.run_all_voronoicell_tests();
    WeakHashMap.run_all_weakhashmap_tests();
    WeakKeyDictionary.run_all_weakkeydictionary_tests();
    WeakReferenceMap.run_all_weakreferencemap_tests();
    WeakSet.run_all_weakset_tests();
    WeakValueDictionary.run_all_weakvaluedictionary_tests();
    WeightedGraph.run_all_weightedgraph_tests();
    BitField.run_all_bitfield_tests();
    BiMap.run_all_bimap_tests();
    TtlCache.run_all_ttlcache_tests();
    Noise.run_all_noise_tests();
    Easing.run_all_easing_tests();
    RunLength.run_all_runlength_tests();
    Checksum.run_all_checksum_tests();
    Distributions.run_all_distributions_tests();
    CaseConvert.run_all_caseconvert_tests();
    MacAddress.run_all_macaddress_tests();
    HttpStatus.run_all_httpstatus_tests();
    Cofree.run_all_cofree_tests();
    Either.run_all_either_tests();
    FreeMonad.run_all_freemonad_tests();
    Monad.run_all_monad_tests();
    NonEmpty.run_all_nonempty_tests();
    Reader.run_all_reader_tests();
    State.run_all_state_tests();
    Try.run_all_try_tests();
    Validated.run_all_validated_tests();
    Writer.run_all_writer_tests();
    Bayesian.run_all_bayesian_tests();
    Clustering.run_all_clustering_tests();
    Correlation.run_all_correlation_tests();
    Dimensionality.run_all_dimensionality_tests();
    FeatureEngineering.run_all_featureengineering_tests();
    Hypothesis.run_all_hypothesis_tests();
    Regression.run_all_regression_tests();
    Resampling.run_all_resampling_tests();
    SignalStats.run_all_signalstats_tests();
    TimeSeries.run_all_timeseries_tests();
    AsyncRuntime.run_all_asyncruntime_tests();
    ConfigStore.run_all_configstore_tests();
    Cron.run_all_cron_tests();
    Env.run_all_env_tests();
    FsWatcher.run_all_fswatcher_tests();
    ProcessPool.run_all_processpool_tests();
    Registry.run_all_registry_tests();
    ResourcePool.run_all_resourcepool_tests();
    Scheduler.run_all_scheduler_tests();
    SignalBus.run_all_signalbus_tests();
    Attention.run_all_attention_tests();
    Callback.run_all_callback_tests();
    Checkpoint.run_all_checkpoint_tests();
    CrossValidation.run_all_crossvalidation_tests();
    DBSCAN.run_all_dbscan_tests();
    DecisionTree.run_all_decisiontree_tests();
    EarlyStopping.run_all_earlystopping_tests();
    Embedding.run_all_embedding_tests();
    GradientBoosting.run_all_gradientboosting_tests();
    KMeans.run_all_kmeans_tests();
    LDA.run_all_lda_tests();
    LSTM.run_all_lstm_tests();
    Metric.run_all_metric_tests();
    NaiveBayes.run_all_naivebayes_tests();
    PCA.run_all_pca_tests();
    RandomForest.run_all_randomforest_tests();
    RNN.run_all_rnn_tests();
    SVM.run_all_svm_tests();
    Tokenizer.run_all_tokenizer_tests();
    Trainer.run_all_trainer_tests();
    TransformerBlock.run_all_transformerblock_tests();
    Camera2D.run_all_camera2d_tests();
    Gui.run_all_gui_tests();
    InputAction.run_all_inputaction_tests();
    Lighting.run_all_lighting_tests();
    Material.run_all_material_tests();
    Mesh3D.run_all_mesh3d_tests();
    ParticleSystem.run_all_particlesystem_tests();
    PostProcess.run_all_postprocess_tests();
    Renderer2D.run_all_renderer2d_tests();
    Tileset.run_all_tileset_tests();
    AudioStream.run_all_audiostream_tests();
    Envelope.run_all_envelope_tests();
    Filter.run_all_filter_tests();
    Midi.run_all_midi_tests();
    Mixer.run_all_mixer_tests();
    Reverb.run_all_reverb_tests();
    Sequencer.run_all_sequencer_tests();
    SoundFont.run_all_soundfont_tests();
    Spectrogram.run_all_spectrogram_tests();
    Voice.run_all_voice_tests();
}
"#;

    let output = run_source_with_modules(source, "..")
        .expect("stdlib-expansion aggregators should compile and run");
    assert!(
        !output.iter().any(|line| line.contains("FAIL:")),
        "stdlib-expansion aggregators reported FAIL:\n{}",
        output.join("\n")
    );
    assert!(
        output.iter().any(|line| line.starts_with("PASS")),
        "stdlib-expansion aggregators emitted no PASS marker"
    );
}

