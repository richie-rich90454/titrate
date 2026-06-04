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
