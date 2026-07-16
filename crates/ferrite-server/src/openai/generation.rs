use super::{
    error::OpenAiHttpError,
    stop_filter::{StopSequenceFilter, apply_stop_sequences},
};
use crate::runtime::{
    BatchScheduler, BatchedGenerationEvent, GeneratedText, GenerationCacheOptions,
    GenerationControl,
};
use ferrite_inference::sampling::SamplingConfig;
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;

pub(super) async fn generate_text(
    engine: Option<Arc<crate::runtime::InferenceEngine>>,
    prompt: String,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    sampling: SamplingConfig,
    cache_options: GenerationCacheOptions,
    permit: OwnedSemaphorePermit,
) -> Result<crate::runtime::GeneratedText, OpenAiHttpError> {
    let mut generated = generate_texts(
        engine,
        vec![prompt],
        max_tokens,
        stop_sequences,
        sampling,
        cache_options,
        permit,
    )
    .await?;
    generated
        .pop()
        .ok_or_else(|| OpenAiHttpError::internal("inference did not return a completion"))
}

pub(super) async fn generate_json_object(
    engine: Arc<crate::runtime::InferenceEngine>,
    prompt: String,
    max_tokens: usize,
    sampling: SamplingConfig,
    cache_options: GenerationCacheOptions,
    permit: OwnedSemaphorePermit,
) -> Result<crate::runtime::GeneratedText, OpenAiHttpError> {
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let generated = engine
            .generate_json_object_with_sampling_and_token_callback_and_cache_options(
                &prompt,
                max_tokens,
                sampling,
                cache_options,
                |_| Ok(GenerationControl::Continue),
            )
            .map_err(|error| OpenAiHttpError::internal(error.to_string()))?;
        match serde_json::from_str::<serde_json::Value>(generated.text()) {
            Ok(serde_json::Value::Object(_)) => Ok(generated),
            Ok(_) | Err(_) => Err(OpenAiHttpError::internal(
                "JSON grammar completed with a non-object response",
            )),
        }
    })
    .await
    .map_err(|error| OpenAiHttpError::internal(format!("inference task failed: {error}")))?
}

pub(super) async fn generate_texts(
    engine: Option<Arc<crate::runtime::InferenceEngine>>,
    prompts: Vec<String>,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    sampling: SamplingConfig,
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
                    .generate_with_sampling_and_token_callback_and_cache_options(
                        prompt,
                        max_tokens,
                        sampling.clone(),
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

pub(super) async fn generate_batched_text(
    scheduler: Arc<BatchScheduler>,
    prompt: String,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    cache_options: GenerationCacheOptions,
    permit: OwnedSemaphorePermit,
) -> Result<GeneratedText, OpenAiHttpError> {
    let mut generated = generate_batched_texts(
        scheduler,
        vec![prompt],
        max_tokens,
        stop_sequences,
        cache_options,
        permit,
    )
    .await?;
    generated
        .pop()
        .ok_or_else(|| OpenAiHttpError::internal("batch scheduler did not return a completion"))
}

pub(super) async fn generate_batched_texts(
    scheduler: Arc<BatchScheduler>,
    prompts: Vec<String>,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    cache_options: GenerationCacheOptions,
    permit: OwnedSemaphorePermit,
) -> Result<Vec<GeneratedText>, OpenAiHttpError> {
    let _permit = permit;
    let mut generated = Vec::with_capacity(prompts.len());
    for prompt in prompts {
        let mut events = scheduler
            .submit(
                prompt,
                max_tokens,
                stop_sequences.clone(),
                cache_options.clone(),
            )
            .map_err(|error| OpenAiHttpError::internal(error.to_string()))?;
        let completion = loop {
            match events.recv().await {
                Some(BatchedGenerationEvent::Token { .. }) => {}
                Some(BatchedGenerationEvent::Finished(completion)) => break completion,
                Some(BatchedGenerationEvent::Failed(error)) => {
                    return Err(OpenAiHttpError::internal(error.to_string()));
                }
                None => {
                    return Err(OpenAiHttpError::internal(
                        "batch scheduler stopped before generation completed",
                    ));
                }
            }
        };
        generated.push(apply_stop_sequences(completion, &stop_sequences));
    }
    Ok(generated)
}
