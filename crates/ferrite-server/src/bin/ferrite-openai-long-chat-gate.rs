use ferrite_server::long_chat_gate::{
    format_disconnect_probe_result, format_error_probe_result, format_report, format_run_summary,
    format_scenario_result, LongChatDisconnectProbeResult, LongChatErrorProbeResult,
    LongChatGateConfig, LongChatProofArtifacts,
};
use std::io::Write;

#[tokio::main]
async fn main() {
    let config = match LongChatGateConfig::parse(std::env::args_os()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    let mut artifacts = match LongChatProofArtifacts::create(&config) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    let mut exit_code = 0;
    if let Err(error) = run(&config, &mut artifacts).await {
        eprintln!("{error}");
        if let Err(artifact_error) = artifacts.write_line(&format!("long_chat_run_error={error}")) {
            eprintln!("{artifact_error}");
        }
        exit_code = 1;
    }
    if let Err(error) = artifacts.write_exit_code(exit_code) {
        eprintln!("{error}");
        exit_code = 1;
    }
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

async fn run(
    config: &LongChatGateConfig,
    artifacts: &mut LongChatProofArtifacts,
) -> Result<(), Box<dyn std::error::Error>> {
    emit_line(artifacts, &format_report(config))?;
    let mut error_probe: Option<LongChatErrorProbeResult> = None;
    let mut disconnect_probe: Option<LongChatDisconnectProbeResult> = None;
    let mut results = Vec::new();
    if config.error_probe() {
        let result = config.run_error_probe().await?;
        emit_line(artifacts, &format_error_probe_result(&result))?;
        error_probe = Some(result);
    }
    if config.disconnect_probe() {
        let result = config.run_disconnect_probe().await?;
        emit_line(artifacts, &format_disconnect_probe_result(&result))?;
        disconnect_probe = Some(result);
    }
    if config.execute() {
        let mut stdout = std::io::stdout();
        results = config
            .run_with_observer(|result| {
                let line = format_scenario_result(result);
                writeln!(stdout, "{line}")?;
                stdout.flush()?;
                artifacts.write_line(&line)?;
                Ok(())
            })
            .await?;
    }
    if config.execute() || config.error_probe() || config.disconnect_probe() {
        emit_line(
            artifacts,
            &format_run_summary(
                config,
                &results,
                error_probe.as_ref(),
                disconnect_probe.as_ref(),
            ),
        )?;
    }
    Ok(())
}

fn emit_line(
    artifacts: &mut LongChatProofArtifacts,
    line: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{line}");
    artifacts.write_line(line)?;
    Ok(())
}
