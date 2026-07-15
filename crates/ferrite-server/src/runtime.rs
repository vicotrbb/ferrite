mod cache_options;
mod cache_trace;
mod json_grammar;
mod prefix_cache;
mod scheduler;

pub use cache_options::GenerationCacheOptions;
pub use cache_trace::{PromptCacheLookup, PromptCacheTrace};
pub use prefix_cache::PrefixCacheStats;
pub use scheduler::{BatchScheduler, BatchedGenerationEvent};

use ferrite_inference::sampling::{Sampler, SamplingConfig};
use ferrite_inference::scalar::{
    PromptEvaluationControl as ScalarPromptEvaluationControl,
    PromptEvaluationLocation as ScalarPromptEvaluationLocation, Q8KActivationMatvecPolicy,
    ScalarExecutionOptions, ScalarLlamaModel, ScalarLlamaSession,
};
use ferrite_model::{
    gguf::parse_gguf,
    model_file::MappedModelFile,
    tokenizer::{GgufTokenizer, TokenizationControl},
};
use json_grammar::JsonObjectConstraint;
use prefix_cache::{fnv64_bytes, RuntimePrefixCache};
use std::{error::Error, fmt, path::Path, sync::Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationControl {
    Continue,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptEvaluationControl {
    Continue,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptEvaluationLocation {
    prompt_token_index: usize,
    token_id: usize,
    layer_index: Option<usize>,
}

impl PromptEvaluationLocation {
    pub fn prompt_token_index(self) -> usize {
        self.prompt_token_index
    }

    pub fn token_id(self) -> usize {
        self.token_id
    }

    pub fn layer_index(self) -> Option<usize> {
        self.layer_index
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GenerationStage {
    PromptTokenized,
    PrefixCacheKeyBuilt,
    SessionStarted,
    PrefixCacheLookupFinished,
    PrefixCacheRestored,
    PromptEvaluationStarted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationFinishReason {
    Stop,
    Length,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationFinishSource {
    Length,
    Eos,
    GenerationControl,
    StopSequence,
    StructuredOutput,
}

impl GenerationFinishSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Eos => "eos",
            Self::GenerationControl => "generation_control",
            Self::StopSequence => "stop_sequence",
            Self::StructuredOutput => "structured_output",
        }
    }
}

#[derive(Debug)]
pub struct InferenceEngine {
    model: ScalarLlamaModel,
    tokenizer: GgufTokenizer,
    execution_options: ScalarExecutionOptions,
    model_fingerprint: String,
    tokenizer_fingerprint: String,
    chat_template: Option<String>,
    chat_template_bos_token: Option<String>,
    chat_template_fingerprint: String,
    prefix_cache: Mutex<RuntimePrefixCache>,
}

impl InferenceEngine {
    pub fn load(path: &Path) -> Result<Self, RuntimeError> {
        let bytes = std::fs::read(path)
            .map_err(|error| RuntimeError::new(format!("failed to read model: {error}")))?;
        let gguf = parse_gguf(&bytes)
            .map_err(|error| RuntimeError::new(format!("failed to parse GGUF: {error}")))?;
        let chat_template = gguf
            .chat_template()
            .map_err(|error| RuntimeError::new(format!("failed to load chat template: {error}")))?
            .map(str::to_owned);
        let tokenizer = GgufTokenizer::from_gguf(&gguf)
            .map_err(|error| RuntimeError::new(format!("failed to load tokenizer: {error}")))?;
        let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)
            .map_err(|error| RuntimeError::new(format!("failed to load scalar model: {error}")))?;
        Self::from_loaded_parts(model, tokenizer, fnv64_bytes(&bytes), chat_template)
    }

    /// Loads a model while retaining zero-copy ranges of a read-only mapping.
    ///
    /// # Safety
    ///
    /// The underlying file must not be modified or truncated until the engine
    /// and every clone of its mapped tensors have been dropped.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be mapped or its GGUF metadata,
    /// tokenizer, model tensors, or numeric values are invalid.
    #[allow(
        unsafe_code,
        reason = "the caller accepts the model-file stability requirement"
    )]
    pub unsafe fn load_mapped(path: &Path) -> Result<Self, RuntimeError> {
        // SAFETY: this method transfers the documented file-stability
        // requirement to its caller.
        let mapped_model = unsafe { MappedModelFile::open(path) }
            .map_err(|error| RuntimeError::new(format!("failed to read model: {error}")))?;
        let bytes = mapped_model.as_bytes();
        let gguf = parse_gguf(bytes)
            .map_err(|error| RuntimeError::new(format!("failed to parse GGUF: {error}")))?;
        let chat_template = gguf
            .chat_template()
            .map_err(|error| RuntimeError::new(format!("failed to load chat template: {error}")))?
            .map(str::to_owned);
        let tokenizer = GgufTokenizer::from_gguf(&gguf)
            .map_err(|error| RuntimeError::new(format!("failed to load tokenizer: {error}")))?;
        let model = ScalarLlamaModel::from_gguf_mapped(&gguf, &mapped_model)
            .map_err(|error| RuntimeError::new(format!("failed to load scalar model: {error}")))?;
        Self::from_loaded_parts(model, tokenizer, fnv64_bytes(bytes), chat_template)
    }

    fn from_loaded_parts(
        model: ScalarLlamaModel,
        tokenizer: GgufTokenizer,
        content_hash: u64,
        chat_template: Option<String>,
    ) -> Result<Self, RuntimeError> {
        let chat_template_bos_token = tokenizer
            .bos_token_id()
            .and_then(|token_id| tokenizer.token(token_id))
            .map(str::to_owned);
        let chat_template_fingerprint = chat_template.as_ref().map_or_else(
            || "runtime-rendered-prompt-v1".to_owned(),
            |template| {
                format!(
                    "gguf-chat-template-fnv64:{:016x}",
                    fnv64_bytes(template.as_bytes())
                )
            },
        );
        Ok(Self {
            model,
            tokenizer,
            execution_options: ScalarExecutionOptions::default(),
            model_fingerprint: format!("gguf-model-fnv64:{content_hash:016x}"),
            tokenizer_fingerprint: format!("gguf-tokenizer-fnv64:{content_hash:016x}"),
            chat_template,
            chat_template_bos_token,
            chat_template_fingerprint,
            prefix_cache: Mutex::new(RuntimePrefixCache::default()),
        })
    }

    pub(crate) fn chat_template(&self) -> Option<&str> {
        self.chat_template.as_deref()
    }

    pub(crate) fn chat_template_bos_token(&self) -> Option<&str> {
        self.chat_template_bos_token.as_deref()
    }

    /// Returns the loaded tokenizer vocabulary size.
    pub fn vocabulary_size(&self) -> usize {
        self.tokenizer.len()
    }

    /// Returns the model's GGUF-declared maximum context length.
    pub fn model_context_length(&self) -> Option<usize> {
        self.model.context_length()
    }

    pub fn with_execution_options(mut self, execution_options: ScalarExecutionOptions) -> Self {
        self.execution_options = execution_options;
        self
    }

    pub(crate) fn start_session(&self) -> Result<ScalarLlamaSession<'_>, RuntimeError> {
        self.model
            .start_session_with_options(self.execution_options)
            .map_err(|error| RuntimeError::new(format!("failed to start session: {error}")))
    }

    pub fn validate_session_configuration(&self) -> Result<(), RuntimeError> {
        let _session = self.start_session()?;
        Ok(())
    }

    pub(crate) fn batch_decode_compatible(&self) -> bool {
        self.execution_options.q8_k_activation_matvec_policy()
            == Q8KActivationMatvecPolicy::DefaultOnly
    }

    pub(crate) fn validate_kv_capacity(
        &self,
        prompt_tokens: usize,
        max_tokens: usize,
    ) -> Result<(), RuntimeError> {
        let required = prompt_tokens
            .checked_add(max_tokens.saturating_sub(1))
            .ok_or_else(|| RuntimeError::new("requested KV token count overflowed"))?;
        if let Some(context_length) = self.model.context_length() {
            if required > context_length {
                return Err(RuntimeError::new(format!(
                    "request needs capacity for {required} KV tokens but the model context length is {context_length}"
                )));
            }
        }
        if let ferrite_inference::scalar::KvBackend::Locus {
            max_tokens: cap, ..
        } = self.execution_options.kv_backend()
        {
            if required > cap {
                return Err(RuntimeError::new(format!(
                    "request needs capacity for {required} KV tokens but the configured Locus limit is {cap}"
                )));
            }
        }
        Ok(())
    }

    pub fn generate(&self, prompt: &str, max_tokens: usize) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_token_callback(prompt, max_tokens, |_| Ok(GenerationControl::Continue))
    }

    pub fn generate_with_sampling(
        &self,
        prompt: &str,
        max_tokens: usize,
        sampling: SamplingConfig,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_sampling_and_token_callback_and_cache_options(
            prompt,
            max_tokens,
            sampling,
            GenerationCacheOptions::default(),
            |_| Ok(GenerationControl::Continue),
        )
    }

    pub fn generate_with_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        cache_options: GenerationCacheOptions,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_token_callback_and_cache_options(
            prompt,
            max_tokens,
            cache_options,
            |_| Ok(GenerationControl::Continue),
        )
    }

    pub fn generate_with_token_callback(
        &self,
        prompt: &str,
        max_tokens: usize,
        mut on_token: impl FnMut(&str) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_token_callback_and_cache_options(
            prompt,
            max_tokens,
            GenerationCacheOptions::default(),
            &mut on_token,
        )
    }

    pub fn generate_with_token_callback_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        cache_options: GenerationCacheOptions,
        mut on_token: impl FnMut(&str) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_token_event_callback_and_cache_options(
            prompt,
            max_tokens,
            cache_options,
            |token_text, _token_ids| on_token(token_text),
        )
    }

    pub fn generate_with_token_event_callback_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        cache_options: GenerationCacheOptions,
        mut on_token: impl FnMut(&str, &[usize]) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_prompt_callback_and_cache_options(
            prompt,
            max_tokens,
            cache_options,
            |_, _| PromptEvaluationControl::Continue,
            &mut on_token,
        )
    }

    pub fn generate_with_sampling_and_token_callback_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        sampling: SamplingConfig,
        cache_options: GenerationCacheOptions,
        mut on_token: impl FnMut(&str) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_sampling_and_token_event_callback_and_cache_options(
            prompt,
            max_tokens,
            sampling,
            cache_options,
            |token_text, _token_ids| on_token(token_text),
        )
    }

    pub(crate) fn generate_json_object_with_sampling_and_token_callback_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        sampling: SamplingConfig,
        cache_options: GenerationCacheOptions,
        mut on_token: impl FnMut(&str) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        let constraint = JsonObjectConstraint::new(&self.tokenizer)?;
        self.generate_with_stage_callbacks_and_constraint(
            prompt,
            max_tokens,
            sampling,
            cache_options,
            Some(constraint),
            || PromptEvaluationControl::Continue,
            |_| {},
            |_, _| PromptEvaluationControl::Continue,
            |_| PromptEvaluationControl::Continue,
            |token_text, _token_ids| on_token(token_text),
        )
    }

    pub fn generate_with_sampling_and_token_event_callback_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        sampling: SamplingConfig,
        cache_options: GenerationCacheOptions,
        mut on_token: impl FnMut(&str, &[usize]) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_stage_callbacks_and_cache_options(
            prompt,
            max_tokens,
            sampling,
            cache_options,
            || PromptEvaluationControl::Continue,
            |_| {},
            |_, _| PromptEvaluationControl::Continue,
            |_| PromptEvaluationControl::Continue,
            &mut on_token,
        )
    }

    pub(crate) fn generate_with_prompt_callback_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        cache_options: GenerationCacheOptions,
        mut on_prompt_token: impl FnMut(usize, usize) -> PromptEvaluationControl,
        mut on_token: impl FnMut(&str, &[usize]) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_prompt_callbacks_and_cache_options(
            prompt,
            max_tokens,
            cache_options,
            &mut on_prompt_token,
            |_| PromptEvaluationControl::Continue,
            &mut on_token,
        )
    }

    pub(crate) fn generate_with_prompt_callbacks_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        cache_options: GenerationCacheOptions,
        mut on_prompt_token: impl FnMut(usize, usize) -> PromptEvaluationControl,
        mut on_prompt_cancellation_poll: impl FnMut(PromptEvaluationLocation) -> PromptEvaluationControl,
        mut on_token: impl FnMut(&str, &[usize]) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_stage_callbacks_and_cache_options(
            prompt,
            max_tokens,
            SamplingConfig::default(),
            cache_options,
            || PromptEvaluationControl::Continue,
            |_| {},
            &mut on_prompt_token,
            &mut on_prompt_cancellation_poll,
            &mut on_token,
        )
    }

    // The callback tower mirrors the generation lifecycle stages one-to-one;
    // collapsing them into a struct is deferred to the scheduler-owned decode
    // loop planned for concurrent serving.
    #[allow(
        clippy::too_many_arguments,
        reason = "callbacks map one-to-one to observable generation lifecycle stages"
    )]
    pub(crate) fn generate_with_stage_callbacks_and_cache_options(
        &self,
        prompt: &str,
        max_tokens: usize,
        sampling: SamplingConfig,
        cache_options: GenerationCacheOptions,
        mut on_tokenization_cancellation_poll: impl FnMut() -> PromptEvaluationControl,
        mut on_generation_stage: impl FnMut(GenerationStage),
        mut on_prompt_token: impl FnMut(usize, usize) -> PromptEvaluationControl,
        mut on_prompt_cancellation_poll: impl FnMut(PromptEvaluationLocation) -> PromptEvaluationControl,
        mut on_token: impl FnMut(&str, &[usize]) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        self.generate_with_stage_callbacks_and_constraint(
            prompt,
            max_tokens,
            sampling,
            cache_options,
            None,
            &mut on_tokenization_cancellation_poll,
            &mut on_generation_stage,
            &mut on_prompt_token,
            &mut on_prompt_cancellation_poll,
            &mut on_token,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "constraint selection and lifecycle callbacks are independent generation policies"
    )]
    fn generate_with_stage_callbacks_and_constraint(
        &self,
        prompt: &str,
        max_tokens: usize,
        sampling: SamplingConfig,
        cache_options: GenerationCacheOptions,
        mut constraint: Option<JsonObjectConstraint>,
        mut on_tokenization_cancellation_poll: impl FnMut() -> PromptEvaluationControl,
        mut on_generation_stage: impl FnMut(GenerationStage),
        mut on_prompt_token: impl FnMut(usize, usize) -> PromptEvaluationControl,
        mut on_prompt_cancellation_poll: impl FnMut(PromptEvaluationLocation) -> PromptEvaluationControl,
        mut on_token: impl FnMut(&str, &[usize]) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GeneratedText, RuntimeError> {
        if max_tokens == 0 {
            return Err(RuntimeError::new("max tokens must be greater than zero"));
        }
        sampling
            .validate()
            .map_err(|error| RuntimeError::new(format!("invalid sampling policy: {error}")))?;
        let prompt_token_ids = self
            .tokenizer
            .encode_with_cancellation(prompt, || {
                map_tokenization_control(on_tokenization_cancellation_poll())
            })
            .map_err(|error| RuntimeError::new(format!("failed to tokenize prompt: {error}")))?;
        on_generation_stage(GenerationStage::PromptTokenized);
        if prompt_token_ids.is_empty() {
            return Err(RuntimeError::new("prompt must contain at least one token"));
        }
        self.validate_kv_capacity(prompt_token_ids.len(), max_tokens)?;
        let prefix_cache_key = self.prefix_cache_key_for_tokens(&prompt_token_ids, &cache_options);
        on_generation_stage(GenerationStage::PrefixCacheKeyBuilt);

        let mut session = self.start_session()?;
        on_generation_stage(GenerationStage::SessionStarted);
        let mut cached_prompt_tokens = 0;
        let mut prompt_cache_trace = None;
        let next = if cache_options.prefix_cache_enabled() {
            let lookup = self.prefix_cache_lookup(&prefix_cache_key)?;
            on_generation_stage(GenerationStage::PrefixCacheLookupFinished);
            if cache_options.prompt_cache_trace_enabled() {
                prompt_cache_trace = Some(lookup.to_trace(&prefix_cache_key, true));
            }
            if let Some(cached) = lookup.into_value() {
                session
                    .restore_cache_snapshot(cached.snapshot())
                    .map_err(|error| {
                        RuntimeError::new(format!("failed to restore prompt cache: {error}"))
                    })?;
                on_generation_stage(GenerationStage::PrefixCacheRestored);
                cached_prompt_tokens = cached.snapshot().cached_token_count();
                let suffix = &prompt_token_ids[cached_prompt_tokens..];
                if suffix.is_empty() {
                    if let Some(next) = cached.next_token().cloned() {
                        next
                    } else {
                        let recompute_start =
                            cached_prompt_tokens.checked_sub(1).ok_or_else(|| {
                                RuntimeError::new(
                                    "prefix cache hit cannot recover logits for an empty prompt",
                                )
                            })?;
                        session.truncate_cache(recompute_start).map_err(|error| {
                            RuntimeError::new(format!(
                                "failed to truncate prompt cache for logits recovery: {error}"
                            ))
                        })?;
                        cached_prompt_tokens = recompute_start;
                        on_generation_stage(GenerationStage::PromptEvaluationStarted);
                        let next = session
                            .accept_prompt_with_control_and_location_cancellation(
                                &prompt_token_ids[recompute_start..],
                                |index, token_id| {
                                    Ok(map_prompt_control(on_prompt_token(
                                        recompute_start + index,
                                        token_id,
                                    )))
                                },
                                |location| {
                                    Ok(map_prompt_control(on_prompt_cancellation_poll(
                                        map_prompt_location(location, recompute_start),
                                    )))
                                },
                            )
                            .map_err(|error| {
                                RuntimeError::new(format!(
                                    "failed to recover logits from prompt cache: {error}"
                                ))
                            })?;
                        self.store_prefix_cache_value(
                            prefix_cache_key.clone(),
                            session.cache_snapshot().map_err(|error| {
                                RuntimeError::new(format!(
                                    "failed to snapshot recovered prompt cache: {error}"
                                ))
                            })?,
                            next.clone(),
                        )?;
                        next
                    }
                } else {
                    on_generation_stage(GenerationStage::PromptEvaluationStarted);
                    let next = session
                        .accept_prompt_with_control_and_location_cancellation(
                            suffix,
                            |index, token_id| {
                                Ok(map_prompt_control(on_prompt_token(
                                    cached_prompt_tokens + index,
                                    token_id,
                                )))
                            },
                            |location| {
                                Ok(map_prompt_control(on_prompt_cancellation_poll(
                                    map_prompt_location(location, cached_prompt_tokens),
                                )))
                            },
                        )
                        .map_err(|error| {
                            RuntimeError::new(format!("failed to evaluate prompt suffix: {error}"))
                        })?;
                    self.store_prefix_cache_value(
                        prefix_cache_key.clone(),
                        session.cache_snapshot().map_err(|error| {
                            RuntimeError::new(format!("failed to snapshot prompt cache: {error}"))
                        })?,
                        next.clone(),
                    )?;
                    next
                }
            } else {
                on_generation_stage(GenerationStage::PromptEvaluationStarted);
                let next = session
                    .accept_prompt_with_control_and_location_cancellation(
                        &prompt_token_ids,
                        |index, token_id| Ok(map_prompt_control(on_prompt_token(index, token_id))),
                        |location| {
                            Ok(map_prompt_control(on_prompt_cancellation_poll(
                                map_prompt_location(location, 0),
                            )))
                        },
                    )
                    .map_err(|error| {
                        RuntimeError::new(format!("failed to evaluate prompt: {error}"))
                    })?;
                self.store_prefix_cache_value(
                    prefix_cache_key.clone(),
                    session.cache_snapshot().map_err(|error| {
                        RuntimeError::new(format!("failed to snapshot prompt cache: {error}"))
                    })?,
                    next.clone(),
                )?;
                next
            }
        } else {
            on_generation_stage(GenerationStage::PromptEvaluationStarted);
            session
                .accept_prompt_with_control_and_location_cancellation(
                    &prompt_token_ids,
                    |index, token_id| Ok(map_prompt_control(on_prompt_token(index, token_id))),
                    |location| {
                        Ok(map_prompt_control(on_prompt_cancellation_poll(
                            map_prompt_location(location, 0),
                        )))
                    },
                )
                .map_err(|error| RuntimeError::new(format!("failed to evaluate prompt: {error}")))?
        };
        if cache_options.prompt_cache_trace_enabled() && !cache_options.prefix_cache_enabled() {
            prompt_cache_trace = Some(PromptCacheTrace::new(
                false,
                prefix_cache_key.namespace().map(str::to_owned),
                prefix_cache_key.prefix_token_count(),
                prefix_cache_key.prefix_token_hash(),
                PromptCacheLookup::Disabled,
            ));
        }
        let uses_fused_greedy_path = constraint.is_none() && sampling.uses_fused_greedy_path();
        let mut sampler = if uses_fused_greedy_path {
            None
        } else {
            let mut sampler = Sampler::new(sampling).map_err(|error| {
                RuntimeError::new(format!("failed to initialize sampler: {error}"))
            })?;
            sampler.observe_all(&prompt_token_ids);
            Some(sampler)
        };
        let mut token_id = match sampler.as_mut() {
            Some(sampler) => select_token(sampler, &next.logits, constraint.as_mut())?,
            None => next.token_id,
        };
        let mut generated_token_ids = Vec::with_capacity(max_tokens);
        let mut token_id_chunks = Vec::with_capacity(max_tokens);
        let mut token_texts = Vec::with_capacity(max_tokens);
        let mut token_text_buffer = TokenTextBuffer::new();
        let mut finish_reason = GenerationFinishReason::Length;
        let mut finish_source = GenerationFinishSource::Length;
        let mut stopped_on_eos = false;

        for output_index in 0..max_tokens {
            generated_token_ids.push(token_id);
            if self.tokenizer.is_end_of_generation_token(token_id) {
                finish_reason = GenerationFinishReason::Stop;
                finish_source = GenerationFinishSource::Eos;
                stopped_on_eos = true;
                break;
            }
            let control = token_text_buffer.emit_ready_text(
                &generated_token_ids,
                |ids| self.decode_token_text(ids),
                |token_text, token_ids| {
                    let control = on_token(token_text, token_ids)?;
                    token_texts.push(token_text.to_owned());
                    token_id_chunks.push(token_ids.to_vec());
                    Ok(control)
                },
            )?;
            if control == GenerationControl::Stop {
                finish_reason = GenerationFinishReason::Stop;
                finish_source = GenerationFinishSource::GenerationControl;
                break;
            }
            if constraint
                .as_ref()
                .is_some_and(JsonObjectConstraint::is_complete)
            {
                finish_reason = GenerationFinishReason::Stop;
                finish_source = GenerationFinishSource::StructuredOutput;
                break;
            }
            if output_index + 1 == max_tokens {
                break;
            }
            token_id = match sampler.as_mut() {
                Some(sampler) => {
                    let next = session.accept_token(token_id).map_err(|error| {
                        RuntimeError::new(format!("failed to generate next logits: {error}"))
                    })?;
                    select_token(sampler, &next.logits, constraint.as_mut())?
                }
                None => session.accept_token_id(token_id).map_err(|error| {
                    RuntimeError::new(format!("failed to generate next token: {error}"))
                })?,
            };
        }

        if constraint
            .as_ref()
            .is_some_and(|constraint| !constraint.is_complete())
        {
            return Err(RuntimeError::new(
                "structured output ended before a complete JSON object was generated",
            ));
        }

        let visible_token_ids = if stopped_on_eos {
            &generated_token_ids[..generated_token_ids.len().saturating_sub(1)]
        } else {
            &generated_token_ids
        };
        let text = if visible_token_ids.is_empty() {
            String::new()
        } else {
            self.tokenizer.decode(visible_token_ids).map_err(|error| {
                RuntimeError::new(format!("failed to decode completion: {error}"))
            })?
        };
        GeneratedText::with_finish_reason(
            text,
            prompt_token_ids.len(),
            generated_token_ids.len(),
            token_texts,
            finish_reason,
        )
        .with_finish_source(finish_source)
        .with_token_id_chunks(token_id_chunks)?
        .with_cached_prompt_tokens(cached_prompt_tokens)?
        .with_optional_prompt_cache_trace(prompt_cache_trace)
    }

    fn decode_token_text(&self, token_ids: &[usize]) -> Result<Option<String>, RuntimeError> {
        self.tokenizer
            .decode_if_complete(token_ids)
            .map_err(|error| RuntimeError::new(format!("failed to decode token text: {error}")))
    }
}

fn select_token(
    sampler: &mut Sampler,
    logits: &[f32],
    constraint: Option<&mut JsonObjectConstraint>,
) -> Result<usize, RuntimeError> {
    match constraint {
        Some(constraint) => constraint.select_token(sampler, logits),
        None => sampler
            .sample(logits)
            .map_err(|error| RuntimeError::new(format!("failed to sample token: {error}"))),
    }
}

fn map_prompt_control(control: PromptEvaluationControl) -> ScalarPromptEvaluationControl {
    match control {
        PromptEvaluationControl::Continue => ScalarPromptEvaluationControl::Continue,
        PromptEvaluationControl::Cancel => ScalarPromptEvaluationControl::Cancel,
    }
}

fn map_tokenization_control(control: PromptEvaluationControl) -> TokenizationControl {
    match control {
        PromptEvaluationControl::Continue => TokenizationControl::Continue,
        PromptEvaluationControl::Cancel => TokenizationControl::Cancel,
    }
}

fn map_prompt_location(
    location: ScalarPromptEvaluationLocation,
    prompt_token_offset: usize,
) -> PromptEvaluationLocation {
    PromptEvaluationLocation {
        prompt_token_index: prompt_token_offset + location.prompt_token_index(),
        token_id: location.token_id(),
        layer_index: location.layer_index(),
    }
}

#[derive(Debug, Default)]
struct TokenTextBuffer {
    emitted_token_count: usize,
}

impl TokenTextBuffer {
    fn new() -> Self {
        Self::default()
    }

    fn emit_ready_text(
        &mut self,
        generated_token_ids: &[usize],
        decode: impl FnOnce(&[usize]) -> Result<Option<String>, RuntimeError>,
        mut on_text: impl FnMut(&str, &[usize]) -> Result<GenerationControl, RuntimeError>,
    ) -> Result<GenerationControl, RuntimeError> {
        if self.emitted_token_count >= generated_token_ids.len() {
            return Ok(GenerationControl::Continue);
        }

        let pending_token_ids = &generated_token_ids[self.emitted_token_count..];
        let Some(text) = decode(pending_token_ids)? else {
            return Ok(GenerationControl::Continue);
        };

        self.emitted_token_count = generated_token_ids.len();
        on_text(&text, pending_token_ids)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedText {
    text: String,
    prompt_tokens: usize,
    cached_prompt_tokens: usize,
    prompt_cache_trace: Option<PromptCacheTrace>,
    completion_tokens: usize,
    token_texts: Vec<String>,
    token_id_chunks: Vec<Vec<usize>>,
    finish_reason: GenerationFinishReason,
    finish_source: GenerationFinishSource,
}

impl GeneratedText {
    pub fn new(
        text: String,
        prompt_tokens: usize,
        completion_tokens: usize,
        token_texts: Vec<String>,
    ) -> Self {
        Self::with_finish_reason(
            text,
            prompt_tokens,
            completion_tokens,
            token_texts,
            GenerationFinishReason::Stop,
        )
    }

    pub fn with_finish_reason(
        text: String,
        prompt_tokens: usize,
        completion_tokens: usize,
        token_texts: Vec<String>,
        finish_reason: GenerationFinishReason,
    ) -> Self {
        let finish_source = match finish_reason {
            GenerationFinishReason::Stop => GenerationFinishSource::GenerationControl,
            GenerationFinishReason::Length => GenerationFinishSource::Length,
        };
        Self {
            text,
            prompt_tokens,
            cached_prompt_tokens: 0,
            prompt_cache_trace: None,
            completion_tokens,
            token_texts,
            token_id_chunks: Vec::new(),
            finish_reason,
            finish_source,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn prompt_tokens(&self) -> usize {
        self.prompt_tokens
    }

    pub fn cached_prompt_tokens(&self) -> usize {
        self.cached_prompt_tokens
    }

    pub fn with_cached_prompt_tokens(
        mut self,
        cached_prompt_tokens: usize,
    ) -> Result<Self, RuntimeError> {
        if cached_prompt_tokens > self.prompt_tokens {
            return Err(RuntimeError::new(format!(
                "cached prompt tokens {cached_prompt_tokens} exceed prompt tokens {}",
                self.prompt_tokens
            )));
        }
        self.cached_prompt_tokens = cached_prompt_tokens;
        Ok(self)
    }

    pub fn prompt_cache_trace(&self) -> Option<&PromptCacheTrace> {
        self.prompt_cache_trace.as_ref()
    }

    pub fn with_prompt_cache_trace(
        mut self,
        prompt_cache_trace: PromptCacheTrace,
    ) -> Result<Self, RuntimeError> {
        if prompt_cache_trace.prompt_token_count() != self.prompt_tokens {
            return Err(RuntimeError::new(format!(
                "prompt cache trace token count {} does not match prompt tokens {}",
                prompt_cache_trace.prompt_token_count(),
                self.prompt_tokens
            )));
        }
        self.prompt_cache_trace = Some(prompt_cache_trace);
        Ok(self)
    }

    fn with_optional_prompt_cache_trace(
        self,
        prompt_cache_trace: Option<PromptCacheTrace>,
    ) -> Result<Self, RuntimeError> {
        match prompt_cache_trace {
            Some(trace) => self.with_prompt_cache_trace(trace),
            None => Ok(self),
        }
    }

    pub fn completion_tokens(&self) -> usize {
        self.completion_tokens
    }

    pub fn token_texts(&self) -> &[String] {
        &self.token_texts
    }

    pub fn token_id_chunks(&self) -> &[Vec<usize>] {
        &self.token_id_chunks
    }

    pub fn with_token_id_chunks(
        mut self,
        token_id_chunks: Vec<Vec<usize>>,
    ) -> Result<Self, RuntimeError> {
        if token_id_chunks.len() != self.token_texts.len() {
            return Err(RuntimeError::new(format!(
                "token id chunk count {} does not match token text count {}",
                token_id_chunks.len(),
                self.token_texts.len()
            )));
        }
        let token_id_count = token_id_chunks.iter().map(Vec::len).sum::<usize>();
        if token_id_count > self.completion_tokens {
            return Err(RuntimeError::new(format!(
                "token id count {token_id_count} exceeds completion tokens {}",
                self.completion_tokens
            )));
        }
        self.token_id_chunks = token_id_chunks;
        Ok(self)
    }

    pub fn finish_reason(&self) -> GenerationFinishReason {
        self.finish_reason
    }

    pub fn finish_source(&self) -> GenerationFinishSource {
        self.finish_source
    }

    pub fn with_finish_source(mut self, finish_source: GenerationFinishSource) -> Self {
        self.finish_source = finish_source;
        self
    }

    pub(crate) fn with_filtered_stop_text(mut self, text: String) -> Self {
        self.token_texts = if text.is_empty() {
            Vec::new()
        } else {
            vec![text.clone()]
        };
        self.token_id_chunks.clear();
        self.text = text;
        self.finish_reason = GenerationFinishReason::Stop;
        self.finish_source = GenerationFinishSource::StopSequence;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn generate_with_token_callback_reports_each_token_piece(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;

        let mut pieces = Vec::new();
        let generated = engine.generate_with_token_callback("hello", 1, |piece| {
            pieces.push(piece.to_owned());
            Ok(GenerationControl::Continue)
        })?;

        assert_eq!(pieces, ["winner"]);
        assert_eq!(generated.text(), "winner");
        assert_eq!(generated.token_texts(), pieces);
        assert_eq!(
            generated.token_id_chunks().len(),
            generated.token_texts().len()
        );
        assert!(generated
            .token_id_chunks()
            .iter()
            .all(|chunk| !chunk.is_empty()));
        Ok(())
    }

    #[test]
    fn sampled_generation_is_seeded_and_isolated_per_request(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let sampling = SamplingConfig::default()
            .with_temperature(1.0)
            .with_seed(Some(42));

        let expected = engine.generate_with_sampling("hello", 4, sampling.clone())?;
        let _unrelated = engine.generate_with_sampling(
            "winner",
            4,
            SamplingConfig::default()
                .with_temperature(1.0)
                .with_seed(Some(7)),
        )?;
        let actual = engine.generate_with_sampling("hello", 4, sampling)?;

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn sampled_generation_applies_logit_bias() -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let mut logit_bias = std::collections::BTreeMap::new();
        logit_bias.insert(1, 100.0);

        let generated = engine.generate_with_sampling(
            "hello",
            1,
            SamplingConfig::default().with_logit_bias(logit_bias),
        )?;

        assert_eq!(generated.text(), "hello");
        Ok(())
    }

    #[test]
    fn default_sampling_preserves_greedy_generation() -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;

        assert_eq!(
            engine.generate_with_sampling("hello", 4, SamplingConfig::default())?,
            engine.generate("hello", 4)?
        );
        Ok(())
    }

    #[test]
    fn generate_marks_eos_finish_source() -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model_with_eos_token_id(2)?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;

        let generated = engine.generate("hello", 4)?;

        assert_eq!(generated.finish_reason(), GenerationFinishReason::Stop);
        assert_eq!(generated.finish_source(), GenerationFinishSource::Eos);
        Ok(())
    }

    #[test]
    fn generate_marks_eot_as_eos_finish_source() -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model_with_eot_token_id(2)?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;

        let generated = engine.generate("hello", 4)?;

        assert_eq!(generated.finish_reason(), GenerationFinishReason::Stop);
        assert_eq!(generated.finish_source(), GenerationFinishSource::Eos);
        assert_eq!(generated.completion_tokens(), 1);
        assert_eq!(generated.text(), "");
        Ok(())
    }

    #[test]
    fn generate_with_prompt_callback_cancels_before_next_prompt_token(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let mut observed_tokens = Vec::new();

        let error = match engine.generate_with_prompt_callback_and_cache_options(
            "hellowinner",
            1,
            GenerationCacheOptions::default(),
            |index, token_id| {
                observed_tokens.push((index, token_id));
                if index == 1 {
                    PromptEvaluationControl::Cancel
                } else {
                    PromptEvaluationControl::Continue
                }
            },
            |_, _| Ok(GenerationControl::Continue),
        ) {
            Ok(_) => return Err("generation should stop when prompt callback cancels".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "failed to evaluate prompt: prompt evaluation cancelled"
        );
        assert_eq!(observed_tokens, [(0, 1), (1, 2)]);
        Ok(())
    }

    #[test]
    fn generate_with_prompt_cancellation_poll_stops_during_prompt_token_evaluation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let mut polls = 0;

        let error = match engine.generate_with_prompt_callbacks_and_cache_options(
            "hello",
            1,
            GenerationCacheOptions::default(),
            |_, _| PromptEvaluationControl::Continue,
            |_| {
                polls += 1;
                if polls == 2 {
                    PromptEvaluationControl::Cancel
                } else {
                    PromptEvaluationControl::Continue
                }
            },
            |_, _| Ok(GenerationControl::Continue),
        ) {
            Ok(_) => return Err("generation should stop during prompt token evaluation".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "failed to evaluate prompt: prompt evaluation cancelled"
        );
        assert_eq!(polls, 2);
        Ok(())
    }

    #[test]
    fn prompt_cancellation_poll_reports_prompt_location() -> Result<(), Box<dyn std::error::Error>>
    {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let mut locations = Vec::new();

        let error = match engine.generate_with_prompt_callbacks_and_cache_options(
            "hello",
            1,
            GenerationCacheOptions::default(),
            |_, _| PromptEvaluationControl::Continue,
            |location| {
                locations.push(location);
                if location.layer_index() == Some(0) {
                    PromptEvaluationControl::Cancel
                } else {
                    PromptEvaluationControl::Continue
                }
            },
            |_, _| Ok(GenerationControl::Continue),
        ) {
            Ok(_) => return Err("generation should stop during prompt layer evaluation".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "failed to evaluate prompt: prompt evaluation cancelled"
        );
        assert_eq!(locations[0].prompt_token_index(), 0);
        assert_eq!(locations[0].token_id(), 1);
        assert_eq!(locations[0].layer_index(), None);
        assert_eq!(locations[1].prompt_token_index(), 0);
        assert_eq!(locations[1].token_id(), 1);
        assert_eq!(locations[1].layer_index(), Some(0));
        Ok(())
    }

    #[test]
    fn generation_stage_callback_reports_prefill_setup_order(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let mut stages = Vec::new();

        let generated = engine.generate_with_stage_callbacks_and_cache_options(
            "hello",
            1,
            SamplingConfig::default(),
            GenerationCacheOptions::default(),
            || PromptEvaluationControl::Continue,
            |stage| stages.push(stage),
            |_, _| PromptEvaluationControl::Continue,
            |_| PromptEvaluationControl::Continue,
            |_, _| Ok(GenerationControl::Stop),
        )?;

        assert_eq!(generated.finish_reason(), GenerationFinishReason::Stop);
        assert_eq!(
            stages,
            vec![
                GenerationStage::PromptTokenized,
                GenerationStage::PrefixCacheKeyBuilt,
                GenerationStage::SessionStarted,
                GenerationStage::PromptEvaluationStarted,
            ]
        );
        Ok(())
    }

    #[test]
    fn generation_tokenization_cancellation_stops_before_prompt_evaluation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let mut stages = Vec::new();

        let error = match engine.generate_with_stage_callbacks_and_cache_options(
            "hello",
            1,
            SamplingConfig::default(),
            GenerationCacheOptions::default(),
            || PromptEvaluationControl::Cancel,
            |stage| stages.push(stage),
            |_, _| PromptEvaluationControl::Continue,
            |_| PromptEvaluationControl::Continue,
            |_, _| Ok(GenerationControl::Continue),
        ) {
            Ok(_) => return Err("generation should stop during tokenization".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "failed to tokenize prompt: tokenization cancelled"
        );
        assert!(stages.is_empty());
        Ok(())
    }

    #[test]
    fn generated_text_records_cached_prompt_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let generated = GeneratedText::new("winner".to_owned(), 4, 1, vec!["winner".to_owned()])
            .with_cached_prompt_tokens(3)?;

        assert_eq!(generated.prompt_tokens(), 4);
        assert_eq!(generated.cached_prompt_tokens(), 3);
        Ok(())
    }

    #[test]
    fn generated_text_records_prompt_cache_trace() -> Result<(), Box<dyn std::error::Error>> {
        let trace = PromptCacheTrace::new(
            true,
            Some("tenant-a:thread-1".to_owned()),
            4,
            0x1234,
            PromptCacheLookup::SharedPrefixHit,
        )
        .with_selected_entry(2, 0x4567)
        .with_shared_prefix_tokens(2);

        let generated = GeneratedText::new("winner".to_owned(), 4, 1, vec!["winner".to_owned()])
            .with_prompt_cache_trace(trace.clone())?;

        assert_eq!(generated.prompt_cache_trace(), Some(&trace));
        Ok(())
    }

    #[test]
    fn generated_text_rejects_cached_prompt_tokens_above_prompt_tokens(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let error = match GeneratedText::new("winner".to_owned(), 2, 1, vec!["winner".to_owned()])
            .with_cached_prompt_tokens(3)
        {
            Ok(_) => return Err("cached prompt tokens above prompt token count should fail".into()),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("cached prompt tokens 3 exceed prompt tokens 2"));
        Ok(())
    }

    #[test]
    fn prefix_cache_key_uses_tokenized_prompt_and_cache_namespace(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;

        let key = engine.prefix_cache_key_for_prompt(
            "winner",
            &GenerationCacheOptions::from_namespace(Some("tenant-a:thread-1".to_owned())),
        )?;

        assert_eq!(key.prefix_tokens(), &[2]);
        assert_eq!(key.prefix_token_count(), 1);
        assert_eq!(key.namespace(), Some("tenant-a:thread-1"));
        assert!(key.fingerprints().model().starts_with("gguf-model-fnv64:"));
        assert!(key
            .fingerprints()
            .tokenizer()
            .starts_with("gguf-tokenizer-fnv64:"));
        assert_eq!(key.fingerprints().template(), "runtime-rendered-prompt-v1");
        assert_eq!(
            key.fingerprints().execution(),
            "scalar:default_only:kernels=auto:kv=vec"
        );
        assert_eq!(key.fingerprints().request_shape(), "text-generation-v1");
        Ok(())
    }

    #[test]
    fn prefix_cache_key_isolates_kernel_providers() -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let automatic = InferenceEngine::load(&model_path)?;
        let portable = InferenceEngine::load(&model_path)?.with_execution_options(
            ScalarExecutionOptions::default()
                .with_kernel_provider(ferrite_inference::scalar::KernelProvider::Portable),
        );
        remove_fixture_model(&model_path)?;
        let options = GenerationCacheOptions::default();

        let automatic_key = automatic.prefix_cache_key_for_prompt("winner", &options)?;
        let portable_key = portable.prefix_cache_key_for_prompt("winner", &options)?;

        assert_ne!(
            automatic_key.fingerprints().execution(),
            portable_key.fingerprints().execution()
        );
        assert!(portable_key
            .fingerprints()
            .execution()
            .contains("kernels=portable"));
        Ok(())
    }

    #[test]
    fn exact_prefix_cache_reuses_prompt_snapshot_when_enabled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let cache_options =
            GenerationCacheOptions::from_namespace(Some("tenant-a:thread-1".to_owned()))
                .with_prefix_cache_enabled(true);

        let first = engine.generate_with_cache_options("hello", 1, cache_options.clone())?;
        let second = engine.generate_with_cache_options("hello", 1, cache_options)?;

        assert_eq!(first.text(), "winner");
        assert_eq!(second.text(), first.text());
        assert_eq!(first.prompt_tokens(), 1);
        assert_eq!(first.cached_prompt_tokens(), 0);
        assert_eq!(second.prompt_tokens(), 1);
        assert_eq!(second.cached_prompt_tokens(), 1);
        Ok(())
    }

    #[test]
    fn prefix_cache_limits_bound_entries_and_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?.with_prefix_cache_limits(1, 1)?;
        remove_fixture_model(&model_path)?;
        let cache_options =
            GenerationCacheOptions::from_namespace(Some("tenant-a:bounded".to_owned()))
                .with_prefix_cache_enabled(true);

        let first = engine.generate_with_cache_options("hello", 1, cache_options.clone())?;
        let second = engine.generate_with_cache_options("hello", 1, cache_options)?;
        let stats = engine.prefix_cache_stats()?;

        assert_eq!(first.cached_prompt_tokens(), 0);
        assert_eq!(second.cached_prompt_tokens(), 0);
        assert_eq!(stats.entries(), 0);
        assert_eq!(stats.estimated_kv_bytes(), 0);
        assert_eq!(stats.max_entries(), 1);
        assert_eq!(stats.max_bytes(), 1);
        Ok(())
    }

    #[test]
    fn prefix_cache_eviction_keeps_active_snapshot_lease_valid(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?.with_prefix_cache_limits(1, u128::MAX)?;
        remove_fixture_model(&model_path)?;
        let cache_options =
            GenerationCacheOptions::from_namespace(Some("tenant-a:leased".to_owned()))
                .with_prefix_cache_enabled(true);
        engine.generate_with_cache_options("hello", 1, cache_options.clone())?;
        let hello_key = engine.prefix_cache_key_for_prompt("hello", &cache_options)?;
        let lease = engine
            .prefix_cache_lookup(&hello_key)?
            .into_value()
            .ok_or("expected cached hello snapshot")?;
        assert_eq!(lease.snapshot_owner_count(), 2);

        engine.generate_with_cache_options("winner", 1, cache_options)?;

        assert_eq!(engine.prefix_cache_stats()?.entries(), 1);
        assert_eq!(lease.snapshot_owner_count(), 1);
        let mut restored = engine.start_session()?;
        restored.restore_cache_snapshot(lease.snapshot())?;
        assert_eq!(restored.cached_token_count(), 1);
        assert!(engine
            .prefix_cache_lookup(&hello_key)?
            .into_value()
            .is_none());
        Ok(())
    }

    #[test]
    fn prefix_cache_churn_stays_within_configured_limits() -> Result<(), Box<dyn std::error::Error>>
    {
        const MAX_ENTRIES: usize = 3;
        const MAX_BYTES: u128 = 1024;
        let model_path = write_fixture_model()?;
        let engine =
            InferenceEngine::load(&model_path)?.with_prefix_cache_limits(MAX_ENTRIES, MAX_BYTES)?;
        remove_fixture_model(&model_path)?;
        let cache_options =
            GenerationCacheOptions::from_namespace(Some("tenant-a:churn".to_owned()))
                .with_prefix_cache_enabled(true);

        for token_count in 1..=64 {
            let prompt = "hello".repeat(token_count);
            let generated =
                engine.generate_with_cache_options(&prompt, 1, cache_options.clone())?;
            assert_eq!(generated.prompt_tokens(), token_count);
            let stats = engine.prefix_cache_stats()?;
            assert!(stats.entries() <= MAX_ENTRIES);
            assert!(stats.estimated_kv_bytes() <= MAX_BYTES);
        }

        let stats = engine.prefix_cache_stats()?;
        assert!(stats.entries() <= MAX_ENTRIES);
        assert!(stats.estimated_kv_bytes() <= MAX_BYTES);
        Ok(())
    }

    #[test]
    fn prefix_cache_namespaces_never_share_state() -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let tenant_a = GenerationCacheOptions::from_namespace(Some("tenant-a".to_owned()))
            .with_prefix_cache_enabled(true);
        let tenant_b = GenerationCacheOptions::from_namespace(Some("tenant-b".to_owned()))
            .with_prefix_cache_enabled(true);

        let first_a = engine.generate_with_cache_options("hello", 1, tenant_a.clone())?;
        let first_b = engine.generate_with_cache_options("hello", 1, tenant_b)?;
        let second_a = engine.generate_with_cache_options("hello", 1, tenant_a)?;

        assert_eq!(first_a.cached_prompt_tokens(), 0);
        assert_eq!(first_b.cached_prompt_tokens(), 0);
        assert_eq!(second_a.cached_prompt_tokens(), 1);
        Ok(())
    }

    #[cfg(all(feature = "locus-kv", unix))]
    #[test]
    fn locus_kv_capacity_fails_before_partial_prompt_evaluation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?.with_execution_options(
            ScalarExecutionOptions::default().with_kv_backend(
                ferrite_inference::scalar::KvBackend::Locus {
                    tokens_per_block: 1,
                    max_tokens: 2,
                },
            ),
        );
        remove_fixture_model(&model_path)?;

        let error = match engine.generate("hellowinner", 2) {
            Ok(_) => return Err("request above the Locus capacity should fail".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "request needs capacity for 3 KV tokens but the configured Locus limit is 2"
        );
        Ok(())
    }

    #[test]
    fn generation_rejects_requests_beyond_the_gguf_context_before_evaluation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        assert_eq!(engine.model_context_length(), Some(128));
        let prompt = "hello".repeat(128);

        let error = match engine.generate(&prompt, 2) {
            Ok(_) => return Err("request above the GGUF context should fail".into()),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "request needs capacity for 129 KV tokens but the model context length is 128"
        );
        Ok(())
    }

    #[test]
    fn sampled_generation_recovers_logits_from_greedy_cache_entry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let prompt = "hellowinner";
        let cache_options =
            GenerationCacheOptions::from_namespace(Some("tenant-a:greedy-to-sampled".to_owned()))
                .with_prefix_cache_enabled(true)
                .with_prompt_cache_trace_enabled(true);
        let prompt_token_ids = engine.tokenizer.encode(prompt)?;
        let key = engine.prefix_cache_key_for_tokens(&prompt_token_ids, &cache_options);
        let mut session = engine.start_session()?;
        session.accept_token_context_only(prompt_token_ids[0])?;
        let next_token_id = session.accept_token_id(prompt_token_ids[1])?;
        let snapshot = session.cache_snapshot()?;
        engine.store_prefix_cache_greedy_value(key, snapshot, next_token_id)?;
        let sampling = SamplingConfig::default()
            .with_temperature(1.0)
            .with_seed(Some(42));

        let uncached = engine.generate_with_sampling(prompt, 4, sampling.clone())?;
        let cached = engine.generate_with_sampling_and_token_callback_and_cache_options(
            prompt,
            4,
            sampling,
            cache_options,
            |_| Ok(GenerationControl::Continue),
        )?;

        assert_eq!(cached.text(), uncached.text());
        assert_eq!(cached.token_id_chunks(), uncached.token_id_chunks());
        assert_eq!(cached.finish_reason(), uncached.finish_reason());
        assert_eq!(cached.cached_prompt_tokens(), 1);
        assert_eq!(
            cached.prompt_cache_trace().map(PromptCacheTrace::lookup),
            Some(PromptCacheLookup::ExactHit)
        );
        Ok(())
    }

    #[test]
    fn prefix_cache_reuses_longest_prompt_prefix_when_enabled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let cache_options =
            GenerationCacheOptions::from_namespace(Some("tenant-a:thread-1".to_owned()))
                .with_prefix_cache_enabled(true);

        let cached_prefix =
            engine.generate_with_cache_options("hello", 1, cache_options.clone())?;
        let uncached_full_prompt = engine.generate("hellowinner", 1)?;
        let cached_full_prompt =
            engine.generate_with_cache_options("hellowinner", 1, cache_options)?;

        assert_eq!(cached_prefix.cached_prompt_tokens(), 0);
        assert_eq!(uncached_full_prompt.text(), cached_full_prompt.text());
        assert_eq!(uncached_full_prompt.prompt_tokens(), 2);
        assert_eq!(cached_full_prompt.prompt_tokens(), 2);
        assert_eq!(cached_full_prompt.cached_prompt_tokens(), 1);
        Ok(())
    }

    #[test]
    fn prefix_cache_reuses_shared_prompt_prefix_when_prompts_diverge(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let cache_options =
            GenerationCacheOptions::from_namespace(Some("tenant-a:thread-1".to_owned()))
                .with_prefix_cache_enabled(true);

        let divergent_cached_prompt =
            engine.generate_with_cache_options("hellowinner", 1, cache_options.clone())?;
        let uncached_requested_prompt = engine.generate("hellohello", 1)?;
        let cached_requested_prompt =
            engine.generate_with_cache_options("hellohello", 1, cache_options)?;

        assert_eq!(divergent_cached_prompt.cached_prompt_tokens(), 0);
        assert_eq!(
            uncached_requested_prompt.text(),
            cached_requested_prompt.text()
        );
        assert_eq!(cached_requested_prompt.prompt_tokens(), 2);
        assert_eq!(cached_requested_prompt.cached_prompt_tokens(), 1);
        Ok(())
    }

    #[test]
    fn prefix_cache_trace_explains_shared_prompt_prefix_hit(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model_path = write_fixture_model()?;
        let engine = InferenceEngine::load(&model_path)?;
        remove_fixture_model(&model_path)?;
        let cache_options =
            GenerationCacheOptions::from_namespace(Some("tenant-a:thread-1".to_owned()))
                .with_prefix_cache_enabled(true)
                .with_prompt_cache_trace_enabled(true);

        let first = engine.generate_with_cache_options("hellowinner", 1, cache_options.clone())?;
        let second = engine.generate_with_cache_options("hellohello", 1, cache_options)?;
        let trace = second
            .prompt_cache_trace()
            .ok_or("expected prompt cache trace")?;

        assert_eq!(
            first.prompt_cache_trace().map(PromptCacheTrace::lookup),
            Some(PromptCacheLookup::Miss)
        );
        assert_eq!(second.cached_prompt_tokens(), 1);
        assert_eq!(trace.namespace(), Some("tenant-a:thread-1"));
        assert_eq!(trace.prompt_token_count(), 2);
        assert_eq!(trace.selected_entry_token_count(), Some(2));
        assert_eq!(trace.shared_prefix_tokens(), 1);
        assert_eq!(trace.lookup(), PromptCacheLookup::SharedPrefixHit);
        Ok(())
    }

    #[test]
    fn token_text_buffer_waits_for_decodable_utf8_sequence(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = TokenTextBuffer::new();
        let mut generated_token_ids = vec![13];
        let mut pieces = Vec::new();
        let mut token_id_chunks = Vec::new();

        let control = buffer.emit_ready_text(
            &generated_token_ids,
            decode_partial_bpe,
            |piece, token_ids| {
                pieces.push(piece.to_owned());
                token_id_chunks.push(token_ids.to_vec());
                Ok(GenerationControl::Continue)
            },
        )?;

        assert_eq!(control, GenerationControl::Continue);
        assert!(pieces.is_empty());

        generated_token_ids.push(14);
        let control = buffer.emit_ready_text(
            &generated_token_ids,
            decode_partial_bpe,
            |piece, token_ids| {
                pieces.push(piece.to_owned());
                token_id_chunks.push(token_ids.to_vec());
                Ok(GenerationControl::Continue)
            },
        )?;

        assert_eq!(control, GenerationControl::Continue);
        assert_eq!(pieces, ["é"]);
        assert_eq!(token_id_chunks, [vec![13, 14]]);
        Ok(())
    }

    fn decode_partial_bpe(ids: &[usize]) -> Result<Option<String>, RuntimeError> {
        match ids {
            [13] => Ok(None),
            [13, 14] => Ok(Some("é".to_owned())),
            other => Err(RuntimeError::new(format!(
                "unexpected token ids: {other:?}"
            ))),
        }
    }

    fn write_fixture_model() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "ferrite-runtime-fixture-{}-{}.gguf",
            std::process::id(),
            FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, ferrite_fixtures::scalar_llama_f32_gguf_fixture())?;
        Ok(path)
    }

    fn write_fixture_model_with_eos_token_id(
        eos_token_id: u64,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "ferrite-runtime-eos-fixture-{}-{}.gguf",
            std::process::id(),
            FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(
            &path,
            ferrite_fixtures::scalar_llama_f32_gguf_fixture_with_eos_token_id(eos_token_id),
        )?;
        Ok(path)
    }

    fn write_fixture_model_with_eot_token_id(
        eot_token_id: u64,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "ferrite-runtime-eot-fixture-{}-{}.gguf",
            std::process::id(),
            FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(
            &path,
            ferrite_fixtures::scalar_llama_f32_gguf_fixture_with_eot_token_id(eot_token_id),
        )?;
        Ok(path)
    }

    fn remove_fixture_model(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}
