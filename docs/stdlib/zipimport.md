# ZipImport

The `tt.lang.ZipImport` module mirrors Python's `zipimport` module. It loads modules from a single ZIP archive (a `.zip` or `.tr.zip` file) using the `zipimporter` class plus module-level convenience functions that search across multiple archives.

## Import

```titrate
import tt::lang::ZipImport;
import tt::compression::ZipFile;
```

## ModuleSpec

`ModuleSpec` describes a resolved module within a zip archive.

- `ModuleSpec.init(name: string, archive: string)`
- `name: string` — fully-qualified module name
- `archive: string` — path to the zip file
- `entry: string` — entry path within the archive
- `isPackage: bool` — `true` when the entry ends with `/` (a `__init__.tr`)
- `loader: zipimporter` — the importer that produced the spec (or `null`)

## zipimporter

`zipimporter` is bound to a single ZIP archive and resolves module names against the entries it contains.

- `zipimporter.init(path: string)` — opens the archive at `path`; sets `valid = true` when the file exists
- `archive: string` — the bound archive path
- `prefix: string` — sub-directory prefix within the archive (default `""`)
- `zip: ZipFile` — the underlying `ZipFile` instance (or unset when invalid)
- `valid: bool` — `true` when the archive was opened successfully

### find_module

Find a module by name. Returns the matching entry path within the archive, or `""` if not found. A module `a.b` is mapped to candidate entries `a/b.tr` and `a/b/__init__.tr` (prefixed with `this.prefix` when set).

**Parameters:** `fullname: string`
**Returns:** `string`

### find_spec

Find a module spec by name. Returns a `ModuleSpec` or `null` if not found.

**Parameters:** `fullname: string`
**Returns:** `ModuleSpec`

```titrate
let zi: zipimporter = new zipimporter("modules.zip");
let spec: ModuleSpec = zi.find_spec("mypkg.utils");
if (spec != null) {
    io::println(spec.entry + " in " + spec.archive);
}
```

### load_module

Load and return a module by name. The module source is read from the archive and returned as a `Variant` tagged `"module"` containing the source string. Returns an empty `Variant` when the module or archive is unavailable.

**Parameters:** `fullname: string`
**Returns:** `Variant`

### is_valid

Return `true` when the archive was opened successfully.

**Parameters:** none
**Returns:** `bool`

### get_source

Return the source text for a module, or `""` if not available.

**Parameters:** `fullname: string`
**Returns:** `string`

## Module-level functions

### find_module

Search `archives` for `fullname`. Returns the path of the first archive that contains the module, or `""` if none does.

**Parameters:** `fullname: string`, `archives: ArrayList<string>`
**Returns:** `string`

### load_module

Search `archives` for `fullname` and return the loaded module from the first archive that contains it. Returns an empty `Variant` when no archive matches.

**Parameters:** `fullname: string`, `archives: ArrayList<string>`
**Returns:** `Variant`

```titrate
let archives: ArrayList<string> = new ArrayList<string>();
archives.add("stdlib.zip");
archives.add("vendor.zip");
let mod: Variant = load_module("helpers.strings", archives);
```

### find_spec

Search `archives` for `fullname` and return the first matching `ModuleSpec`, or `null` if none matches.

**Parameters:** `fullname: string`, `archives: ArrayList<string>`
**Returns:** `ModuleSpec`

## Notes

- A module `a.b.c` is mapped to entries `a/b/c.tr` and `a/b/c/__init__.tr`.
- `::`-separated names are normalized to dotted form before entry resolution.
- The `prefix` field allows a `zipimporter` to look only inside a sub-directory of an archive.
