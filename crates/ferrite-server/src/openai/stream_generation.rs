use super::{
    error::OpenAiHttpError,
    schema::{ChatCompletionStreamContext, CompletionStreamContext},
    stop_filter::StopSequenceFilter,
    stream_lifecycle::{StreamDisconnectPoint, StreamFinishReason, StreamLifecycle},
    streaming,
};
use crate::runtime::{
    GenerationCacheOptions, GenerationControl, GenerationFinishSource, PromptEvaluationControl,
};
use axum::response::Response;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;

pub(super) struct CompletionStreamOptions {
    stop_sequences: Vec<String>,
    include_usage: bool,
    include_obfuscation: bool,
    echo_prompt: Option<String>,
}

impl CompletionStreamOptions {
    pub(super) fn new(
        stop_sequences: Vec<String>,
        include_usage: bool,
        include_obfuscation: bool,
    ) -> Self {
        Self {
            stop_sequences,
            include_usage,
            include_obfuscation,
            echo_prompt: None,
        }
    }

    pub(super) fn with_echo_prompt(mut self, prompt: Option<String>) -> Self {
        self.echo_prompt = prompt;
        self
    }
}

pub(super) struct ChatStreamOptions {
    stop_sequences: Vec<String>,
    include_usage: bool,
    include_obfuscation: bool,
    service_tier: Option<&'static str>,
}

impl ChatStreamOptions {
    pub(super) fn new(
        stop_sequences: Vec<String>,
        include_usage: bool,
        include_obfuscation: bool,
        service_tier: Option<&'static str>,
    ) -> Self {
        Self {
            stop_sequences,
            include_usage,
            include_obfuscation,
            service_tier,
        }
    }
}

pub(super) fn completion_stream_response(
    engine: Arc<crate::runtime::InferenceEngine>,
    model: String,
    prompt: String,
    max_tokens: usize,
    options: CompletionStreamOptions,
    cache_options: GenerationCacheOptions,
    permit: OwnedSemaphorePermit,
) -> Response {
    let include_usage = options.include_usage;
    let context = CompletionStreamContext::new(model)
        .with_usage_field(include_usage)
        .with_obfuscation_field(options.include_obfuscation);
    let initial_chunks = options
        .echo_prompt
        .into_iter()
        .map(|prompt| context.token(prompt))
        .collect();
    let token_context = context.clone();
    stream_generated_text(
        StreamGenerationInput::new(
            engine,
            prompt,
            max_tokens,
            options.stop_sequences,
            initial_chunks,
        )
        .with_cache_options(cache_options),
        move |piece, _token_ids| token_context.token(piece.to_owned()),
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
    engine: Arc<crate::runtime::InferenceEngine>,
    model: String,
    prompt: String,
    max_tokens: usize,
    options: ChatStreamOptions,
    cache_options: GenerationCacheOptions,
    permit: OwnedSemaphorePermit,
) -> Response {
    let include_usage = options.include_usage;
    let context = ChatCompletionStreamContext::new(model)
        .with_usage_field(include_usage)
        .with_obfuscation_field(options.include_obfuscation)
        .with_service_tier(options.service_tier);
    let token_context = context.clone();
    let include_token_ids = options.stop_sequences.is_empty();
    stream_generated_text(
        StreamGenerationInput::new(
            engine,
            prompt,
            max_tokens,
            options.stop_sequences,
            vec![context.role()],
        )
        .with_cache_options(cache_options),
        move |piece, token_ids| {
            if include_token_ids {
                token_context.token_with_ids(piece.to_owned(), token_ids.unwrap_or(&[]))
            } else {
                token_context.token(piece.to_owned())
            }
        },
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
    engine: Arc<crate::runtime::InferenceEngine>,
    prompt: String,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    cache_options: GenerationCacheOptions,
    initial_chunks: Vec<T>,
}

impl<T> StreamGenerationInput<T> {
    fn new(
        engine: Arc<crate::runtime::InferenceEngine>,
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
            cache_options: GenerationCacheOptions::default(),
            initial_chunks,
        }
    }

    fn with_cache_options(mut self, cache_options: GenerationCacheOptions) -> Self {
        self.cache_options = cache_options;
        self
    }
}

fn stream_generated_text<T>(
    input: StreamGenerationInput<T>,
    mut token_chunk: impl FnMut(&str, Option<&[usize]>) -> T + Send + 'static,
    final_chunks: impl FnOnce(&crate::runtime::GeneratedText) -> Vec<T> + Send + 'static,
    permit: OwnedSemaphorePermit,
) -> Response
where
    T: serde::Serialize + Send + 'static,
{
    let (sender, response) = streaming::channel_response();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let lifecycle = RefCell::new(StreamLifecycle::new());
        let result = (|| -> Result<(), OpenAiHttpError> {
            for chunk in input.initial_chunks {
                if let Err(error) = sender.send_json_blocking(&chunk) {
                    lifecycle
                        .borrow_mut()
                        .record_disconnect(StreamDisconnectPoint::BeforeGeneration);
                    return Err(error);
                }
            }
            if sender.is_closed() {
                lifecycle
                    .borrow_mut()
                    .record_disconnect(StreamDisconnectPoint::BeforeGeneration);
                return Ok(());
            }
            let include_token_ids = input.stop_sequences.is_empty();
            let mut stop_filter = StopSequenceFilter::new(input.stop_sequences);
            let engine = input.engine;
            {
                let mut lifecycle = lifecycle.borrow_mut();
                lifecycle.record_engine_lock_acquired();
                lifecycle.record_generation_started();
            }
            let mut generated = engine
                .generate_with_stage_callbacks_and_cache_options(
                    &input.prompt,
                    input.max_tokens,
                    input.cache_options,
                    || {
                        lifecycle
                            .borrow_mut()
                            .observe_tokenization_stream_state(sender.is_closed())
                    },
                    |stage| {
                        lifecycle.borrow_mut().record_generation_stage(stage);
                    },
                    |_, _| {
                        lifecycle.borrow_mut().record_prompt_token_started();
                        PromptEvaluationControl::Continue
                    },
                    |location| {
                        let mut lifecycle = lifecycle.borrow_mut();
                        lifecycle.record_prompt_cancellation_poll();
                        lifecycle.observe_stream_state(
                            StreamDisconnectPoint::PromptEvaluation,
                            location.prompt_token_index(),
                            location.layer_index(),
                            sender.is_closed(),
                        )
                    },
                    |piece, token_ids| {
                        lifecycle
                            .borrow_mut()
                            .record_generated_chunk(token_ids.len());
                        if include_token_ids {
                            sender
                                .send_json_blocking(&token_chunk(piece, Some(token_ids)))
                                .map_err(|error| {
                                    lifecycle
                                        .borrow_mut()
                                        .record_disconnect(StreamDisconnectPoint::TokenStreaming);
                                    crate::runtime::RuntimeError::new(error.to_string())
                                })?;
                        } else {
                            for visible_piece in stop_filter.push(piece) {
                                sender
                                    .send_json_blocking(&token_chunk(&visible_piece, None))
                                    .map_err(|error| {
                                        lifecycle.borrow_mut().record_disconnect(
                                            StreamDisconnectPoint::TokenStreaming,
                                        );
                                        crate::runtime::RuntimeError::new(error.to_string())
                                    })?;
                            }
                            if stop_filter.stopped() {
                                return Ok(GenerationControl::Stop);
                            }
                        }
                        Ok(GenerationControl::Continue)
                    },
                )
                .map_err(|error| OpenAiHttpError::internal(error.to_string()))?;
            if stop_filter.stopped() {
                generated = generated.with_finish_source(GenerationFinishSource::StopSequence);
            }
            if !include_token_ids {
                for visible_piece in stop_filter.finish() {
                    if let Err(error) =
                        sender.send_json_blocking(&token_chunk(&visible_piece, None))
                    {
                        lifecycle
                            .borrow_mut()
                            .record_disconnect(StreamDisconnectPoint::FinalChunks);
                        return Err(error);
                    }
                }
            }
            for chunk in final_chunks(&generated) {
                if let Err(error) = sender.send_json_blocking(&chunk) {
                    lifecycle
                        .borrow_mut()
                        .record_disconnect(StreamDisconnectPoint::FinalChunks);
                    return Err(error);
                }
            }
            if let Err(error) = sender.send_done_blocking() {
                lifecycle
                    .borrow_mut()
                    .record_disconnect(StreamDisconnectPoint::FinalChunks);
                return Err(error);
            }
            Ok(())
        })();
        let lifecycle = lifecycle.into_inner();
        let finish_reason = match (&result, lifecycle.has_disconnect()) {
            (_, true) => StreamFinishReason::Cancelled,
            (Ok(()), false) => StreamFinishReason::Completed,
            (Err(_), false) => StreamFinishReason::Failed,
        };
        eprintln!("{}", lifecycle.finish(finish_reason).log_line());
        if result.is_err() {
            let _ = sender.send_done_blocking();
        }
    });
    response
}
