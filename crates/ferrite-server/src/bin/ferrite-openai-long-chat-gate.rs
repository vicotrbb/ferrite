use ferrite_server::long_chat_gate::{
    format_disconnect_probe_result, format_error_probe_result, format_report, format_run_summary,
    format_scenario_result, LongChatDisconnectProbeResult, LongChatErrorProbeResult,
    LongChatGateConfig,
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
    let mut error_probe: Option<LongChatErrorProbeResult> = None;
    let mut disconnect_probe: Option<LongChatDisconnectProbeResult> = None;
    let mut results = Vec::new();
    if config.error_probe() {
        let result = config.run_error_probe().await?;
        println!("{}", format_error_probe_result(&result));
        error_probe = Some(result);
    }
    if config.disconnect_probe() {
        let result = config.run_disconnect_probe().await?;
        println!("{}", format_disconnect_probe_result(&result));
        disconnect_probe = Some(result);
    }
    if config.execute() {
        let mut stdout = std::io::stdout();
        results = config
            .run_with_observer(|result| {
                writeln!(stdout, "{}", format_scenario_result(result))?;
                stdout.flush()?;
                Ok(())
            })
            .await?;
    }
    if config.execute() || config.error_probe() || config.disconnect_probe() {
        println!(
            "{}",
            format_run_summary(
                &config,
                &results,
                error_probe.as_ref(),
                disconnect_probe.as_ref()
            )
        );
    }
    Ok(())
}
