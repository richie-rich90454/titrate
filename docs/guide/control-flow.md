# Control Flow

## if / else

```titrate
if x > 0 {
    io::println("positive");
} else {
    io::println("non-positive");
}
```

## while

```titrate
var i: int = 0;
while i < 10 {
    io::println(Integer.toString(i));
    i = i + 1;
}
```

## for

```titrate
for item in collection {
    io::println(item);
}
```

## break and continue

```titrate
while true {
    if done { break; }
    if skip { continue; }
}
```
