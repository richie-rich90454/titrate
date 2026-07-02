---
layout: home

hero:
  name: Titrate
  text: The language for precise systems
  tagline: Memory-safe without GC. Zero-cost generics. Scientific computing built in. Write code that is as reliable as it is expressive.
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: Language Guide
      link: /guide/variables
    - theme: alt
      text: View on GitHub
      link: https://github.com/richie-rich90454/titrate

features:
  - icon:
      src: /icons/vm.svg
    title: Bytecode VM
    details: Compile to optimized bytecode and run on the Titrate VM with built-in garbage-free memory management. Significantly faster than tree-walking interpretation.
  - icon:
      src: /icons/compat.svg
    title: LLVM Native Backend
    details: Compile to standalone native executables via LLVM. Release-mode builds run 3–6× faster than the bytecode VM for compute-bound workloads. One flag, one binary.
  - icon:
      src: /icons/shield.svg
    title: Ownership and Regions
    details: Move semantics, borrowing, and region-based allocation -- memory safety without garbage collection or manual free.
  - icon:
      src: /icons/generics.svg
    title: Zero-Cost Generics
    details: Monomorphizing compiler generates specialized code for each type instance. No boxing, no vtables, no runtime overhead.
  - icon:
      src: /icons/module.svg
    title: Module System
    details: Organize code with imports, control visibility with public and private, and detect circular dependencies at compile time.
  - icon:
      src: /icons/flask.svg
    title: Scientific Computing
    details: Bioinformatics, physics simulation, materials science, signal processing, image/audio processing, ML, computational geometry, and more -- all in the standard library.
  - icon:
      src: /icons/result.svg
    title: Result-Based Error Handling
    details: No exceptions, no null pointer errors. Use `Result<T, E>` with `ok()` and `err()` to make error handling explicit and type-safe at compile time.
  - icon:
      src: /icons/operator.svg
    title: Operator Overloading
    details: Define natural syntax for your types with `fn operator+`, `fn operator*`, and more. Build expressive DSLs for math, physics, and data.
  - icon:
      src: /icons/library.svg
    title: Rich Standard Library
    details: Collections, I/O, JSON, CSV, XML, TCP, HTTP, SHA-256, HMAC, Base64, bioinformatics, physics, ML, HFT, simulation -- everything you need, out of the box.
---

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run your first program
echo 'public fn main(): void { io::println("Hello, Titrate!"); }' > hello.tr
trc hello.tr

# Or use the build tool
pipette new myproject
pipette run

# Compile to a standalone native executable (3–6× faster for compute-bound code)
trc --native --release hello.tr
```

::: tip Native Backend
Titrate ships with an LLVM native backend that compiles `.tr` programs
to standalone executables. For compute-bound workloads — simulations,
numerical kernels, signal processing — release-mode native builds run
**3–6× faster** than the bytecode VM. See the
[native backend guides](/guide/native-intro) to get started.
:::

## Language at a Glance

```titrate
// Variables -- let for mutable with inference, var for mutable with explicit type
let name = "Titrate";
var count: int = 0;

// Functions with name: Type parameter order
public fn greet(name: string): void {
    io::println("Hello, " + name);
}

// Classes with fn init() constructors
public class Point {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }
}

// Generics with monomorphization -- zero runtime overhead
public class Box<T> {
    public T value;

    public fn init(value: T) { this.value = value; }
}

// Result-based error handling -- no exceptions
fn parseAndDouble(s: string): Result<int, string> {
    // Integer.parseInt returns int; check for parse failure
    let n: int = Integer.parseInt(s);
    // Simple check: if result is 0 but input is not "0", assume failure
    if (n == 0 && s != "0") {
        return err("not a number");
    }
    return ok(n * 2);
}

// Closures that capture by reference
let double = fn(x: int): int => x * 2;
let numbers = new ArrayList<int>();
numbers.forEach(fn(n: int): void {
    io::println(Integer.toString(double(n)));
});

// Operator overloading
public class Vec2 {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    public fn operator+(other: Vec2): Vec2 {
        return new Vec2(this.x + other.x, this.y + other.y);
    }
}

// Ranges and iterators
for (i in 0..10) {
    io::println(Integer.toString(i));
}
```

## See It In Action

### Molecular Dynamics Simulation

```titrate
import tt::chem::Atom;
import tt::chem::Molecule;
import tt::chem::ForceField;
import tt::chem::MDSimulation;
import tt::chem::VerletIntegrator;

public fn main(): void {
    let water: Molecule = new Molecule("water");
    water.addAtom(Atom.oxygen(0.0, 0.0, 0.0));
    water.addAtom(Atom.hydrogen(0.9572, 0.0, 0.0));
    water.addAtom(Atom.hydrogen(-0.2399, 0.9270, 0.0));

    let ff: ForceField = new ForceField();
    ff.addBondTerm(0, 1, 450.0, 0.9572);
    ff.addAngleTerm(1, 0, 2, 55.0, 104.52);

    let integrator: VerletIntegrator = new VerletIntegrator(1.0, "berendsen", 300.0);
    let md: MDSimulation = new MDSimulation(water, ff, integrator);
    md.run(1000);

    io::println("Energy: " + Double.toString(ff.totalEnergy(water)));
}
```

### JSON API Client with Error Handling

```titrate
import tt::json::Json;
import tt::json::JsonValue;
import tt::net::HttpClient;
import tt::lang::Integer;

public fn fetchUser(id: int): Result<JsonValue, string> {
    let client: HttpClient = new HttpClient();
    let url: string = "https://api.example.com/users/" + Integer.toString(id);
    let response: HttpResponse = client.get(url);

    if (response.getStatusCode() == 200) {
        let body: string = response.getBody();
        let parsed: JsonValue = Json.parse(body);
        if (!parsed.isNull()) { return ok(parsed); }
        return err("Failed to parse JSON");
    }
    return err("HTTP request failed with status: " + Integer.toString(response.getStatusCode()));
}
```

### Data Processing with NDArray

```titrate
import tt::math::ndarray::NDArray;
import tt::math::ndarray::NDArrayReduce;
import tt::math::ndarray::NDArrayMath;

public fn normalize(data: NDArray<double>): NDArray<double> {
    let mean: double = NDArrayReduce.mean(data);
    let std: double = NDArrayReduce.stddev(data);
    return NDArrayMath.map(data, fn(x: double): double => (x - mean) / std);
}

public fn correlation(x: NDArray<double>, y: NDArray<double>): double {
    let nx: NDArray<double> = normalize(x);
    let ny: NDArray<double> = normalize(y);
    return NDArrayMath.dot(nx, ny) / (nx.size() as double);
}
```

### Custom Collection with Generics

```titrate
public class RingBuffer<T> implements Iterable<T> {
    private ArrayList<T> data;
    private int head;
    private int count;

    public fn init(capacity: int) {
        this.data = new ArrayList<T>();
        this.head = 0;
        this.count = 0;
    }

    public fn push(item: T): void {
        this.data.add(item);
        this.count = this.count + 1;
    }

    public fn iterator(): Iterator<T> {
        return new RingBufferIterator<T>(this);
    }
}

// Use in for-in loops naturally
let buf = new RingBuffer<string>(3);
buf.push("first");
buf.push("second");
for (item in buf) {
    io::println(item);
}
```

## Why Titrate?

### Type Safety Without Runtime Cost

Titrate's monomorphizing compiler generates specialized code for each generic type instance. `ArrayList<int>` runs just as fast as hand-written code for integers -- no boxing, no type checks at runtime.

### Memory Safety Without Garbage Collection

Ownership semantics, move checking, and region-based allocation give you memory safety guarantees without the pause times and overhead of a garbage collector. When a value goes out of scope, it is cleaned up immediately.

### Clean, Modern Syntax

Inspired by Rust and Python, Titrate uses `name: Type` parameter order, `fn` declarations, and lowercase `string` from the start. But it also supports C-family sugar forms (`int x = 5`, `++i`, `ClassName(params)`) so developers from C, C++, and ECMAScript feel at home immediately.

### Scientific Computing Built In

Chemistry simulations (Atom, Molecule, ForceField, MD, RHF), bioinformatics (Sequence, Alignment, PhyloTree), physics (Particle, ForceField, NBodySimulator), machine learning (Tensor, Model, Optimizer), signal processing (FFT2, Filter, Wavelet), image processing, audio processing, computational geometry, NLP, HFT, and discrete-event simulation are all part of the standard library -- not third-party packages.

### Comprehensive Standard Library

Collections (ArrayList, HashMap, HashSet, Vec), I/O (File, BufferedReader), serialization (JSON, CSV, XML), networking (TCP, HTTP), cryptography (SHA-256, HMAC, Base64), and more -- all included out of the box.

## Comparison

How does Titrate compare to other systems languages for common tasks?

<div class="comparison-grid">

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name titrate-lang">Titrate</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="check">✓</span></div>
  </div>
</div>

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name">C</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="check">✓</span></div>
  </div>
</div>

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name">Rust</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="cross">✗</span></div>
  </div>
</div>

<div class="comparison-card">
  <div class="comparison-header">
    <span class="lang-name">Python</span>
  </div>
  <div class="comparison-features">
    <div class="feature-row"><span class="feature-name">Memory safety</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">No garbage collector</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Zero-cost generics</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Simple syntax</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Scientific computing stdlib</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Result-based error handling</span><span class="cross">✗</span></div>
    <div class="feature-row"><span class="feature-name">Operator overloading</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">Fast compile times</span><span class="check">✓</span></div>
    <div class="feature-row"><span class="feature-name">C-family sugar forms</span><span class="cross">✗</span></div>
  </div>
</div>

</div>

## Standard Library Ecosystem

Titrate ships with a comprehensive standard library organized into intuitive modules:

::: ecosystem-showcase

<!-- Core Collections -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📦</span>
<h4 class="ecosystem-card-title">tt::util</h4>
</div>
<p class="ecosystem-card-types">ArrayList, HashMap, HashSet, Vec, Stack, Queue</p>
<code class="ecosystem-card-code">let list = new ArrayList&lt;int&gt;();
list.add(42);</code>
<a class="ecosystem-card-link" href="/stdlib/util/ArrayList"></a>
</div>

<!-- I/O -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📄</span>
<h4 class="ecosystem-card-title">tt::io</h4>
</div>
<p class="ecosystem-card-types">File, BufferedReader, FileWriter, Path</p>
<code class="ecosystem-card-code">let f = File.open("data.txt");
let content = f.readAll();</code>
<a class="ecosystem-card-link" href="/stdlib/io/File"></a>
</div>

<!-- JSON -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-data">Data</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📋</span>
<h4 class="ecosystem-card-title">tt::json</h4>
</div>
<p class="ecosystem-card-types">JsonValue, Json, JsonPath, JsonSchema</p>
<code class="ecosystem-card-code">let obj = Json.parse(text);
let v = obj.get("key");</code>
<a class="ecosystem-card-link" href="/stdlib/json/Json"></a>
</div>

<!-- XML -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-data">Data</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🏷️</span>
<h4 class="ecosystem-card-title">tt::xml</h4>
</div>
<p class="ecosystem-card-types">XmlNode, Xml, XPath, XmlBuilder</p>
<code class="ecosystem-card-code">let doc = Xml.parse(xmlStr);
let nodes = XPath.select(doc, "//item");</code>
<a class="ecosystem-card-link" href="/stdlib/xml/Xml"></a>
</div>

<!-- Math -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔢</span>
<h4 class="ecosystem-card-title">tt::math</h4>
</div>
<p class="ecosystem-card-types">Math, MathAdvanced, MathTrig, Special</p>
<code class="ecosystem-card-code">let x = MathAdvanced.sqrt(2.0);
let y = MathTrig.sin(Math.PI()/4);</code>
<a class="ecosystem-card-link" href="/stdlib/math/Math"></a>
</div>

<!-- Linear Algebra -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📐</span>
<h4 class="ecosystem-card-title">tt::math::linalg</h4>
</div>
<p class="ecosystem-card-types">Matrix, MatrixDecomp, SparseMatrix</p>
<code class="ecosystem-card-code">let m = Matrix.fromArray(data);
let det = MatrixDecomp.det(m);</code>
<a class="ecosystem-card-link" href="/stdlib/math/linalg/Matrix"></a>
</div>

<!-- NDArray -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📊</span>
<h4 class="ecosystem-card-title">tt::math::ndarray</h4>
</div>
<p class="ecosystem-card-types">NDArray, NDArrayReduce, NDArrayMath</p>
<code class="ecosystem-card-code">let arr = NDArray.zeros(3, 4);
let mean = NDArrayReduce.mean(arr);</code>
<a class="ecosystem-card-link" href="/stdlib/math/ndarray/NDArray"></a>
</div>

<!-- Chemistry -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🧪</span>
<h4 class="ecosystem-card-title">tt::chem</h4>
</div>
<p class="ecosystem-card-types">Atom, Molecule, ForceField, MD, RHF</p>
<code class="ecosystem-card-code">let mol = new Molecule("water");
mol.addAtom(Atom.oxygen(0, 0, 0));</code>
<a class="ecosystem-card-link" href="/stdlib/chem/Molecule"></a>
</div>

<!-- Bioinformatics -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🧬</span>
<h4 class="ecosystem-card-title">tt::bio</h4>
</div>
<p class="ecosystem-card-types">Sequence, Alignment, PhyloTree, CodonTable</p>
<code class="ecosystem-card-code">let seq = Sequence.fromDNA("ATGC");
let aligned = Alignment.align(a, b);</code>
<a class="ecosystem-card-link" href="/stdlib/bio/Sequence"></a>
</div>

<!-- Physics -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">⚡</span>
<h4 class="ecosystem-card-title">tt::physics</h4>
</div>
<p class="ecosystem-card-types">Particle, ForceField, NBodySimulator, RigidBody</p>
<code class="ecosystem-card-code">let sim = new NBodySimulator();
sim.addParticle(p, mass, pos);</code>
<a class="ecosystem-card-link" href="/stdlib/physics/NBodySimulator"></a>
</div>

<!-- Materials -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">💎</span>
<h4 class="ecosystem-card-title">tt::materials</h4>
</div>
<p class="ecosystem-card-types">CrystalStructure, XRayDiffraction, Elasticity</p>
<code class="ecosystem-card-code">let crystal = CrystalStructure.load("cif");
let pattern = XRayDiffraction.simulate(crystal);</code>
<a class="ecosystem-card-link" href="/stdlib/materials/CrystalStructure"></a>
</div>

<!-- Signal Processing -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📈</span>
<h4 class="ecosystem-card-title">tt::sigproc</h4>
</div>
<p class="ecosystem-card-types">FFT2, Filter, Wavelet, Spectrogram</p>
<code class="ecosystem-card-code">let spectrum = FFT2.transform(signal);
let filtered = Filter.lowpass(spectrum, cutoff);</code>
<a class="ecosystem-card-link" href="/stdlib/sigproc/FFT2"></a>
</div>

<!-- Image Processing -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🖼️</span>
<h4 class="ecosystem-card-title">tt::image</h4>
</div>
<p class="ecosystem-card-types">Image, Kernel, Morphology, Threshold</p>
<code class="ecosystem-card-code">let img = Image.load("photo.png");
let edges = Kernel.apply(img, sobel);</code>
<a class="ecosystem-card-link" href="/stdlib/image/Image"></a>
</div>

<!-- Audio Processing -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🎵</span>
<h4 class="ecosystem-card-title">tt::audio</h4>
</div>
<p class="ecosystem-card-types">AudioBuffer, Pitch, Mfcc</p>
<code class="ecosystem-card-code">let audio = AudioBuffer.load("song.wav");
let pitch = Pitch.detect(audio);</code>
<a class="ecosystem-card-link" href="/stdlib/audio/AudioBuffer"></a>
</div>

<!-- Machine Learning -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🤖</span>
<h4 class="ecosystem-card-title">tt::ml</h4>
</div>
<p class="ecosystem-card-types">Tensor, Layer, Optimizer, Model</p>
<code class="ecosystem-card-code">let model = Model.sequential([
  Layer.dense(128, "relu")
]);</code>
<a class="ecosystem-card-link" href="/stdlib/ml/Model"></a>
</div>

<!-- Computational Geometry -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔺</span>
<h4 class="ecosystem-card-title">tt::geom</h4>
</div>
<p class="ecosystem-card-types">ConvexHull, Delaunay, SpatialIndex</p>
<code class="ecosystem-card-code">let hull = ConvexHull.compute(points);
let mesh = Delaunay.triangulate(hull);</code>
<a class="ecosystem-card-link" href="/stdlib/geom/ConvexHull"></a>
</div>

<!-- NLP -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">💬</span>
<h4 class="ecosystem-card-title">tt::nlp</h4>
</div>
<p class="ecosystem-card-types">Tokenizer, Stemmer, Classifier</p>
<code class="ecosystem-card-code">let tokens = Tokenizer.tokenize(text);
let stems = Stemmer.stemAll(tokens);</code>
<a class="ecosystem-card-link" href="/stdlib/nlp/Tokenizer"></a>
</div>

<!-- High-Frequency Trading -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">💹</span>
<h4 class="ecosystem-card-title">tt::hft</h4>
</div>
<p class="ecosystem-card-types">FixParser, OrderRouter, RiskManager, Backtest</p>
<code class="ecosystem-card-code">let order = FixParser.parse(msg);
Backtest.run(strategy, data);</code>
<a class="ecosystem-card-link" href="/stdlib/hft/Backtest"></a>
</div>

<!-- Discrete-Event Simulation -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">⏱️</span>
<h4 class="ecosystem-card-title">tt::sim</h4>
</div>
<p class="ecosystem-card-types">Simulation, Resource, Process, Monitor</p>
<code class="ecosystem-card-code">let sim = new Simulation();
sim.addProcess(myProcess);</code>
<a class="ecosystem-card-link" href="/stdlib/sim/Simulation"></a>
</div>

<!-- Quantitative Finance -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-special">Special</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">💰</span>
<h4 class="ecosystem-card-title">tt::finance</h4>
</div>
<p class="ecosystem-card-types">BlackScholes, Portfolio, YieldCurve</p>
<code class="ecosystem-card-code">let price = BlackScholes.call(
  S, K, T, r, sigma);</code>
<a class="ecosystem-card-link" href="/stdlib/finance/BlackScholes"></a>
</div>

<!-- Advanced Cryptography -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔐</span>
<h4 class="ecosystem-card-title">tt::crypto2</h4>
</div>
<p class="ecosystem-card-types">AES, RSA, ECDSA, KDF</p>
<code class="ecosystem-card-code">let cipher = AES.encrypt(key, data);
let sig = ECDSA.sign(privKey, msg);</code>
<a class="ecosystem-card-link" href="/stdlib/crypto2/AES"></a>
</div>

<!-- Networking -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🌐</span>
<h4 class="ecosystem-card-title">tt::net</h4>
</div>
<p class="ecosystem-card-types">TcpClient, TcpServer, HttpClient</p>
<code class="ecosystem-card-code">let client = new HttpClient();
let resp = client.get(url);</code>
<a class="ecosystem-card-link" href="/stdlib/net/HttpClient"></a>
</div>

<!-- Hash Functions -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔒</span>
<h4 class="ecosystem-card-title">tt::crypto</h4>
</div>
<p class="ecosystem-card-types">SHA256, HMAC, Hash</p>
<code class="ecosystem-card-code">let hash = SHA256.hash(data);
let mac = HMAC.compute(key, msg);</code>
<a class="ecosystem-card-link" href="/stdlib/crypto/SHA256"></a>
</div>

<!-- Encoding -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🔡</span>
<h4 class="ecosystem-card-title">tt::encoding</h4>
</div>
<p class="ecosystem-card-types">Base64, Hex, Url</p>
<code class="ecosystem-card-code">let enc = Base64.encode(data);
let dec = Hex.decode("0x1A2B");</code>
<a class="ecosystem-card-link" href="/stdlib/encoding/Base64"></a>
</div>

<!-- Argument Parsing -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">⚙️</span>
<h4 class="ecosystem-card-title">tt::argparse</h4>
</div>
<p class="ecosystem-card-types">ArgumentParser</p>
<code class="ecosystem-card-code">let parser = new ArgumentParser();
parser.addArg("--input", "file");</code>
<a class="ecosystem-card-link" href="/stdlib/argparse/ArgumentParser"></a>
</div>

<!-- CSV -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-data">Data</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📊</span>
<h4 class="ecosystem-card-title">tt::csv</h4>
</div>
<p class="ecosystem-card-types">CsvReader, CsvWriter</p>
<code class="ecosystem-card-code">let reader = CsvReader.open("data.csv");
let row = reader.next();</code>
<a class="ecosystem-card-link" href="/stdlib/csv/CsvReader"></a>
</div>

<!-- Random -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-core">Core</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">🎲</span>
<h4 class="ecosystem-card-title">tt::random</h4>
</div>
<p class="ecosystem-card-types">Random, Prng, Sampling</p>
<code class="ecosystem-card-code">let rng = new Random();
let n = rng.nextInt(0, 100);</code>
<a class="ecosystem-card-link" href="/stdlib/random/Random"></a>
</div>

<!-- Statistics -->
<div class="ecosystem-card">
<span class="ecosystem-badge ecosystem-badge-science">Science</span>
<div class="ecosystem-card-header">
<span class="ecosystem-card-icon">📉</span>
<h4 class="ecosystem-card-title">tt::statistics</h4>
</div>
<p class="ecosystem-card-types">Statistics, Bootstrap, TimeSeries</p>
<code class="ecosystem-card-code">let stats = Statistics.fromData(data);
let ci = Bootstrap.confidenceInterval(95);</code>
<a class="ecosystem-card-link" href="/stdlib/statistics/Statistics"></a>
</div>

:::

## Community and Contributing

Titrate is open source and actively developed. Join us!

- **GitHub** -- [richie-rich90454/titrate](https://github.com/richie-rich90454/titrate) -- Report issues, request features, contribute code
- **Contributing** -- Check the [contributing guide](/guide/contributing) for guidelines
- **Build and Test** -- `cargo test --lib`, `cargo test --test stdlib_test`, `cargo test --test mega_test`

```bash
# Clone and build
git clone https://github.com/richie-rich90454/titrate.git
cd titrate
cargo build --release

# Run the full test suite
cargo test --lib; cargo test --test stdlib_test; cargo test --test mega_test
```
