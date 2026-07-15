//! Entry point for Ferrite's OpenAI-compatible HTTP server.

use ferrite_inference::scalar::{
    CpuKernelCapabilities, KernelProvider, KvBackend, Q8KActivationMatvecPolicy,
    ScalarExecutionOptions,
};
use ferrite_server::{
    config::{ServerConfig, ServerKvBackend},
    runtime::{InferenceEngine, RuntimeError},
    state::ServerState,
};
use std::error::Error;

#[tokio::main]
async fn main() {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    if let Some(argument) = arguments.get(1) {
        if argument == "--help" || argument == "-h" {
            println!("{}", ferrite_server::config::usage());
            return;
        }
        if argument == "--version" || argument == "-V" {
            println!("{}", version_output("ferrite-server"));
            return;
        }
    }

    if let Err(error) = run(arguments).await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn version_output(binary: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    match option_env!("FERRITE_BUILD_SHA") {
        Some(revision) => format!("{binary} {version} ({revision})"),
        None => format!("{binary} {version}"),
    }
}

async fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::parse(arguments)?;
    let cpu_capabilities = CpuKernelCapabilities::detect();
    let use_memory_bound_pool = config.experimental_residual_q8_activation_matvec()
        && config.kernel_provider() == KernelProvider::Auto
        && cpu_capabilities.i8mm();
    let inference_threads = ferrite_inference::threading::init_server_global_pool(
        config.inference_threads(),
        use_memory_bound_pool,
    );
    println!("inference_threads={inference_threads}");
    println!("kernel_provider={}", config.kernel_provider().as_str());
    println!(
        "cpu_features={}",
        cpu_capabilities.detected_feature_labels()
    );
    let listener = tokio::net::TcpListener::bind(config.bind_addr()).await?;
    let state = match config.model_path() {
        Some(path) => {
            let policy = if config.experimental_residual_q8_activation_matvec() {
                Q8KActivationMatvecPolicy::ExperimentalResidualI8mm
            } else {
                Q8KActivationMatvecPolicy::DefaultOnly
            };
            let kv_backend = match config.kv_backend() {
                ServerKvBackend::Vec => KvBackend::Vec,
                ServerKvBackend::Locus => KvBackend::Locus {
                    tokens_per_block: config.kv_tokens_per_block(),
                    max_tokens: config.kv_max_tokens().ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "locus KV backend is missing its max-token budget",
                        )
                    })?,
                },
            };
            let execution_options = ScalarExecutionOptions::default()
                .with_kernel_provider(config.kernel_provider())
                .with_q8_k_activation_matvec_policy(policy)
                .with_kv_backend(kv_backend);
            let engine = load_stable_model(path)?
                .with_execution_options(execution_options)
                .with_prefix_cache_limits(
                    config.prefix_cache_max_entries(),
                    config.prefix_cache_max_bytes(),
                )?;
            engine.validate_session_configuration()?;
            ServerState::with_engine(config.model_id().to_owned(), engine)
        }
        None => ServerState::new(config.model_id().to_owned()),
    };
    let state = match config.api_key() {
        Some(api_key) => state.with_api_key(api_key),
        None => state,
    }
    .with_token_limits(config.token_limits());
    if config.experimental_residual_q8_activation_matvec() {
        println!("q8_k_activation_matvec_policy=experimental_residual_i8mm");
    }
    let mut state = state
        .with_inference_wait_timeout(config.inference_wait_timeout())
        .with_prefix_cache_enabled(config.experimental_prefix_cache_enabled())
        .with_max_concurrent_inferences(config.max_concurrent_inferences());
    if let Some(max_batch_streams) = config.experimental_batched_decode_max_streams() {
        let max_batch_queue = config
            .experimental_batched_decode_max_queue()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "batched decode is missing its queue limit",
                )
            })?;
        state = state.with_batched_decode_and_queue(max_batch_streams, max_batch_queue)?;
        println!(
            "experimental_batched_decode max_batch_streams={max_batch_streams} max_batch_queue={max_batch_queue}"
        );
    }
    let app = ferrite_server::router(state);
    axum::serve(listener, app).await?;
    Ok(())
}

#[allow(
    unsafe_code,
    reason = "the server process treats its configured model artifact as immutable while loaded"
)]
fn load_stable_model(path: &std::path::Path) -> Result<InferenceEngine, RuntimeError> {
    // SAFETY: Ferrite never modifies or truncates its configured model.
    // Operators must not replace the backing artifact while the server runs.
    unsafe { InferenceEngine::load_mapped(path) }
}
