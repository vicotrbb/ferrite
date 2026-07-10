use ferrite_inference::scalar::{Q8KActivationMatvecPolicy, ScalarExecutionOptions};
use ferrite_server::{config::ServerConfig, runtime::InferenceEngine, state::ServerState};
use std::error::Error;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::parse(std::env::args_os())?;
    #[cfg(target_arch = "aarch64")]
    let use_memory_bound_pool = config.experimental_residual_q8_activation_matvec()
        && std::arch::is_aarch64_feature_detected!("i8mm");
    #[cfg(not(target_arch = "aarch64"))]
    let use_memory_bound_pool = false;
    let inference_threads = if use_memory_bound_pool {
        ferrite_inference::threading::init_memory_bound_global_pool(config.inference_threads())
    } else {
        ferrite_inference::threading::init_global_pool(config.inference_threads())
    };
    println!("inference_threads={inference_threads}");
    let listener = tokio::net::TcpListener::bind(config.bind_addr()).await?;
    let state = match config.model_path() {
        Some(path) => {
            let policy = if config.experimental_residual_q8_activation_matvec() {
                Q8KActivationMatvecPolicy::ExperimentalResidualI8mm
            } else {
                Q8KActivationMatvecPolicy::DefaultOnly
            };
            let execution_options =
                ScalarExecutionOptions::default().with_q8_k_activation_matvec_policy(policy);
            let engine = InferenceEngine::load(path)?.with_execution_options(execution_options);
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
        state = state.with_batched_decode(max_batch_streams)?;
        println!("experimental_batched_decode max_batch_streams={max_batch_streams}");
    }
    let app = ferrite_server::router(state);
    axum::serve(listener, app).await?;
    Ok(())
}
