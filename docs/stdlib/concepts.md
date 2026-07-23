# `tt::concepts` — C++20 Concept Predicates

Runtime concept-checking predicates matching C++20 `<concepts>`.

## Core Language Concepts

```titrate
sameAs("int", "int")         // true
derivedFrom("Dog", "Animal") // true
convertibleTo("int", "double")
integral("int")              // true
floatingPoint("double")      // true
arithmetic("int")            // true
```

## Object Concepts

```titrate
destructible("MyClass")
constructibleFrom("MyClass", ["int", "string"])
defaultInitializable("MyClass")
moveConstructible("MyClass")
copyConstructible("MyClass")
```

## Iterator & Range Concepts

```titrate
iterator("ArrayListIterator")
forwardIterator("ArrayListIterator")
randomAccessIterator("ArrayListIterator")
range("ArrayList")
sizedRange("ArrayList")
```
