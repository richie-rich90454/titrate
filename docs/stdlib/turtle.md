# Turtle

The `tt.ui.Turtle` module mirrors Python's `turtle` module. It provides turtle graphics via the cross-platform `Canvas` from `tt::ui::Tkinter`. Movement commands (`forward`, `backward`, `left`, `right`, `goto`) update the turtle's state and draw on the parent `Screen`'s underlying `Canvas`. Because Titrate has no native display, the `Screen` exposes the `Canvas` so callers can `renderText()` / `renderSvg()` the result.

## Import

```titrate
import tt::ui::Turtle;
```

## Constants — default shape names

- `SHAPE_CLASSIC: string = "classic"`
- `SHAPE_ARROW: string = "arrow"`
- `SHAPE_TURTLE: string = "turtle"`
- `SHAPE_CIRCLE: string = "circle"`
- `SHAPE_SQUARE: string = "square"`
- `SHAPE_TRIANGLE: string = "triangle"`

## Default-singleton functions

The module keeps a default `Screen` and `Turtle` so the top-level functions can be used without explicit setup, mirroring Python's `turtle` module.

### getscreen

Return the default `Screen`, creating it if necessary.

**Parameters:** none
**Returns:** `Screen`

### getturtle

Return the default `Turtle`, creating a `Screen` + `Turtle` if necessary.

**Parameters:** none
**Returns:** `Turtle`

## Screen

`Screen` wraps a `Tk` root with a single `Canvas` that all turtles draw on. The coordinate system has `(0, 0)` at the center of the screen, x increasing to the right, and y increasing upward (standard turtle orientation).

- `Screen.init()`
- `setup(width: int, height: int): void`
- `getTitle(): string` / `setTitle(t: string): void`
- `bgcolor(color: string): void` / `getBgcolor(): string`
- `getCanvas(): Canvas`
- `toCanvasX(x: double): double` / `toCanvasY(y: double): double` — convert turtle coords to canvas coords
- `mainloop(): void` — run the Tk event loop (no-op in headless mode)
- `bye(): void` — close the screen and release all turtles
- `clear(): void` — clear all drawings from the canvas
- `renderSvg(): string` — render the screen as an SVG document
- `renderText(): string` — render the screen as a textual scene description

## Turtle

`Turtle` is the on-screen cursor. It tracks position, heading, pen state, colors, and shape. Movement commands draw on the parent `Screen`'s `Canvas`.

- `Turtle.init(screen: Screen)` — registers itself with `screen`
- `_x: double` / `_y: double` — position in turtle space
- `_heading: double` — degrees, `0` = east, `90` = north
- `_penDown: bool` / `_pencolor: string` / `_fillcolor: string` / `_pensize: int` / `_speed: int` / `_shape: string` / `_visible: bool`

### Movement

- `forward(distance: double): void` — move forward `distance` pixels in the current heading
- `backward(distance: double): void`
- `right(angle: double): void` — turn right (clockwise) by `angle` degrees
- `left(angle: double): void` — turn left (counter-clockwise) by `angle` degrees
- `goto(x: double, y: double): void` — move directly to `(x, y)`, drawing a line if the pen is down
- `setpos(x: double, y: double): void` — alias for `goto`
- `setx(x: double): void` / `sety(y: double): void`
- `setheading(angle: double): void`
- `heading(): double` / `xcor(): double` / `ycor(): double`
- `position(): ArrayList<double>` — `[x, y]`
- `home(): void` — move to `(0, 0)` with heading 0

### Pen control

- `penup(): void` / `pendown(): void` / `isdown(): bool`
- `pensize(size: int): void` / `getPensize(): int`
- `speed(s: int): void` / `getSpeed(): int`
- `color(c: string): void` — set both pen and fill color
- `pencolor(c: string): void` / `getPencolor(): string`
- `fillcolor(c: string): void` / `getFillcolor(): string`

### Filling

- `begin_fill(): void` — start recording points for a filled polygon
- `end_fill(): void` — stop recording and emit the accumulated polygon, filled with the current fill color

### Drawing primitives

- `circle(radius: double, extent: double): void` — draw a circle (or arc) with the given `radius` and `extent` (degrees, default 360 = full circle)
- `dot(size: int, color: string): void` — draw a filled dot of the given diameter at the turtle's position
- `stamp(): int` — stamp a copy of the turtle's shape at the current position; returns the canvas item id
- `write(text: string, options: HashMap<string, string>): void` — write `text` at the turtle's position

### Appearance

- `shape(name: string): void` / `getShape(): string`
- `showturtle(): void` / `hideturtle(): void` / `isvisible(): bool`

### State

- `clear(): void` — clear all drawings from the canvas
- `reset(): void` — reset the turtle to its initial state (origin, heading 0, pen down)
- `distance(x: double, y: double): double` — Euclidean distance from the turtle to `(x, y)`
- `getScreen(): Screen`

## Top-level functions (operate on the default turtle)

The following thin wrappers operate on the default turtle returned by `getturtle()`: `forward`, `backward`, `right`, `left`, `penup`, `pendown`, `goto`, `setpos`, `setx`, `sety`, `setheading`, `home`, `circle`, `dot`, `stamp`, `write`, `color`, `pencolor`, `fillcolor`, `pensize`, `speed`, `shape`, `showturtle`, `hideturtle`.

## Example

```titrate
let s: Screen = getscreen();
s.setup(400, 300);
let t: Turtle = getturtle();
t.color("blue");
t.begin_fill();
let i: int = 0;
while (i < 4) {
    t.forward(100.0);
    t.left(90.0);
    i = i + 1;
}
t.end_fill();
io::println(s.renderSvg());
```

## Notes

- The turtle's coordinate system has `(0, 0)` at the center of the screen and y increasing upward; this is converted to canvas coordinates (origin top-left, y down) before drawing.
- Heading is normalized to `[0, 360)` after each turn.
- A `circle` with positive `radius` draws to the left of the turtle; negative `radius` draws to the right.
- `end_fill()` only emits a polygon if at least three vertices have been accumulated.
- `stamp()` approximates the turtle as a small triangle pointing along the heading.
