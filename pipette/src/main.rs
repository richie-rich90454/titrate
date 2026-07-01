// Titrate build tool – pipette CLI entry point
// Precision in every step – richie-rich90454, 2026

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "new" => {
            if args.len() < 3 {
                eprintln!("Usage: pipette new <name>");
                process::exit(1);
            }
            let name = &args[2];
            match pipette::project::create_project(name) {
                Ok(dir) => {
                    println!("Created project '{}' in {}", name, dir.display());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        "build" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            let profile = if args.len() > 2 && args[2] == "--release" {
                pipette::BuildProfile::Release
            } else {
                pipette::BuildProfile::Debug
            };
            match pipette::build_with_profile(&project_dir, profile) {
                Ok(output) => {
                    println!("Build succeeded: {}", output.display());
                }
                Err(e) => {
                    eprintln!("Build failed: {}", e);
                    process::exit(1);
                }
            }
        }
        "run" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            if let Err(e) = pipette::run(&project_dir) {
                eprintln!("Run failed: {}", e);
                process::exit(1);
            }
        }
        "test" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            let filter = if args.len() > 2 && args[2] == "--filter" {
                if args.len() > 3 {
                    Some(args[3].as_str())
                } else {
                    eprintln!("Error: --filter requires a value");
                    process::exit(1);
                }
            } else {
                None
            };
            if let Err(e) = pipette::test(&project_dir, filter) {
                eprintln!("Tests failed: {}", e);
                process::exit(1);
            }
        }
        "doc" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            if let Err(e) = pipette::doc(&project_dir) {
                eprintln!("Doc generation failed: {}", e);
                process::exit(1);
            }
        }
        "clean" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            if let Err(e) = pipette::clean(&project_dir) {
                eprintln!("Clean failed: {}", e);
                process::exit(1);
            }
        }
        "lint" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            if let Err(e) = pipette::lint(&project_dir) {
                eprintln!("Lint failed: {}", e);
                process::exit(1);
            }
        }
        "fmt" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            if let Err(e) = pipette::fmt(&project_dir) {
                eprintln!("Format failed: {}", e);
                process::exit(1);
            }
        }
        "bench" => {
            if args.len() > 2 && args[2] == "--native-vs-bytecode" {
                if args.len() < 4 {
                    eprintln!("Error: --native-vs-bytecode requires a path argument");
                    process::exit(1);
                }
                let path = std::path::PathBuf::from(&args[3]);
                if let Err(e) = pipette::bench_native_vs_bytecode_path(&path) {
                    eprintln!("Benchmarks failed: {}", e);
                    process::exit(1);
                }
            } else {
                let project_dir = match pipette::project::find_project() {
                    Some(dir) => dir,
                    None => {
                        eprintln!(
                            "Error: No Titrate.toml found in current or parent directories"
                        );
                        process::exit(1);
                    }
                };
                if args.len() > 2 && args[2] == "--compare-native" {
                    if let Err(e) = pipette::bench_compare_native(&project_dir) {
                        eprintln!("Benchmarks failed: {}", e);
                        process::exit(1);
                    }
                } else if let Err(e) = pipette::bench(&project_dir) {
                    eprintln!("Benchmarks failed: {}", e);
                    process::exit(1);
                }
            }
        }
        "outdated" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            if let Err(e) = pipette::outdated(&project_dir) {
                eprintln!("Outdated check failed: {}", e);
                process::exit(1);
            }
        }
        "tree" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            if let Err(e) = pipette::tree(&project_dir) {
                eprintln!("Tree failed: {}", e);
                process::exit(1);
            }
        }
        "watch" => {
            let project_dir = match pipette::project::find_project() {
                Some(dir) => dir,
                None => {
                    eprintln!("Error: No Titrate.toml found in current or parent directories");
                    process::exit(1);
                }
            };
            if let Err(e) = pipette::watch(&project_dir) {
                eprintln!("Watch error: {}", e);
                process::exit(1);
            }
        }
        "coverage" => {
            // Coverage operates on the Rust workspace, not a Titrate project,
            // so we look for the workspace root (a directory containing a
            // Cargo.toml with a [workspace] section) instead of Titrate.toml.
            let workspace_dir = match find_workspace_root() {
                Some(dir) => dir,
                None => {
                    eprintln!(
                        "Error: No workspace Cargo.toml found in current or parent directories"
                    );
                    process::exit(1);
                }
            };
            let native = args.iter().any(|a| a == "--native");
            if let Err(e) = pipette::coverage(&workspace_dir, native) {
                eprintln!("Coverage failed: {}", e);
                process::exit(1);
            }
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("pipette – Titrate build tool and package manager");
    eprintln!();
    eprintln!("Usage: pipette <command> [args]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  new <name>     Create a new project");
    eprintln!("  build          Compile the project [--release for optimized build]");
    eprintln!("  run            Build and run the project");
    eprintln!("  test           Run tests");
    eprintln!("  bench          Run benchmark files [--compare-native | --native-vs-bytecode <path>]");
    eprintln!("  coverage       Run tests and collect coverage [--native for native builds]");
    eprintln!("  doc            Generate API documentation");
    eprintln!("  watch          Watch for changes and rebuild");
    eprintln!("  clean          Remove build output directory");
    eprintln!("  lint           Run the analyzer on all .tr files");
    eprintln!("  fmt            Format .tr source files");
    eprintln!("  outdated       Check for newer versions of dependencies");
    eprintln!("  tree           Show the dependency tree");
}

/// Walk up from the current directory looking for a `Cargo.toml` that
/// declares a `[workspace]` section. Used by the `coverage` subcommand,
/// which instruments the Rust toolchain rather than a Titrate project.
fn find_workspace_root() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.is_file() {
            if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                if contents.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}
