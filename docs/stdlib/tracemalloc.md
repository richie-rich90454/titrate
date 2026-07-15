# Tracemalloc

The `tt.sys.Tracemalloc` module provides memory allocation tracing. It mirrors Python's `tracemalloc` module, exposing `start`/`stop` to enable allocation tracing, `takeSnapshot` to capture the current set of tracked allocations, `getTracedMemory` to query the total and peak traced byte counts, and `getObjectTraceback` to look up the call site that allocated a specific object. Since Titrate is GC-managed, this module maintains a software shadow of allocations recorded through the `track`/`untrack` helpers; native heap events are not automatically captured.

## Import

```titrate
import tt::sys::Tracemalloc;
```

## Classes

### Trace

A single traced allocation record: size in bytes plus the call-site frames.

**Fields:**
- `size: long` — allocation size in bytes
- `frames: ArrayList<Traceback.Frame>` — call-site stack frames
- `count: int` — number of allocations aggregated in this trace

**Constructors:**
- `init(size: long, frames: ArrayList<Traceback.Frame>)`

**Methods:**
- `toString(): string` — pretty-print this trace as a multi-line string

### Snapshot

A snapshot of all allocations at a point in time.

**Fields:**
- `traces: ArrayList<Trace>`
- `totalSize: long`
- `peakSize: long`

**Constructors:**
- `init()` — creates an empty snapshot

**Methods:**
- `size(): int` — number of distinct traces in the snapshot
- `computeTotalSize(): long` — compute the total traced bytes across all traces (updates `totalSize`)
- `getTop(limit: int): ArrayList<Trace>` — return the traces sorted by size, descending, limited to `limit`
- `formatTop(limit: int): string` — format the top `limit` traces as a multi-line string

## Functions

### start

- `Tracemalloc.start(nframe: int): bool` — start tracing allocations. `nframe` is the maximum number of frames to record per allocation (default `25`). Returns `true` if tracing was successfully started; `false` if already started.
- `Tracemalloc.startDefault(): bool` — start with default settings (25 frames per trace)

```titrate
Tracemalloc.startDefault();
```

### stop

- `Tracemalloc.stop(): void` — stop tracing. Existing traces remain available for snapshots.

### isTracing

- `Tracemalloc.isTracing(): bool` — true if tracing is currently active

### getTracedMemory

- `Tracemalloc.getTracedMemory(): ArrayList<long>` — returns a 2-element list `[current, peak]` of bytes currently allocated and the peak observed allocation size since tracing started

```titrate
let mem: ArrayList<long> = Tracemalloc.getTracedMemory();
io::println("current=" + Long.toString(mem.get(0)) + " peak=" + Long.toString(mem.get(1)));
```

### takeSnapshot

- `Tracemalloc.takeSnapshot(): Snapshot` — take a snapshot of the current set of traced allocations

### getObjectTraceback

- `Tracemalloc.getObjectTraceback(objectId: int): ArrayList<Traceback.Frame>` — return the call-site frames for the given object id, or `null` if not tracked

### getObjectTrace

- `Tracemalloc.getObjectTrace(objectId: int): Trace` — return the `Trace` record for a given object id, or `null` if untracked

### getAllocatedBlocks

- `Tracemalloc.getAllocatedBlocks(): int` — number of distinct allocations currently tracked

### getAllocationsCount

- `Tracemalloc.getAllocationsCount(): int` — number of allocation events recorded since `start()`

### resetPeak

- `Tracemalloc.resetPeak(): void` — reset the peak counter to the current allocation size

### track

- `Tracemalloc.track(size: long): int` — record an allocation of `size` bytes at the current call site. Returns an integer object id that can be passed to `getObjectTraceback`. Returns `0` if tracing is not enabled.

### untrack

- `Tracemalloc.untrack(objectId: int): void` — record that the object with `objectId` has been freed/dereferenced

### clearTraces

- `Tracemalloc.clearTraces(): void` — clear all recorded traces without stopping tracing

## Usage Example

```titrate
import tt::sys::Tracemalloc;

public fn main(): void {
    Tracemalloc.startDefault();
    let id1: int = Tracemalloc.track(1024);
    let id2: int = Tracemalloc.track(2048);
    let mem: ArrayList<long> = Tracemalloc.getTracedMemory();
    io::println("Traced: " + Long.toString(mem.get(0)) + " bytes");
    let snap: Snapshot = Tracemalloc.takeSnapshot();
    io::println(snap.formatTop(10));
    Tracemalloc.untrack(id1);
    Tracemalloc.stop();
}
```
