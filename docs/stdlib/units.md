# units

The `tt.units` module provides type-safe SI base units, derived units, and physical constants for dimensional analysis and scientific computing.

```titrate
import tt.units.Base;
import tt.units.Derived;
import tt.units.Constants;
```

## Base Units

SI base unit types with arithmetic and comparison.

| Class | Symbol | Operations |
|-------|--------|------------|
| `Meter` | m | `plus`, `minus`, `times(double)`, `div(double)`, `times(Meter) → MeterSquared` |
| `MeterSquared` | m² | `getValue`, `equals`, `compareTo` |
| `Second` | s | `plus`, `minus`, `times(double)`, `div(double)` |
| `Kilogram` | kg | `plus`, `minus`, `times(double)`, `div(double)` |
| `Kelvin` | K | `plus`, `minus` |
| `Mole` | mol | `plus`, `minus` |
| `Ampere` | A | `plus`, `minus` |
| `Candela` | cd | `plus`, `minus`, `times(double)`, `div(double)` |

Each base unit class provides:
- `getValue(): double` — raw numeric value
- `toDouble(): double` — convert to plain double
- `toString(): string` — formatted with unit symbol (e.g. `"5.0 m"`)
- `equals(other)`, `compareTo(other)` — comparison

```titrate
let distance = new Meter(100.0);
let area = distance.times(distance);  // MeterSquared
io::println(area.toString());  // "10000.0 m²"
```

## Derived Units

SI derived units with automatic dimensional composition.

| Class | Symbol | Derived From | Key Operations |
|-------|--------|-------------|----------------|
| `Newton` | N | kg·m/s² | `times(Meter) → Joule` |
| `Joule` | J | kg·m²/s² | `div(Second) → Watt` |
| `Watt` | W | J/s | `times(Second) → Joule` |
| `Pascal` | Pa | N/m² | — |
| `Coulomb` | C | A·s | — |
| `Volt` | V | J/C | `times(Coulomb) → Joule`, `div(Ampere) → Ohm` |
| `Ohm` | Ω | V/A | — |
| `Farad` | F | C/V | — |
| `Henry` | H | V·s/A | — |
| `Tesla` | T | V·s/m² | — |
| `Weber` | Wb | V·s | — |
| `Hertz` | Hz | 1/s | — |
| `Becquerel` | Bq | 1/s | — |
| `Gray` | Gy | J/kg | — |
| `Sievert` | Sv | J/kg | — |
| `Lux` | lx | cd/m² | — |
| `Lumen` | lm | cd·sr | — |
| `Katal` | kat | mol/s | — |

Each derived unit provides `fromBase(...)` static methods for construction from base units.

```titrate
let force = Newton.fromBase(new Meter(10.0), new Kilogram(2.0), new Second(1.0));
io::println(force.toString());  // "200.0 N"

let energy = force.times(new Meter(5.0));  // Joule
io::println(energy.toString());  // "1000.0 J"
```

## Constants

Physical and mathematical constants.

| Constant | Value |
|----------|-------|
| `BOLTZMANN()` | 1.380649e-23 J/K |
| `AVOGADRO()` | 6.02214076e23 mol⁻¹ |
| `GAS_CONSTANT()` | 8.314462618 J/(mol·K) |
| `PLANCK()` | 6.62607015e-34 J·s |
| `SPEED_OF_LIGHT()` | 299792458.0 m/s |
| `ELEMENTARY_CHARGE()` | 1.602176634e-19 C |
| `COULOMB_CONSTANT()` | 8.9875517923e9 N·m²/C² |
| `GRAVITATIONAL()` | 6.67430e-11 N·m²/kg² |
| `PI()` | 3.14159265358979 |
| `ELECTRON_MASS()` | 9.1093837015e-31 kg |
| `PROTON_MASS()` | 1.67262192369e-27 kg |
| `NEUTRON_MASS()` | 1.67492749804e-27 kg |
| `ATOMIC_MASS_UNIT()` | 1.66053906660e-27 kg |
| `RYDBERG()` | 1.0973731568160e7 m⁻¹ |
| `BOHR_RADIUS()` | 5.29177210903e-11 m |
| `FINE_STRUCTURE()` | 7.2973525693e-3 |
| `STEFAN_BOLTZMANN()` | 5.670374419e-8 W/(m²·K⁴) |
| `WIEN_DISPLACEMENT()` | 2.897771955e-3 m·K |
| `PERMEABILITY_VACUUM()` | 1.25663706212e-6 H/m |
| `PERMITTIVITY_VACUUM()` | 8.8541878128e-12 F/m |
| `MOLAR_GAS_VOLUME()` | 22.413969545014e-3 m³/mol |

```titrate
let k = Constants.BOLTZMANN();
let na = Constants.AVOGADRO();
let r = Constants.GAS_CONSTANT();
// R = k * N_A (within floating-point precision)
```
