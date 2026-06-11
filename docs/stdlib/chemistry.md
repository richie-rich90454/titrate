# chemistry

The `tt.chem` module provides computational chemistry tools including molecular modeling, force fields, molecular dynamics, and quantum chemistry (RHF).

```titrate
import tt.chem.Atom;
import tt.chem.Bond;
import tt.chem.Molecule;
import tt.chem.ForceField;
import tt.chem.VerletIntegrator;
import tt.chem.MDSimulation;
import tt.chem.RHF;
```

## Atom

Represents a chemical atom with symbol, atomic number, mass, charge, and 3D position.

- `Atom(symbol: String, atomicNumber: int, mass: double)` — create an atom
- `setPosition(px: double, py: double, pz: double): void` — set 3D coordinates
- `distanceTo(other: Atom): double` — Euclidean distance to another atom
- `radius(): double` — empirical atomic radius (Å)
- `electronegativity(): double` — Pauling electronegativity
- `electronConfiguration(): String` — electron configuration string
- `valence(): int` — valence electron count
- `vanDerWaalsRadius(): double` — Van der Waals radius (Å)
- `covalentRadius(): double` — covalent radius (Å)
- `ionizationEnergy(): double` — first ionization energy (eV)

**Static methods:**
- `Atom.getElement(symbol: String): Atom` — get atom from periodic table
- `Atom.getMass(symbol: String): double` — atomic mass
- `Atom.getRadius(symbol: String): double` — atomic radius
- `Atom.periodicTable(): HashMap<String, Atom>` — full periodic table (elements 1-118)

```titrate
let carbon = Atom.getElement("C");
carbon.setPosition(0.0, 0.0, 0.0);
io::println(carbon.electronegativity());  // 2.55
io::println(carbon.covalentRadius());     // 0.76
```

## Bond

Represents a chemical bond between two atoms by index.

- `Bond(atom1: int, atom2: int, bondType: String, order: int)` — create a bond
- `length(atoms: ArrayList<Atom>): double` — bond length between the two atoms
- `angle(other: Bond, atoms: ArrayList<Atom>): double` — angle at shared atom (radians)
- `dihedral(b2: Bond, b3: Bond, atoms: ArrayList<Atom>): double` — dihedral angle (radians)
- `isAromatic(): bool` — check if aromatic bond

```titrate
let b = new Bond(0, 1, "single", 1);
// bond length depends on atom positions
```

## Molecule

Represents a molecule with atoms and bonds.

- `Molecule(name: String)` — create an empty molecule
- `addAtom(atom: Atom): int` — add atom, returns index
- `addBond(bond: Bond): void` — add a bond
- `getAtom(i: int): Atom` — get atom by index
- `getBond(i: int): Bond` — get bond by index
- `atomCount(): int`, `bondCount(): int`
- `removeAtom(index: int): void` — remove atom and its bonds
- `removeBond(index: int): void` — remove a bond
- `findBond(a1: int, a2: int): int` — find bond index, or -1
- `getBondsOf(atomIndex: int): ArrayList<Bond>` — bonds involving an atom
- `getNeighborsOf(atomIndex: int): ArrayList<int>` — bonded atom indices
- `centroid(): ArrayList<double>` — geometric center (cx, cy, cz)
- `centerOfMass(): ArrayList<double>` — mass-weighted center
- `translate(dx: double, dy: double, dz: double): void` — shift all atoms
- `molecularWeight(): double` — total molecular weight
- `toXYZ(): String` — export to XYZ format
- `Molecule.fromXYZ(xyzString: String): Molecule` — import from XYZ
- `toString(): String` — molecular formula (Hill system)

```titrate
let water = new Molecule("water");
let o = Atom.getElement("O");
o.setPosition(0.0, 0.0, 0.0);
let h1 = Atom.getElement("H");
h1.setPosition(0.96, 0.0, 0.0);
let h2 = Atom.getElement("H");
h2.setPosition(-0.24, 0.93, 0.0);
water.addAtom(o);
water.addAtom(h1);
water.addAtom(h2);
water.addBond(new Bond(0, 1, "single", 1));
water.addBond(new Bond(0, 2, "single", 1));
io::println(water.toString());  // "H2O"
```

## ForceField

Molecular mechanics force field with harmonic bonds, angles, Lennard-Jones, Coulomb, torsion, and improper terms.

- `ForceField()` — create with default parameters
- `ForceField(bk, br, ak, at, le, ls, ck)` — custom parameters
- `bondEnergy(mol: Molecule): double` — harmonic bond energy
- `angleEnergy(mol: Molecule): double` — harmonic angle energy
- `lennardJonesEnergy(mol: Molecule): double` — LJ 6-12 potential
- `coulombEnergy(mol: Molecule): double` — Coulomb electrostatic energy
- `torsionEnergy(mol: Molecule): double` — dihedral torsion energy
- `improperEnergy(mol: Molecule): double` — improper torsion energy
- `totalEnergy(mol: Molecule): double` — sum of all terms
- `gradient(mol: Molecule): ArrayList<double>` — analytical gradient (3N values)
- `computeForces(mol: Molecule): ArrayList<double>` — forces via finite differences

**Static helpers:**
- `ForceField.computeAngle(a1, center, a2): double` — angle in radians
- `ForceField.computeDihedral(a1, a2, a3, a4): double` — dihedral in radians
- `ForceField.computeImproper(a1, center, a2, a3): double` — out-of-plane angle
- `ForceField.partialCharge(symbol: String): double` — simplified partial charges

```titrate
let ff = new ForceField();
double e = ff.totalEnergy(water);
io::println("Total energy: " + e.toString());
```

## Integrator (VerletIntegrator)

Velocity Verlet integrator for molecular dynamics with Berendsen thermostat.

- `VerletIntegrator(dt: double, thermostat: String, targetTemp: double)` — create integrator
- `setTimeStep(newDt: double): void` — change time step
- `initializeVelocities(mol: Molecule): void` — Maxwell-Boltzmann initialization
- `step(mol: Molecule, ff: ForceField): void` — single MD step
- `getStepCount(): int` — steps taken
- `totalSimulationTime(): double` — total simulated time

**Static:**
- `VerletIntegrator.kineticEnergy(mol, velocities): double`
- `VerletIntegrator.kineticTemperature(mol, velocities): double`

## MD (MDSimulation)

Molecular dynamics simulation driver.

- `MDSimulation(mol: Molecule, ff: ForceField, integrator: VerletIntegrator)` — create simulation
- `setOutputFrequency(n: int): void` — how often to print/write
- `setPBC(bx: double, by: double, bz: double): void` — enable periodic boundary conditions
- `run(steps: int): void` — run N steps
- `currentEnergy(): double` — current total energy
- `writeXYZFrame(): String` — current frame as XYZ
- `minimumImageDistance(a1: Atom, a2: Atom): double` — PBC-aware distance

```titrate
let ff = new ForceField();
let integ = new VerletIntegrator(0.001, "berendsen", 300.0);
let sim = new MDSimulation(water, ff, integ);
sim.setOutputFrequency(100);
sim.run(1000);
```

## RHF

Restricted Hartree-Fock quantum chemistry calculation.

- `RHF(mol: Molecule, basisSize: int)` — create RHF calculator
- `compute(): double` — run SCF and return total energy
- `getTotalEnergy(): double` — total energy
- `getOrbitalEnergies(): ArrayList<double>` — orbital energies
- `getDensityMatrix(): ArrayList<ArrayList<double>>` — density matrix
- `fockMatrix(): ArrayList<ArrayList<double>>` — Fock matrix
- `isConverged(): bool` — whether SCF converged
- `getIterations(): int` — number of SCF iterations
- `computeNuclearRepulsion(): double` — nuclear repulsion energy
- `totalElectrons(): int` — total electron count

```titrate
let rhf = new RHF(water, 7);  // 7 basis functions for STO-3G H2O
double energy = rhf.compute();
io::println("RHF energy: " + energy.toString());
io::println("Converged: " + rhf.isConverged().toString());
```
