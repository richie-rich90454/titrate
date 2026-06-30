---
title: chem
description: Computational chemistry tools for Titrate — atoms, molecules, force fields, MD, and quantum chemistry.
---

# chem

The `tt.chem` module provides computational chemistry tools: atoms, molecules, force fields, molecular dynamics, and quantum chemistry (RHF). It also includes thermochemistry, kinetics, electrochemistry, and periodic table utilities.

```titrate
import tt::chem::Atom;
import tt::chem::Molecule;
import tt::chem::ForceField;
import tt::chem::MD;
import tt::chem::PeriodicTable;
```

## Atom

Represents a chemical atom with symbol, atomic number, mass, charge, and 3D position.

- `fn init(symbol: string, atomicNumber: int, mass: double)`
- `setPosition(px: double, py: double, pz: double): void`
- `distanceTo(other: Atom): double`
- `radius(): double`
- `electronegativity(): double`
- `electronConfiguration(): string`
- `valence(): int`
- `vanDerWaalsRadius(): double`
- `covalentRadius(): double`
- `ionizationEnergy(): double`
- `toString(): string`

### Factory Functions

- `Atom.hydrogen(x: double, y: double, z: double): Atom`
- `Atom.oxygen(x: double, y: double, z: double): Atom`
- `Atom.carbon(x: double, y: double, z: double): Atom`
- `Atom.nitrogen(x: double, y: double, z: double): Atom`
- `Atom.getElement(symbol: string): Atom`
- `Atom.getMass(symbol: string): double`
- `Atom.getRadius(symbol: string): double`
- `Atom.periodicTable(): HashMap<string, Atom>`

```titrate
let carbon: Atom = Atom.getElement("C");
carbon.setPosition(0.0, 0.0, 0.0);
io::println(carbon.electronegativity());  // 2.55
```

## Molecule

Represents a molecule with atoms and bonds.

- `fn init(name: string)`
- `addAtom(atom: Atom): int` — returns the atom index
- `addBond(bond: Bond): void`
- `getAtom(i: int): Atom`
- `getBond(i: int): Bond`
- `atomCount(): int`
- `bondCount(): int`
- `removeAtom(index: int): void`
- `removeBond(index: int): void`
- `findBond(a1: int, a2: int): int`
- `getBondsOf(atomIndex: int): ArrayList<Bond>`
- `getNeighborsOf(atomIndex: int): ArrayList<int>`
- `centroid(): ArrayList<double>`
- `centerOfMass(): ArrayList<double>`
- `translate(dx: double, dy: double, dz: double): void`
- `molecularWeight(): double`
- `toXYZ(): string`
- `toString(): string` — molecular formula in Hill order

- `Molecule.fromXYZ(xyzString: string): Molecule`

```titrate
let water: Molecule = new Molecule("water");
water.addAtom(Atom.oxygen(0.0, 0.0, 0.0));
water.addAtom(Atom.hydrogen(0.96, 0.0, 0.0));
water.addAtom(Atom.hydrogen(-0.24, 0.93, 0.0));
water.addBond(new Bond(0, 1, "single", 1));
water.addBond(new Bond(0, 2, "single", 1));

io::println(water.toString());         // "H2O"
io::println(water.molecularWeight());  // ~18.015
```

## Bond

Represents a chemical bond between two atoms by index.

- `fn init(atom1: int, atom2: int, bondType: string, order: int)`
- `length(atoms: ArrayList<Atom>): double`
- `angle(other: Bond, atoms: ArrayList<Atom>): double`
- `dihedral(b2: Bond, b3: Bond, atoms: ArrayList<Atom>): double`
- `isAromatic(): bool`

## ForceField

Molecular mechanics force field with harmonic bonds, angles, Lennard-Jones, Coulomb, torsion, and improper terms.

- `fn init()` — default parameters
- `fn init(bk: double, br: double, ak: double, at: double, le: double, ls: double, ck: double)` — custom parameters
- `bondEnergy(mol: Molecule): double`
- `angleEnergy(mol: Molecule): double`
- `lennardJonesEnergy(mol: Molecule): double`
- `coulombEnergy(mol: Molecule): double`
- `torsionEnergy(mol: Molecule): double`
- `improperEnergy(mol: Molecule): double`
- `totalEnergy(mol: Molecule): double`
- `gradient(mol: Molecule): ArrayList<ArrayList<double>>`

### Static Helpers

- `ForceField.computeAngle(a1: Atom, center: Atom, a2: Atom): double`
- `ForceField.computeDihedral(a1: Atom, a2: Atom, a3: Atom, a4: Atom): double`
- `ForceField.computeImproper(a1: Atom, center: Atom, a2: Atom, a3: Atom): double`
- `ForceField.partialCharge(symbol: string): double`

```titrate
let ff: ForceField = new ForceField();
let e: double = ff.totalEnergy(water);
io::println("Total energy: " + Double.toString(e));
```

## MD (Molecular Dynamics)

`MDSimulation` drives molecular dynamics simulations.

- `fn init(mol: Molecule, ff: ForceField, integ: VerletIntegrator)`
- `setOutputFrequency(n: int): void`
- `setPBC(bx: double, by: double, bz: double): void` — periodic boundary conditions
- `applyPBC(): void`
- `minimumImageDistance(a1: Atom, a2: Atom): double`
- `writeXYZFrame(): string`
- `run(steps: int): void`
- `currentEnergy(): double`

```titrate
let integ: VerletIntegrator = new VerletIntegrator(0.001, "berendsen", 300.0);
let sim: MDSimulation = new MDSimulation(water, ff, integ);
sim.setOutputFrequency(100);
sim.run(1000);
```

## PeriodicTable

Full element database loaded from `data/chem/periodic_table.json`.

- `PeriodicTable.getElement(symbol: string): Element`
- `PeriodicTable.getElementByNumber(number: int): Element`
- `PeriodicTable.allElements(): ArrayList<Element>`
- `PeriodicTable.alkaliMetals(): ArrayList<Element>`
- `PeriodicTable.nobleGases(): ArrayList<Element>`
- `PeriodicTable.halogens(): ArrayList<Element>`
- `PeriodicTable.transitionMetals(): ArrayList<Element>`
- `PeriodicTable.lanthanides(): ArrayList<Element>`
- `PeriodicTable.actinides(): ArrayList<Element>`

## ReactionBalancer

- `ReactionBalancer.balance(equation: string): string`
- `ReactionBalancer.oxidationState(compound: string): HashMap<string, int>`
- `ReactionBalancer.isRedox(equation: string): bool`

## Thermochemistry

- `Thermochemistry.enthalpyOfFormation(compound: string): double`
- `Thermochemistry.hessLaw(reactions: ArrayList<string>): double`
- `Thermochemistry.gibbsFreeEnergy(enthalpy: double, entropy: double, temperature: double): double`
- `Thermochemistry.vanTHoff(keq1: double, t1: double, keq2: double, t2: double): double`

## Kinetics

- `Kinetics.zeroOrderRate(k: double, t: double): double`
- `Kinetics.firstOrderRate(k: double, t: double, a0: double): double`
- `Kinetics.secondOrderRate(k: double, t: double, a0: double): double`
- `Kinetics.halfLife(k: double, order: int): double`
- `Kinetics.arrheniusRate(a: double, ea: double, t: double): double`

## Electrochemistry

- `Electrochemistry.nernstPotential(e0: double, n: int, q: double, t: double): double`
- `Electrochemistry.cellPotential(cathode: double, anode: double): double`
- `Electrochemistry.gibbsFromPotential(e: double, n: int): double`
- `Electrochemistry.faradayElectrolysis(current: double, time: double, n: int): double`

## RHF

Restricted Hartree-Fock quantum chemistry calculation.

- `fn init(mol: Molecule, basisSize: int)`
- `compute(): double` — run SCF and return total energy
- `getTotalEnergy(): double`
- `getOrbitalEnergies(): ArrayList<double>`
- `getDensityMatrix(): ArrayList<ArrayList<double>>`
- `fockMatrix(): ArrayList<ArrayList<double>>`
- `isConverged(): bool`
- `getIterations(): int`
- `computeNuclearRepulsion(): double`
- `totalElectrons(): int`

```titrate
let rhf: RHF = new RHF(water, 7);
let energy: double = rhf.compute();
io::println("RHF energy: " + Double.toString(energy));
io::println("Converged: " + Boolean.toString(rhf.isConverged()));
```
