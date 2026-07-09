use super::{
    error::OpenAiHttpError,
    stop_filter::{apply_stop_sequences, StopSequenceFilter},
};
use crate::runtime::{GenerationCacheOptions, GenerationControl};
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;

pub(super) async fn generate_text(
    engine: Option<Arc<crate::runtime::InferenceEngine>>,
    prompt: String,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    cache_options: GenerationCacheOptions,
    permit: OwnedSemaphorePermit,
) -> Result<crate::runtime::GeneratedText, OpenAiHttpError> {
    let mut generated = generate_texts(
        engine,
        vec![prompt],
        max_tokens,
        stop_sequences,
        cache_options,
        permit,
    )
    .await?;
    generated
        .pop()
        .ok_or_else(|| OpenAiHttpError::internal("inference did not return a completion"))
}

pub(super) async fn generate_texts(
    engine: Option<Arc<crate::runtime::InferenceEngine>>,
    prompts: Vec<String>,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    cache_options: GenerationCacheOptions,
    permit: OwnedSemaphorePermit,
) -> Result<Vec<crate::runtime::GeneratedText>, OpenAiHttpError> {
    let Some(engine) = engine else {
        return Err(OpenAiHttpError::service_unavailable(
            "no model is loaded; start ferrite-server with --model",
        ));
    };

    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        prompts
            .iter()
            .map(|prompt| {
                let mut stop_filter = StopSequenceFilter::new(stop_sequences.clone());
                let cache_options = cache_options.clone();
                engine
                    .generate_with_token_callback_and_cache_options(
                        prompt,
                        max_tokens,
                        cache_options,
                        |piece| {
                            let _ = stop_filter.push(piece);
                            if stop_filter.stopped() {
                                Ok(GenerationControl::Stop)
                            } else {
                                Ok(GenerationControl::Continue)
                            }
                        },
                    )
                    .map(|generated| apply_stop_sequences(generated, &stop_sequences))
                    .map_err(|error| OpenAiHttpError::internal(error.to_string()))
            })
            .collect()
    })
    .await
    .map_err(|error| OpenAiHttpError::internal(format!("inference task failed: {error}")))?
}
