use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::build::build;

/// Watch for file changes and rebuild.
pub fn watch(project_dir: &Path) -> Result<(), String> {
    println!("Watching for changes... (Ctrl+C to stop)");

    // Initial build
    match build(project_dir) {
        Ok(_) => println!("Initial build succeeded."),
        Err(e) => eprintln!("Initial build failed: {}", e),
    }

    let src_dir = project_dir.join("src");
    let mut last_mtimes = get_mtimes(&src_dir);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        let current_mtimes = get_mtimes(&src_dir);
        if current_mtimes != last_mtimes {
            println!("\nChange detected, rebuilding...");
            match build(project_dir) {
                Ok(_) => println!("Build succeeded."),
                Err(e) => eprintln!("Build failed: {}", e),
            }
            last_mtimes = current_mtimes;
        }
    }
}

pub fn get_mtimes(dir: &Path) -> HashMap<PathBuf, SystemTime> {
    let mut mtimes = HashMap::new();
    if !dir.exists() {
        return mtimes;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                mtimes.extend(get_mtimes(&path));
            } else if path.extension().is_some_and(|ext| ext == "tr") {
                if let Ok(meta) = fs::metadata(&path) {
                    if let Ok(mtime) = meta.modified() {
                        mtimes.insert(path, mtime);
                    }
                }
            }
        }
    }
    mtimes
}
