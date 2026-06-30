use ferrite_server::long_chat_gate::{format_report, format_scenario_result, LongChatGateConfig};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = LongChatGateConfig::parse(std::env::args_os())?;
    println!("{}", format_report(&config));
    if config.execute() {
        for result in config.run().await? {
            println!("{}", format_scenario_result(&result));
        }
    }
    Ok(())
}
