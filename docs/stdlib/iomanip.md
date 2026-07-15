# Iomanip

The `tt.io.Iomanip` module provides stream manipulators mirroring C++'s `<iomanip>` header. Manipulators are returned as small helper objects that `Writer` and `Format` apply when formatting the next value(s). Because Titrate's `Writer` keeps its own format state, each manipulator is a thin command object the writer consumes.

## Import

```titrate
import tt::io::Iomanip;
```

## Field-width and fill

### setw

Set the field width for the next formatted output only.

**Parameters:** `n: int`
**Returns:** `Manipulator`

```titrate
let w: Writer = Stdout.out();
w.apply(setw(10));
w.writeInt(42);
```

### setfill

Set the fill character used for padding.

**Parameters:** `c: char`
**Returns:** `Manipulator`

```titrate
w.apply(setfill('0'));
```

### setbase

Set the numeric base for integer output (8, 10, or 16).

**Parameters:** `b: int`
**Returns:** `Manipulator`

## Precision

### setprecision

Set the floating-point precision used by subsequent output.

**Parameters:** `p: int`
**Returns:** `Manipulator`

```titrate
w.apply(setprecision(4));
w.writeDouble(3.14159);  // 3.142
```

## Numeric-base flags

These return manipulators that change the integer formatting base.

- `hex(): Manipulator` — base 16, lowercase
- `dec(): Manipulator` — base 10 (default)
- `oct(): Manipulator` — base 8

## Floating-point format flags

- `fixed(): Manipulator` — fixed-point notation
- `scientific(): Manipulator` — scientific notation
- `hexfloat(): Manipulator` — hexadecimal floating-point
- `defaultfloat(): Manipulator` — reset to default float formatting

## Boolean formatting

- `boolalpha(): Manipulator` — print booleans as `true`/`false`
- `noboolalpha(): Manipulator` — print booleans as `1`/`0`

## Showpoint flags

- `showpoint(): Manipulator` — always include the decimal point
- `noshowpoint(): Manipulator` — omit the decimal point when not needed
- `showpos(): Manipulator` — prefix non-negative numbers with `+`
- `noshowpos(): Manipulator` — do not prefix non-negative numbers

## Adjustfield flags

- `left(): Manipulator` — left-align within the field
- `right(): Manipulator` — right-align within the field
- `internal(): Manipulator` — pad between sign and value

## Money

### put_money

Format a monetary amount with the locale's currency facet.

**Parameters:** `amount: long` (overload: `amount: long, intl: bool`)
**Returns:** `Manipulator`

### get_money

Parse a monetary amount from the input stream.

**Parameters:** `intl: bool`
**Returns:** `Manipulator` (yields the parsed `long`)

## Time

### put_time

Format a `tm`-style time structure using the given format string (same spec as C's `strftime`).

**Parameters:** `t: TimeStruct`, `fmt: string`
**Returns:** `Manipulator`

```titrate
w.apply(put_time(now, "%Y-%m-%d %H:%M:%S"));
```

### get_time

Parse a time string using the given format string.

**Parameters:** `t: TimeStruct`, `fmt: string`
**Returns:** `Manipulator`

## Quoted strings

### quoted

Wrap a string in double quotes and escape inner quotes/backslashes, suitable for round-trip parsing. The manipulator is bidirectional: on output it quotes the string, on input it strips quotes and unescapes.

**Overloads:**
- `quoted(s: string): Manipulator`
- `quoted(s: string, escape: char, delim: char): Manipulator`

```titrate
w.apply(quoted("hello \"world\""));
// emits: "hello \"world\""
```

## resetiosflags / setiosflags

- `resetiosflags(mask: int): Manipulator` — clear the format flags in `mask`
- `setiosflags(mask: int): Manipulator` — set the format flags in `mask`
