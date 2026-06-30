use ferrite_server::long_chat_gate::{format_report, LongChatGateConfig};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse(std::env::args_os())?;
    println!("{}", format_report(&config));
    Ok(())
}
