# dataclass

The `tt.dataclass` module provides Python-style data classes with auto-generated `toString`, `equals`, `hashCode`, and `copyWith` methods.

```titrate
import tt.dataclass.Dataclass;
import tt.dataclass.DataclassInfo;
```

## DataclassInfo

A descriptor that holds the structure of a dataclass.

- `name: String` — the dataclass name
- `fields: ArrayList<String>` — ordered field names
- `defaults: ArrayList<Object>` — default values for each field

## Dataclass

All methods are static.

- `create(name: String, fields: ArrayList<String>, defaults: ArrayList<Object>): Object` — create a `DataclassInfo` descriptor
- `newInstance(dataclass: Object, values: ArrayList<Object>): Object` — instantiate a dataclass from its descriptor and positional values; missing values fall back to defaults, then null
- `toString(instance: Object, className: String, fields: ArrayList<String>): String` — auto-generated `ClassName(field=value, ...)` string
- `equals(a: Object, b: Object, fields: ArrayList<String>): bool` — structural equality over the given fields
- `hashCode(instance: Object, fields: ArrayList<String>): int` — hash code computed from field values (seed 17, multiplier 31)
- `copyWith(instance: Object, fields: ArrayList<String>, overrides: HashMap<String, Object>): Object` — shallow copy with specified field overrides

```titrate
let fields = new ArrayList<String>();
fields.add("x"); fields.add("y");
let defaults = new ArrayList<Object>();
defaults.add(0); defaults.add(0);

let Point = Dataclass::create("Point", fields, defaults);

let vals = new ArrayList<Object>();
vals.add(3); vals.add(4);
let p = Dataclass::newInstance(Point, vals);

io::println(Dataclass::toString(p, "Point", fields));  // Point(x=3, y=4)

let overrides = new HashMap<string, Object>();
overrides.put("x", 10);
let p2 = Dataclass::copyWith(p, fields, overrides);
io::println(Dataclass::toString(p2, "Point", fields)); // Point(x=10, y=4)
```
