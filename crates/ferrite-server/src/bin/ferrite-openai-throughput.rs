//! Command-line throughput and streaming-metric client for a Ferrite server.

use ferrite_server::throughput_client::{
    format_result, run_completion_benchmark, ThroughputClientConfig,
};

#[tokio::main]
async fn main() {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if let Some(argument) = arguments.get(1) {
        if argument == "--help" || argument == "-h" {
            println!("{}", ferrite_server::throughput_client::usage());
            return;
        }
        if argument == "--version" || argument == "-V" {
            println!("ferrite-openai-throughput {}", env!("CARGO_PKG_VERSION"));
            return;
        }
    }

    if let Err(error) = run(arguments).await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse(arguments)?;
    let result = run_completion_benchmark(&config).await?;
    println!("{}", format_result(&config, result));
    Ok(())
}
