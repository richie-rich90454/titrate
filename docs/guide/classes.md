# Classes

## Defining a Class

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

The `super()` call must match the parent class constructor signature.

## Interfaces

Define interfaces with method signatures:

```titrate
interface Drawable {
    fn draw(): void;
}

interface Printable {
    fn toString(): string;
}
```

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

A class can implement multiple interfaces:

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

## Generic Classes

Classes can declare type parameters in angle brackets:

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

### Constraint Syntax

Restrict type parameters to types that implement specific interfaces:

```titrate
class SortedList<T: Comparable> {
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
