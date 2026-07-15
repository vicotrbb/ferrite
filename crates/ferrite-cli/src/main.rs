//! Entry point for Ferrite's local generation and benchmarking CLI.

mod args;
mod benchmark;
mod model_acquisition;
mod profile;
mod run;

fn main() {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if let Some(argument) = arguments.get(1) {
        if argument == "--help" || argument == "-h" {
            println!("{}", args::usage());
            return;
        }
        if argument == "--version" || argument == "-V" {
            println!("{}", version_output("ferrite"));
            return;
        }
    }

    if let Err(error) = run::run(arguments) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn version_output(binary: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    match option_env!("FERRITE_BUILD_SHA") {
        Some(revision) => format!("{binary} {version} ({revision})"),
        None => format!("{binary} {version}"),
    }
}
