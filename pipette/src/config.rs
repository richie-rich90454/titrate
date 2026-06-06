// Titrate.toml manifest parsing for pipette
// Precision in every step – richie-rich90454, 2026

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// The parsed project manifest.
#[derive(Debug, Clone)]
pub struct Config {
    pub package: PackageInfo,
    pub dependencies: HashMap<String, String>,
}

/// Package metadata from the `[package]` section.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub entry: String,
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

/// Minimal TOML parser – handles the flat structure we need.
fn parse_toml(contents: &str) -> Result<Config, String> {
    let mut config = Config::default();
    let mut current_section = String::new();

    for line in contents.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section header like [package] or [dependencies]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].trim().to_string();
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
                    if !value.is_empty() {
                        config.dependencies.insert(key, value);
                    }
                }
                _ => {} // Ignore unknown sections
            }
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
# No dependencies yet in Alpha 0.2
"#,
        name
    )
}
