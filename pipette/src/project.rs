// Project creation and management for pipette
// Precision in every step – richie-rich90454, 2026

use std::fs;
use std::path::{Path, PathBuf};

use crate::config;

/// Create a new Titrate project skeleton.
pub fn create_project(name: &str) -> Result<PathBuf, String> {
    let project_dir = PathBuf::from(name);

    if project_dir.exists() {
        return Err(format!(
            "Directory '{}' already exists",
            project_dir.display()
        ));
    }

    // Create project directory
    fs::create_dir_all(project_dir.join("src"))
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    // Write Titrate.toml
    let toml_content = config::default_toml(name);
    fs::write(project_dir.join("Titrate.toml"), toml_content)
        .map_err(|e| format!("Failed to write Titrate.toml: {}", e))?;

    // Write src/main.tr
    let main_content = format!(
        r#"public void main() {{
    io::println("Hello from {}!");
}}
"#,
        name
    );
    fs::write(project_dir.join("src").join("main.tr"), main_content)
        .map_err(|e| format!("Failed to write src/main.tr: {}", e))?;

    Ok(project_dir)
}

/// Walk up from CWD to find a directory containing Titrate.toml.
pub fn find_project() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;

    loop {
        if dir.join("Titrate.toml").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Load the project config from the given directory.
pub fn load_config(dir: &Path) -> Result<config::Config, String> {
    config::load_config(dir)
}
