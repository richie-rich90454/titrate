use std::fs;
use std::path::Path;

/// Remove the build output directory (build/ or target/).
pub fn clean(project_dir: &Path) -> Result<(), String> {
    let build_dir = project_dir.join("build");
    let target_dir = project_dir.join("target");

    let mut removed = false;

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)
            .map_err(|e| format!("Failed to remove build directory: {}", e))?;
        println!("Removed {}", build_dir.display());
        removed = true;
    }

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)
            .map_err(|e| format!("Failed to remove target directory: {}", e))?;
        println!("Removed {}", target_dir.display());
        removed = true;
    }

    if !removed {
        println!("Nothing to clean (no build/ or target/ directory found)");
    }

    Ok(())
}
