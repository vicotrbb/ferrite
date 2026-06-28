mod args;
mod benchmark;
mod profile;
mod run;

fn main() {
    if let Err(error) = run::run(std::env::args_os()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
