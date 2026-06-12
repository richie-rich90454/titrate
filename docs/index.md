---
layout: home

hero:
  name: Titrate
  text: A systems programming language
  tagline: Precise. Safe. Expressive.
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: Language Reference
      link: /reference/lexer

features:
  - title: Bytecode VM
    details: 10x faster than tree-walking interpretation. Compile to bytecode and run on the Titrate VM.
  - title: Generics with Monomorphization
    details: Write generic code with zero runtime overhead. The compiler specializes each type instance at compile time.
  - title: Module System
    details: Organize code with imports, control visibility with public and private, and detect circular dependencies.
  - title: Ownership & Regions
    details: Move semantics, borrowing, and region-based allocation — memory safety without garbage collection.
---

## Why Titrate?

- **Type-safe generics with monomorphization** — no runtime overhead. `ArrayList<int>` runs just as fast as hand-written code for integers.
- **Ownership and regions** — memory safety without garbage collection. Move semantics, borrowing, and region-based allocation give you control without the footguns.
- **Clean, modern syntax** — inspired by Rust and Python, with `name: Type` parameter order, `fn` declarations, and `string` (lowercase) built in from the start.
- **Bytecode VM with built-in standard library** — compile to bytecode and run on the Titrate VM, with collections, I/O, JSON, and more ready to go.
- **Scientific computing built-in** — chemistry (Atom, Molecule, ForceField, MD), linear algebra (Matrix, NDArray), and units of measure are part of the standard library.

## Example Scenarios

### Molecular Dynamics Simulation

```titrate
import tt::chem::Atom;
import tt::chem::Molecule;
import tt::chem::ForceField;
import tt::chem::MD;
import tt::chem::Integrator;

public fn main(): void {
    let water: Molecule = new Molecule();
    water.addAtom(new Atom("O", 0.0, 0.0, 0.0));
    water.addAtom(new Atom("H", 0.9572, 0.0, 0.0));
    water.addAtom(new Atom("H", -0.2399, 0.9270, 0.0));
    water.addBond(new Bond(0, 1, 1));
    water.addBond(new Bond(0, 2, 1));

    let ff: ForceField = new ForceField();
    ff.addBondTerm(0, 1, 450.0, 0.9572);
    ff.addBondTerm(0, 2, 450.0, 0.9572);
    ff.addAngleTerm(1, 0, 2, 55.0, 104.52);

    let integrator: Integrator = new Integrator(1.0);
    let md: MD = new MD(water, ff, integrator);
    md.run(1000);

    let energy: double = ff.energy(water);
    io::println("Final energy: " + Double.toString(energy));
}
```

### JSON API Client

```titrate
import tt::http::HttpClient;
import tt::json::JsonValue;
import tt::result::Result;

public fn fetchUser(id: int): Result<JsonValue, string> {
    let client: HttpClient = new HttpClient();
    let url: string = "https://api.example.com/users/" + Integer.toString(id);
    let response: Result<string, string> = client.get(url);

    if (response.isOk()) {
        let body: string = response.unwrap();
        let parsed: Result<JsonValue, string> = JsonValue.parse(body);
        if (parsed.isOk()) {
            return ok(parsed.unwrap());
        } else {
            return err("Failed to parse JSON");
        }
    } else {
        return err("HTTP request failed: " + response.unwrapErr());
    }
}
```

### Data Processing Pipeline

```titrate
import tt::ndarray::NDArray;
import tt::itertools::Itertools;
import tt::math::Math;

public fn processDataset(data: NDArray<double>): NDArray<double> {
    // Normalize each column to zero mean, unit variance
    let mean: double = data.mean();
    let std: double = data.std();
    let normalized: NDArray<double> = (data - mean) / std;

    // Apply log transform to positive values
    let transformed: NDArray<double> = normalized.apply(fn(x: double): double {
        if (x > 0.0) {
            return Math.log(x);
        } else {
            return 0.0;
        }
    });

    return transformed;
}
```

### CLI Tool

```titrate
import tt::argparse::ArgumentParser;
import tt::fs::File;
import tt::logging::Logger;

public fn main(): void {
    let parser: ArgumentParser = new ArgumentParser("titrate-tool");
    parser.addArg("input", true, "Input file path");
    parser.addArg("output", false, "Output file path");
    parser.addFlag("verbose", "Enable verbose logging");

    let args: Result<Map<string, string>, string> = parser.parse();
    if (args.isErr()) {
        io::println("Error: " + args.unwrapErr());
        return;
    }

    let opts: Map<string, string> = args.unwrap();
    if (opts.containsKey("verbose")) {
        Logger.setLevel("DEBUG");
    }

    let inputPath: string = opts.get("input");
    let content: Result<string, string> = File.readAll(inputPath);
    if (content.isErr()) {
        io::println("Failed to read file: " + content.unwrapErr());
        return;
    }

    io::println("Read " + Integer.toString(String.length(content.unwrap())) + " bytes");
}
```
