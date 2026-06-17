# Scientific Simulation with Titrate

Titrate's `tt.sim` and `tt.physics` modules provide a comprehensive toolkit for building simulations — from discrete-event queuing models to particle dynamics and quantum mechanics. This guide covers the core abstractions and walks through a complete queuing simulation.

## Discrete-Event Simulation

The `tt.sim` module implements a discrete-event simulation (DES) engine. Events are scheduled on a virtual timeline and processed in chronological order, making it ideal for modeling systems where state changes happen at specific moments — network packets, manufacturing lines, customer queues, and more.

### Simulation, Resource, and Process

The three core abstractions are:

- **`Simulation`** — the event scheduler and clock
- **`Resource`** — a constrained capacity unit (e.g., a server, a machine)
- **`Process`** — a coroutine-like entity that yields to the scheduler

```titrate
import tt::sim::Simulation;
import tt::sim::Resource;
import tt::sim::Process;
```

### Creating a Simulation

```titrate
let sim = new Simulation();

// Set simulation end time
sim.setEndTime(1000.0);  // virtual time units

// Create a resource with limited capacity
let server = new Resource("server", 2);  // 2 concurrent users
```

### Scheduling Events

```titrate
// Schedule a one-time event at time t
sim.schedule(5.0, fn(): void {
    io::println("Event fired at t=5.0");
});

// Schedule a recurring event
sim.scheduleRecurring(10.0, fn(): void {
    io::println("Tick at t=" + Double.toString(sim.currentTime()));
});
```

### Defining Processes

Processes are the building blocks of DES models. A process can request a resource, hold it for a duration, then release it:

```titrate
public fn customerProcess(sim: Simulation, server: Resource, id: int): void {
    let arriveTime = sim.currentTime();
    io::println("Customer " + Integer.toString(id) + " arrives at t=" +
                Double.toString(arriveTime));

    // Request the resource (blocks until available)
    server.request();

    let waitTime = sim.currentTime() - arriveTime;
    io::println("Customer " + Integer.toString(id) + " waited " +
                Double.toString(waitTime) + " units");

    // Hold the resource for a service duration
    let serviceTime = 5.0 + Math.random() * 10.0;
    sim.hold(serviceTime);

    // Release the resource
    server.release();

    io::println("Customer " + Integer.toString(id) + " departs at t=" +
                Double.toString(sim.currentTime()));
}
```

### Running the Simulation

```titrate
public fn main(): void {
    let sim = new Simulation();
    let server = new Resource("server", 2);

    // Spawn customer processes at staggered times
    for (i in 0..10) {
        let id = i;
        sim.schedule(Double.parseDouble(Integer.toString(i)) * 3.0, fn(): void {
            customerProcess(sim, server, id);
        });
    }

    sim.run();
    io::println("Simulation complete at t=" + Double.toString(sim.currentTime()));
}
```

## Physics Simulation

The `tt.physics` module provides types for particle-based physics simulations — from simple projectile motion to N-body gravitational systems.

```titrate
import tt::physics::Particle;
import tt::physics::ForceField;
import tt::physics::NBodySimulator;
```

### Creating Particles

```titrate
// Create a particle with position, velocity, and mass
let p1 = new Particle(0.0, 0.0, 0.0,   // position (x, y, z)
                       1.0, 0.0, 0.0,   // velocity (vx, vy, vz)
                       1.0);            // mass

let p2 = new Particle(10.0, 0.0, 0.0,
                       -1.0, 0.0, 0.0,
                       1.0);
```

### Applying Forces

```titrate
// Create a gravitational force field
let gravity = ForceField.gravity(9.81);  // m/s²

// Create a spring force field
let spring = ForceField.spring(1.0, 5.0);  // stiffness k=1.0, rest length=5.0

// Apply forces to particles
gravity.apply(p1);
spring.apply(p1, p2);  // spring between p1 and p2
```

### N-Body Simulation

```titrate
let sim = new NBodySimulator(0.01);  // timestep = 0.01

sim.addParticle(p1);
sim.addParticle(p2);

// Run for 1000 steps
for (i in 0..1000) {
    sim.step();
}

// Read back positions
let positions = sim.positions();
for (i in 0..positions.size()) {
    let p = positions.get(i);
    io::println("Particle " + Integer.toString(i) + ": (" +
                Double.toString(p.x) + ", " +
                Double.toString(p.y) + ", " +
                Double.toString(p.z) + ")");
}
```

## Rigid Body Dynamics

For simulations involving solid objects with rotation, the `RigidBody` class tracks inertia tensors, angular momentum, and torque.

```titrate
import tt::physics::RigidBody;
```

### Creating a Rigid Body

```titrate
// A box with mass 5.0 kg and dimensions 2.0 x 1.0 x 0.5
let box = new RigidBody(5.0);  // mass
box.setBoxInertia(2.0, 1.0, 0.5);  // width, height, depth

// Set initial position and orientation
box.setPosition(0.0, 10.0, 0.0);
box.setOrientation(1.0, 0.0, 0.0, 0.0);  // quaternion (w, x, y, z)
```

### Applying Torque and Force

```titrate
// Apply a force at the center of mass
box.applyForce(0.0, -49.05, 0.0);  // gravity on 5 kg

// Apply a torque
box.applyTorque(0.0, 0.0, 10.0);  // 10 N·m around z-axis

// Apply a force at an offset (creates both force and torque)
box.applyForceAt(0.0, 0.0, 5.0, 1.0, 0.0, 0.0);  // force + offset
```

### Integration Step

```titrate
let dt = 0.016;  // ~60 Hz

// Update linear and angular motion
box.integrate(dt);

// Read back state
let pos = box.position();
let vel = box.velocity();
let angVel = box.angularVelocity();
let quat = box.orientation();

io::println("Position: (" + Double.toString(pos.x) + ", " +
            Double.toString(pos.y) + ", " + Double.toString(pos.z) + ")");
io::println("Angular velocity: (" + Double.toString(angVel.x) + ", " +
            Double.toString(angVel.y) + ", " + Double.toString(angVel.z) + ")");
```

## Quantum Mechanics

The `WaveFunction` class provides tools for basic quantum mechanics calculations — probability densities, expectation values, and superposition states.

```titrate
import tt::physics::WaveFunction;
```

### Creating a Wave Function

```titrate
// Create a particle-in-a-box wave function (n=1 ground state)
let psi = WaveFunction.particleInBox(1, 1.0);  // quantum number n=1, box length L=1.0

// Evaluate at a position
let amplitude = psi.evaluate(0.3);   // complex amplitude at x=0.3
let probability = psi.probability(0.3);  // |ψ(0.3)|²
```

### Probability Density

```titrate
// Compute probability density over a range
let density = psi.probabilityDensity(0.0, 1.0, 100);  // 100 sample points

for (i in 0..density.size()) {
    let point = density.get(i);
    io::println("x=" + Double.toString(point.x) +
                "  P=" + Double.toString(point.y));
}
```

### Expectation Values

```titrate
// Compute expectation value of position <x>
let expX = psi.expectationX();
io::println("<x> = " + Double.toString(expX));

// Compute expectation value of momentum <p>
let expP = psi.expectationP();
io::println("<p> = " + Double.toString(expP));

// Compute uncertainty Δx
let deltaX = psi.uncertaintyX();
io::println("Δx = " + Double.toString(deltaX));
```

### Superposition States

```titrate
// Create a superposition of n=1 and n=2 states
let psi1 = WaveFunction.particleInBox(1, 1.0);
let psi2 = WaveFunction.particleInBox(2, 1.0);

let superposition = psi1.add(psi2, 1.0 / MathAdvanced.sqrt(2.0),
                                   1.0 / MathAdvanced.sqrt(2.0));
// Equal superposition: ψ = (1/√2)(ψ₁ + ψ₂)

let prob = superposition.probability(0.25);
io::println("P(0.25) = " + Double.toString(prob));
```

## End-to-End Example: Queuing Simulation with Monitoring

This example models an M/M/c queuing system (Poisson arrivals, exponential service, c servers) and tracks key performance metrics.

```titrate
import tt::sim::Simulation;
import tt::sim::Resource;
import tt::sim::Process;
import tt::math::Math;

public class QueueMonitor {
    public ArrayList<double> waitTimes;
    public ArrayList<double> serviceTimes;
    public int totalServed;
    public int totalArrived;

    public fn init() {
        this.waitTimes = new ArrayList<double>();
        this.serviceTimes = new ArrayList<double>();
        this.totalServed = 0;
        this.totalArrived = 0;
    }

    public fn recordWait(t: double): void {
        this.waitTimes.add(t);
    }

    public fn recordService(t: double): void {
        this.serviceTimes.add(t);
    }

    public fn avgWait(): double {
        if (this.waitTimes.size() == 0) { return 0.0; }
        let sum = 0.0;
        for (w in this.waitTimes) {
            sum = sum + w;
        }
        return sum / Double.parseDouble(Integer.toString(this.waitTimes.size()));
    }

    public fn avgService(): double {
        if (this.serviceTimes.size() == 0) { return 0.0; }
        let sum = 0.0;
        for (s in this.serviceTimes) {
            sum = sum + s;
        }
        return sum / Double.parseDouble(Integer.toString(this.serviceTimes.size()));
    }

    public fn printReport(): void {
        io::println("=== Queue Simulation Report ===");
        io::println("Total arrived: " + Integer.toString(this.totalArrived));
        io::println("Total served:  " + Integer.toString(this.totalServed));
        io::println("Avg wait time: " + Double.toString(this.avgWait()));
        io::println("Avg service:   " + Double.toString(this.avgService()));
    }
}

public fn customer(sim: Simulation, server: Resource, monitor: QueueMonitor, id: int): void {
    monitor.totalArrived = monitor.totalArrived + 1;
    let arriveTime = sim.currentTime();

    // Wait for a server to become available
    server.request();

    let waited = sim.currentTime() - arriveTime;
    monitor.recordWait(waited);

    // Exponential service time with mean 5.0
    let serviceTime = -5.0 * MathAdvanced.ln(1.0 - Math.random());
    monitor.recordService(serviceTime);
    sim.hold(serviceTime);

    server.release();
    monitor.totalServed = monitor.totalServed + 1;
}

public fn main(): void {
    let sim = new Simulation();
    sim.setEndTime(500.0);

    let server = new Resource("teller", 3);  // 3 tellers
    let monitor = new QueueMonitor();

    // Poisson arrivals: mean inter-arrival time = 2.0
    let t = 0.0;
    let id = 0;
    while (t < 500.0) {
        let interArrival = -2.0 * MathAdvanced.ln(1.0 - Math.random());
        t = t + interArrival;
        let customerId = id;
        sim.schedule(t, fn(): void {
            customer(sim, server, monitor, customerId);
        });
        id = id + 1;
    }

    sim.run();
    monitor.printReport();
}
```

::: tip Choosing the right abstraction
Use `Simulation` + `Resource` when your system is event-driven (queues, networks, logistics). Use `NBodySimulator` or `RigidBody` when your system is time-stepped (physics, robotics, games). The two can be combined — for example, a factory simulation might use DES for scheduling and physics for robotic arm kinematics.
:::

## What's Next?

- [Physics Simulation](./physics-guide) — deep dive into particle dynamics and force fields
- [Scientific Computing](./scientific-computing) — NDArray and Matrix for numerical work
- [Standard Library](./stdlib) — full module reference
