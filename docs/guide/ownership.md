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

```titrate
unsafe {
    // ownership rules are suspended
    let ptr = malloc(4);
    *ptr = 0xFEED;
    free(ptr);
}
```
