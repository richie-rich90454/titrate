# Ownership

## Owned Values

```titrate
let x: Owned<int> = new int(5);
let y = x;  // x is moved
// io::println(Integer.toString(x));  // ERROR: use after move
```

## Borrowing

```titrate
let x: Owned<int> = new int(5);
let y = &x;      // immutable borrow
// x = new int(6);  // ERROR: cannot move while borrowed
```

## Regions

```titrate
region r {
    let ptr = r.alloc(42);
    // ptr is valid within this region
}
// ptr is no longer valid
```

## Unsafe Blocks

`unsafe` blocks suspend ownership and borrowing checks. This is useful for low-level operations that the compiler cannot verify:

```titrate
unsafe {
    // ownership rules are suspended inside this block
    let x: Owned<int> = new int(5);
    let y = x;       // would normally move x
    let z = x;       // allowed in unsafe: no move check
}
```

Use `unsafe` sparingly — it disables the safety guarantees that the compiler provides.
