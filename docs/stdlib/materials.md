# materials

The `tt.materials` module provides materials science tools including crystal structure analysis, X-ray diffraction simulation, binary phase diagrams, and elasticity calculations.

```titrate
import tt.materials.CrystalStructure;
import tt.materials.XRayDiffraction;
import tt.materials.PhaseDiagram;
import tt.materials.Elasticity;
```

## CrystalStructure

Crystal structure analysis with Bravais lattice unit cells, Miller indices, and space group lookup from `data/materials/space_groups.json`.

### UnitCell

A crystallographic unit cell defined by lattice parameters a, b, c, α, β, γ.

- `fn init(a: double, b: double, c: double, al: double, be: double, ga: double)` — create unit cell with lengths (Å) and angles (degrees)
- `volume(): double` — compute unit cell volume

### MillerIndices

Crystallographic plane indices (hkl) with d-spacing calculation.

- `fn init(h: int, k: int, l: int)` — create Miller indices
- `dSpacing(cell: UnitCell): double` — compute interplanar d-spacing for a given unit cell
- `toString(): string` — string representation `"(h k l)"`

### CrystalStructure Functions

- `getSpaceGroup(number: int): HashMap<string, string>` — look up space group by number (1–230), returns symbol, crystal system, and lattice type
- `getSpaceGroupBySymbol(symbol: string): int` — look up space group number by Hermann-Mauguin symbol
- `cubicCell(a: double): UnitCell` — create a cubic unit cell
- `tetragonalCell(a: double, c: double): UnitCell` — create a tetragonal unit cell
- `orthorhombicCell(a: double, b: double, c: double): UnitCell` — create an orthorhombic unit cell
- `hexagonalCell(a: double, c: double): UnitCell` — create a hexagonal unit cell

```titrate
import tt.materials.CrystalStructure;

let cell = CrystalStructure.cubicCell(3.615);  // Cu FCC
io::println(cell.volume());  // ≈ 47.24

let hkl = new CrystalStructure.MillerIndices(1, 1, 1);
io::println(hkl.dSpacing(cell));  // d-spacing for (111)
io::println(hkl.toString());      // "(1 1 1)"

let sg = CrystalStructure.getSpaceGroup(225);
io::println(sg.get("symbol"));         // "Fm-3m"
io::println(sg.get("crystal_system")); // "cubic"

let hex = CrystalStructure.hexagonalCell(2.95, 4.68);
io::println(hex.volume());
```

## XRayDiffraction

X-ray diffraction simulation with Bragg's law, atomic scattering factors, structure factor calculation, and powder diffraction intensity. Scattering factors are loaded from `data/materials/scattering_factors.json`.

### AtomPosition

Atomic position in a crystal structure (fractional coordinates).

- `fn init(sym: string, px: double, py: double, pz: double)` — create atom position with element symbol and fractional coordinates

### XRayDiffraction Functions

- `braggAngle(dSpacing: double, wavelength: double, n: int): double` — compute Bragg angle θ from d-spacing (returns -1.0 if no solution)
- `braggDSpacing(theta: double, wavelength: double, n: int): double` — compute d-spacing from Bragg angle
- `scatteringFactor(symbol: string, sinThetaOverLambda: double): double` — atomic scattering factor using Cromer-Mann coefficients
- `structureFactor(h: int, k: int, l: int, atoms: ArrayList<AtomPosition>, sinThetaOverLambda: double): double` — structure factor |F(hkl)| magnitude
- `powderIntensity(fHkl: double, theta: double, multiplicity: int, lpFactor: double): double` — powder diffraction intensity with Lorentz-polarization factor

```titrate
import tt.materials.XRayDiffraction;

// Bragg's law
let theta: double = XRayDiffraction.braggAngle(2.08, 1.5406, 1);
io::println(Double.toString(theta));  // Bragg angle in radians

// Scattering factor for Cu
let f: double = XRayDiffraction.scatteringFactor("Cu", 0.3);
io::println(Double.toString(f));

// Structure factor for FCC Cu (4 atoms)
let atoms = new ArrayList<XRayDiffraction.AtomPosition>();
atoms.add(new XRayDiffraction.AtomPosition("Cu", 0.0, 0.0, 0.0));
atoms.add(new XRayDiffraction.AtomPosition("Cu", 0.5, 0.5, 0.0));
atoms.add(new XRayDiffraction.AtomPosition("Cu", 0.5, 0.0, 0.5));
atoms.add(new XRayDiffraction.AtomPosition("Cu", 0.0, 0.5, 0.5));
let fHkl: double = XRayDiffraction.structureFactor(1, 1, 1, atoms, 0.3);
io::println(Double.toString(fHkl));

// Powder diffraction intensity
let intensity: double = XRayDiffraction.powderIntensity(fHkl, theta, 8, 1.0);
io::println(Double.toString(intensity));
```

## PhaseDiagram

Binary phase diagram modeling with liquidus/solidus curves, eutectic and peritectic points, lever rule, and cooling curves.

### PhasePoint

A point on a phase boundary.

- `fn init(t: double, c: double, p: string)` — create phase point with temperature, composition, and phase name

### BinaryPhaseDiagram

A binary A-B phase diagram with liquidus and solidus curves.

- `fn init(a: string, b: string)` — create diagram for two components
- `addLiquidusPoint(temp: double, comp: double): void` — add a point to the liquidus curve
- `addSolidusPoint(temp: double, comp: double): void` — add a point to the solidus curve
- `setEutectic(temp: double, comp: double): void` — set eutectic point
- `setPeritectic(temp: double, comp: double): void` — set peritectic point
- `public string componentA` — name of component A
- `public string componentB` — name of component B
- `public double eutecticTemp` — eutectic temperature
- `public double eutecticComp` — eutectic composition
- `public double peritecticTemp` — peritectic temperature
- `public double peritecticComp` — peritectic composition

### PhaseDiagram Functions

- `leverRule(composition: double, compAlpha: double, compBeta: double): double` — compute fraction of β phase using the lever rule
- `coolingCurve(diagram: BinaryPhaseDiagram, composition: double, startTemp: double, endTemp: double, steps: int): ArrayList<double>` — generate a simplified cooling curve with eutectic plateau

```titrate
import tt.materials.PhaseDiagram;

let diagram = new PhaseDiagram.BinaryPhaseDiagram("Cu", "Ag");
diagram.addLiquidusPoint(1085.0, 0.0);
diagram.addLiquidusPoint(962.0, 0.5);
diagram.addLiquidusPoint(961.0, 0.72);
diagram.addLiquidusPoint(900.0, 0.9);
diagram.setEutectic(779.0, 0.28);

// Lever rule
let fraction: double = PhaseDiagram.leverRule(0.4, 0.05, 0.72);
io::println(Double.toString(fraction));

// Cooling curve
let curve = PhaseDiagram.coolingCurve(diagram, 0.3, 1100.0, 600.0, 100);
```

## Elasticity

Stress-strain tensors, elastic constants, and isotropic Hooke's law with compliance/stiffness calculations.

### StressTensor

Symmetric second-order stress tensor (6 independent components).

- `fn init()` — create zero stress tensor
- `hydrostaticStress(): double` — hydrostatic (mean) stress
- `vonMisesStress(): double` — von Mises equivalent stress

### StrainTensor

Symmetric second-order strain tensor (6 independent components).

- `fn init()` — create zero strain tensor
- `volumetricStrain(): double` — volumetric (trace) strain

### ElasticConstants

Isotropic elastic constants derived from Young's modulus and Poisson's ratio.

- `fn init(e: double, nu: double)` — create from Young's modulus E and Poisson's ratio ν (computes shear and bulk moduli)
- `fromShearBulk(g: double, k: double): void` — set from shear modulus G and bulk modulus K
- `lameLambda(): double` — Lamé's first parameter λ
- `pWaveModulus(): double` — P-wave modulus M
- `public double youngsModulus` — Young's modulus E
- `public double poissonRatio` — Poisson's ratio ν
- `public double shearModulus` — shear modulus G
- `public double bulkModulus` — bulk modulus K

### Elasticity Functions

- `hookeStress(strain: StrainTensor, constants: ElasticConstants): StressTensor` — compute stress from strain (isotropic Hooke's law)
- `hookeStrain(stress: StressTensor, constants: ElasticConstants): StrainTensor` — compute strain from stress (compliance)
- `strainEnergyDensity(stress: StressTensor, strain: StrainTensor): double` — elastic strain energy density

```titrate
import tt.materials.Elasticity;

// Define elastic constants for steel
let steel = new ElasticConstants.ElasticConstants(200e9, 0.3);
io::println(Double.toString(steel.shearModulus));  // ≈ 76.9 GPa
io::println(Double.toString(steel.bulkModulus));    // ≈ 166.7 GPa
io::println(Double.toString(steel.lameLambda()));   // Lamé's λ

// Apply strain and compute stress
let strain = new Elasticity.StrainTensor();
strain.xx = 0.001;  // 0.1% tensile strain in x
let stress = Elasticity.hookeStress(strain, steel);
io::println(Double.toString(stress.xx));  // ≈ 2.0e8 Pa
io::println(Double.toString(stress.vonMisesStress()));

// Reverse: stress to strain
let stress2 = new Elasticity.StressTensor();
stress2.xx = 100e6;  // 100 MPa
let strain2 = Elasticity.hookeStrain(stress2, steel);
io::println(Double.toString(strain2.xx));

// Strain energy density
let energy: double = Elasticity.strainEnergyDensity(stress, strain);
io::println(Double.toString(energy));
```
