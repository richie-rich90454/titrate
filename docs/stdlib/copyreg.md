# CopyReg

The `tt.serialization.Copyreg` module provides a Python `copyreg` analog for registering pickle dispatch: `constructor`, `pickle_support`, `add_extension`, `remove_extension`, and lookups for constructors and reducers by class name.

## Import

```titrate
import tt::serialization::Copyreg;
```

## Classes

### Constructor

A reconstruction function paired with a class name. The callable takes a tuple of state and rebuilds the object.

**Fields:**
- `className: string`
- `callable: fn(ArrayList<Variant>): Variant`
- `argNames: ArrayList<string>`

**Methods:**
- `init(className: string, callable: fn(ArrayList<Variant>): Variant)`
- `invoke(state: ArrayList<Variant>): Variant` — invoke the constructor with the given state arguments

### Reducer

Pairs an object's deconstruction with its reconstruction.

**Fields:**
- `constructor: Constructor`
- `stateFn: fn(Variant): ArrayList<Variant>`

**Methods:**
- `init(constructor: Constructor, stateFn: fn(Variant): ArrayList<Variant>)`
- `reduce(obj: Variant): (string, ArrayList<Variant>)` — produce the state tuple for an object as `(className, state)`

## Functions

### constructor

Register a constructor function for a class. Mirrors `copyreg.constructor()`.

**Parameters:** `className: string`, `callable: fn(ArrayList<Variant>): Variant`
**Returns:** `Constructor`

### pickle_support

Register pickle support for a class by pairing a reducer with its constructor. Mirrors `copyreg.pickle()`.

**Parameters:** `className: string`, `ctor: fn(ArrayList<Variant>): Variant`, `stateFn: fn(Variant): ArrayList<Variant>`
**Returns:** `Reducer`

```titrate
pickle_support("MyClass",
    fn(args: ArrayList<Variant>): Variant => new MyClass(args.get(0) as int),
    fn(obj: Variant): ArrayList<Variant> {
        let s = new ArrayList<Variant>();
        s.add((obj as MyClass).x);
        return s;
    });
```

### lookupConstructor

Look up the constructor registered for a class name. Returns `null` if none.

**Parameters:** `className: string`
**Returns:** `Constructor`

### lookupReducer

Look up the reducer registered for a class name. Returns `null` if none.

**Parameters:** `className: string`
**Returns:** `Reducer`

### add_extension

Add a pickle extension: maps a class name to a short integer code for compact serialization. Mirrors `copyreg.add_extension()`. Throws `ValueError` if already registered.

**Parameters:** `className: string`, `code: int`

### remove_extension

Remove a pickle extension by class name. Mirrors `copyreg.remove_extension()`.

**Parameters:** `className: string`, `code: int`

### nameForCode

Look up the class name for a given extension code. Returns `""` if not registered.

**Parameters:** `code: int`
**Returns:** `string`

### codeForName

Look up the extension code for a given class name. Returns `-1` if not registered.

**Parameters:** `className: string`
**Returns:** `int`

### clearRegistry

Clear all registered constructors, reducers, and extensions.

### registeredNames

Return the list of all registered class names.

**Returns:** `ArrayList<string>`
