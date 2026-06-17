# dataclass

The `tt.dataclass` module provides Python-style data classes with auto-generated `toString`, `equals`, and `copyWith` methods.

```titrate
import tt.dataclass.Dataclass;
import tt.dataclass.DataclassInfo;
```

## DataclassInfo

A descriptor that holds the structure of a dataclass.

- `name: string` — the dataclass name
- `fields: ArrayList<string>` — ordered field names
- `defaults: ArrayList<Variant>` — default values for each field

## Dataclass

All methods are static.

- `create(name: string, fields: ArrayList<string>, defaults: ArrayList<Variant>): DataclassInfo` — create a `DataclassInfo` descriptor
- `newInstance(dataclass: DataclassInfo, values: ArrayList<Variant>): Variant` — instantiate a dataclass from its descriptor and positional values; missing values fall back to defaults, then null
- `toString(instance: Variant, className: string, fields: ArrayList<string>): string` — auto-generated `ClassName(field=value, ...)` string
- `equals(a: Variant, b: Variant, fields: ArrayList<string>): bool` — structural equality over the given fields
- `copyWith(instance: Variant, fields: ArrayList<string>, overrides: HashMap<string, Variant>): Variant` — shallow copy with specified field overrides

```titrate
let fields = new ArrayList<string>();
fields.add("x"); fields.add("y");
let defaults = new ArrayList<Variant>();
defaults.add(0); defaults.add(0);

let Point = Dataclass.create("Point", fields, defaults);

let vals = new ArrayList<Variant>();
vals.add(3); vals.add(4);
let p = Dataclass.newInstance(Point, vals);

io::println(Dataclass.toString(p, "Point", fields));  // Point(x=3, y=4)

let overrides = new HashMap<string, Variant>();
overrides.put("x", 10);
let p2 = Dataclass.copyWith(p, fields, overrides);
io::println(Dataclass.toString(p2, "Point", fields)); // Point(x=10, y=4)
```

## Frozen Dataclass

- `Dataclass.frozen(name: string, fields: ArrayList<string>): DataclassDef` — create immutable dataclass
- Frozen dataclasses prevent field modification after construction

## kw_only Fields

- `Dataclass.kwOnly(name: string, fields: ArrayList<string>): DataclassDef` — keyword-only fields

## default_factory

- `Dataclass.field(defaultFactory: fn(): Variant): FieldDef` — field with default factory
- `Dataclass.field(default: Variant): FieldDef` — field with default value

## post_init

- `Dataclass.postInit(fn: fn(): void): void` — post-initialization hook

## Ordered Dataclass

- `Dataclass.ordered(name: string, fields: ArrayList<string>): DataclassDef` — create comparable dataclass

## Validator

- `Dataclass.validator(field: string, fn: fn(Variant): bool): void` — add field validator
