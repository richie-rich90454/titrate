# sim

The `tt.sim` module provides discrete-event simulation inspired by SimPy. It includes an event scheduler, process management, limited-capacity resources, and statistical monitoring. Use it to model queuing systems, network simulations, manufacturing processes, and other event-driven systems.

```titrate
import tt.sim.Simulation;
import tt.sim.Resource;
import tt.sim.Process;
import tt.sim.Monitor;
```

## Simulation

The core event scheduler and simulation clock. Manages the event queue, simulation time, and process lifecycle.

- `fn init()` — create a new simulation with time starting at 0.0
- `now(): double` — get the current simulation time
- `schedule(delay: double, callback: fn(): void): void` — schedule a callback to run after the given delay
- `scheduleAt(time: double, callback: fn(): void): void` — schedule a callback at an absolute simulation time
- `run(until: double): void` — run the simulation until the specified time
- `step(): bool` — process the next event; returns false if no events remain
- `pause(): void` — pause the simulation
- `stop(): void` — stop the simulation and discard remaining events
- `isRunning(): bool` — check if the simulation is currently running
- `eventCount(): long` — get the total number of events processed
- `reset(): void` — reset the simulation to time 0.0 and clear all events

```titrate
let sim: Simulation = new Simulation();
sim.schedule(5.0, fn(): void => io::println("Event at t=5"));
sim.schedule(2.0, fn(): void => io::println("Event at t=2"));
sim.run(10.0);
// Output: Event at t=2, then Event at t=5
```

### Step-by-Step Execution

```titrate
let sim: Simulation = new Simulation();
sim.schedule(1.0, fn(): void => io::println("first"));
sim.schedule(3.0, fn(): void => io::println("second"));

while (sim.step()) {
    io::println("Time: " + Double.toString(sim.now()));
    if (sim.now() >= 3.0) {
        sim.stop();
        break;
    }
}
```

## Resource

A limited-capacity resource that processes can request and release. Supports queuing, preemption, and capacity management.

- `fn init(sim: Simulation, capacity: int)` — create a resource with the given simulation and capacity
- `fn init(sim: Simulation, capacity: int, preemptible: bool)` — create a resource, optionally allowing preemption
- `request(process: Process): void` — request one unit of the resource for a process; queues if at capacity
- `release(process: Process): void` — release a held resource unit; wakes the next queued process
- `available(): int` — get the number of currently available units
- `used(): int` — get the number of currently used units
- `capacity(): int` — get the total capacity
- `queueLength(): int` — get the number of processes waiting in the queue
- `isPreemptible(): bool` — check if the resource supports preemption
- `setCapacity(capacity: int): void` — change the resource capacity

```titrate
let sim: Simulation = new Simulation();
let server: Resource = new Resource(sim, 2);  // 2 servers available

let p1: Process = new Process(sim, "customer-1");
let p2: Process = new Process(sim, "customer-2");
let p3: Process = new Process(sim, "customer-3");

server.request(p1);  // served immediately
server.request(p2);  // served immediately
server.request(p3);  // queued (capacity = 2)

let avail: int = server.available();    // 0
let queued: int = server.queueLength(); // 1

server.release(p1);  // p3 is now served
```

### Preemptible Resource

```titrate
let sim: Simulation = new Simulation();
let gpu: Resource = new Resource(sim, 1, true);  // preemptible, 1 unit

let lowPriority: Process = new Process(sim, "low");
let highPriority: Process = new Process(sim, "high");

gpu.request(lowPriority);
// high-priority process arrives — preempts low-priority
gpu.request(highPriority);
```

## Process

SimPy-style process definition with yield-based waiting, timeouts, and interrupts. Processes are coroutine-like entities that interact with the simulation clock.

- `fn init(sim: Simulation, name: string)` — create a named process in the given simulation
- `name(): string` — get the process name
- `isAlive(): bool` — check if the process is still active
- `isWaiting(): bool` — check if the process is waiting on a resource or delay
- `priority(): int` — get the process priority (lower = higher priority)
- `setPriority(priority: int): void` — set the process priority
- `timeout(delay: double): void` — suspend the process for the given simulation time delay
- `waitUntil(time: double): void` — suspend the process until the given simulation time
- `request(resource: Resource): void` — request a resource and suspend until it is available
- `release(resource: Resource): void` — release a previously acquired resource
- `interrupt(message: string): void` — interrupt the process with a message
- `isInterrupted(): bool` — check if the process was interrupted since the last wait
- `interruptMessage(): string` — get the interrupt message
- `start(entryPoint: fn(Process): void): void` — start the process with an entry-point function

```titrate
let sim: Simulation = new Simulation();
let desk: Resource = new Resource(sim, 1);

public fn customer(p: Process): void {
    io::println(p.name() + " arrives at " + Double.toString(sim.now()));
    p.request(desk);
    io::println(p.name() + " served at " + Double.toString(sim.now()));
    p.timeout(5.0);
    p.release(desk);
    io::println(p.name() + " leaves at " + Double.toString(sim.now()));
}

let c1: Process = new Process(sim, "customer-1");
let c2: Process = new Process(sim, "customer-2");

c1.start(customer);
c2.start(customer);

sim.run(20.0);
```

### Process with Interrupts

```titrate
let sim: Simulation = new Simulation();

public fn task(p: Process): void {
    p.timeout(100.0);
    if (p.isInterrupted()) {
        io::println("Interrupted: " + p.interruptMessage());
    }
}

let proc: Process = new Process(sim, "worker");
proc.start(task);

// Simulate an interrupt at t=10
sim.schedule(10.0, fn(): void => proc.interrupt("shutdown"));
sim.run(200.0);
```

## Monitor

Statistical collection and analysis for simulation data. Supports time-weighted statistics, histograms, confidence intervals, and report generation.

- `fn init(name: string)` — create a named monitor
- `observe(value: double): void` — record an observation
- `observeAt(value: double, time: double): void` — record an observation at a specific simulation time (for time-weighted stats)
- `count(): long` — get the number of observations
- `mean(): double` — get the arithmetic mean
- `variance(): double` — get the sample variance
- `stdDev(): double` — get the sample standard deviation
- `min(): double` — get the minimum observed value
- `max(): double` — get the maximum observed value
- `sum(): double` — get the sum of all observations
- `median(): double` — get the median
- `percentile(p: double): double` — get the p-th percentile (0.0 to 1.0)
- `timeWeightedMean(): double` — get the time-weighted mean (requires `observeAt` data)
- `timeWeightedVariance(): double` — get the time-weighted variance
- `confidenceInterval(level: double): (double, double)` — get the confidence interval at the given level (e.g., 0.95); returns (lower, upper) tuple
- `histogram(buckets: int): ArrayList<long>` — get a histogram with the specified number of buckets
- `reset(): void` — clear all observations
- `name(): string` — get the monitor name
- `report(): string` — generate a summary report string with all statistics

```titrate
let waitTimes: Monitor = new Monitor("wait_time");

waitTimes.observe(2.5);
waitTimes.observe(3.1);
waitTimes.observe(1.8);
waitTimes.observe(4.2);

let avg: double = waitTimes.mean();
let std: double = waitTimes.stdDev();
let p95: double = waitTimes.percentile(0.95);
let (lo, hi): (double, double) = waitTimes.confidenceInterval(0.95);

io::println(waitTimes.report());
```

### Time-Weighted Statistics

```titrate
let queueLen: Monitor = new Monitor("queue_length");

queueLen.observeAt(0.0, 0.0);   // queue empty at t=0
queueLen.observeAt(3.0, 2.0);   // 3 in queue at t=2
queueLen.observeAt(1.0, 5.0);   // 1 in queue at t=5
queueLen.observeAt(0.0, 8.0);   // queue empty at t=8

let twMean: double = queueLen.timeWeightedMean();
let twVar: double = queueLen.timeWeightedVariance();
```

### Histogram and Report

```titrate
let latencies: Monitor = new Monitor("latency");

for (i in 0..1000) {
    latencies.observe(Math.random() * 100.0);
}

let hist: ArrayList<long> = latencies.histogram(10);
let report: string = latencies.report();
io::println(report);
```
