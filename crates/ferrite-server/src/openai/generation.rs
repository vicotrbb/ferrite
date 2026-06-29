use super::{
    error::OpenAiHttpError,
    schema::{ChatCompletionStreamContext, CompletionStreamContext},
    streaming,
};
use axum::response::Response;
use std::sync::{Arc, Mutex};
use tokio::sync::OwnedSemaphorePermit;

pub(super) fn completion_stream_response(
    engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
    model: String,
    prompt: String,
    max_tokens: usize,
    include_usage: bool,
    permit: OwnedSemaphorePermit,
) -> Response {
    let context = CompletionStreamContext::new(model).with_usage_field(include_usage);
    let token_context = context.clone();
    stream_generated_text(
        engine,
        prompt,
        max_tokens,
        Vec::new(),
        move |piece| token_context.token(piece.to_owned()),
        move |generated| {
            let mut chunks = vec![context.stop()];
            if include_usage {
                chunks.push(context.usage(generated));
            }
            chunks
        },
        permit,
    )
}

pub(super) fn chat_stream_response(
    engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
    model: String,
    prompt: String,
    max_tokens: usize,
    include_usage: bool,
    permit: OwnedSemaphorePermit,
) -> Response {
    let context = ChatCompletionStreamContext::new(model).with_usage_field(include_usage);
    let token_context = context.clone();
    stream_generated_text(
        engine,
        prompt,
        max_tokens,
        vec![context.role()],
        move |piece| token_context.token(piece.to_owned()),
        move |generated| {
            let mut chunks = vec![context.stop()];
            if include_usage {
                chunks.push(context.usage(generated));
            }
            chunks
        },
        permit,
    )
}

fn stream_generated_text<T>(
    engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
    prompt: String,
    max_tokens: usize,
    initial_chunks: Vec<T>,
    mut token_chunk: impl FnMut(&str) -> T + Send + 'static,
    final_chunks: impl FnOnce(&crate::runtime::GeneratedText) -> Vec<T> + Send + 'static,
    permit: OwnedSemaphorePermit,
) -> Response
where
    T: serde::Serialize + Send + 'static,
{
    let (sender, response) = streaming::channel_response();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let result = (|| -> Result<(), OpenAiHttpError> {
            for chunk in initial_chunks {
                sender.send_json_blocking(&chunk)?;
            }
            let engine = engine
                .lock()
                .map_err(|_| OpenAiHttpError::internal("inference engine lock is poisoned"))?;
            let generated = engine
                .generate_with_token_callback(&prompt, max_tokens, |piece| {
                    sender
                        .send_json_blocking(&token_chunk(piece))
                        .map_err(|error| crate::runtime::RuntimeError::new(error.to_string()))?;
                    Ok(())
                })
                .map_err(|error| OpenAiHttpError::internal(error.to_string()))?;
            for chunk in final_chunks(&generated) {
                sender.send_json_blocking(&chunk)?;
            }
            sender.send_done_blocking()
        })();
        if result.is_err() {
            let _ = sender.send_done_blocking();
        }
    });
    response
}

pub(super) async fn generate_text(
    engine: Option<Arc<Mutex<crate::runtime::InferenceEngine>>>,
    prompt: String,
    max_tokens: usize,
    permit: OwnedSemaphorePermit,
) -> Result<crate::runtime::GeneratedText, OpenAiHttpError> {
    let mut generated = generate_texts(engine, vec![prompt], max_tokens, permit).await?;
    generated
        .pop()
        .ok_or_else(|| OpenAiHttpError::internal("inference did not return a completion"))
}

pub(super) async fn generate_texts(
    engine: Option<Arc<Mutex<crate::runtime::InferenceEngine>>>,
    prompts: Vec<String>,
    max_tokens: usize,
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
                engine
                    .generate(prompt, max_tokens)
                    .map_err(|error| OpenAiHttpError::internal(error.to_string()))
            })
            .collect()
    })
    .await
    .map_err(|error| OpenAiHttpError::internal(format!("inference task failed: {error}")))?
}
