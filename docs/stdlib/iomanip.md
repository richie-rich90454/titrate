# `tt::iomanip` — Stream Manipulators

The `Iomanip` module provides C++ `<iomanip>`-style stream format manipulators.

## Basic Manipulators

```titrate
import tt::io::Iomanip;

let state: FormatState = new FormatState();
applyAll(state, [setw(10), setprecision(3), hex()]);
```

## Functions

| Function | Description |
|----------|-------------|
| `setw(n)` | Set field width |
| `setprecision(n)` | Set float precision |
| `setfill(c)` | Set fill character |
| `setbase(b)` | Set numeric base |
| `hex()` / `dec()` / `oct()` | Set numeric base |
| `fixed()` / `scientific()` | Float notation |
| `boolalpha()` / `noboolalpha()` | Bool display style |
| `put_money(cents)` | Money formatting |
| `put_time(ms, fmt)` | Time formatting |
| `quoted(s)` | Quoted string output |
