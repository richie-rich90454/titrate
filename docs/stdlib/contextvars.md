# ContextVars

The `tt.concurrent.ContextVars` module provides a Python `contextvars` analog: `ContextVar`, `Context`, `copy_context`, and `Token` for context-local state that flows across async boundaries and copied contexts. It is backed by a `HashMap<name, value>` snapshot model.

## Import

```titrate
import tt::concurrent::ContextVars;
```

## Classes

### Token

Returned by `ContextVar.set()`; used to restore a previous value.

**Fields:**
- `var: ContextVar`
- `oldValue: Variant`
- `used: bool`

**Methods:**
- `init(contextVar: ContextVar, oldValue: Variant)`
- `reset(context: Context): void` — restore the variable to its previous value (throws if already used)

### ContextVar

A variable whose value is scoped to the current `Context`.

**Fields:**
- `name: string`
- `defaultValue: Variant`

**Methods:**
- `init(name: string)`
- `setDefault(value: Variant): void` — set the default value used when no value is bound
- `get(): Variant` — get the value bound in the current context, or the default (throws `LookupError` if neither)
- `getOrDefault(defaultValue: Variant): Variant` — get with a fallback
- `set(value: Variant): Token` — bind a new value in the current context; returns a `Token` to restore
- `clear(): void` — remove the binding for this variable

```titrate
let user: ContextVar = new ContextVar("user");
user.setDefault("anonymous");
let token: Token = user.set("alice");
io::println(user.get());  // alice
token.reset(currentContext());
io::println(user.get());  // anonymous
```

### Context

A mapping of variable names to values. `copy_context()` creates a shallow copy that inherits all current bindings.

**Methods:**
- `init()`
- `has(name: string): bool`
- `get(name: string): Variant` — `null` if absent
- `setRaw(name: string, value: Variant): void` — set a value (internal; used by `ContextVar.set` and `Token.reset`)
- `remove(name: string): void`
- `copy(): Context` — return a shallow copy
- `run(func: fn(): void): void` — run a function within this context, restoring the previous context after
- `size(): int` — number of bindings
- `vars(): ArrayList<string>` — list of bound variable names

## Functions

### currentContext

Return the current `Context`, creating one if none exists.

**Returns:** `Context`

### copy_context

Return a shallow copy of the current context. Mirrors `contextvars.copy_context()`.

**Returns:** `Context`

```titrate
let ctx: Context = copy_context();
ctx.run(fn(): void {
    // runs with the copied bindings
});
```

### runInCopy

Run a function in a freshly copied context (does not pollute the caller).

**Parameters:** `func: fn(): void`
**Returns:** `void`
