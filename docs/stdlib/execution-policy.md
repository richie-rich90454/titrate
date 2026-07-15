# ExecutionPolicy

The `tt::concurrent::ExecutionPolicy` module provides C++ `<execution>` parity. It defines marker types that select sequential vs. parallel dispatch for the `Algorithms` overloads. In Titrate, these are thin descriptors; the Algorithms module inspects the policy's `kind` and may delegate to `ThreadPoolExecutor` for the parallel variants.

## Import

```titrate
import tt::concurrent::ExecutionPolicy;
```

## API Reference

### Marker Types

The module exposes four marker classes mirroring the C++ execution policy types. Each has a `toString()` method returning its canonical name.

#### `Seq`

Marker type for the sequential execution policy (`std::execution::sequenced_policy`).

#### `Par`

Marker type for the parallel execution policy (`std::execution::parallel_policy`).

#### `ParUnseq`

Marker type for the parallel+unsequenced policy (`std::execution::parallel_unsequenced_policy`).

#### `UnsequencedPolicy`

Marker type for the unsequenced execution policy (`std::execution::unsequenced_policy`).

### `ExecutionPolicy`

A tagged container that holds one of the marker types above. Algorithms accept `ExecutionPolicy` and dispatch on the `kind` string.

**Fields:**
- `kind: string` — the policy kind identifier (`"seq"`, `"par"`, `"par_unseq"`, `"unseq"`)
- `policy: Variant` — the underlying marker instance

**Methods:**

- `isSequential(): bool` — returns true if this policy requests sequential execution
- `isParallel(): bool` — returns true if this policy requests parallel execution (`par` or `par_unseq`)
- `isUnsequenced(): bool` — returns true if this policy permits vectorization (`par_unseq` or `unseq`)
- `equals(other: Variant): bool` — compares two policies for equality by kind
- `toString(): string` — returns `"ExecutionPolicy(<kind>)"`

### Free Functions

#### `seq(): ExecutionPolicy`

Returns a sequential execution policy (mirrors `std::execution::seq`).

#### `par(): ExecutionPolicy`

Returns a parallel execution policy (mirrors `std::execution::par`).

#### `parUnseq(): ExecutionPolicy`

Returns a parallel+unsequenced execution policy (mirrors `std::execution::par_unseq`).

#### `unsequenced(): ExecutionPolicy`

Returns an unsequenced execution policy (mirrors `std::execution::unseq`).

#### `isParallel(policy: ExecutionPolicy): bool`

Returns true if the given policy requests any parallel dispatch.

#### `isSequential(policy: ExecutionPolicy): bool`

Returns true if the given policy requests sequential dispatch.

#### `isUnsequenced(policy: ExecutionPolicy): bool`

Returns true if the given policy permits unsequenced (vectorized) execution.

#### `name(policy: ExecutionPolicy): string`

Returns the human-readable name of the policy kind. Returns `"unknown"` for `null`.

## Usage Examples

### Selecting an Execution Policy

```titrate
import tt::concurrent::ExecutionPolicy;
import tt::io::IO;

public fn main(): void {
    let seqPolicy: ExecutionPolicy = ExecutionPolicy.seq();
    let parPolicy: ExecutionPolicy = ExecutionPolicy.par();

    IO.println("seq is sequential: " + seqPolicy.isSequential());   // true
    IO.println("par is parallel: " + parPolicy.isParallel());       // true
    IO.println("par kind: " + ExecutionPolicy.name(parPolicy));     // par
}
```

### Inspecting Policy Properties

```titrate
import tt::concurrent::ExecutionPolicy;

let p: ExecutionPolicy = ExecutionPolicy.parUnseq();
// par_unseq is both parallel and unsequenced
if (p.isParallel() && p.isUnsequenced()) {
    io::println("policy permits vectorized parallel dispatch");
}
```

### Comparing Policies

```titrate
import tt::concurrent::ExecutionPolicy;

let a: ExecutionPolicy = ExecutionPolicy.seq();
let b: ExecutionPolicy = ExecutionPolicy.seq();
let c: ExecutionPolicy = ExecutionPolicy.par();

io::println(a.equals(b));  // true  (same kind)
io::println(a.equals(c));  // false (different kind)
```
