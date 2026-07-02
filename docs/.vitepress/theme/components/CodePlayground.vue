<script setup>
import { ref, computed, onMounted } from 'vue'
import { useData } from 'vitepress'

const { isDark } = useData()

// Example snippets demonstrating different Titrate features
const examples = [
  {
    id: 'hello',
    name: 'Hello World',
    code: `// Hello World in Titrate
public fn main(): void {
    io::println("Hello, World!");
}`,
    output: 'Hello, World!'
  },
  {
    id: 'variables',
    name: 'Variables & Types',
    code: `// Variable declarations
let name = "Alice";           // inferred as string
var age: int = 30;            // explicit type
const MAX: int = 100;         // compile-time constant

// Numeric types
let count = 42;               // int
let price = 19.99;            // double
let precise = 3.14159q;       // quad-precision

public fn main(): void {
    io::println("Name: " + name);
    io::println("Age: " + Integer.toString(age));
}`,
    output: 'Name: Alice\nAge: 30'
  },
  {
    id: 'functions',
    name: 'Functions',
    code: `// Function declarations
public fn greet(name: string): void {
    io::println("Hello, " + name + "!");
}

// Generic function
public fn map<T, R>(list: ArrayList<T>, f: fn(T): R): ArrayList<R> {
    let result = new ArrayList<R>();
    for (item in list) {
        result.add(f(item));
    }
    return result;
}

// Arrow closure
let square = fn(x: int): int => x * x;

public fn main(): void {
    greet("Titrate");
    io::println("Square of 5: " + Integer.toString(square(5)));
}`,
    output: 'Hello, Titrate!\nSquare of 5: 25'
  },
  {
    id: 'classes',
    name: 'Classes & OOP',
    code: `// Class declaration
public class Point {
    public double x;
    public double y;

    public fn init(x: double, y: double) {
        this.x = x;
        this.y = y;
    }

    public fn distance(other: Point): double {
        let dx = this.x - other.x;
        let dy = this.y - other.y;
        return MathAdvanced.sqrt(dx * dx + dy * dy);
    }
}

public fn main(): void {
    let p1 = new Point(0.0, 0.0);
    let p2 = new Point(3.0, 4.0);
    let dist = p1.distance(p2);
    io::println("Distance: " + Double.toString(dist));
}`,
    output: 'Distance: 5.0'
  },
  {
    id: 'generics',
    name: 'Generics',
    code: `// Generic collections
let list: ArrayList<string> = new ArrayList<string>();
list.add("apple");
list.add("banana");
list.add("cherry");

// Generic HashMap
let map: HashMap<string, int> = new HashMap<string, int>();
map.put("one", 1);
map.put("two", 2);

public fn main(): void {
    io::println("List size: " + Integer.toString(list.size()));
    io::println("Map value: " + Integer.toString(map.get("one")));
}`,
    output: 'List size: 3\nMap value: 1'
  },
  {
    id: 'result',
    name: 'Error Handling',
    code: `// Result type for recoverable errors
public fn divide(a: int, b: int): Result<int, string> {
    if (b == 0) {
        return Result.err("Division by zero");
    }
    return Result.ok(a / b);
}

public fn main(): void {
    let r = divide(10, 2);
    if (r.isOk()) {
        io::println("Result: " + Integer.toString(r.unwrap()));
    } else {
        io::println("Error: " + r.getError());
    }
    
    // Error propagation with ?
    // let value = mightFail()?;
}`,
    output: 'Result: 5'
  },
  {
    id: 'pattern',
    name: 'Pattern Matching',
    code: `// Switch with pattern matching
enum JsonValue {
    Null,
    Bool(bool),
    Number(double),
    Str(string)
}

public fn describe(val: JsonValue): string {
    switch (val) {
        case Null => "null value";
        case Bool(b) => b ? "true" : "false";
        case Number(n) => "number: " + Double.toString(n);
        case Str(s) => "string: " + s;
        case _ => "unknown";
    }
}

public fn main(): void {
    let val = JsonValue.Number(42.0);
    io::println(describe(val));
}`,
    output: 'number: 42.0'
  },
  {
    id: 'ranges',
    name: 'Ranges & Iterators',
    code: `// Range expressions
let exclusive: Range = 1..10;      // 1, 2, ..., 9
let inclusive: Range = 1..=10;     // 1, 2, ..., 10

// For-in loop
public fn main(): void {
    let sum = 0;
    for (i in 1..=5) {
        sum = sum + i;
    }
    io::println("Sum 1 to 5: " + Integer.toString(sum));
    
    // While loop
    var count: int = 0;
    while (count < 3) {
        io::println("Count: " + Integer.toString(count));
        count++;
    }
}`,
    output: 'Sum 1 to 5: 15\nCount: 0\nCount: 1\nCount: 2'
  },
  {
    id: 'math',
    name: 'Math & Science',
    code: `// Math module usage
// Note: Math is split into Math, MathAdvanced, MathTrig

let pi = Math.PI();
let e = Math.E();
let absVal = Math.abs(-42);
let floorVal = Math.floor(3.7);

// Advanced math functions
let sqrt = MathAdvanced.sqrt(2.0);
let pow = MathAdvanced.pow(2.0, 10.0);
let ln = MathAdvanced.ln(e);

// Trigonometry
let sin = MathTrig.sin(pi / 2.0);
let cos = MathTrig.cos(0.0);

public fn main(): void {
    io::println("PI: " + Double.toString(pi));
    io::println("sqrt(2): " + Double.toString(sqrt));
    io::println("sin(pi/2): " + Double.toString(sin));
}`,
    output: 'PI: 3.141592653589793\nsqrt(2): 1.4142135623730951\nsin(pi/2): 1.0'
  },
  {
    id: 'closure',
    name: 'Closures',
    code: `// Block closure
let double = fn(x: int): int {
    return x * 2;
};

// Arrow closure
let add = fn(a: int, b: int): int => a + b;

// Function type
let mapper: fn(int): string = fn(x: int): string {
    return Integer.toString(x);
};

public fn main(): void {
    io::println("Double 5: " + Integer.toString(double(5)));
    io::println("Add 3+4: " + Integer.toString(add(3, 4)));
    io::println("Map 42: " + mapper(42));
}`,
    output: 'Double 5: 10\nAdd 3+4: 7\nMap 42: 42'
  }
]

// Current state
const selectedExample = ref('hello')
const code = ref(examples[0].code)
const customCode = ref('')
const isCustomMode = ref(false)
const showOutput = ref(true)

// Computed properties
const currentOutput = computed(() => {
  if (isCustomMode.value) {
    return '// Run the code to see output\n// (This is a documentation demo)'
  }
  const example = examples.find(e => e.id === selectedExample.value)
  return example ? example.output : ''
})

const currentExample = computed(() => {
  return examples.find(e => e.id === selectedExample.value)
})

// Methods
function selectExample(id) {
  selectedExample.value = id
  const example = examples.find(e => e.id === id)
  if (example) {
    code.value = example.code
    isCustomMode.value = false
    customCode.value = ''
  }
}

function toggleCustomMode() {
  isCustomMode.value = !isCustomMode.value
  if (isCustomMode.value) {
    customCode.value = code.value
    code.value = customCode.value
  } else {
    const example = examples.find(e => e.id === selectedExample.value)
    if (example) {
      code.value = example.code
    }
  }
}

function updateCode(newCode) {
  if (isCustomMode.value) {
    customCode.value = newCode
    code.value = newCode
  }
}

function resetCode() {
  const example = examples.find(e => e.id === selectedExample.value)
  if (example) {
    code.value = example.code
    customCode.value = ''
    isCustomMode.value = false
  }
}

// Syntax highlighting (simple token-based approach for documentation)
const highlightedCode = computed(() => {
  return highlightTitrate(code.value, isDark.value)
})

function highlightTitrate(source, isDark) {
  // Simple token-based highlighting for documentation demo
  // This is a simplified version - the actual syntax highlighting
  // uses the titrate-lang.js TextMate grammar with Shiki
  
  const colors = isDark ? {
    keyword: '#ff99bb',
    type: '#f0b4f0',
    string: '#a8e6a3',
    number: '#ffa86d',
    comment: '#7a8599',
    function: '#82d4ff',
    operator: '#d4d4d4',
    punctuation: '#9a9a9a',
    identifier: '#f8f8f8',
    class: '#f0b4f0',
    constant: '#ffa86d'
  } : {
    keyword: '#d73a49',
    type: '#6f42c1',
    string: '#032f62',
    number: '#005cc5',
    comment: '#6a737d',
    function: '#6f42c1',
    operator: '#24292e',
    punctuation: '#24292e',
    identifier: '#24292e',
    class: '#6f42c1',
    constant: '#005cc5'
  }
  
  let result = source
  
  // Comments
  result = result.replace(/(\/\/.*$)/gm, `<span style="color: ${colors.comment}; font-style: italic;">$1</span>`)
  result = result.replace(/(\/\*[\s\S]*?\*\/)/g, `<span style="color: ${colors.comment}; font-style: italic;">$1</span>`)
  
  // Strings
  result = result.replace(/("(?:[^"\\]|\\.)*")/g, `<span style="color: ${colors.string};">$1</span>`)
  result = result.replace(/('(?:[^'\\]|\\.)*')/g, `<span style="color: ${colors.string};">$1</span>`)
  
  // Keywords
  const keywords = ['fn', 'let', 'var', 'const', 'class', 'interface', 'enum', 'extends', 'implements', 
                    'import', 'module', 'public', 'private', 'if', 'else', 'while', 'for', 'return', 
                    'break', 'continue', 'switch', 'case', 'default', 'in', 'new', 'this', 'super', 
                    'as', 'is', 'true', 'false', 'null', 'throw', 'try', 'catch', 'unsafe', 'region',
                    'do', 'with', 'where', 'type', 'Result', 'Ok', 'Err', 'Owned']
  for (const kw of keywords) {
    result = result.replace(new RegExp(`\\b(${kw})\\b`, 'g'), `<span style="color: ${colors.keyword};">$1</span>`)
  }
  
  // Primitive types
  const types = ['void', 'bool', 'byte', 'short', 'int', 'long', 'vast', 'uvast', 'float', 'double', 
                 'half', 'quad', 'char', 'string', 'size', 'u8', 'u16', 'u32', 'u64', 'bool']
  for (const t of types) {
    result = result.replace(new RegExp(`\\b(${t})\\b`, 'g'), `<span style="color: ${colors.type};">$1</span>`)
  }
  
  // Numbers (hex, octal, binary, decimal, float)
  result = result.replace(/\b(0[xX][0-9a-fA-F_]+)\b/g, `<span style="color: ${colors.number};">$1</span>`)
  result = result.replace(/\b(0[oO][0-7_]+)\b/g, `<span style="color: ${colors.number};">$1</span>`)
  result = result.replace(/\b(0[bB][01_]+)\b/g, `<span style="color: ${colors.number};">$1</span>`)
  result = result.replace(/\b([0-9][0-9_]*\.[0-9][0-9_]*[hq]?)\b/g, `<span style="color: ${colors.number};">$1</span>`)
  result = result.replace(/\b([0-9][0-9_]*)\b(?!\s*[a-zA-Z])/g, `<span style="color: ${colors.number};">$1</span>`)
  
  // Class names (PascalCase identifiers)
  result = result.replace(/\b([A-Z][a-zA-Z0-9_]*)\b/g, `<span style="color: ${colors.class};">$1</span>`)
  
  // Operators
  result = result.replace(/([+\-*/%=&|!<>]+)/g, `<span style="color: ${colors.operator};">$1</span>`)
  result = result.replace(/(->|=>|::|\?\s)/g, `<span style="color: ${colors.operator};">$1</span>`)
  
  return result
}

onMounted(() => {
  // Initialize with first example
  code.value = examples[0].code
})
</script>

<template>
  <div class="code-playground" role="region" aria-label="Interactive code playground">
    <!-- Header with example selector -->
    <div class="playground-header">
      <div class="example-selector">
        <label for="example-select">Example:</label>
        <select
          id="example-select"
          v-model="selectedExample"
          @change="selectExample(selectedExample)"
          class="example-dropdown"
          aria-label="Select a code example"
        >
          <option v-for="example in examples" :key="example.id" :value="example.id">
            {{ example.name }}
          </option>
        </select>
      </div>

      <div class="mode-controls">
        <button
          @click="toggleCustomMode"
          :class="['mode-btn', { active: isCustomMode }]"
          title="Toggle custom editing mode"
          aria-label="Toggle custom editing mode"
          :aria-pressed="isCustomMode"
        >
          {{ isCustomMode ? 'Custom Mode' : 'Edit Example' }}
        </button>
        <button
          @click="resetCode"
          class="reset-btn"
          title="Reset to original example"
          aria-label="Reset code to original example"
        >
          Reset
        </button>
        <button
          @click="showOutput = !showOutput"
          :class="['output-toggle', { active: showOutput }]"
          title="Toggle output panel"
          aria-label="Toggle output panel visibility"
          :aria-pressed="showOutput"
          :aria-expanded="showOutput"
        >
          {{ showOutput ? 'Hide Output' : 'Show Output' }}
        </button>
      </div>
    </div>
    
    <!-- Main content area -->
    <div class="playground-content">
      <!-- Code input panel -->
      <div class="code-panel">
        <div class="panel-header">
          <span class="panel-title">Code Input</span>
          <span class="lang-badge">Titrate</span>
        </div>
        
        <!-- Editable textarea -->
        <div class="code-editor">
          <textarea
            v-model="code"
            @input="updateCode($event.target.value)"
            class="code-input"
            spellcheck="false"
            placeholder="Enter Titrate code here..."
            aria-label="Code editor - enter Titrate code"
            aria-describedby="code-editor-description"
          ></textarea>
          
          <!-- Syntax highlighted overlay -->
          <div class="code-highlight" v-html="highlightedCode"></div>
        </div>
        
        <!-- Example description -->
        <div v-if="currentExample && !isCustomMode" class="example-info">
          <p id="code-editor-description" class="example-description">{{ currentExample.name }} example demonstrates basic Titrate syntax.</p>
        </div>
        <div v-else class="example-info">
          <p id="code-editor-description" class="example-description">Edit the code to experiment with Titrate syntax.</p>
        </div>
      </div>
      
      <!-- Output panel -->
      <div v-if="showOutput" class="output-panel" role="region" aria-label="Code output">
        <div class="panel-header">
          <span class="panel-title">Output</span>
          <span class="output-badge">Mock Result</span>
        </div>
        
        <div class="output-content">
          <pre class="output-text">{{ currentOutput }}</pre>
        </div>
        
        <div class="output-note">
          <p>This is a documentation demo. In a real playground, the code would be executed and actual output displayed.</p>
        </div>
      </div>
    </div>
    
    <!-- Footer with tips -->
    <div class="playground-footer">
      <div class="tips">
        <p><strong>Tips:</strong></p>
        <ul>
          <li>Select different examples from the dropdown to explore Titrate features</li>
          <li>Click "Edit Example" to modify the code and experiment</li>
          <li>Use "Reset" to restore the original example code</li>
        </ul>
      </div>
    </div>
  </div>
</template>

<style scoped>
.code-playground {
  border: 1px solid var(--vp-c-divider);
  border-radius: var(--titrate-radius-lg);
  margin: 1.5rem 0;
  background: var(--vp-c-bg-soft);
  overflow: hidden;
  transition: border-color var(--titrate-duration-normal) var(--titrate-ease);
}

.code-playground:hover {
  border-color: var(--vp-c-brand-1);
}

/* Header */
.playground-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.75rem 1rem;
  background: var(--titrate-gradient-hero);
  border-bottom: 1px solid var(--vp-c-divider);
}

.example-selector {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.example-selector label {
  font-weight: 600;
  font-size: 0.85em;
  color: var(--vp-c-text-1);
}

.example-dropdown {
  padding: 0.4rem 0.75rem;
  border-radius: var(--titrate-radius-sm);
  border: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  font-size: 0.85em;
  cursor: pointer;
  transition: all var(--titrate-duration-fast) var(--titrate-ease);
}

.example-dropdown:hover {
  border-color: var(--vp-c-brand-1);
}

.example-dropdown:focus {
  outline: var(--titrate-interactive-focus-ring);
  border-color: var(--titrate-accent-blue);
}

.mode-controls {
  display: flex;
  gap: 0.5rem;
}

.mode-btn,
.reset-btn,
.output-toggle {
  padding: 0.4rem 0.75rem;
  border-radius: var(--titrate-radius-sm);
  border: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg);
  color: var(--vp-c-text-2);
  font-size: 0.8em;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--titrate-duration-fast) var(--titrate-ease);
}

.mode-btn:hover,
.reset-btn:hover,
.output-toggle:hover {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
}

.mode-btn:focus-visible,
.reset-btn:focus-visible,
.output-toggle:focus-visible {
  outline: 2px solid var(--titrate-accent-blue);
  outline-offset: 2px;
  border-color: var(--titrate-accent-blue);
}

.mode-btn.active,
.output-toggle.active {
  background: var(--vp-c-brand-soft);
  color: var(--vp-c-brand-1);
  border-color: var(--vp-c-brand-1);
}

/* Content area */
.playground-content {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
  padding: 1rem;
}

@media (max-width: 768px) {
  .playground-content {
    grid-template-columns: 1fr;
  }
}

/* Panels */
.code-panel,
.output-panel {
  border-radius: var(--titrate-radius-md);
  border: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg);
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: var(--vp-c-brand-soft);
  border-bottom: 1px solid var(--vp-c-divider);
}

.panel-title {
  font-weight: 600;
  font-size: 0.85em;
  color: var(--vp-c-brand-1);
}

.lang-badge,
.output-badge {
  padding: 0.2rem 0.5rem;
  border-radius: var(--titrate-radius-sm);
  font-size: 0.7em;
  font-weight: 600;
  letter-spacing: 0.02em;
}

.lang-badge {
  background: rgba(135, 100, 184, 0.15);
  color: var(--titrate-accent-purple);
}

.output-badge {
  background: rgba(16, 124, 16, 0.15);
  color: var(--titrate-accent-green);
}

/* Code editor */
.code-editor {
  position: relative;
  min-height: 280px;
}

.code-input {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  min-height: 280px;
  padding: 0.75rem;
  border: none;
  background: transparent;
  color: transparent;
  caret-color: var(--vp-c-text-1);
  font-family: var(--vp-font-family-mono);
  font-size: 0.85em;
  line-height: 1.6;
  resize: vertical;
  z-index: 2;
  resize: none;
}

.code-input:focus {
  outline: none;
}

.code-highlight {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  min-height: 280px;
  padding: 0.75rem;
  overflow: auto;
  font-family: var(--vp-font-family-mono);
  font-size: 0.85em;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
  z-index: 1;
  pointer-events: none;
}

/* Example info */
.example-info {
  padding: 0.5rem 0.75rem;
  border-top: 1px solid var(--vp-c-divider);
  background: var(--vp-c-brand-mute);
}

.example-description {
  font-size: 0.8em;
  color: var(--vp-c-text-2);
  margin: 0;
}

/* Output panel */
.output-content {
  padding: 0.75rem;
  min-height: 180px;
  background: var(--vp-c-bg-soft);
}

.output-text {
  font-family: var(--vp-font-family-mono);
  font-size: 0.85em;
  line-height: 1.6;
  color: var(--vp-c-text-1);
  margin: 0;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.output-note {
  padding: 0.5rem 0.75rem;
  border-top: 1px solid var(--vp-c-divider);
  background: var(--titrate-gradient-hero);
}

.output-note p {
  font-size: 0.75em;
  color: var(--vp-c-text-3);
  margin: 0;
}

/* Footer */
.playground-footer {
  padding: 0.75rem 1rem;
  border-top: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg-soft);
}

.tips p {
  margin: 0 0 0.25rem 0;
  font-size: 0.8em;
  color: var(--vp-c-text-2);
}

.tips ul {
  margin: 0;
  padding-left: 1.25em;
  font-size: 0.75em;
  color: var(--vp-c-text-3);
}

.tips li {
  margin: 0.15em 0;
}

/* Dark mode adjustments */
.dark .lang-badge {
  background: rgba(196, 167, 255, 0.18);
  color: #c4a7ff;
}

.dark .output-badge {
  background: rgba(130, 217, 122, 0.18);
  color: #82d97a;
}

.dark .code-highlight {
  color: #f8f8f8;
}

.dark .code-panel,
.dark .output-panel {
  background: var(--titrate-neutral-2);
}

/* Responsive */
@media (max-width: 480px) {
  .playground-header {
    flex-direction: column;
    align-items: stretch;
  }
  
  .example-selector {
    width: 100%;
  }
  
  .example-dropdown {
    width: 100%;
  }
  
  .mode-controls {
    width: 100%;
    flex-wrap: wrap;
  }
  
  .mode-btn,
  .reset-btn,
  .output-toggle {
    flex: 1;
    min-width: 80px;
  }
}
</style>