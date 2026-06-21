# Interfaces

Interfaces define contracts — sets of method signatures that a type promises to implement. They let you write code that works with *any* type satisfying the contract, without caring about the specific implementation. If you've used interfaces in Java, TypeScript, or Go, you'll recognize the concept. Titrate's version is simple, explicit, and pairs naturally with [generics](./generics) for powerful abstractions.

## What Are Interfaces?

An interface is a blueprint of behavior. It says "any type that implements me must provide these methods," but it doesn't say *how* those methods work. This separation of "what" from "how" is the heart of interface-based design:

- **Decoupling** — Functions depend on interfaces, not concrete types. Swap implementations without changing callers.
- **Polymorphism** — One function works with many types, as long as they satisfy the interface.
- **Testability** — Pass mock implementations in tests instead of real dependencies.

## Defining Interfaces

Use the `interface` keyword followed by a name and a body of method signatures:

```titrate
interface Printable {
    fn toString(): string;
}
```

Method signatures in interfaces have no body — they just declare the name, parameters, and return type. Parameters use the same `name: Type` order as everywhere else in Titrate:

```titrate
interface Drawable {
    fn draw(): void;
}

interface Measurable {
    fn measure(): double;
}

interface Serializable {
    fn serialize(): string;
    fn deserialize(data: string): void;
}
```

An interface can declare multiple methods. Any class that `implements` the interface must provide implementations for *all* of them.

## Implementing Interfaces

A class declares that it implements an interface using the `implements` keyword. It must then provide a concrete implementation for every method the interface defines:

```titrate
interface Printable {
    fn toString(): string;
}

class Report implements Printable {
    public string title;
    public string body;

    public fn init(title: string, body: string) {
        this.title = title;
        this.body = body;
    }

    public fn toString(): string {
        return this.title + ": " + this.body;
    }
}
```

If a class claims to implement an interface but is missing a method, the compiler will raise an error. This guarantee is what makes interfaces useful — you can trust that any `Printable` actually has a `toString()` method.

### Implementing with Existing Methods

Sometimes a class already has a method that satisfies an interface. You can simply declare `implements` and the compiler checks that the signatures match:

```titrate
interface Comparable<T> {
    fn compareTo(other: T): int;
}

class Score implements Comparable<Score> {
    public int value;

    public fn init(value: int) {
        this.value = value;
    }

    public fn compareTo(other: Score): int {
        return this.value - other.value;
    }
}
```

## Multiple Interface Implementation

A class can implement more than one interface, separated by commas. This is how you compose behaviors from multiple contracts:

```titrate
interface Printable {
    fn toString(): string;
}

interface Drawable {
    fn draw(): void;
}

class Shape implements Printable, Drawable {
    public string kind;
    public double size;

    public fn init(kind: string, size: double) {
        this.kind = kind;
        this.size = size;
    }

    public fn toString(): string {
        return this.kind + " (" + Double.toString(this.size) + ")";
    }

    public fn draw(): void {
        io::println("Drawing " + this.toString());
    }
}
```

The class must implement every method from every interface it declares. If `Shape` implements both `Printable` and `Drawable`, it needs both `toString()` and `draw()`.

## Interface Inheritance with `extends`

Interfaces can inherit from other interfaces using `extends`. This lets you build hierarchies of contracts — a sub-interface adds more method requirements on top of its parent:

```titrate
interface Readable {
    fn read(): string;
}

interface Writable extends Readable {
    fn write(data: string): void;
}
```

Any class implementing `Writable` must provide both `write()` and `read()` — it inherits the `read()` requirement from `Readable`.

An interface can extend multiple parent interfaces:

```titrate
interface Serializable {
    fn serialize(): string;
}

interface Deserializable {
    fn deserialize(data: string): void;
}

interface Persistent extends Serializable, Deserializable {
    fn save(): void;
}
```

A class implementing `Persistent` must provide `serialize()`, `deserialize()`, and `save()`.

## Generic Interfaces

Interfaces become even more powerful with type parameters. The classic example is `Comparable<T>` — it defines a comparison contract that works for any type:

```titrate
interface Comparable<T> {
    fn compareTo(other: T): int;
}
```

The type parameter `T` represents the type being compared to. When a class implements `Comparable<Score>`, the `compareTo` method takes a `Score` — not a generic `object`:

```titrate
class Score implements Comparable<Score> {
    public int value;

    public fn init(value: int) {
        this.value = value;
    }

    public fn compareTo(other: Score): int {
        return this.value - other.value;
    }
}
```

Another common generic interface is a container that can be iterated:

```titrate
interface Iterable<T> {
    fn iterator(): Iterator<T>;
}

interface Iterator<T> {
    fn hasNext(): bool;
    fn next(): T;
}
```

Generic interfaces are used throughout the standard library — `ArrayList<E>` implements `Iterable<E>`, `HashMap<K, V>` implements `Iterable<KeyValuePair<K, V>>`, and so on.

## Using Interfaces as Types

One of the main reasons to define interfaces is to use them as types in function signatures, variables, and return types. When you use an interface as a type, you can pass *any* implementing class:

```titrate
public fn printAll(items: ArrayList<Printable>): void {
    for (item in items) {
        io::println(item.toString());
    }
}
```

This function doesn't care whether `items` contains `Report` objects, `Shape` objects, or any other class that implements `Printable`. It only calls `toString()`, which the interface guarantees exists.

```titrate
let reports: ArrayList<Printable> = new ArrayList<Printable>();
reports.add(new Report("Q1", "Revenue up 12%"));
reports.add(new Report("Q2", "Revenue flat"));

printAll(reports);  // works because Report implements Printable
```

### Interface-Typed Variables

You can declare variables with an interface type and assign any implementing instance:

```titrate
let p: Printable = new Report("Title", "Body");
io::println(p.toString());  // calls Report's toString
```

The variable `p` has the static type `Printable`, so you can only call methods defined by the interface. To access class-specific methods, you'd need to cast:

```titrate
if (p is Report) {
    let r: Report = p as Report;
    io::println(r.title);
}
```

## Interface vs Class: When to Use Which

| Use an interface when | Use a class when |
|---|---|
| You need to define a contract without implementation | You need to bundle state + behavior |
| Multiple unrelated types should share behavior | You need constructors and fields |
| You want to accept any implementing type in a function | You need inheritance (`extends`) |
| You're designing a generic abstraction (e.g., `Comparable<T>`) | You need operator overloading |
| You want to decouple callers from implementations | You need a single concrete implementation |

A good rule of thumb: **define an interface when you have multiple implementations, or when you want to decouple code from a specific implementation.** If there's only ever going to be one class, a class alone is simpler.

## Common Interface Patterns

### Comparable

The `Comparable<T>` interface is the standard way to make types orderable:

```titrate
interface Comparable<T> {
    fn compareTo(other: T): int;
}

class Student implements Comparable<Student> {
    public string name;
    public int grade;

    public fn init(name: string, grade: int) {
        this.name = name;
        this.grade = grade;
    }

    public fn compareTo(other: Student): int {
        return other.grade - this.grade;  // descending by grade
    }
}
```

### Printable / Display

A `Printable` or `Display` interface standardizes how objects are rendered as strings:

```titrate
interface Printable {
    fn toString(): string;
}

class Money implements Printable {
    public double amount;
    public string currency;

    public fn init(amount: double, currency: string) {
        this.amount = amount;
        this.currency = currency;
    }

    public fn toString(): string {
        return this.currency + " " + Double.toString(this.amount);
    }
}
```

### Serializable

A `Serializable` interface defines how objects convert to and from a persistent format:

```titrate
interface Serializable {
    fn serialize(): string;
    fn deserialize(data: string): void;
}

class Config implements Serializable {
    public string host;
    public int port;

    public fn init(host: string, port: int) {
        this.host = host;
        this.port = port;
    }

    public fn serialize(): string {
        return this.host + ":" + Integer.toString(this.port);
    }

    public fn deserialize(data: string): void {
        let parts: ArrayList<string> = String.split(data, ":");
        this.host = parts.get(0);
        this.port = Integer.parseInt(parts.get(1));
    }
}
```

### Observer / Event Listener

Interfaces are ideal for callback patterns where the caller doesn't need to know the concrete type:

```titrate
interface OnClickListener {
    fn onClick(): void;
}

class Button {
    private OnClickListener listener;

    public fn setOnClickListener(listener: OnClickListener): void {
        this.listener = listener;
    }

    public fn click(): void {
        if (this.listener != null) {
            this.listener.onClick();
        }
    }
}
```

## Interfaces and Generics Together

The real power of interfaces emerges when combined with generics. You can write functions that accept *any* type implementing an interface:

```titrate
public fn findMax<T: Comparable<T>>(items: ArrayList<T>): T {
    let max: T = items.get(0);
    var i: int = 1;
    while (i < items.size()) {
        if (items.get(i).compareTo(max) > 0) {
            max = items.get(i);
        }
        i = i + 1;
    }
    return max;
}
```

This function works for `Student`, `Score`, or any type that implements `Comparable<T>`. The constraint `<T: Comparable<T>>` ensures the compiler that `compareTo` is available.

## Try It Yourself

1. **Define a `Sortable` interface** with a method `sortKey(): string`. Then create two classes that implement it — say, `Book` (sort by title) and `Employee` (sort by name). Write a function that sorts a list of `Sortable` items by their sort key.

2. **Create a `Validator<T>` interface** with a method `validate(value: T): bool`. Implement it for `string` (non-empty) and `int` (positive). Write a generic function `check<T>(v: Validator<T>, value: T): Result<T, string>` that returns `ok(value)` if valid, or `err("validation failed")` if not.

3. **Build a plugin system** — define a `Plugin` interface with `name(): string` and `run(): void`. Create three plugins that implement it. Write a `PluginManager` class that holds an `ArrayList<Plugin>` and has a `runAll()` method.

::: tip
Start simple — define the interface first, then one implementation, then a function that uses the interface type. Once that works, adding more implementations is easy.
:::

## What's Next?

- [Generics](./generics) — type parameters and constraints
- [Classes](./classes) — defining classes with fields and methods
- [Error Handling](./error-handling) — `Result` types and the `?` operator
