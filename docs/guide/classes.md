# Classes

## Defining a Class

```titrate
class Circle {
    double radius;

    public Circle(double r) {
        super("Circle");
        this.radius = r;
    }

    public fn area(): double {
        return 3.14159 * this.radius * this.radius;
    }
}
```

## Inheritance

```titrate
class ColoredCircle extends Circle {
    string color;
}
```

## Interfaces

```titrate
interface Drawable {
    fn draw(): void;
}
```

## Generic Classes

Classes can declare type parameters in angle brackets:

```titrate
class Box<T> {
    T value;

    public Box(T value) {
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
    ArrayList<T> items;

    public fn insert(item: T): void {
        // T is guaranteed to have compareTo
        this.items.add(item);
        this.items.sort();
    }
}
```

Multiple constraints use `+`:

```titrate
class Renderer<T: Display + Serializable> {
    public fn render(item: T): string {
        return item.toString();
    }
}
```

See [Generics](./generics) for the full generics guide.
