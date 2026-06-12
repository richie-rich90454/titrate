# chainmap

The `tt.util` module provides `ChainMap<K, V>` — a layered dictionary that searches through multiple maps in order, like a chain of scopes.

```titrate
import tt.util.ChainMap;
```

## ChainMap

A map that groups multiple dictionaries into a single view. Lookups search each map in order, returning the first match. New writes go to the first (topmost) map. This is useful for scoped configurations, variable bindings, and layered overrides.

- `fn init()` — create a ChainMap with an empty first map
- `fn init(first: HashMap<K, V>)` — create a ChainMap with the given map as the first layer
- `get(key: K): V` — search all maps in order and return the first matching value
- `getOrDefault(key: K, default: V): V` — like `get`, but return `default` if the key is not found in any map
- `containsKey(key: K): bool` — check if any map in the chain contains the key
- `put(key: V, value: V): void` — put a key-value pair into the first (topmost) map
- `remove(key: K): void` — remove a key from the first map only
- `push(map: HashMap<K, V>): void` — add a new map to the front of the chain
- `pop(): HashMap<K, V>` — remove and return the first map from the chain
- `pushBack(map: HashMap<K, V>): void` — add a map to the end of the chain
- `parents(): ArrayList<HashMap<K, V>>` — return all maps except the first
- `keys(): ArrayList<K>` — return all unique keys across all maps
- `size(): int` — number of unique keys across all maps
- `newChild(): ChainMap<K, V>` — create a new ChainMap with this one as a parent

```titrate
let defaults: HashMap<string, string> = new HashMap<string, string>();
defaults.put("color", "blue");
defaults.put("size", "medium");

let overrides: HashMap<string, string> = new HashMap<string, string>();
overrides.put("color", "red");

let chain: ChainMap<string, string> = new ChainMap<string, string>(overrides);
chain.push(defaults);

io::println(chain.get("color"));  // "red"   (found in overrides first)
io::println(chain.get("size"));   // "medium" (found in defaults)

chain.put("weight", "light");     // writes to the topmost map
io::println(chain.get("weight")); // "light"
```
