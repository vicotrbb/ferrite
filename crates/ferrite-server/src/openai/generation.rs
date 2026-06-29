use super::{
    error::OpenAiHttpError,
    schema::{ChatCompletionStreamContext, CompletionStreamContext},
    stop_filter::{apply_stop_sequences, StopSequenceFilter},
    streaming,
};
use crate::runtime::GenerationControl;
use axum::response::Response;
use std::sync::{Arc, Mutex};
use tokio::sync::OwnedSemaphorePermit;

pub(super) struct CompletionStreamOptions {
    stop_sequences: Vec<String>,
    include_usage: bool,
}

impl CompletionStreamOptions {
    pub(super) fn new(stop_sequences: Vec<String>, include_usage: bool) -> Self {
        Self {
            stop_sequences,
            include_usage,
        }
    }
}

pub(super) struct ChatStreamOptions {
    stop_sequences: Vec<String>,
    include_usage: bool,
    service_tier: Option<&'static str>,
}

impl ChatStreamOptions {
    pub(super) fn new(
        stop_sequences: Vec<String>,
        include_usage: bool,
        service_tier: Option<&'static str>,
    ) -> Self {
        Self {
            stop_sequences,
            include_usage,
            service_tier,
        }
    }
}

pub(super) fn completion_stream_response(
    engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
    model: String,
    prompt: String,
    max_tokens: usize,
    options: CompletionStreamOptions,
    permit: OwnedSemaphorePermit,
) -> Response {
    let include_usage = options.include_usage;
    let context = CompletionStreamContext::new(model).with_usage_field(include_usage);
    let token_context = context.clone();
    stream_generated_text(
        StreamGenerationInput::new(
            engine,
            prompt,
            max_tokens,
            options.stop_sequences,
            Vec::new(),
        ),
        move |piece| token_context.token(piece.to_owned()),
        move |generated| {
            let mut chunks = vec![context.finish(generated.finish_reason())];
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
    options: ChatStreamOptions,
    permit: OwnedSemaphorePermit,
) -> Response {
    let include_usage = options.include_usage;
    let context = ChatCompletionStreamContext::new(model)
        .with_usage_field(include_usage)
        .with_service_tier(options.service_tier);
    let token_context = context.clone();
    stream_generated_text(
        StreamGenerationInput::new(
            engine,
            prompt,
            max_tokens,
            options.stop_sequences,
            vec![context.role()],
        ),
        move |piece| token_context.token(piece.to_owned()),
        move |generated| {
            let mut chunks = vec![context.finish(generated.finish_reason())];
            if include_usage {
                chunks.push(context.usage(generated));
            }
            chunks
        },
        permit,
    )
}

struct StreamGenerationInput<T> {
    engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
    prompt: String,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    initial_chunks: Vec<T>,
}

impl<T> StreamGenerationInput<T> {
    fn new(
        engine: Arc<Mutex<crate::runtime::InferenceEngine>>,
        prompt: String,
        max_tokens: usize,
        stop_sequences: Vec<String>,
        initial_chunks: Vec<T>,
    ) -> Self {
        Self {
            engine,
            prompt,
            max_tokens,
            stop_sequences,
            initial_chunks,
        }
    }
}

fn stream_generated_text<T>(
    input: StreamGenerationInput<T>,
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
            for chunk in input.initial_chunks {
                sender.send_json_blocking(&chunk)?;
            }
            let mut stop_filter = StopSequenceFilter::new(input.stop_sequences);
            let engine = input
                .engine
                .lock()
                .map_err(|_| OpenAiHttpError::internal("inference engine lock is poisoned"))?;
            let generated = engine
                .generate_with_token_callback(&input.prompt, input.max_tokens, |piece| {
                    for visible_piece in stop_filter.push(piece) {
                        sender
                            .send_json_blocking(&token_chunk(&visible_piece))
                            .map_err(|error| {
                                crate::runtime::RuntimeError::new(error.to_string())
                            })?;
                    }
                    if stop_filter.stopped() {
                        Ok(GenerationControl::Stop)
                    } else {
                        Ok(GenerationControl::Continue)
                    }
                })
                .map_err(|error| OpenAiHttpError::internal(error.to_string()))?;
            for visible_piece in stop_filter.finish() {
                sender.send_json_blocking(&token_chunk(&visible_piece))?;
            }
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
