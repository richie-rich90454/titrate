# toml

The `tt.config` module provides a TOML parser for reading TOML configuration files.

```titrate
import tt.config.Toml;
```

## Toml

Parser for TOML (Tom's Obvious Minimal Language) configuration format.

- `Toml.parse(input: string): HashMap<string, Variant>` — parse a TOML string into a map of values
- `Toml.get(table: HashMap<string, Variant>, key: string): Variant` — get a value from a parsed TOML table
- `Toml.hasKey(table: HashMap<string, Variant>, key: string): bool` — check if a key exists in the parsed TOML
- `Toml.sections(table: HashMap<string, Variant>): ArrayList<string>` — list all section names in the parsed TOML
- `Toml.getString(table: HashMap<string, Variant>, key: string): string` — get a string value from the parsed TOML
- `Toml.getInt(table: HashMap<string, Variant>, key: int): int` — get an integer value from the parsed TOML
- `Toml.getDouble(table: HashMap<string, Variant>, key: string): double` — get a double value from the parsed TOML
- `Toml.getBool(table: HashMap<string, Variant>, key: string): bool` — get a boolean value from the parsed TOML

```titrate
let input: string = "[server]\nhost = \"localhost\"\nport = 8080\ndebug = true";
let config: HashMap<string, Variant> = Toml.parse(input);

let host: string = Toml.getString(config, "server.host");
let port: int = Toml.getInt(config, "server.port");
let debug: bool = Toml.getBool(config, "server.debug");

io::println(host);   // "localhost"
io::println(Integer.toString(port));  // 8080
```
