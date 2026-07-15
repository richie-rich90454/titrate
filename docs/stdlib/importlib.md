# ImportLib

The `tt.lang.ImportLib` module mirrors Python's `importlib` package. It provides dynamic module loading, lookup, and cache invalidation, plus a `resources` helper that surfaces data files packaged with a module. Names may use either dotted (`tt.lang.String`) or `::`-separated (`tt::lang::String`) form.

## Import

```titrate
import tt::lang::ImportLib;
```

## ModuleInfo

`ModuleInfo` describes a loaded or loadable module.

- `ModuleInfo.init(name: string)`
- `name: string` — the resolved module name
- `fileName: string` — path to the source file (empty if not on disk)
- `loader: string` — loader name (`"tt"` by default)
- `isPackage: bool` — `true` when the module is a package directory

## Loader

`Loader` is a placeholder for the module-loading protocol.

- `Loader.init(name: string)`
- `loadModule(): Variant` — returns a `Variant` tagged `"module"` containing the module name
- `isPackage(): bool` — always `false` for the default loader

## Functions

### import_module

Import a module by dotted or `::`-separated name. Returns a `Variant` tagged `"module"` containing the resolved module name. An empty name returns an empty `Variant`.

**Parameters:** `name: string`
**Returns:** `Variant`

```titrate
let m: Variant = import_module("tt::lang::String");
io::println(m.toString());  // tt.lang.String
```

### reload

Reload a previously imported module. The cached info for the module is cleared before re-importing. Returns the freshly imported module.

**Parameters:** `module: Variant`
**Returns:** `Variant`

### find_loader

Find the loader for a module name. Returns `null` when the module file cannot be located on disk.

**Parameters:** `name: string`
**Returns:** `Loader`

```titrate
let l: Loader = find_loader("tt.lang.String");
if (l != null) {
    io::println(l.name);
}
```

### invalidate_caches

Clear the in-memory module cache so subsequent `import_module` / `find_loader` calls re-resolve against the file system.

**Parameters:** none
**Returns:** `void`

```titrate
invalidate_caches();
let m: Variant = import_module("tt.lang.String");
```

### resources

Return the list of data files associated with a package. Maps to `DataFile.list()` on the package's data directory.

**Parameters:** `packageName: string`
**Returns:** `ArrayList<string>`

```titrate
let files: ArrayList<string> = resources("lang");
var i: int = 0;
while (i < files.size()) {
    io::println(files.get(i));
    i = i + 1;
}
```

## Notes

- Module paths are resolved against `lib/tt/`. A name `a.b.c` maps to `lib/tt/a/b/c.tr`.
- Until the VM exposes a dynamic-load native, `import_module` returns a placeholder `Variant` tagged with the resolved module name rather than a real module handle.
- The cache is keyed by the exact name string passed in. `invalidate_caches()` clears the entire map.
