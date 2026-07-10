mod args;
mod benchmark;
mod profile;
mod run;

fn main() {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if arguments
        .get(1)
        .is_some_and(|argument| argument == "--help" || argument == "-h")
    {
        println!("{}", args::usage());
        return;
    }

    if let Err(error) = run::run(arguments) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
