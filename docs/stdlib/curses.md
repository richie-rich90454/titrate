# Curses

The `tt.ui.Curses` module mirrors Python's `curses` module. It provides a terminal UI abstraction with `initscr`/`endwin`, a `Window` class with `addstr`/`addch`/`refresh`/`getch`, color support, and input-key constants. Because Titrate has no native terminal driver, the screen is modeled as an in-memory character grid that can be serialized via `render()`. Input is simulated via an input queue that tests can populate with `pushKey()`.

## Import

```titrate
import tt::ui::Curses;
```

## Color constants

- `COLOR_BLACK: int = 0`
- `COLOR_RED: int = 1`
- `COLOR_GREEN: int = 2`
- `COLOR_YELLOW: int = 3`
- `COLOR_BLUE: int = 4`
- `COLOR_MAGENTA: int = 5`
- `COLOR_CYAN: int = 6`
- `COLOR_WHITE: int = 7`

## Attribute constants

- `A_NORMAL: int = 0`
- `A_BOLD: int = 1`
- `A_UNDERLINE: int = 2`
- `A_REVERSE: int = 4`
- `A_BLINK: int = 8`
- `A_DIM: int = 16`

## Key constants

- `KEY_DOWN: int = 258`
- `KEY_UP: int = 259`
- `KEY_LEFT: int = 260`
- `KEY_RIGHT: int = 261`
- `KEY_HOME: int = 262`
- `KEY_BACKSPACE: int = 263`
- `KEY_DC: int = 330` — delete character
- `KEY_PPAGE: int = 339` — page up
- `KEY_NPAGE: int = 338` — page down
- `KEY_END: int = 360`
- `KEY_ENTER: int = 343`

## Module functions

### initscr

Initialize the curses library. Creates a default `stdscr` Window of 24x80 and returns it. Resets all module state.

**Parameters:** none
**Returns:** `Window`

### endwin

De-initialize the curses library. Clears the `stdscr` reference.

**Parameters:** none
**Returns:** `void`

### cbreak

Enter cbreak mode: characters are available to the program immediately without waiting for a newline.

**Parameters:** none
**Returns:** `void`

### nocbreak

Exit cbreak mode.

**Parameters:** none
**Returns:** `void`

### noecho

Turn off echo: typed characters are not displayed.

**Parameters:** none
**Returns:** `void`

### echo

Turn on echo.

**Parameters:** none
**Returns:** `void`

### start_color

Initialize the color subsystem. Must be called before `init_pair`.

**Parameters:** none
**Returns:** `void`

### init_pair

Define a color pair with the given foreground and background colors.

**Parameters:** `pair: int`, `fg: int`, `bg: int`
**Returns:** `void`

```titrate
start_color();
init_pair(1, COLOR_RED(), COLOR_BLACK());
```

### color_pair

Return the attribute value that selects color pair `pair`. The returned value is OR-ed into the `attr` argument of `addstr`/`addch`.

**Parameters:** `pair: int`
**Returns:** `int`

### pair_content

Look up the foreground/background of a color pair. Returns `[fg, bg]`, or `[-1, -1]` if the pair is undefined.

**Parameters:** `pair: int`
**Returns:** `ArrayList<int>`

### newwin

Create a new `Window` of the given dimensions at the given position.

**Parameters:** `nlines: int`, `ncols: int`, `beginY: int`, `beginX: int`
**Returns:** `Window`

### Other module functions

- `getStdscr(): Window` — return `stdscr`, or `null` if `initscr` has not been called
- `isCbreak(): bool`
- `isEcho(): bool`
- `hasColors(): bool`

## Window class

`Window` represents a rectangular region of the terminal screen. It holds a grid of cells (each a character plus attributes) and a cursor position.

- `Window.init(rows: int, cols: int, beginY: int, beginX: int)`
- `addstr(y: int, x: int, s: string, attr: int): void` — write `s` at `(y, x)` with attributes
- `addstrCursor(s: string, attr: int): void` — write at the current cursor position
- `addch(y: int, x: int, ch: string, attr: int): void` — write a single character
- `addchCursor(ch: string, attr: int): void`
- `move(y: int, x: int): void` — move the cursor to `(y, x)`
- `keypad(flag: bool): void` — enable/disable keypad mode (function/arrow keys translated to `KEY_*` values)
- `nodelay(flag: bool): void` — enable/disable non-blocking input
- `pushKey(key: int): void` — push a key code onto the input queue (for tests)
- `getch(): int` — get the next key; returns `-1` if the queue is empty
- `getkey(): string` — get the next key as a string; returns `""` if no input
- `refresh(): void` — serialize the grid to `_lastRender`
- `getRender(): string` — return the last rendered output
- `render(): string` — serialize the grid to a string, one line per row
- `clear(): void` / `erase(): void` — reset all cells to spaces with default attributes
- `box(vertCh: string, horCh: string): void` — draw a border
- `border(ls: string, rs: string, ts: string, bs: string, tl: string, tr: string, bl: string, br: string): void` — draw a custom border
- `hline(y: int, x: int, ch: string, n: int): void` — horizontal line
- `vline(y: int, x: int, ch: string, n: int): void` — vertical line
- `getCursorY(): int` / `getCursorX(): int`
- `getMaxY(): int` / `getMaxX(): int`

## Example

```titrate
let stdscr: Window = initscr();
cbreak();
noecho();
start_color();
init_pair(1, COLOR_RED(), COLOR_BLACK());
stdscr.addstr(0, 0, "Hello, curses!", color_pair(1) | A_BOLD());
stdscr.refresh();
io::println(stdscr.getRender());
endwin();
```

## Notes

- All state is in-memory; there is no real terminal driver, so `refresh()` only updates `_lastRender` rather than flushing to a terminal.
- Input is simulated: tests push keys with `pushKey()` and `getch()` pops them from the queue.
- When nodelay mode is on, `getch()` returns `-1` if no input is available; otherwise it blocks (also returns `-1` since there is no real terminal to block on).
- The default `stdscr` is 24 rows by 80 columns.
