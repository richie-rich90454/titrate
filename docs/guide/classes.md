# Classes

Classes are Titrate's way of bundling data and behavior together. If you've used classes in other languages, you'll feel right at home — but Titrate has its own style. No `static` keyword and constructors are typically called `fn init()`. Once you get the hang of it, it's a clean and consistent way to write object-oriented code.

## Defining a Class

A class bundles fields (data) and methods (behavior) into a single unit. Fields need an access modifier (`public` or `private`) and constructors are defined with `fn init()`:

```titrate
class Circle {
    public double radius;

    public fn init(r: double) {
        this.radius = r;
    }

    public fn area(): double {
        return 3.14159 * this.radius * this.radius;
    }
}
```

A few things to notice:

- **Fields** are declared with an access modifier — `public` or `private`. There's no default; you must be explicit.
- **Constructors** use `fn init()` (or `fn ClassName()`). This is the method the compiler calls when you write `new Circle(5.0)`.
- **`this.`** is the preferred way to access instance fields and methods. This makes it clear when you're working with instance data versus local variables.

### Try It Yourself

Create a `Rectangle` class with `width` and `height` fields, a constructor and an `area()` method. Then use it:

```titrate
class Rectangle {
    public double width;
    public double height;

    public fn init(w: double, h: double) {
        this.width = w;
        this.height = h;
    }

    public fn area(): double {
        return this.width * this.height;
    }
}

public fn main(): void {
    let rect: Rectangle = new Rectangle(3.0, 4.0);
    io::println(Double.toString(rect.area()));  // 12.0
}
```

Try adding a `perimeter()` method that returns `2.0 * (this.width + this.height)`.

## Inheritance

Use `extends` to inherit from a base class. Call the parent constructor with `super()`:

```titrate
class Animal {
    public string name;

    public fn init(name: string) {
        this.name = name;
    }

    public fn speak(): string {
        return "...";
    }
}

class Dog extends Animal {
    public string breed;

    public fn init(name: string, breed: string) {
        super(name);
        this.breed = breed;
    }

    public fn speak(): string {
        return "Woof!";
    }
}
```

The `super()` call must match the parent class constructor signature. Think of it as forwarding arguments up the chain — the parent class gets to set up its fields before the child class does.

::: tip
Always call `super()` as the first thing in your child constructor. The parent needs to be initialized before the child can safely use inherited fields.
:::

## Interfaces

Interfaces define a contract — a set of methods that a type promises to implement. They let you write code that works with *any* type that satisfies the contract, without caring about the specific implementation:

```titrate
interface Drawable {
    fn draw(): void;
}

interface Printable {
    fn toString(): string;
}
```

Notice that interface methods don't have a body — they just declare the signature. Any class that `implements` the interface must provide the implementation.

## Implementing Interfaces

A class declares that it implements one or more interfaces using `implements`:

```titrate
class Report implements Printable {
    public string title;

    public fn toString(): string {
        return this.title;
    }
}
```

A class can implement multiple interfaces, separated by commas:

```titrate
class Shape implements Drawable, Printable {
    public fn draw(): void {
        io::println("drawing shape");
    }

    public fn toString(): string {
        return "Shape";
    }
}
```

::: tip
Interfaces are great for decoupling your code. If a function accepts a `Printable`, it works with *any* class that implements `Printable` — not just the ones you've written today.
:::

## Generic Classes

Classes can declare type parameters in angle brackets. This lets you write a class once and reuse it with different types:

```titrate
class Box<T> {
    public T value;

    public fn init(value: T) {
        this.value = value;
    }

    public fn get(): T {
        return this.value;
    }
}
```

When you create an instance, you provide the concrete type:

```titrate
let intBox: Box<int> = new Box<int>(42);
let strBox: Box<string> = new Box<string>("hello");
```

### Constraint Syntax

Sometimes you need the type parameter to support certain operations. Restrict type parameters to types that implement specific interfaces:

```titrate
class SortedList<T: Comparable<T>> {
    public ArrayList<T> items;

    public fn insert(item: T): void {
        // T is guaranteed to have compareTo
        this.items.add(item);
        this.items.sort();
    }
}
```

Multiple constraints use `+`:

```titrate
class Renderer<T: Display + Printable> {
    public fn render(item: T): string {
        return item.toString();
    }
}
```

See [Generics](./generics) for the full generics guide.

## When to Use Classes

Classes aren't the only way to organize code in Titrate — you also have top-level functions, enums, and interfaces. Here's when classes are the right tool:

- **You have state that needs to be bundled with behavior.** A `Circle` has a `radius` and methods that use it — that's a natural class.
- **You need multiple instances with their own data.** Each `Dog` has its own `name` and `breed`.
- **You want to use inheritance or interfaces.** If you need `extends` or `implements`, you need a class.

When *not* to use classes:

- **Pure utility functions** that don't carry state — use top-level `fn` declarations instead. Titrate doesn't have a `static` keyword, so there's no need to wrap utility functions in a class.
- **Simple data-only containers** — consider whether a tuple or a simple struct-like class is really needed, or if function parameters would suffice.

## Common Patterns

### Factory Functions

Titrate only supports one `fn init()` per class. If you need alternate ways to construct an object, use factory functions — top-level functions that create and return instances:

```titrate
public fn hydrogen(x: double, y: double, z: double): Atom {
    let a: Atom = new Atom("H", 1, 1.008);
    a.setPosition(x, y, z);
    return a;
}
```

### Alternate Init Methods

You can also add a regular method on the class to configure it after construction:

```titrate
public class Regex {
    public string pattern;
    public string flags;

    public fn init(p: string) {
        this.pattern = p;
        this.flags = "";
    }

    public fn initWithFlags(p: string, f: string) {
        this.pattern = p;
        this.flags = f;
    }
}
```

### Operator Overloading in Classes

Classes can define custom behavior for operators using `fn operator<op>` syntax:

```titrate
class Vec2 {
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
```

See [Operator Overloading](./operator-overloading) for the full guide.
