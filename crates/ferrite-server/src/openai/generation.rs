use super::{
    error::OpenAiHttpError,
    stop_filter::{apply_stop_sequences, StopSequenceFilter},
};
use crate::runtime::GenerationControl;
use std::sync::{Arc, Mutex};
use tokio::sync::OwnedSemaphorePermit;

pub(super) async fn generate_text(
    engine: Option<Arc<Mutex<crate::runtime::InferenceEngine>>>,
    prompt: String,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    permit: OwnedSemaphorePermit,
) -> Result<crate::runtime::GeneratedText, OpenAiHttpError> {
    let mut generated =
        generate_texts(engine, vec![prompt], max_tokens, stop_sequences, permit).await?;
    generated
        .pop()
        .ok_or_else(|| OpenAiHttpError::internal("inference did not return a completion"))
}

pub(super) async fn generate_texts(
    engine: Option<Arc<Mutex<crate::runtime::InferenceEngine>>>,
    prompts: Vec<String>,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    permit: OwnedSemaphorePermit,
) -> Result<Vec<crate::runtime::GeneratedText>, OpenAiHttpError> {
    let Some(engine) = engine else {
        return Err(OpenAiHttpError::service_unavailable(
            "no model is loaded; start ferrite-server with --model",
        ));
    };

    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let engine = engine
            .lock()
            .map_err(|_| OpenAiHttpError::internal("inference engine lock is poisoned"))?;
        prompts
            .iter()
            .map(|prompt| {
                let mut stop_filter = StopSequenceFilter::new(stop_sequences.clone());
                engine
                    .generate_with_token_callback(prompt, max_tokens, |piece| {
                        let _ = stop_filter.push(piece);
                        if stop_filter.stopped() {
                            Ok(GenerationControl::Stop)
                        } else {
                            Ok(GenerationControl::Continue)
                        }
                    })
                    .map(|generated| apply_stop_sequences(generated, &stop_sequences))
                    .map_err(|error| OpenAiHttpError::internal(error.to_string()))
            })
            .collect()
    })
    .await
    .map_err(|error| OpenAiHttpError::internal(format!("inference task failed: {error}")))?
}
