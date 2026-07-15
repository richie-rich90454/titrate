# FileCmp

The `tt.io.FileCmp` module provides a Python `filecmp` analog for file and directory comparison: `cmp`, `cmpfiles`, and the `DirCmp` class.

## Import

```titrate
import tt::io::FileCmp;
```

## Functions

### cmp

Compare two files. `shallow=true` compares only `os.stat()` signatures (size); `shallow=false` compares actual contents. Returns `true` if the files are equal.

**Parameters:** `f1: string`, `f2: string`, `shallow: bool`
**Returns:** `bool`

```titrate
if (cmp("a.txt", "b.txt", false)) {
    io::println("files identical");
}
```

### cmpfiles

Compare files in two directories listed by `names`. Returns a triple of `(match, mismatch, errors)` `ArrayList<string>`.

**Parameters:** `aDir: string`, `bDir: string`, `names: ArrayList<string>`
**Returns:** `(ArrayList<string>, ArrayList<string>, ArrayList<string>)`

```titrate
let names = new ArrayList<string>();
names.add("a.txt"); names.add("b.txt");
let (match, mismatch, errors) = cmpfiles("dirA", "dirB", names);
```

## Class: DirCmp

Compare two directories, classifying entries into `leftOnly`, `rightOnly`, `commonDirs`, `commonFiles`, and `commonFunny`.

**Fields:**
- `left: string`, `right: string`
- `leftOnly: ArrayList<string>`
- `rightOnly: ArrayList<string>`
- `commonDirs: ArrayList<string>`
- `commonFiles: ArrayList<string>`
- `commonFunny: ArrayList<string>`
- `sameFiles: HashMap<string, bool>`
- `diffFiles: HashMap<string, bool>`
- `funnyFiles: HashMap<string, bool>`
- `subdirs: DirCmp`

**Methods:**
- `init(left: string, right: string)` — runs phase 1 (classify) and phase 2 (compare common files)
- `fullCompare(): ArrayList<DirCmp>` — recursively compare common subdirectories
- `report(): void` — print the comparison to stdout

```titrate
let d: DirCmp = new DirCmp("dirA", "dirB");
d.report();
io::println(d.leftOnly);
io::println(d.diffFiles.keys());
```
