mod languages;
mod model;
mod relationships;
mod scanner;
mod watcher;

use clap::{Parser, Subcommand};
use model::ClassDiagram;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "rust-engine", about = "Live Class Diagram analysis engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze a project once and print the resulting diagram as JSON.
    Analyze { root: PathBuf },
    /// Analyze a project, then keep watching it and print an updated
    /// diagram (one JSON object per line) whenever relevant files change.
    Watch { root: PathBuf },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Analyze { root } => print_diagram(&analyze_project(&root)),
        Command::Watch { root } => watcher::run(&root, |diagram| print_diagram(&diagram)),
    }
}

pub fn analyze_project(root: &Path) -> ClassDiagram {
    let mut classes = Vec::new();
    for file in scanner::collect_source_files(root) {
        if let Ok(source) = std::fs::read_to_string(&file) {
            classes.extend(languages::parse_file(&file, &source));
        }
    }
    relationships::build_diagram(classes)
}

fn print_diagram(diagram: &ClassDiagram) {
    match serde_json::to_string(diagram) {
        Ok(json) => println!("{json}"),
        Err(err) => eprintln!("failed to serialize diagram: {err}"),
    }
}
