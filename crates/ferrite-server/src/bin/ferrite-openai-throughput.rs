use ferrite_server::throughput_client::{
    format_result, run_completion_benchmark, ThroughputClientConfig,
};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = ThroughputClientConfig::parse(std::env::args_os())?;
    let result = run_completion_benchmark(&config).await?;
    println!("{}", format_result(&config, result));
    Ok(())
}
