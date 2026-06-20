# gc

The `tt.sys.Gc` module provides garbage-collector control. The Titrate VM uses reference counting, so GC control is limited: `collect()` issues a hint to the VM to release unused resources. The module tracks whether collection is enabled and a running count of collection cycles.

```titrate
import tt.sys.Gc;
```

## Top-level Functions

- `fn enable(): void` — enable automatic garbage collection
- `fn disable(): void` — disable automatic garbage collection
- `fn collect(): void` — request a garbage-collection cycle (no-op when disabled); increments the collection counter
- `fn isEnabled(): bool` — return true if GC is currently enabled
- `fn getStats(): HashMap<string, int>` — return statistics about the collector with keys `"collections"` (cycle count), `"enabled"` (1 or 0), and `"pid"` (OS process id)
- `fn resetStats(): void` — reset the collection counter to zero

```titrate
import tt.sys.Gc;

Gc.disable();
Gc.collect();                 // no-op while disabled
Gc.enable();
Gc.collect();
let stats = Gc.getStats();
io::println(Integer.toString(stats.get("collections"))); // 1
```
