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

- `fn init(symbol: string, atomicNumber: int, mass: double)` — create an atom
- `setPosition(px: double, py: double, pz: double): void` — set 3D coordinates
- `distanceTo(other: Atom): double` — Euclidean distance to another atom
- `radius(): double` — empirical atomic radius (Å)
- `electronegativity(): double` — Pauling electronegativity
- `electronConfiguration(): string` — electron configuration string
- `valence(): int` — valence electron count
- `vanDerWaalsRadius(): double` — Van der Waals radius (Å)
- `covalentRadius(): double` — covalent radius (Å)
- `ionizationEnergy(): double` — first ionization energy (eV)

**Static methods:**
- `Atom.getElement(symbol: string): Atom` — get atom from periodic table
- `Atom.getMass(symbol: string): double` — atomic mass
- `Atom.getRadius(symbol: string): double` — atomic radius
- `Atom.periodicTable(): HashMap<string, Atom>` — full periodic table (elements 1-118)

```titrate
let carbon = Atom.getElement("C");
carbon.setPosition(0.0, 0.0, 0.0);
io::println(carbon.electronegativity());  // 2.55
io::println(carbon.covalentRadius());     // 0.76
```

## Bond

Represents a chemical bond between two atoms by index.

- `fn init(atom1: int, atom2: int, bondType: string, order: int)` — create a bond
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

- `fn init(name: string)` — create an empty molecule
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
- `toXYZ(): string` — export to XYZ format
- `Molecule.fromXYZ(xyzString: string): Molecule` — import from XYZ
- `toString(): string` — molecular formula (Hill system)

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

- `fn init()` — create with default parameters
- `fn init(bk: double, br: double, ak: double, at: double, le: double, ls: double, ck: double)` — custom parameters
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
- `ForceField.partialCharge(symbol: string): double` — simplified partial charges

```titrate
let ff = new ForceField();
let e: double = ff.totalEnergy(water);
io::println("Total energy: " + Double.toString(e));
```

## Integrator (VerletIntegrator)

Velocity Verlet integrator for molecular dynamics with Berendsen thermostat.

- `fn init(dt: double, thermostat: string, targetTemp: double)` — create integrator
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

- `fn init(mol: Molecule, ff: ForceField, integrator: VerletIntegrator)` — create simulation
- `setOutputFrequency(n: int): void` — how often to print/write
- `setPBC(bx: double, by: double, bz: double): void` — enable periodic boundary conditions
- `run(steps: int): void` — run N steps
- `currentEnergy(): double` — current total energy
- `writeXYZFrame(): string` — current frame as XYZ
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

- `fn init(mol: Molecule, basisSize: int)` — create RHF calculator
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
let energy: double = rhf.compute();
io::println("RHF energy: " + Double.toString(energy));
io::println("Converged: " + Boolean.toString(rhf.isConverged()));
```

## PeriodicTable

Full element database loaded from `data/chem/periodic_table.json`.

- `PeriodicTable.getElement(symbol: string): Element` — get element by symbol
- `PeriodicTable.getElementByNumber(number: int): Element` — get element by atomic number
- `PeriodicTable.allElements(): ArrayList<Element>` — all 118 elements
- `PeriodicTable.alkaliMetals(): ArrayList<Element>` — alkali metal group
- `PeriodicTable.nobleGases(): ArrayList<Element>` — noble gas group
- `PeriodicTable.halogens(): ArrayList<Element>` — halogen group
- `PeriodicTable.transitionMetals(): ArrayList<Element>` — transition metals
- `PeriodicTable.lanthanides(): ArrayList<Element>` — lanthanide series
- `PeriodicTable.actinides(): ArrayList<Element>` — actinide series

## Element

- `fn init(symbol: string, name: string, number: int, mass: double)` — create element
- `getSymbol(): string`, `getName(): string`, `getNumber(): int`, `getMass(): double`
- `getElectronegativity(): double`, `getElectronConfig(): string`
- `getAtomicRadius(): double`, `getIonizationEnergy(): double`

## ReactionBalancer

- `ReactionBalancer.balance(equation: string): string` — balance a chemical equation
- `ReactionBalancer.oxidationState(compound: string): HashMap<string, int>` — compute oxidation states
- `ReactionBalancer.isRedox(equation: string): bool` — check if reaction is redox

## Thermochemistry

- `Thermochemistry.enthalpyOfFormation(compound: string): double` — standard enthalpy of formation
- `Thermochemistry.hessLaw(reactions: ArrayList<string>): double` — Hess's law calculation
- `Thermochemistry.gibbsFreeEnergy(enthalpy: double, entropy: double, temperature: double): double` — ΔG = ΔH - TΔS
- `Thermochemistry.vanTHoff(keq1: double, t1: double, keq2: double, t2: double): double` — Van 't Hoff equation

## Kinetics

- `Kinetics.zeroOrderRate(k: double, t: double): double` — [A] = [A]₀ - kt
- `Kinetics.firstOrderRate(k: double, t: double, a0: double): double` — [A] = [A]₀e^(-kt)
- `Kinetics.secondOrderRate(k: double, t: double, a0: double): double` — 1/[A] = 1/[A]₀ + kt
- `Kinetics.halfLife(k: double, order: int): double` — half-life for given order
- `Kinetics.arrheniusRate(a: double, ea: double, t: double): double` — Arrhenius rate constant

## Electrochemistry

- `Electrochemistry.nernstPotential(e0: double, n: int, q: double, t: double): double` — Nernst equation
- `Electrochemistry.cellPotential(cathode: double, anode: double): double` — E°cell = E°cathode - E°anode
- `Electrochemistry.gibbsFromPotential(e: double, n: int): double` — ΔG = -nFE
- `Electrochemistry.faradayElectrolysis(current: double, time: double, n: int): double` — Faraday's law

## Spectroscopy

Spectroscopic calculations for UV-Vis, IR, NMR, and Mass Spectrometry.

### UVVis

Ultraviolet-Visible spectroscopy calculations based on the Beer-Lambert law.

- `UVVis.absorbance(transmittance: double): double` — A = -log10(T)
- `UVVis.transmittance(absorbance: double): double` — T = 10^(-A)
- `UVVis.beerLambert(absorptivity: double, pathLength: double, concentration: double): double` — A = ε·l·c
- `UVVis.concentration(absorbance: double, absorptivity: double, pathLength: double): double` — c = A / (ε·l)
- `UVVis.molarAbsorptivity(absorbance: double, pathLength: double, concentration: double): double` — ε = A / (l·c)
- `UVVis.lambdaMax(wavelengths: ArrayList<double>, absorbances: ArrayList<double>): double` — wavelength of maximum absorbance

```titrate
let a: double = UVVis.beerLambert(2235.0, 1.0, 0.0001);
io::println("Absorbance: " + Double.toString(a));  // 0.2235
let c: double = UVVis.concentration(a, 2235.0, 1.0);
io::println("Concentration: " + Double.toString(c));  // 0.0001
```

### IR

Infrared spectroscopy calculations including wavenumber conversion and peak analysis.

- `IR.wavenumber(wavelengthNm: double): double` — convert wavelength (nm) to wavenumber (cm⁻¹)
- `IR.wavelength(wavenumber: double): double` — convert wavenumber (cm⁻¹) to wavelength (nm)
- `IR.frequency(wavenumber: double): double` — frequency (Hz) from wavenumber
- `IR.energy(wavenumber: double): double` — photon energy (J) from wavenumber
- `IR.peakPositions(wavenumbers: ArrayList<double>, intensities: ArrayList<double>, threshold: double): ArrayList<double>` — detect peak positions above threshold

```titrate
let wn: double = IR.wavenumber(2500.0);  // 4000.0 cm⁻¹
io::println("Wavenumber: " + Double.toString(wn));
```

### NMR

Nuclear Magnetic Resonance spectroscopy calculations.

- `NMR.chemicalShift(sampleFreq: double, referenceFreq: double): double` — δ (ppm) = (ν_sample - ν_ref) / ν_ref × 10⁶
- `NMR.larmorFrequency(gyromagneticRatio: double, fieldStrength: double): double` — ω = γ·B₀
- `NMR.couplingConstant(spectrum: ArrayList<double>, peak1Index: int, peak2Index: int): double` — J coupling constant (Hz)
- `NMR.multiplicity(nEquivalentNeighbors: int): int` — n+1 rule for peak count
- `NMR.integration(peaks: ArrayList<double>, startIndex: int, endIndex: int): double` — integrate peak area

```titrate
let delta: double = NMR.chemicalShift(400.000128e6, 400.000000e6);
io::println("Chemical shift: " + Double.toString(delta) + " ppm");  // 0.32 ppm
```

### MassSpec

Mass spectrometry calculations including mass-to-charge ratio and isotope patterns.

- `MassSpec.massToCharge(mass: double, charge: int): double` — m/z = mass / charge
- `MassSpec.charge(mass: double, mz: double): int` — charge state from mass and m/z
- `MassSpec.isotopePattern(monoisotopicMass: double, numPeaks: int): ArrayList<double>` — theoretical isotope distribution
- `MassSpec.resolution(mass: double, deltaM: double): double` — resolving power R = m / Δm
- `MassSpec.basePeak(masses: ArrayList<double>, intensities: ArrayList<double>): double` — mass at maximum intensity
- `MassSpec.monoisotopicMass(isotopeMasses: ArrayList<double>, abundances: ArrayList<double>): double` — most abundant isotope mass
