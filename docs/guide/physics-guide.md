# Physics Simulation with Titrate

Titrate's `tt.physics` module provides a comprehensive toolkit for physics simulation — from simple particle dynamics to N-body gravitational systems and rigid body mechanics. This guide covers the core abstractions, force fields, integration methods, and walks through a complete solar system simulation.

## Particle Dynamics

The `Particle` class represents a point mass with position, velocity, and acceleration. Combined with an integration scheme, it forms the basis of all particle-based simulations.

```titrate
import tt::physics::Particle;
import tt::physics::ForceField;
import tt::physics::NBodySimulator;
```

### Creating Particles

```titrate
// Particle with position (x, y, z), velocity (vx, vy, vz), and mass
let p = new Particle(0.0, 10.0, 0.0,    // position
                     5.0, 0.0, 0.0,      // velocity
                     1.0);               // mass (kg)

// Access properties
io::println("Position: (" + Double.toString(p.x) + ", " +
            Double.toString(p.y) + ", " + Double.toString(p.z) + ")");
io::println("Velocity: (" + Double.toString(p.vx) + ", " +
            Double.toString(p.vy) + ", " + Double.toString(p.vz) + ")");
io::println("Mass: " + Double.toString(p.mass));
```

### Applying Forces

Forces are applied by setting the acceleration on each particle. Newton's second law (`F = ma`) gives `a = F/m`:

```titrate
// Apply gravity (F = mg, a = g)
let g = 9.81;
p.ax = 0.0;
p.ay = -g;
p.az = 0.0;

// Apply a custom force
let fx = 10.0;
let fy = 0.0;
let fz = 0.0;
p.ax = p.ax + fx / p.mass;
p.ay = p.ay + fy / p.mass;
p.az = p.az + fz / p.mass;
```

### Verlet Integration

Verlet integration is a numerical method for integrating equations of motion. It is more stable and energy-conserving than simple Euler integration:

```titrate
public fn verletStep(p: Particle, dt: double): void {
    // Velocity Verlet algorithm:
    // 1. Update position: x(t+dt) = x(t) + v(t)*dt + 0.5*a(t)*dt²
    p.x = p.x + p.vx * dt + 0.5 * p.ax * dt * dt;
    p.y = p.y + p.vy * dt + 0.5 * p.ay * dt * dt;
    p.z = p.z + p.vz * dt + 0.5 * p.az * dt * dt;

    // 2. Store old acceleration
    let oldAx = p.ax;
    let oldAy = p.ay;
    let oldAz = p.az;

    // 3. Compute new acceleration (from forces at new position)
    // ... (update p.ax, p.ay, p.az based on new position) ...

    // 4. Update velocity: v(t+dt) = v(t) + 0.5*(a(t) + a(t+dt))*dt
    p.vx = p.vx + 0.5 * (oldAx + p.ax) * dt;
    p.vy = p.vy + 0.5 * (oldAy + p.ay) * dt;
    p.vz = p.vz + 0.5 * (oldAz + p.az) * dt;
}
```

### Simple Projectile Simulation

```titrate
public fn main(): void {
    let p = new Particle(0.0, 0.0, 0.0,    // start at origin
                         10.0, 20.0, 0.0,   // initial velocity
                         1.0);              // mass

    let dt = 0.01;  // 10ms timestep
    let g = 9.81;

    // Simulate until particle hits the ground
    while (p.y >= 0.0) {
        // Apply gravity
        p.ax = 0.0;
        p.ay = -g;
        p.az = 0.0;

        // Simple Euler integration (for illustration)
        p.vx = p.vx + p.ax * dt;
        p.vy = p.vy + p.ay * dt;
        p.vz = p.vz + p.az * dt;

        p.x = p.x + p.vx * dt;
        p.y = p.y + p.vy * dt;
        p.z = p.z + p.vz * dt;
    }

    io::println("Landed at x=" + Double.toString(p.x) +
                ", y=" + Double.toString(p.y));
}
```

## Force Fields

The `ForceField` class provides pre-built force models that compute accelerations on particles.

### Gravitational Force

```titrate
// Constant gravitational field (near-surface approximation)
let gravity = ForceField.gravity(9.81);  // g = 9.81 m/s²

// Apply to a particle
gravity.apply(p);  // sets p.ay = -9.81
```

### Electromagnetic Force

```titrate
// Lorentz force: F = q(E + v × B)
let emField = ForceField.electromagnetic(
    1.6e-19,     // charge (Coulombs)
    0.0, 0.0, 0.0,    // electric field E (V/m)
    0.0, 0.0, 1.0     // magnetic field B (Tesla)
);

emField.apply(p);  // computes F = q(E + v×B) and sets acceleration
```

### Spring Force

```titrate
// Hooke's law: F = -k(x - x₀)
let spring = ForceField.spring(10.0, 5.0);  // stiffness k=10.0, rest length=5.0

// Apply spring force between two particles
spring.apply(p1, p2);
```

### Lennard-Jones Potential

The Lennard-Jones potential models intermolecular interactions with both attractive and repulsive terms:

```titrate
// V(r) = 4ε[(σ/r)¹² - (σ/r)⁶]
let lj = ForceField.lennardJones(1.0, 3.4);  // ε=1.0 (depth), σ=3.4 (size parameter)

// Apply between two particles
lj.apply(p1, p2);
```

### Coulomb Force

```titrate
// Electrostatic force: F = kₑ * q₁ * q₂ / r²
let coulomb = ForceField.coulomb(8.99e9);  // Coulomb constant kₑ

// Apply between two charged particles
coulomb.apply(p1, p2);
```

## N-Body Simulation

The `NBodySimulator` class simulates gravitational interactions between many bodies. For large N, it uses the Barnes-Hut algorithm to reduce complexity from O(n²) to O(n log n).

### Basic N-Body Setup

```titrate
let sim = new NBodySimulator(0.001);  // timestep

// Add particles
sim.addParticle(new Particle(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1000.0));   // heavy center
sim.addParticle(new Particle(5.0, 0.0, 0.0, 0.0, 14.0, 0.0, 1.0));     // orbiting body
sim.addParticle(new Particle(-3.0, 0.0, 0.0, 0.0, -10.0, 0.0, 0.5));   // another orbiter

// Set gravitational constant
sim.setG(1.0);  // use G=1 for simplified units

// Run simulation
for (i in 0..10000) {
    sim.step();
}
```

### Barnes-Hut Algorithm

For large numbers of particles, the Barnes-Hut algorithm approximates distant gravitational interactions using a quadtree (2D) or octree (3D):

```titrate
let sim = new NBodySimulator(0.01);

// Enable Barnes-Hut approximation
sim.setBarnesHut(true);
sim.setTheta(0.5);  // opening angle threshold (lower = more accurate, slower)

// Add many particles
for (i in 0..1000) {
    let x = (Math.random() - 0.5) * 100.0;
    let y = (Math.random() - 0.5) * 100.0;
    let z = (Math.random() - 0.5) * 100.0;
    sim.addParticle(new Particle(x, y, z, 0.0, 0.0, 0.0, 1.0));
}

sim.run(1000);  // 1000 steps
```

### Reading Simulation Results

```titrate
// Get all particle positions
let positions = sim.positions();
for (i in 0..positions.size()) {
    let p = positions.get(i);
    io::println("Body " + Integer.toString(i) + ": (" +
                Double.toString(p.x) + ", " +
                Double.toString(p.y) + ", " +
                Double.toString(p.z) + ")");
}

// Get total energy (kinetic + potential)
let ke = sim.kineticEnergy();
let pe = sim.potentialEnergy();
let total = ke + pe;
io::println("KE=" + Double.toString(ke) +
            " PE=" + Double.toString(pe) +
            " Total=" + Double.toString(total));
```

## Rigid Body Dynamics

For solid objects with rotation, the `RigidBody` class extends particle dynamics with inertia tensors, angular momentum, and torque.

```titrate
import tt::physics::RigidBody;
```

### Inertia Tensor

The inertia tensor describes how mass is distributed relative to the center of rotation:

```titrate
let body = new RigidBody(5.0);  // mass = 5.0 kg

// Set inertia for common shapes
body.setBoxInertia(2.0, 1.0, 0.5);       // width, height, depth
body.setSphereInertia(0.5);               // radius
body.setCylinderInertia(0.3, 1.0);        // radius, height

// Or set manually (3×3 symmetric matrix)
body.setInertia(2.0, 0.0, 0.0,
                0.0, 3.0, 0.0,
                0.0, 0.0, 4.0);
```

### Angular Momentum

```titrate
// Angular momentum L = I * ω (inertia tensor × angular velocity)
let Lx = body.angularMomentumX();
let Ly = body.angularMomentumY();
let Lz = body.angularMomentumZ();

// Set angular velocity directly
body.setAngularVelocity(0.0, 1.0, 0.0);  // rotating around Y axis at 1 rad/s
```

### Torque

Torque is the rotational analog of force — it changes angular momentum:

```titrate
// Apply torque directly
body.applyTorque(0.0, 0.0, 5.0);  // 5 N·m around Z axis

// Apply force at an offset (creates torque)
// τ = r × F
body.applyForceAt(10.0, 0.0, 0.0,    // force (N)
                  0.0, 1.0, 0.0);     // offset from center of mass (m)
```

### Quaternion Orientation

Rigid bodies use quaternions to represent orientation, avoiding gimbal lock:

```titrate
import tt::math::transform::Quaternion;

// Get current orientation
let q = body.orientation();
io::println("Orientation: (" + Double.toString(q.w) + ", " +
            Double.toString(q.x) + ", " +
            Double.toString(q.y) + ", " +
            Double.toString(q.z) + ")");

// Set orientation
body.setOrientation(1.0, 0.0, 0.0, 0.0);  // identity (no rotation)

// Convert to Euler angles for display
let euler = q.toEuler();
io::println("Euler: roll=" + Double.toString(euler.x) +
            " pitch=" + Double.toString(euler.y) +
            " yaw=" + Double.toString(euler.z));
```

### Integration Step

```titrate
let dt = 0.016;  // ~60 Hz timestep

// Integrate both linear and angular motion
body.integrate(dt);

// Read updated state
let pos = body.position();
let vel = body.velocity();
let angVel = body.angularVelocity();

io::println("Pos: (" + Double.toString(pos.x) + ", " +
            Double.toString(pos.y) + ", " + Double.toString(pos.z) + ")");
io::println("Vel: (" + Double.toString(vel.x) + ", " +
            Double.toString(vel.y) + ", " + Double.toString(vel.z) + ")");
io::println("AngVel: (" + Double.toString(angVel.x) + ", " +
            Double.toString(angVel.y) + ", " + Double.toString(angVel.z) + ")");
```

## Unit Systems

The `tt.units` module ensures dimensional consistency and provides conversion between unit systems — essential for physics simulations where mixing units leads to subtle bugs.

```titrate
import tt::units::Base;
import tt::units::Derived;
import tt::units::Constants;
```

### SI Units

```titrate
// SI base units (default)
let length = Base.meter(1.0);
let time = Base.second(1.0);
let mass = Base.kilogram(1.0);
let current = Base.ampere(1.0);
let temp = Base.kelvin(300.0);
```

### CGS Units

```titrate
// CGS: centimeter-gram-second
let length = Base.centimeter(100.0);  // 1 meter in CGS
let mass = Base.gram(1000.0);         // 1 kilogram in CGS
let time = Base.second(1.0);
```

### Natural Units

```titrate
// Natural units (ℏ = c = 1)
let energy = Base.electronvolt(1.0);  // 1 eV
let length = Base.electronvolt(1.0).inverse();  // ℏc / 1 eV
```

### Dimensional Analysis

```titrate
// The unit system prevents mismatched operations
let a = Base.meter(5.0);
let b = Base.second(2.0);
// let bad = a + b;  // Runtime error: cannot add m and s

// Multiplication creates derived units
let speed = a / b;  // 2.5 m/s
let force = Base.kilogram(3.0) * speed / b;  // 3.75 N

// Convert between unit systems
let distance = Base.meter(1000.0);
let inKm = distance.to(Derived.kilometer);  // 1.0 km
```

### Physical Constants

```titrate
let c = Constants.speedOfLight;      // 299792458 m/s
let h = Constants.planck;            // 6.62607015e-34 J·s
let G = Constants.gravitational;     // 6.67430e-11 m³/(kg·s²)
let k = Constants.boltzmann;         // 1.380649e-23 J/K
let na = Constants.avogadro;         // 6.02214076e23 /mol
let e = Constants.elementaryCharge;  // 1.602176634e-19 C
```

## End-to-End Example: Simulating a Solar System

This example simulates the inner solar system (Sun + 4 rocky planets) using gravitational N-body dynamics with Velocity Verlet integration.

```titrate
import tt::physics::Particle;
import tt::physics::NBodySimulator;
import tt::math::Math;
import tt::math::MathAdvanced;

public class CelestialBody {
    public string name;
    public Particle particle;
    public double initialDist;

    public fn init(n: string, p: Particle, dist: double) {
        this.name = n;
        this.particle = p;
        this.initialDist = dist;
    }
}

public fn main(): void {
    // Use astronomical units (AU) and years
    // G*M_sun = 4π² AU³/yr² (Kepler's third law)
    let GM = 4.0 * Math.PI() * Math.PI();

    let sim = new NBodySimulator(0.001);  // timestep in years (~8.7 hours)
    sim.setG(1.0);  // we factor G into the Sun's mass

    // Sun at the origin (mass = 1.0 in solar masses)
    let sun = new Particle(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    sim.addParticle(sun);

    // Mercury: 0.387 AU, orbital velocity ≈ 2π*0.387/0.241 ≈ 10.09 AU/yr
    let mercury = new Particle(0.387, 0.0, 0.0, 0.0, 10.09, 0.0, 1.66e-7);
    sim.addParticle(mercury);

    // Venus: 0.723 AU, v ≈ 2π*0.723/0.615 ≈ 7.39 AU/yr
    let venus = new Particle(0.723, 0.0, 0.0, 0.0, 7.39, 0.0, 2.45e-6);
    sim.addParticle(venus);

    // Earth: 1.0 AU, v ≈ 2π AU/yr
    let earth = new Particle(1.0, 0.0, 0.0, 0.0, 2.0 * Math.PI(), 0.0, 3.0e-6);
    sim.addParticle(earth);

    // Mars: 1.524 AU, v ≈ 2π*1.524/1.881 ≈ 5.09 AU/yr
    let mars = new Particle(1.524, 0.0, 0.0, 0.0, 5.09, 0.0, 3.23e-7);
    sim.addParticle(mars);

    let bodies = new ArrayList<string>();
    bodies.add("Sun");
    bodies.add("Mercury");
    bodies.add("Venus");
    bodies.add("Earth");
    bodies.add("Mars");

    // Simulate for 2 Earth years
    let totalSteps = 2000;  // 0.001 yr/step × 2000 = 2 years
    let reportInterval = 200;

    io::println("=== Solar System Simulation ===");
    io::println("Simulating " + Integer.toString(totalSteps) +
                " steps (2 Earth years)");
    io::println("");

    for (step in 0..totalSteps) {
        sim.step();

        // Report positions periodically
        if (step % reportInterval == 0) {
            let time = Double.parseDouble(Integer.toString(step)) * 0.001;
            io::println("t = " + Double.toString(time) + " yr:");

            let positions = sim.positions();
            for (i in 0..positions.size()) {
                let p = positions.get(i);
                let name = bodies.get(i);
                let dist = MathAdvanced.sqrt(p.x * p.x + p.y * p.y + p.z * p.z);
                io::println("  " + name + ": (" +
                            Double.toString(p.x) + ", " +
                            Double.toString(p.y) + ", " +
                            Double.toString(p.z) + ") dist=" +
                            Double.toString(dist) + " AU");
            }
            io::println("");
        }
    }

    // Final energy check
    let ke = sim.kineticEnergy();
    let pe = sim.potentialEnergy();
    io::println("Final energy — KE: " + Double.toString(ke) +
                " PE: " + Double.toString(pe) +
                " Total: " + Double.toString(ke + pe));
    io::println("(Energy should be approximately conserved)");
}
```

::: tip Choosing a timestep
The timestep `dt` must be small enough to resolve the fastest dynamics in your system. For orbits, a good rule of thumb is `dt < T_orbit / 100`, where `T_orbit` is the shortest orbital period. If energy drifts significantly over time, reduce `dt` or switch to a symplectic integrator.
:::

::: tip Barnes-Hut for large N
When simulating more than ~100 bodies (e.g., star clusters), enable Barnes-Hut with `sim.setBarnesHut(true)` to reduce the O(n²) pairwise force computation to O(n log n). The `theta` parameter controls the accuracy-speed tradeoff — `theta=0.5` is a good default.
:::

## What's Next?

- [Scientific Simulation](./simulation-guide) — discrete-event simulation and quantum mechanics
- [Scientific Computing](./scientific-computing) — NDArray and Matrix for numerical work
- [3D Graphics](./3d-graphics-guide) — geometry primitives and transforms
- [Standard Library](./stdlib) — full module reference
