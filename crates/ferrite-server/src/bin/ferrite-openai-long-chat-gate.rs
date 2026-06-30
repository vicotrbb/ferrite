use ferrite_server::long_chat_gate::{
    format_disconnect_probe_result, format_error_probe_result, format_report,
    format_scenario_result, LongChatGateConfig,
};
use std::io::Write;

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
    if config.error_probe() {
        println!(
            "{}",
            format_error_probe_result(&config.run_error_probe().await?)
        );
    }
    if config.disconnect_probe() {
        println!(
            "{}",
            format_disconnect_probe_result(&config.run_disconnect_probe().await?)
        );
    }
    if config.execute() {
        let mut stdout = std::io::stdout();
        config
            .run_with_observer(|result| {
                writeln!(stdout, "{}", format_scenario_result(result))?;
                stdout.flush()?;
                Ok(())
            })
            .await?;
    }
    Ok(())
}
