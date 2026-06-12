use std::path::Path;

use crate::config;
use crate::project;

/// Check that all dependencies listed in the config are available.
/// For version dependencies, validates the version format.
/// For git dependencies, checks that the URL is non-empty.
pub fn resolve_dependencies(cfg: &config::Config) -> Result<(), String> {
    if cfg.dependencies.is_empty() {
        return Ok(());
    }

    for (name, dep) in &cfg.dependencies {
        match dep {
            config::Dependency::Version(v) => {
                if v.is_empty() {
                    return Err(format!(
                        "Dependency '{}' has an empty version string",
                        name
                    ));
                }
                // Validate version format (basic semver check)
                if !is_valid_version(v) {
                    return Err(format!(
                        "Dependency '{}' has invalid version '{}'. Expected semver format (e.g., \"1.0.0\")",
                        name, v
                    ));
                }
            }
            config::Dependency::Git { url } => {
                if url.is_empty() {
                    return Err(format!(
                        "Dependency '{}' has an empty git URL",
                        name
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Basic semver version validation.
fn is_valid_version(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() < 1 || parts.len() > 4 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u32>().is_ok() || *p == "*")
}

/// Check for newer versions of dependencies.
/// Since there is no remote registry yet, this reports the current versions
/// and notes that a registry is not available.
pub fn outdated(project_dir: &Path) -> Result<(), String> {
    let cfg = project::load_config(project_dir)?;

    if cfg.dependencies.is_empty() {
        println!("No dependencies found in Titrate.toml");
        return Ok(());
    }

    println!("Checking dependencies for updates...\n");
    println!("{:<20} {:<15} {:<15}", "Dependency", "Current", "Latest");
    println!("{}", "-".repeat(50));

    for (name, dep) in &cfg.dependencies {
        match dep {
            config::Dependency::Version(v) => {
                println!("{:<20} {:<15} {:<15}", name, v, "(unavailable)");
            }
            config::Dependency::Git { url: _ } => {
                println!("{:<20} {:<15} {:<15}", name, "git", "(unavailable)");
            }
        }
    }

    println!("\nNote: Remote version checking is not yet available.");
    println!("      Update checking requires a Titrate package registry.");

    Ok(())
}

/// Show the dependency tree.
pub fn tree(project_dir: &Path) -> Result<(), String> {
    let cfg = project::load_config(project_dir)?;

    println!("{}", cfg.package.name);

    if cfg.dependencies.is_empty() {
        println!("  (no dependencies)");
        return Ok(());
    }

    let mut dep_names: Vec<&String> = cfg.dependencies.keys().collect();
    dep_names.sort();

    let dep_count = dep_names.len();
    for (i, name) in dep_names.iter().enumerate() {
        let dep = &cfg.dependencies[*name];
        let is_last = i == dep_count - 1;
        let prefix = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        match dep {
            config::Dependency::Version(v) => {
                println!("{}{} v{}", prefix, name, v);
            }
            config::Dependency::Git { url } => {
                println!("{}{} (git)", prefix, name);
                println!("{}└── {}", child_prefix, url);
            }
        }
    }

    Ok(())
}
