# Tkinter

The `tt.ui.Tkinter` module mirrors Python's `tkinter` module. It provides a cross-platform canvas-based Tk analog with `Tk`, `Widget`, `Frame`, `Label`, `Button`, `Entry`, and `Canvas` classes plus the `pack`/`grid`/`place` geometry managers. Because Titrate has no native windowing system, widgets maintain logical state and the `Tk` root can serialize the scene to a text or SVG representation via `renderText()` / `renderSvg()`. This makes the module suitable for headless UI testing, snapshot rendering, and as a backend for turtle graphics.

## Import

```titrate
import tt::ui::Tkinter;
```

## Constants — geometry-manager sides

- `TOP: int = 1`
- `BOTTOM: int = 2`
- `LEFT: int = 3`
- `RIGHT: int = 4`
- `BOTH: int = 5`
- `X: int = 6`
- `Y: int = 7`
- `CENTER: int = 8`

## Tk

`Tk` is the root window. It tracks the top-level widget tree, the event-loop state, and provides factory hooks for creating child widgets.

- `Tk.init()`
- `title(t: string): void` / `getTitle(): string`
- `geometry(spec: string): void` — set window size from `"WxH"` (e.g. `"320x240"`)
- `getWidth(): int` / `getHeight(): int`
- `setBg(color: string): void` / `getBg(): string`
- `bind(event: string, callback: fn(): void): void` — register a top-level callback for a symbolic event (e.g. `"<Key-q>"`)
- `unbind(event: string): void`
- `fireEvent(event: string): bool` — synthesize an event; returns `true` if a handler ran
- `mainloop(): void` — run the event loop (processes pending events in headless mode)
- `isRunning(): bool`
- `destroy(): void`
- `addChild(w: Widget): void` / `getChildren(): ArrayList<Widget>`
- `renderText(): string` — render the root and all packed/grided children as a textual scene
- `renderSvg(): string` — render the root as an SVG document

## Widget

`Widget` is the base class for all UI elements.

- `Widget.init(master: Widget)` — registers itself with `master.children`
- `config(options: HashMap<string, string>): void` — set multiple options at once
- `setOption(key: string, value: string): void` / `getOption(key: string): string`
- `bind(event: string, callback: fn(): void): void` / `unbind(event: string): void`
- `fireEvent(event: string): bool`
- `pack(options: HashMap<string, string>): void` — place via the pack geometry manager (recognized: `side`, `fill`, `expand`)
- `packDefault(): void` — pack with `side=top`
- `grid(options: HashMap<string, string>): void` — place via the grid geometry manager (recognized: `row`, `column`, `rowspan`, `columnspan`, `sticky`)
- `place(x: int, y: int, w: int, h: int): void` — place at an explicit position
- `getX(): int` / `getY(): int` / `getWidth(): int` / `getHeight(): int`
- `setVisible(v: bool): void` / `isVisible(): bool`
- `destroy(): void`
- `renderText(depth: int): string` / `renderSvg(depth: int): string`

## Frame

`Frame` is a rectangular container that holds other widgets. Its background color and border can be set via the `bg` and `relief` options.

- `Frame.init(master: Widget)` — defaults to 200x200

## Label

`Label` displays a non-editable text string.

- `Label.init(master: Widget)` — defaults to 80x24
- `setText(t: string): void` / `getText(): string`

## Button

`Button` is a `Label`-like widget that invokes a callback when clicked.

- `Button.init(master: Widget)`
- `command: fn(): void`
- `setCommand(cb: fn(): void): void` — set the click callback
- `invoke(): void` — simulate a click (invokes the command)
- `setText(t: string): void` / `getText(): string`

## Entry

`Entry` is a single-line text input field.

- `Entry.init(master: Widget)` — defaults to 120x24
- `setShow(show: bool): void` — mask contents if `true` (password field)
- `setText(t: string): void` / `getText(): string`
- `insertChar(c: string): void` — append a single character
- `backspace(count: int): void` — delete `count` characters from the end

## Canvas

`Canvas` is a drawing surface holding a list of canvas items. Each item is assigned an integer id when created.

- `Canvas.init(master: Widget)` — defaults to 400x400
- `setBg(color: string): void` / `getBg(): string`
- `createLine(x1: double, y1: double, x2: double, y2: double, options: HashMap<string, string>): int` — returns the item id
- `createRectangle(x1: double, y1: double, x2: double, y2: double, options: HashMap<string, string>): int`
- `createOval(x1: double, y1: double, x2: double, y2: double, options: HashMap<string, string>): int`
- `createText(x: double, y: double, options: HashMap<string, string>): int`
- `createPolygon(coords: ArrayList<double>, options: HashMap<string, string>): int` — `coords` is `[x1, y1, x2, y2, ...]`
- `createArc(x1: double, y1: double, x2: double, y2: double, options: HashMap<string, string>): int`
- `findItem(id: int): CanvasItem` — returns `null` if not found
- `delete(id: int): bool` — returns `true` if removed
- `deleteAll(): void`
- `move(id: int, dx: double, dy: double): void`
- `setCoords(id: int, coords: ArrayList<double>): void`
- `getCoords(id: int): ArrayList<double>`
- `itemConfig(id: int, options: HashMap<string, string>): void`
- `getItems(): ArrayList<CanvasItem>`

## CanvasItem

A single drawable item on a `Canvas`. Stores an ordered list of control points and an options map.

- `CanvasItem.init(id: int, kind: string)` — `kind` is `"line"`, `"rectangle"`, `"oval"`, `"text"`, `"polygon"`, or `"arc"`
- `id: int`
- `kind: string`
- `points: ArrayList<Point>`
- `options: HashMap<string, string>`
- `addPoint(x: double, y: double): void`
- `getOption(key: string): string` / `setOption(key: string, value: string): void`
- `toString(): string` — one-line summary
- `toSvg(ind: string): string` — render as an SVG element

## Example

```titrate
let root: Tk = new Tk();
root.title("Demo");
root.geometry("320x240");
let lbl: Label = new Label(root);
lbl.setText("Hello, world!");
lbl.packDefault();
io::println(root.renderText());
```

## Notes

- All widgets register themselves with `master.children` on construction.
- The pack manager stacks siblings along the chosen side (`top`/`bottom`/`left`/`right`).
- The grid manager places children at `(row * cellH, col * cellW)` with `cellW = 80` and `cellH = 24`.
- The `renderText()` and `renderSvg()` outputs are useful for snapshot tests because there is no native display.
- Canvas items support the `fill`, `outline`, `width`, and `text` options, which translate to SVG attributes when `renderSvg()` is called.
