# physics

The `tt.physics` module provides physics simulation tools including particle dynamics, force fields, N-body simulation, rigid body mechanics, quantum wave functions, and unit system conversions.

```titrate
import tt.physics.Particle;
import tt.physics.ForceField;
import tt.physics.NBodySimulator;
import tt.physics.RigidBody;
import tt.physics.WaveFunction;
import tt.physics.UnitSystem;
```

## Particle

A point particle with mass, charge, position, velocity, and acceleration. Supports force integration via Euler method.

- `fn init(m: double, q: double)` — create particle with mass and charge
- `setPosition(px: double, py: double, pz: double): void` — set 3D position
- `setVelocity(vx: double, vy: double, vz: double): void` — set velocity
- `setAcceleration(ax: double, ay: double, az: double): void` — set acceleration
- `applyForce(fx: double, fy: double, fz: double): void` — accumulate force (F = ma)
- `update(dt: double): void` — integrate velocity and position, reset acceleration
- `kineticEnergy(): double` — kinetic energy (½mv²)
- `momentum(): double` — momentum magnitude (mv)
- `distanceTo(other: Particle): double` — Euclidean distance to another particle
- `speed(): double` — speed magnitude

```titrate
let p = new Particle(1.0, 0.0);
p.setPosition(0.0, 0.0, 0.0);
p.setVelocity(1.0, 0.0, 0.0);
p.applyForce(0.0, 9.81, 0.0);
p.update(0.01);
io::println(p.kineticEnergy());
io::println(p.speed());
```

## ForceField

Force calculations for gravitational, electromagnetic, spring (Hooke's law), Lennard-Jones, and Coulomb interactions.

### Vec3

3D vector used by force functions.

- `fn init(x: double, y: double, z: double)` — create vector
- `magnitude(): double` — vector magnitude
- `add(other: Vec3): Vec3` — vector addition
- `scale(s: double): Vec3` — scalar multiplication

### Force Functions

- `gravitationalForce(m1: double, m2: double, r: Vec3, G: double): Vec3` — gravitational force between two masses
- `coulombForce(q1: double, q2: double, r: Vec3, k: double): Vec3` — Coulomb force between two charges
- `lorentzForce(q: double, e: Vec3, v: Vec3, b: Vec3): Vec3` — Lorentz force (F = q(E + v×B))
- `springForce(displacement: Vec3, k: double): Vec3` — Hooke's law spring force
- `springForceWithRest(displacement: Vec3, k: double, restLength: double): Vec3` — spring force with rest length
- `lennardJonesForce(r: Vec3, epsilon: double, sigma: double): Vec3` — Lennard-Jones force
- `lennardJonesPotential(dist: double, epsilon: double, sigma: double): double` — Lennard-Jones potential energy

```titrate
import tt.physics.ForceField;

let r = new ForceField.Vec3(1.0, 0.0, 0.0);
let fg = ForceField.gravitationalForce(10.0, 5.0, r, 6.674e-11);
io::println(fg.magnitude());

let displacement = new ForceField.Vec3(0.5, 0.0, 0.0);
let fs = ForceField.springForce(displacement, 100.0);
io::println(fs.x);  // -50.0

let lj = ForceField.lennardJonesPotential(3.4, 0.01, 3.4);
io::println(lj);
```

## NBodySimulator

N-body gravitational simulation with leapfrog and velocity Verlet integration.

### Body

A body in the N-body simulation with mass, position, velocity, and acceleration.

- `fn init(m: double, px: double, py: double, pz: double)` — create body with mass and position

### NBodySimulator Class

- `fn init(g: double, soft: double, timeStep: double)` — create simulator with gravitational constant, softening parameter, and time step
- `addBody(body: Body): void` — add a body to the simulation
- `computeAccelerations(): void` — compute pairwise gravitational accelerations
- `stepLeapfrog(): void` — advance one step using leapfrog (kick-drift-kick) integration
- `stepVerlet(): void` — advance one step using velocity Verlet integration
- `step(): void` — advance one step (uses the configured integrator, default: leapfrog)
- `run(steps: int): void` — run N simulation steps
- `totalEnergy(): double` — compute total energy (kinetic + potential)

```titrate
import tt.physics.NBodySimulator;

let sim = new NBodySimulator(6.674e-11, 0.01, 0.001);
sim.addBody(new NBodySimulator.Body(1e10, 0.0, 0.0, 0.0));
sim.addBody(new NBodySimulator.Body(1e10, 10.0, 0.0, 0.0));
sim.run(1000);
io::println(sim.totalEnergy());
```

## RigidBody

Rigid body dynamics with inertia tensor, angular momentum, torque, and quaternion orientation.

- `fn init(m: double)` — create rigid body with mass
- `setPosition(px: double, py: double, pz: double): void` — set position
- `setVelocity(vx: double, vy: double, vz: double): void` — set linear velocity
- `setAngularVelocity(wx: double, wy: double, wz: double): void` — set angular velocity
- `setInertiaTensor(ixx: double, iyy: double, izz: double): void` — set diagonal inertia tensor
- `setBoxInertia(width: double, height: double, depth: double): void` — compute inertia tensor for a box
- `setSphereInertia(radius: double): void` — compute inertia tensor for a sphere
- `applyForce(fx: double, fy: double, fz: double, dt: double): void` — apply force over time step
- `applyTorque(tx: double, ty: double, tz: double, dt: double): void` — apply torque over time step
- `update(dt: double): void` — integrate position and quaternion orientation
- `kineticEnergy(): double` — total kinetic energy (translational + rotational)
- `angularMomentum(): double` — angular momentum magnitude

```titrate
import tt.physics.RigidBody;

let box = new RigidBody(5.0);
box.setPosition(0.0, 10.0, 0.0);
box.setBoxInertia(2.0, 1.0, 3.0);
box.applyForce(0.0, -49.05, 0.0, 0.016);  // gravity
box.applyTorque(1.0, 0.0, 0.0, 0.016);
box.update(0.016);
io::println(box.kineticEnergy());
```

## WaveFunction

Quantum mechanics wave function representation with probability density, expectation values, uncertainty, and time evolution.

- `fn init(n: int, spacing: double, start: double)` — create wave function with N grid points, spacing dx, and starting position x0
- `size(): int` — number of grid points
- `set(i: int, re: double, im: double): void` — set real and imaginary parts at index
- `getReal(i: int): double` — get real part at index
- `getImag(i: int): double` — get imaginary part at index
- `getProbability(i: int): double` — probability density |ψ|² at index
- `normalize(): void` — normalize the wave function
- `norm(): double` — compute the norm (∫|ψ|²dx)
- `probabilityDensity(): ArrayList<double>` — probability density at all grid points
- `expectationX(): double` — expectation value ⟨x⟩
- `uncertaintyX(): double` — position uncertainty Δx
- `expectationP(): double` — momentum expectation value ⟨p⟩ (finite difference approximation)
- `initGaussian(center: double, sigma: double, k0: double): void` — initialize as a Gaussian wave packet
- `evolve(dt: double, potential: ArrayList<double>, hbar: double, mass: double): void` — time evolution using split-operator method

```titrate
import tt.physics.WaveFunction;

let psi = new WaveFunction(500, 0.01, -2.5);
psi.initGaussian(0.0, 0.5, 10.0);  // center, width, wave number
io::println(psi.expectationX());     // ≈ 0.0
io::println(psi.uncertaintyX());     // ≈ 0.5

// Time evolution in a potential
let potential = new ArrayList<double>();
var i: int = 0;
while (i < 500) {
    potential.add(0.0);  // free particle
    i = i + 1;
}
psi.evolve(0.001, potential, 1.0, 1.0);
io::println(psi.expectationX());
```

## UnitSystem

Unit conversion between SI, CGS, and natural unit systems. Conversion factors are loaded from `data/units/conversions.json`.

- `convert(value: double, fromUnit: string, toUnit: string, category: string): double` — convert a value between units in a category (e.g. `"length"`, `"mass"`, `"temperature"`)
- `getCategories(): ArrayList<string>` — list available unit categories
- `getUnits(category: string): ArrayList<string>` — list units in a category
- `getBaseUnit(category: string): string` — get the base unit for a category
- `siToCgs(value: double, quantity: string): double` — convert SI to CGS
- `cgsToSi(value: double, quantity: string): double` — convert CGS to SI
- `siToNatural(value: double, quantity: string, hbar: double, c: double): double` — convert SI to natural units (ℏ = c = 1)

```titrate
import tt.physics.UnitSystem;

// Unit conversion
let cm: double = UnitSystem.convert(1.0, "m", "cm", "length");
io::println(Double.toString(cm));  // 100.0

// Temperature conversion
let f: double = UnitSystem.convert(100.0, "C", "F", "temperature");
io::println(Double.toString(f));  // 212.0

// SI to CGS
let dyne: double = UnitSystem.siToCgs(1.0, "force");
io::println(Double.toString(dyne));  // 100000.0

// List available categories
let cats = UnitSystem.getCategories();
```
