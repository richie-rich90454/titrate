# enum

The `tt.lang` module provides `EnumValue` and helper functions for working with enumerations — type-safe named constants with ordinal values.

```titrate
import tt.lang.EnumValue;
```

## EnumValue

Represents a single enumeration constant with a name and ordinal position.

- `fn init(name: string, ordinal: int)` — create an enum value with the given name and ordinal
- `toString(): string` — return the name of this enum constant
- `equals(other: EnumValue): bool` — check equality against another enum value
- `getOrdinal(): int` — return the ordinal (position) of this enum constant
- `getName(): string` — return the name of this enum constant

```titrate
let red: EnumValue = new EnumValue("Red", 0);
let green: EnumValue = new EnumValue("Green", 1);

io::println(red.toString());     // "Red"
io::println(Integer.toString(red.getOrdinal()));  // 0
io::println(Boolean.toString(red.equals(green))); // false
```

## Free Functions

- `enumValueOf(values: ArrayList<EnumValue>, name: string): EnumValue` — look up an enum constant by name from the list of values
- `enumFromOrdinal(values: ArrayList<EnumValue>, ordinal: int): EnumValue` — look up an enum constant by ordinal from the list of values

```titrate
let colors: ArrayList<EnumValue> = new ArrayList<EnumValue>();
colors.add(new EnumValue("Red", 0));
colors.add(new EnumValue("Green", 1));
colors.add(new EnumValue("Blue", 2));

let found: EnumValue = enumValueOf(colors, "Green");
io::println(found.toString());  // "Green"

let byOrd: EnumValue = enumFromOrdinal(colors, 2);
io::println(byOrd.toString());  // "Blue"
```

## Flag Enum

- `FlagEnum.create(name: string, values: HashMap<string, int>): FlagEnum` — create flag enum
- `FlagEnum.hasFlag(value: int, flag: int): bool` — check if flag is set
- `FlagEnum.setFlag(value: int, flag: int): int` — set flag
- `FlagEnum.clearFlag(value: int, flag: int): int` — clear flag
- `FlagEnum.toggleFlag(value: int, flag: int): int` — toggle flag

## IntEnum

- `IntEnum.create(name: string, values: HashMap<string, int>): IntEnum` — create integer enum
- `IntEnum.fromValue(enumType: IntEnum, value: int): string` — lookup by integer value

## StrEnum

- `StrEnum.create(name: string, values: HashMap<string, string>): StrEnum` — create string enum
- `StrEnum.fromValue(enumType: StrEnum, value: string): string` — lookup by string value

## Auto-numbering

- `Enum.auto(): int` — auto-assign next integer value
- `Enum.iterate(enumType: Enum): ArrayList<string>` — iterate enum values
- `Enum.fromValue(enumType: Enum, value: Variant): string` — lookup by value
- `Enum.isMember(enumType: Enum, name: string): bool` — membership test
