# Enums

## Defining an Enum

```titrate
enum Shape {
    Circle(double),
    Rectangle(double, double),
    Triangle(double, double, double),
}
```

## Pattern Matching

```titrate
switch shape {
    case Circle(r) => io::println("circle");
    case Rectangle(w, h) => io::println("rectangle");
    case Triangle(a, b, c) => io::println("triangle");
}
```
