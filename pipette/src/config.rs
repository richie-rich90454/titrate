// Titrate.toml manifest parsing for pipette
// Precision in every step – richie-rich90454, 2026

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A dependency specification.
#[derive(Debug, Clone, PartialEq)]
pub enum Dependency {
    /// A version-based dependency: package name and version string.
    Version(String),
    /// A git-based dependency: URL to a git repository.
    Git { url: String },
}

impl Dependency {
    /// Returns the version string if this is a version dependency.
    pub fn version(&self) -> Option<&str> {
        match self {
            Dependency::Version(v) => Some(v),
            Dependency::Git { .. } => None,
        }
    }

    /// Returns the git URL if this is a git dependency.
    pub fn git_url(&self) -> Option<&str> {
        match self {
            Dependency::Version(_) => None,
            Dependency::Git { url } => Some(url),
        }
    }
}

impl std::fmt::Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dependency::Version(v) => write!(f, "\"{}\"", v),
            Dependency::Git { url } => write!(f, "{{ git = \"{}\" }}", url),
        }
    }
}

/// The parsed project manifest.
#[derive(Debug, Clone)]
pub struct Config {
    pub package: PackageInfo,
    pub dependencies: HashMap<String, Dependency>,
    pub native: Option<NativeConfig>,
}

/// Package metadata from the `[package]` section.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub entry: String,
}

/// Native build configuration from the `[native]` section.
///
/// Both fields are optional within the section: if `[native]` is present
/// but a field is omitted, that field defaults to an empty `Vec`.
#[derive(Debug, Clone)]
pub struct NativeConfig {
    pub link_libs: Vec<String>,
    pub link_args: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            package: PackageInfo {
                name: String::new(),
                version: "0.1.0".to_string(),
                entry: "src/main.tr".to_string(),
            },
            dependencies: HashMap::new(),
            native: None,
        }
    }
}

/// Parse a Titrate.toml file from disk.
pub fn load_config(dir: &Path) -> Result<Config, String> {
    let toml_path = dir.join("Titrate.toml");
    let contents = fs::read_to_string(&toml_path)
        .map_err(|e| format!("Failed to read {}: {}", toml_path.display(), e))?;
    parse_toml(&contents)
}

/// Parse a TOML array of strings (e.g. `["ssl", "crypto"]`) into a `Vec<String>`.
///
/// Each element has its surrounding double quotes stripped. Returns an empty
/// `Vec` for `[]`, whitespace-only contents, or any value not wrapped in `[...]`.
/// Commas inside quoted strings are preserved (e.g. `["-Wl,-rpath=."]` yields a
/// single element `-Wl,-rpath=.`).
fn parse_string_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return Vec::new();
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return Vec::new();
    }

    // Walk the inner content, splitting on commas that are NOT inside a
    // double-quoted string. This correctly handles values like
    // `["-Wl,-rpath=.", "ssl"]` where a quoted element contains a comma.
    let mut result: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in inner.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                // Drop the quote characters themselves.
            }
            ',' if !in_quote => {
                let element = current.trim().to_string();
                if !element.is_empty() {
                    result.push(element);
                }
                current.clear();
            }
            c => current.push(c),
        }
    }
    let last = current.trim().to_string();
    if !last.is_empty() {
        result.push(last);
    }
    result
}

/// Minimal TOML parser – handles the flat structure we need,
/// plus nested `[dependencies.name]` sections for git-based deps.
fn parse_toml(contents: &str) -> Result<Config, String> {
    let mut config = Config::default();
    let mut current_section = String::new();

    // Track the current nested dependency name when inside [dependencies.name]
    let mut current_dep_name: Option<String> = None;
    let mut current_dep_git: Option<String> = None;

    for line in contents.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section header like [package] or [dependencies] or [dependencies.name]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Flush any pending nested dependency
            if let Some(name) = current_dep_name.take() {
                if let Some(url) = current_dep_git.take() {
                    config.dependencies.insert(name, Dependency::Git { url });
                }
            }

            current_section = trimmed[1..trimmed.len() - 1].trim().to_string();

            // Check for nested dependency section like [dependencies.mylib]
            if current_section.starts_with("dependencies.") {
                let dep_name = current_section["dependencies.".len()..].trim().to_string();
                current_dep_name = Some(dep_name);
                current_dep_git = None;
            }

            // Initialize [native] config when entering the section. Fields
            // default to empty Vecs; individual keys fill them in below.
            if current_section == "native" && config.native.is_none() {
                config.native = Some(NativeConfig {
                    link_libs: Vec::new(),
                    link_args: Vec::new(),
                });
            }
            continue;
        }

        // Key = "value" or Key = value
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let value = trimmed[eq_pos + 1..].trim().to_string();

            // Strip quotes from string values
            let value = if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                value[1..value.len() - 1].to_string()
            } else {
                value
            };

            match current_section.as_str() {
                "package" => match key.as_str() {
                    "name" => config.package.name = value,
                    "version" => config.package.version = value,
                    "entry" => config.package.entry = value,
                    _ => {} // Ignore unknown keys
                },
                "dependencies" => {
                    // Simple version dependency: mylib = "1.0.0"
                    if !value.is_empty() {
                        config.dependencies.insert(key, Dependency::Version(value));
                    }
                }
                "native" => {
                    if let Some(ref mut native) = config.native {
                        match key.as_str() {
                            "link_libs" => native.link_libs = parse_string_array(&value),
                            "link_args" => native.link_args = parse_string_array(&value),
                            _ => {} // Ignore unknown keys
                        }
                    }
                }
                _ => {
                    // Handle nested [dependencies.name] section
                    if current_section.starts_with("dependencies.") {
                        match key.as_str() {
                            "version" => {
                                // Flush as version dependency if no git key follows
                                if let Some(ref name) = current_dep_name {
                                    // Store temporarily; will be flushed on section change
                                    // For now, just store as version dep
                                    let name = name.clone();
                                    config.dependencies.insert(name, Dependency::Version(value));
                                    current_dep_name = None;
                                }
                            }
                            "git" => {
                                current_dep_git = Some(value);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Flush any remaining nested dependency
    if let Some(name) = current_dep_name.take() {
        if let Some(url) = current_dep_git.take() {
            config.dependencies.insert(name, Dependency::Git { url });
        }
    }

    if config.package.name.is_empty() {
        return Err("Titrate.toml missing [package] name".to_string());
    }

    Ok(config)
}

/// Generate the default Titrate.toml content for a new project.
pub fn default_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
entry = "src/main.tr"

[dependencies]
# No dependencies yet
"#,
        name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_pkg(cfg: &Config, name: &str, version: &str, entry: &str) {
        assert_eq!(cfg.package.name, name);
        assert_eq!(cfg.package.version, version);
        assert_eq!(cfg.package.entry, entry);
    }

    #[test]
    fn parses_full_native_section() {
        let toml = r#"[package]
name = "demo"
version = "0.2.0"
entry = "src/main.tr"

[native]
link_libs = ["ssl", "crypto"]
link_args = ["-L/opt/lib", "-Wl,-rpath=."]
"#;
        let cfg = parse_toml(toml).expect("parse ok");
        assert_pkg(&cfg, "demo", "0.2.0", "src/main.tr");
        let native = cfg.native.expect("native section present");
        assert_eq!(native.link_libs, vec!["ssl".to_string(), "crypto".to_string()]);
        assert_eq!(
            native.link_args,
            vec!["-L/opt/lib".to_string(), "-Wl,-rpath=.".to_string()]
        );
    }

    #[test]
    fn parses_native_section_with_only_link_libs() {
        let toml = r#"[package]
name = "demo"

[native]
link_libs = ["m"]
"#;
        let cfg = parse_toml(toml).expect("parse ok");
        let native = cfg.native.expect("native section present");
        assert_eq!(native.link_libs, vec!["m".to_string()]);
        assert!(native.link_args.is_empty(), "link_args defaults to empty");
    }

    #[test]
    fn parses_native_section_with_only_link_args() {
        let toml = r#"[package]
name = "demo"

[native]
link_args = ["-framework", "OpenGL"]
"#;
        let cfg = parse_toml(toml).expect("parse ok");
        let native = cfg.native.expect("native section present");
        assert!(native.link_libs.is_empty(), "link_libs defaults to empty");
        assert_eq!(
            native.link_args,
            vec!["-framework".to_string(), "OpenGL".to_string()]
        );
    }

    #[test]
    fn parses_empty_native_section() {
        let toml = r#"[package]
name = "demo"

[native]
"#;
        let cfg = parse_toml(toml).expect("parse ok");
        let native = cfg.native.expect("native section present");
        assert!(native.link_libs.is_empty());
        assert!(native.link_args.is_empty());
    }

    #[test]
    fn parses_config_without_native_section() {
        let toml = r#"[package]
name = "demo"
version = "0.1.0"
entry = "src/main.tr"

[dependencies]
"#;
        let cfg = parse_toml(toml).expect("parse ok");
        assert!(cfg.native.is_none(), "native must be None when section absent");
    }

    #[test]
    fn parses_empty_array_values() {
        let toml = r#"[package]
name = "demo"

[native]
link_libs = []
link_args = []
"#;
        let cfg = parse_toml(toml).expect("parse ok");
        let native = cfg.native.expect("native section present");
        assert!(native.link_libs.is_empty());
        assert!(native.link_args.is_empty());
    }
}
