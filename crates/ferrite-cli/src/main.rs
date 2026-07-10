//! Entry point for Ferrite's local generation and benchmarking CLI.

mod args;
mod benchmark;
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
            println!("ferrite {}", env!("CARGO_PKG_VERSION"));
            return;
        }
    }

    if let Err(error) = run::run(arguments) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
