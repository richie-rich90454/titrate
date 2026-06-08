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
            match pipette::build(&project_dir) {
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
            if let Err(e) = pipette::test(&project_dir) {
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
    eprintln!("  build          Compile the project");
    eprintln!("  run            Build and run the project");
    eprintln!("  test           Run tests");
    eprintln!("  doc            Generate API documentation");
    eprintln!("  watch          Watch for changes and rebuild");
}
