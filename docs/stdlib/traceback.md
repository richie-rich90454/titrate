# traceback

The `tt.lang` module provides stack trace formatting utilities for debugging and error reporting.

```titrate
import tt.lang.Traceback;
import tt.lang.Frame;
```

## Traceback

Utilities for formatting and extracting stack trace information.

- `Traceback.format(frames: ArrayList<Frame>): string` — format a list of frames into a readable traceback string
- `Traceback.formatFrame(frame: Frame): string` — format a single frame as a readable string
- `Traceback.extract(): ArrayList<Frame>` — extract the current call stack as a list of frames
- `Traceback.formatException(err: string, frames: ArrayList<Frame>): string` — format an exception message with its traceback

## Frame

Represents a single stack frame with source location information.

- `fn init(file: string, line: int, func: string)` — create a frame with file, line number, and function name
- `getFile(): string` — get the source file path
- `getLine(): int` — get the line number
- `getFunc(): string` — get the function name

```titrate
let frames: ArrayList<Frame> = Traceback.extract();
let trace: string = Traceback.format(frames);
io::println(trace);

let frame: Frame = new Frame("main.tr", 42, "processData");
io::println(Traceback.formatFrame(frame));  // "  File \"main.tr\", line 42, in processData"

let errMsg: string = Traceback.formatException("ValueError: invalid input", frames);
io::println(errMsg);
```
