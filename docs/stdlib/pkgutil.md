# PkgUtil

The `tt.lang.PkgUtil` module mirrors Python's `pkgutil` module. It walks the `lib/tt` package tree, enumerates modules and sub-packages, returns importers for a path, and reads packaged data files.

## Import

```titrate
import tt::lang::PkgUtil;
```

## ModuleInfo

`ModuleInfo` describes a discovered module or package.

- `ModuleInfo.init(name: string)`
- `name: string` — base name (no prefix)
- `isPackage: bool` — `true` when the entry is a directory
- `fileName: string` — full path on disk

## Importer

`Importer` is a placeholder for the path-based import protocol.

- `Importer.init(path: string)`
- `findModule(name: string): string` — returns `"<path>/<name>.tr"`

## Functions

### iter_modules

Iterate the modules and packages directly under `path`. Sub-directories become packages; `.tr` files become modules. Returns an empty list when `path` is not a directory.

**Parameters:** `path: string`
**Returns:** `ArrayList<ModuleInfo>`

```titrate
let mods: ArrayList<ModuleInfo> = iter_modules("lib/tt/lang");
var i: int = 0;
while (i < mods.size()) {
    io::println(mods.get(i).name);
    i = i + 1;
}
```

### walk_packages

Recursively walk packages under `path`, yielding every module found. Each entry's `name` is the fully-qualified dotted name (e.g. `lang.String`).

**Parameters:** `path: string`
**Returns:** `ArrayList<ModuleInfo>`

```titrate
let all: ArrayList<ModuleInfo> = walk_packages("lib/tt/lang");
var i: int = 0;
while (i < all.size()) {
    io::println(all.get(i).name + (all.get(i).isPackage ? "/" : ""));
    i = i + 1;
}
```

### get_importer

Return the importer for `path` (a directory or zip file). Currently returns a plain `Importer` that joins the path with `<name>.tr`.

**Parameters:** `path: string`
**Returns:** `Importer`

### extend_path

Extend the search path with additional directories listed in a `__pkg__.tr` file at `path/__pkg__.tr`. Returns the resulting list of directory strings (always including the original `path`).

**Parameters:** `path: string`
**Returns:** `ArrayList<string>`

```titrate
let dirs: ArrayList<string> = extend_path("lib/tt/lang");
```

### read_code

Read a compiled-code marker from a stream (a file path). Returns the raw text content of the file, or `""` if the file does not exist.

**Parameters:** `stream: string`
**Returns:** `string`

### get_data

Return the data bytes for `package`/`resource`. Reads the file at `lib/tt/data/<package>/<resource>`. Returns `""` when the file is missing.

**Parameters:** `package: string`, `resource: string`
**Returns:** `string`

```titrate
let txt: string = get_data("lang", "keywords.txt");
io::println(txt);
```

## Notes

- `iter_modules` lists only direct children; use `walk_packages` for recursive enumeration.
- A directory entry is treated as a package; a file ending in `.tr` is treated as a module (the `.tr` suffix is stripped from the module name).
- `.` and `..` entries are always skipped.
- `extend_path` parses each non-empty line of `__pkg__.tr` as an additional search directory.
