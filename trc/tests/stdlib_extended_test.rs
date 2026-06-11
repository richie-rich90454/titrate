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
