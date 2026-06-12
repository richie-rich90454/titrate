# configparser

The `tt.config` module provides an INI-style configuration file parser.

```titrate
import tt.config.ConfigParser;
```

## ConfigParser

Parser and writer for INI-style configuration files with sections and key-value pairs.

- `ConfigParser.parse(input: string): ConfigParser` — parse an INI-formatted string and return a ConfigParser instance
- `get(section: string, key: string): string` — get a value from the given section and key
- `get(section: string, key: string, fallback: string): string` — get a value with a fallback if the key is missing
- `set(section: string, key: string, value: string): void` — set a value in the given section
- `hasSection(section: string): bool` — check if a section exists
- `hasKey(section: string, key: string): bool` — check if a key exists within a section
- `sections(): ArrayList<string>` — list all section names
- `keys(section: string): ArrayList<string>` — list all keys in a section
- `write(): string` — serialize the configuration back to an INI-formatted string

```titrate
let ini: string = "[database]\nhost = localhost\nport = 5432\n[logging]\nlevel = info";
let parser: ConfigParser = ConfigParser.parse(ini);

let host: string = parser.get("database", "host");
let port: string = parser.get("database", "port");
let level: string = parser.get("logging", "level", "warn");

io::println(host);   // "localhost"
io::println(port);   // "5432"

parser.set("database", "timeout", "30");
let output: string = parser.write();
```
