# bit

The `tt.math` module provides `Bit` — bit manipulation utilities for low-level integer operations.

```titrate
import tt.math.Bit;
```

## Free Functions

- `Bit.popcount(value: int): int` — count the number of set bits (population count)
- `Bit.countlZero(value: int): int` — count leading zero bits
- `Bit.countrZero(value: int): int` — count trailing zero bits
- `Bit.rotl(value: int, shift: int): int` — rotate bits left
- `Bit.rotr(value: int, shift: int): int` — rotate bits right
- `Bit.hasSingleBit(value: int): bool` — check if exactly one bit is set (power of two)
- `Bit.bitWidth(value: int): int` — minimum number of bits required to represent the value
- `Bit.floor2(value: int): int` — largest power of two less than or equal to the value
- `Bit.ceil2(value: int): int` — smallest power of two greater than or equal to the value
- `Bit.getBit(value: int, pos: int): bool` — get the bit at the given position
- `Bit.setBit(value: int, pos: int): int` — set the bit at the given position to 1
- `Bit.clearBit(value: int, pos: int): int` — clear the bit at the given position to 0
- `Bit.toggleBit(value: int, pos: int): int` — toggle the bit at the given position
- `Bit.mask(count: int): int` — create a bitmask with the lowest `count` bits set
- `Bit.reverseBits(value: int): int` — reverse the bit order
- `Bit.byteSwap(value: int): int` — reverse the byte order (endianness swap)

```titrate
io::println(Integer.toString(Bit.popcount(0b1011)));       // 3
io::println(Integer.toString(Bit.countlZero(16)));          // depends on int width
io::println(Integer.toString(Bit.countrZero(8)));           // 3

io::println(Boolean.toString(Bit.hasSingleBit(16)));        // true
io::println(Boolean.toString(Bit.hasSingleBit(6)));         // false

io::println(Integer.toString(Bit.bitWidth(255)));           // 8
io::println(Integer.toString(Bit.floor2(20)));              // 16
io::println(Integer.toString(Bit.ceil2(20)));               // 32

io::println(Boolean.toString(Bit.getBit(0b1010, 1)));       // true
io::println(Integer.toString(Bit.setBit(0b1000, 0)));       // 0b1001 = 9
io::println(Integer.toString(Bit.clearBit(0b1010, 1)));     // 0b1000 = 8
io::println(Integer.toString(Bit.toggleBit(0b1010, 0)));    // 0b1011 = 11

io::println(Integer.toString(Bit.mask(4)));                 // 0b1111 = 15
io::println(Integer.toString(Bit.rotl(0b1010, 2)));         // rotate left by 2
io::println(Integer.toString(Bit.rotr(0b1010, 2)));         // rotate right by 2

io::println(Integer.toString(Bit.reverseBits(0b1010)));     // bit-reversed value
io::println(Integer.toString(Bit.byteSwap(0x12345678)));    // 0x78563412
```

## Extended Bit Operations

- `Bit.bitFloor(n: int): int` — largest power of 2 ≤ n
- `Bit.bitCeil(n: int): int` — smallest power of 2 ≥ n
- `Bit.popcount(n: int): int` — population count
- `Bit.rotateLeft(n: int, distance: int): int` — left rotation
- `Bit.rotateRight(n: int, distance: int): int` — right rotation
- `Bit.parity(n: int): int` — bit parity (0 or 1)
- `Bit.countLeadingZeros(n: int): int` — count leading zero bits
- `Bit.countTrailingZeros(n: int): int` — count trailing zero bits

## C++ `<bit>` additions (Phase 1-2 parity)

The following functions complete `std::bit_*` parity. `byteSwap` was already available above; `countlOne` and `countrOne` count the run of one-bits at the most / least significant end.

- `Bit.byteSwap(value: int): int` — reverse the byte order (endianness swap); equivalent to `std::byteswap` (C++23)
- `Bit.countlOne(value: int): int` — count leading one bits (`std::countl_one`)
- `Bit.countrOne(value: int): int` — count trailing one bits (`std::countr_one`)

```titrate
// 0xFFFFFF00 has 24 leading one bits in a 32-bit int
io::println(Integer.toString(Bit.countlOne(0xFFFFFF00)));  // 24

// 0x000000FF has 8 trailing one bits
io::println(Integer.toString(Bit.countrOne(0x000000FF)));   // 8

// byteswap reverses byte order
io::println(Integer.toString(Bit.byteSwap(0x12345678)));   // 0x78563412
```
