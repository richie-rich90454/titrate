# RunPy

The `tt.lang.RunPy` module mirrors Python's `runpy` module. It executes a module, file, or source string in a fresh global scope and returns the populated globals map. Until the VM exposes a dynamic eval, execution is simulated: the source is read and its top-level declarations are surfaced as the result globals map.

## Import

```titrate
import tt::lang::RunPy;
```

## RunResult

`RunResult` captures the globals produced by running a module, file, or string.

- `RunResult.init()`
- `globals: HashMap<string, Variant>` — populated with the top-level declaration names
- `source: string` — the executed source text
- `fileName: string` — origin file or `<string>` for `run_string`

## Functions

### run_module

Run a module by dotted or `::`-separated name. Reads the module source from `lib/tt/<...>/<name>.tr` and executes it in a fresh scope annotated with `__name__` set to the module name. When the module file is missing, `source` is set to a comment noting the missing module.

**Parameters:** `moduleName: string`
**Returns:** `RunResult`

```titrate
let r: RunResult = run_module("tt::lang::String");
io::println(r.fileName);
io::println(Integer.toString(r.globals.size()));
```

### run_path

Run a file by path. The file's text is read and executed; `__name__` is set to `"__main__"`. When the path does not exist, `source` is set to a comment noting the missing file.

**Parameters:** `path: string`
**Returns:** `RunResult`

```titrate
let r: RunResult = run_path("examples/hello.tr");
io::println(r.fileName);
```

### run_string

Run a source string. The `fileName` field of the result is `"<string>"`.

**Parameters:** `source: string`
**Returns:** `RunResult`

```titrate
let r: RunResult = run_string("let x: int = 42;\npublic fn doubleIt(n: int): int { return n * 2; }");
```

### _run_code

Execute a source string in a fresh global scope and return the populated result. This is the core execution helper used by `run_path` and `run_string`.

**Parameters:** `source: string`, `fileName: string`
**Returns:** `RunResult`

### _run_module_code

Execute a module's source in a fresh global scope annotated with the module name. Used by `run_module`; sets `__name__` to `moduleName`.

**Parameters:** `source: string`, `fileName: string`, `moduleName: string`
**Returns:** `RunResult`

## Globals population

`_populateGlobals` walks the source line by line, looking for the declaration keywords `public`, `private`, `fn`, `class`, `interface`, `enum`, `let`, `var`, and `const`. The first identifier following the keyword is added to `globals` as a `Variant` tagged `"string"` with that name. The `__name__` global is always set (to `"__main__"` for `run_path`/`run_string`/`_run_code`, or to the supplied module name for `run_module`/`_run_module_code`).

## Notes

- The module-path resolver replaces `::` with `.` and joins segments with `/` under `lib/tt/`. A trailing `.tr` is appended.
- Lines that do not start with a declaration keyword are ignored.
- The simulated execution does not actually evaluate the source; only the names of top-level declarations are surfaced.
