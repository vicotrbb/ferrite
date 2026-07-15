use super::{
    attention::causal_attention,
    math::{add_assign, argmax, rms_norm, swiglu_in_place},
    profile::{ProfiledNextToken, ProfiledTokenId, ScalarMatVecComparison, ScalarProfileEvent},
    InferenceError, NextToken, Q8KActivationMatvecRole, ScalarExecutionOptions, ScalarLlamaModel,
};

mod cache;
mod profiling;
mod snapshot;

use profiling::{profiled_argmax_mul_vec, profiled_layer_mul_vec, profiled_mul_vec};
pub use snapshot::ScalarLlamaSessionSnapshot;

#[derive(Debug)]
/// Mutable generation state for one immutable [`ScalarLlamaModel`].
///
/// A session owns its KV cache and execution policy. Accepting a token appends
/// its per-layer key and value state and advances the cached position.
pub struct ScalarLlamaSession<'a> {
    model: &'a ScalarLlamaModel,
    store: super::kv_store::KvCacheStore,
    cached_token_count: usize,
    options: ScalarExecutionOptions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A callback decision made during prompt evaluation.
pub enum PromptEvaluationControl {
    /// Continue evaluating the prompt.
    Continue,
    /// Stop evaluation and return a cancellation error.
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The token and optional layer at which prompt cancellation is polled.
pub struct PromptEvaluationLocation {
    prompt_token_index: usize,
    token_id: usize,
    layer_index: Option<usize>,
}

impl PromptEvaluationLocation {
    /// Creates the location checked immediately before a prompt token begins.
    pub fn before_token(prompt_token_index: usize, token_id: usize) -> Self {
        Self {
            prompt_token_index,
            token_id,
            layer_index: None,
        }
    }

    /// Creates a location checked before evaluating a transformer layer.
    pub fn layer(prompt_token_index: usize, token_id: usize, layer_index: usize) -> Self {
        Self {
            prompt_token_index,
            token_id,
            layer_index: Some(layer_index),
        }
    }

    /// Returns the zero-based token index within the submitted prompt.
    pub fn prompt_token_index(self) -> usize {
        self.prompt_token_index
    }

    /// Returns the vocabulary token ID at this location.
    pub fn token_id(self) -> usize {
        self.token_id
    }

    /// Returns the zero-based layer index, or `None` before the token begins.
    pub fn layer_index(self) -> Option<usize> {
        self.layer_index
    }
}

impl<'a> ScalarLlamaSession<'a> {
    /// Accepts every token in a nonempty prompt and returns the final next token.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty prompt, an out-of-range token, or an
    /// inference shape, storage, or numeric failure.
    pub fn accept_prompt(&mut self, tokens: &[usize]) -> Result<NextToken, InferenceError> {
        self.accept_prompt_with_control_and_cancellation(
            tokens,
            |_, _| Ok(PromptEvaluationControl::Continue),
            || Ok(PromptEvaluationControl::Continue),
        )
    }

    /// Accepts a prompt while calling `on_prompt_token` before each token.
    ///
    /// The callback receives the prompt index and token ID. Returning
    /// [`PromptEvaluationControl::Cancel`] stops before that token is accepted.
    ///
    /// # Errors
    ///
    /// Returns callback errors, a cancellation error, or any error documented by
    /// [`Self::accept_prompt`].
    pub fn accept_prompt_with_control(
        &mut self,
        tokens: &[usize],
        mut on_prompt_token: impl FnMut(usize, usize) -> Result<PromptEvaluationControl, InferenceError>,
    ) -> Result<NextToken, InferenceError> {
        self.accept_prompt_with_control_and_cancellation(tokens, &mut on_prompt_token, || {
            Ok(PromptEvaluationControl::Continue)
        })
    }

    /// Accepts a prompt while polling a cancellation callback throughout work.
    ///
    /// # Errors
    ///
    /// Returns callback errors, a cancellation error, or any error documented by
    /// [`Self::accept_prompt`].
    pub fn accept_prompt_with_cancellation(
        &mut self,
        tokens: &[usize],
        mut on_cancellation_poll: impl FnMut() -> Result<PromptEvaluationControl, InferenceError>,
    ) -> Result<NextToken, InferenceError> {
        self.accept_prompt_with_control_and_cancellation(
            tokens,
            |_, _| Ok(PromptEvaluationControl::Continue),
            &mut on_cancellation_poll,
        )
    }

    /// Accepts a prompt with per-token control and location-free cancellation.
    ///
    /// # Errors
    ///
    /// Returns callback errors, a cancellation error, or any error documented by
    /// [`Self::accept_prompt`].
    pub fn accept_prompt_with_control_and_cancellation(
        &mut self,
        tokens: &[usize],
        mut on_prompt_token: impl FnMut(usize, usize) -> Result<PromptEvaluationControl, InferenceError>,
        mut on_cancellation_poll: impl FnMut() -> Result<PromptEvaluationControl, InferenceError>,
    ) -> Result<NextToken, InferenceError> {
        self.accept_prompt_with_control_and_location_cancellation(
            tokens,
            &mut on_prompt_token,
            |_| on_cancellation_poll(),
        )
    }

    /// Accepts a prompt with per-token control and location-aware cancellation.
    ///
    /// Cancellation is polled before each token and before each transformer
    /// layer. Tokens successfully completed before cancellation remain cached.
    ///
    /// # Errors
    ///
    /// Returns callback errors, a cancellation error, or any error documented by
    /// [`Self::accept_prompt`].
    pub fn accept_prompt_with_control_and_location_cancellation(
        &mut self,
        tokens: &[usize],
        mut on_prompt_token: impl FnMut(usize, usize) -> Result<PromptEvaluationControl, InferenceError>,
        mut on_cancellation_poll: impl FnMut(
            PromptEvaluationLocation,
        ) -> Result<PromptEvaluationControl, InferenceError>,
    ) -> Result<NextToken, InferenceError> {
        if tokens.is_empty() {
            return Err(InferenceError::new(
                "prompt must contain at least one token",
            ));
        }

        let mut next = None;
        for (index, token_id) in tokens.iter().copied().enumerate() {
            if on_prompt_token(index, token_id)? == PromptEvaluationControl::Cancel {
                return Err(InferenceError::new("prompt evaluation cancelled"));
            }
            if on_cancellation_poll(PromptEvaluationLocation::before_token(index, token_id))?
                == PromptEvaluationControl::Cancel
            {
                return Err(InferenceError::new("prompt evaluation cancelled"));
            }
            let on_layer = |layer_index| {
                on_cancellation_poll(PromptEvaluationLocation::layer(
                    index,
                    token_id,
                    layer_index,
                ))
            };
            if index + 1 == tokens.len() {
                next = Some(self.accept_token_with_layer_control(token_id, on_layer)?);
            } else {
                self.accept_token_context_only_with_layer_control(token_id, on_layer)?;
            }
        }

        next.ok_or_else(|| InferenceError::new("prompt must contain at least one token"))
    }

    /// Accepts one token and returns the selected next token and full logits.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range token or an inference shape,
    /// storage, callback, or numeric failure.
    pub fn accept_token(&mut self, token_id: usize) -> Result<NextToken, InferenceError> {
        let accepted = self.accept_token_inner(token_id, None, None, OutputMode::Logits, |_| {
            Ok(PromptEvaluationControl::Continue)
        })?;
        Ok(NextToken {
            token_id: accepted
                .token_id
                .ok_or_else(|| InferenceError::new("missing next token ID"))?,
            logits: accepted
                .logits
                .ok_or_else(|| InferenceError::new("missing logits for next token"))?,
        })
    }

    /// Accepts one token while polling before each transformer layer.
    ///
    /// # Errors
    ///
    /// Returns callback errors, a cancellation error, or any error documented by
    /// [`Self::accept_token`].
    pub fn accept_token_with_layer_control(
        &mut self,
        token_id: usize,
        on_layer: impl FnMut(usize) -> Result<PromptEvaluationControl, InferenceError>,
    ) -> Result<NextToken, InferenceError> {
        let accepted =
            self.accept_token_inner(token_id, None, None, OutputMode::Logits, on_layer)?;
        Ok(NextToken {
            token_id: accepted
                .token_id
                .ok_or_else(|| InferenceError::new("missing next token ID"))?,
            logits: accepted
                .logits
                .ok_or_else(|| InferenceError::new("missing logits for next token"))?,
        })
    }

    /// Accepts one token and returns only the selected next-token ID.
    ///
    /// This avoids materializing the vocabulary logit vector when the output
    /// matrix supports fused argmax dispatch.
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range token or an inference shape,
    /// storage, or numeric failure.
    pub fn accept_token_id(&mut self, token_id: usize) -> Result<usize, InferenceError> {
        self.accept_token_inner(token_id, None, None, OutputMode::TokenIdOnly, |_| {
            Ok(PromptEvaluationControl::Continue)
        })?
        .token_id
        .ok_or_else(|| InferenceError::new("missing next token ID"))
    }

    /// Accepts one token into the KV context without evaluating the output matrix.
    ///
    /// This is intended for non-final prompt tokens whose next-token result is
    /// not observable. Transformer state and the KV cache are updated exactly
    /// as they are for [`Self::accept_token_id`].
    ///
    /// # Errors
    ///
    /// Returns an error for an out-of-range token or an inference shape,
    /// storage, or numeric failure.
    pub fn accept_token_context_only(&mut self, token_id: usize) -> Result<(), InferenceError> {
        self.accept_token_context_only_with_layer_control(token_id, |_| {
            Ok(PromptEvaluationControl::Continue)
        })
    }

    fn accept_token_context_only_with_layer_control(
        &mut self,
        token_id: usize,
        on_layer: impl FnMut(usize) -> Result<PromptEvaluationControl, InferenceError>,
    ) -> Result<(), InferenceError> {
        self.accept_token_inner(token_id, None, None, OutputMode::ContextOnly, on_layer)?;
        Ok(())
    }

    /// Accepts one token and returns its next ID with matvec profile records.
    ///
    /// # Errors
    ///
    /// Returns any error documented by [`Self::accept_token_id`], plus errors
    /// raised while comparing experimental and reference kernels.
    pub fn accept_token_id_profiled(
        &mut self,
        token_id: usize,
    ) -> Result<ProfiledTokenId, InferenceError> {
        let mut events = Vec::new();
        let mut comparisons = Vec::new();
        let accepted = self.accept_token_inner(
            token_id,
            Some(&mut events),
            Some(&mut comparisons),
            OutputMode::TokenIdOnly,
            |_| Ok(PromptEvaluationControl::Continue),
        )?;
        Ok(ProfiledTokenId {
            token_id: accepted
                .token_id
                .ok_or_else(|| InferenceError::new("missing profiled next token ID"))?,
            events,
            comparisons,
        })
    }

    /// Greedily generates `count` token IDs, beginning with `first_token_id`.
    ///
    /// The first ID is included in the returned sequence and then accepted to
    /// select the following ID.
    ///
    /// # Errors
    ///
    /// Returns any error encountered while accepting a generated token.
    pub fn generate_token_ids(
        &mut self,
        first_token_id: usize,
        count: usize,
    ) -> Result<Vec<usize>, InferenceError> {
        let mut token_id = first_token_id;
        let mut token_ids = Vec::with_capacity(count);
        for _ in 0..count {
            token_ids.push(token_id);
            token_id = self.accept_token_id(token_id)?;
        }
        Ok(token_ids)
    }

    /// Accepts one token and returns full logits with matvec profile records.
    ///
    /// # Errors
    ///
    /// Returns any error documented by [`Self::accept_token`], plus errors
    /// raised while comparing experimental and reference kernels.
    pub fn accept_token_profiled(
        &mut self,
        token_id: usize,
    ) -> Result<ProfiledNextToken, InferenceError> {
        let mut events = Vec::new();
        let mut comparisons = Vec::new();
        let accepted = self.accept_token_inner(
            token_id,
            Some(&mut events),
            Some(&mut comparisons),
            OutputMode::Logits,
            |_| Ok(PromptEvaluationControl::Continue),
        )?;
        let next_token = NextToken {
            token_id: accepted
                .token_id
                .ok_or_else(|| InferenceError::new("missing profiled next token ID"))?,
            logits: accepted
                .logits
                .ok_or_else(|| InferenceError::new("missing logits for profiled next token"))?,
        };
        Ok(ProfiledNextToken {
            next_token,
            events,
            comparisons,
        })
    }

    fn accept_token_inner(
        &mut self,
        token_id: usize,
        mut profile_events: Option<&mut Vec<ScalarProfileEvent>>,
        mut comparison_events: Option<&mut Vec<ScalarMatVecComparison>>,
        output_mode: OutputMode,
        mut on_layer: impl FnMut(usize) -> Result<PromptEvaluationControl, InferenceError>,
    ) -> Result<AcceptedToken, InferenceError> {
        self.ensure_context_position_available()?;
        if token_id >= self.model.config.vocab_size {
            return Err(InferenceError::new(format!(
                "token id {token_id} is out of bounds for vocab size {}",
                self.model.config.vocab_size
            )));
        }

        let position = self.cached_token_count;
        let mut hidden = self.model.weights.token_embedding.row_values(token_id)?;

        for (layer_index, layer) in self.model.weights.layers.iter().enumerate() {
            if on_layer(layer_index)? == PromptEvaluationControl::Cancel {
                return Err(InferenceError::new("prompt evaluation cancelled"));
            }
            let normed = rms_norm(
                &hidden,
                &layer.attn_norm,
                self.model.config.rms_norm_epsilon,
            )?;
            let q_options = self
                .options
                .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::QProj);
            let k_options = self
                .options
                .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::KProj);
            let v_options = self
                .options
                .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::VProj);
            let (mut query, mut key, mut value) =
                if profile_events.is_none() && comparison_events.is_none() {
                    if let Some(qkv) = layer.q_proj.mul_vec_qkv_with_options(
                        &layer.k_proj,
                        &layer.v_proj,
                        &normed,
                        q_options,
                        k_options,
                        v_options,
                    )? {
                        qkv
                    } else {
                        let (query, (key, value)) = rayon::join(
                            || layer.q_proj.mul_vec_with_options(&normed, q_options),
                            || {
                                rayon::join(
                                    || layer.k_proj.mul_vec_with_options(&normed, k_options),
                                    || layer.v_proj.mul_vec_with_options(&normed, v_options),
                                )
                            },
                        );
                        (query?, key?, value?)
                    }
                } else {
                    let query = profiled_layer_mul_vec(
                        &layer.q_proj,
                        &normed,
                        layer_index,
                        "q_proj",
                        profile_events.as_deref_mut(),
                        comparison_events.as_deref_mut(),
                        q_options,
                    )?;
                    let key = profiled_layer_mul_vec(
                        &layer.k_proj,
                        &normed,
                        layer_index,
                        "k_proj",
                        profile_events.as_deref_mut(),
                        comparison_events.as_deref_mut(),
                        k_options,
                    )?;
                    let value = profiled_layer_mul_vec(
                        &layer.v_proj,
                        &normed,
                        layer_index,
                        "v_proj",
                        profile_events.as_deref_mut(),
                        comparison_events.as_deref_mut(),
                        v_options,
                    )?;
                    (query, key, value)
                };
            add_optional_bias(&mut query, layer.q_bias.as_deref())?;
            add_optional_bias(&mut key, layer.k_bias.as_deref())?;
            add_optional_bias(&mut value, layer.v_bias.as_deref())?;

            query = self.model.apply_rope_to_heads(
                &query,
                position,
                self.model.config.attention_head_count,
            )?;
            key = self.model.apply_rope_to_heads(
                &key,
                position,
                self.model.config.attention_head_count_kv,
            )?;

            self.store.push(layer_index, key, value)?;

            let attention =
                causal_attention(&self.model.config, &query, &mut self.store, layer_index)?;
            let attention_output = profiled_layer_mul_vec(
                &layer.o_proj,
                &attention,
                layer_index,
                "o_proj",
                profile_events.as_deref_mut(),
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::OProj),
            )?;
            add_assign(&mut hidden, &attention_output)?;

            let ffn_normed =
                rms_norm(&hidden, &layer.ffn_norm, self.model.config.rms_norm_epsilon)?;
            let gate_options = self
                .options
                .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnGate);
            let up_options = self
                .options
                .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnUp);
            let (mut gate, up) = if profile_events.is_none() && comparison_events.is_none() {
                let paired = layer.ffn_gate.mul_vec_pair_with_options(
                    &layer.ffn_up,
                    &ffn_normed,
                    gate_options,
                    up_options,
                )?;
                if let Some(pair) = paired {
                    pair
                } else {
                    let (gate, up) = rayon::join(
                        || {
                            layer
                                .ffn_gate
                                .mul_vec_with_options(&ffn_normed, gate_options)
                        },
                        || layer.ffn_up.mul_vec_with_options(&ffn_normed, up_options),
                    );
                    (gate?, up?)
                }
            } else {
                let gate = profiled_layer_mul_vec(
                    &layer.ffn_gate,
                    &ffn_normed,
                    layer_index,
                    "ffn_gate",
                    profile_events.as_deref_mut(),
                    comparison_events.as_deref_mut(),
                    gate_options,
                )?;
                let up = profiled_layer_mul_vec(
                    &layer.ffn_up,
                    &ffn_normed,
                    layer_index,
                    "ffn_up",
                    profile_events.as_deref_mut(),
                    comparison_events.as_deref_mut(),
                    up_options,
                )?;
                (gate, up)
            };
            swiglu_in_place(&mut gate, &up)?;
            let ffn_output = profiled_layer_mul_vec(
                &layer.ffn_down,
                &gate,
                layer_index,
                "ffn_down",
                profile_events.as_deref_mut(),
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnDown),
            )?;
            add_assign(&mut hidden, &ffn_output)?;
        }

        let (token_id, logits) = match output_mode {
            OutputMode::Logits => {
                let normed = rms_norm(
                    &hidden,
                    &self.model.weights.output_norm,
                    self.model.config.rms_norm_epsilon,
                )?;
                let output = self
                    .model
                    .weights
                    .output
                    .logits_matrix(&self.model.weights.token_embedding);
                let logits = profiled_mul_vec(
                    output,
                    &normed,
                    "output",
                    profile_events,
                    comparison_events,
                    self.options
                        .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::Output),
                )?;
                (Some(argmax(&logits)?), Some(logits))
            }
            OutputMode::TokenIdOnly => {
                let normed = rms_norm(
                    &hidden,
                    &self.model.weights.output_norm,
                    self.model.config.rms_norm_epsilon,
                )?;
                let output = self
                    .model
                    .weights
                    .output
                    .logits_matrix(&self.model.weights.token_embedding);
                let token_id = profiled_argmax_mul_vec(
                    output,
                    &normed,
                    "output",
                    profile_events,
                    comparison_events,
                    self.options
                        .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::Output),
                )?;
                (Some(token_id), None)
            }
            OutputMode::ContextOnly => (None, None),
        };
        self.cached_token_count += 1;

        Ok(AcceptedToken { token_id, logits })
    }

    fn ensure_context_position_available(&self) -> Result<(), InferenceError> {
        if let Some(context_length) = self.model.context_length {
            if self.cached_token_count >= context_length {
                return Err(InferenceError::new(format!(
                    "model context length {context_length} is exhausted at token position {}",
                    self.cached_token_count
                )));
            }
        }
        Ok(())
    }
}

/// Advances several sessions of the same model by one token each,
/// batching every weight matvec across the sessions so each weight row is
/// streamed from memory once per step instead of once per session.
///
/// Per-session arithmetic (norms, biases, `RoPE`, attention, residuals) and
/// per-stream matvec accumulation order are identical to
/// [`ScalarLlamaSession::accept_token_id`], so each session's next token
/// is bit-identical to what a sequential call would produce. Batched
/// matvecs use default kernel dispatch (no experimental `Q8_K` routing).
///
/// # Errors
///
/// Returns an error when the batch is empty, lengths or model identities do
/// not match, a token is out of range, or any session evaluation fails.
pub fn accept_token_ids_batch(
    sessions: &mut [ScalarLlamaSession<'_>],
    token_ids: &[usize],
) -> Result<Vec<usize>, InferenceError> {
    accept_token_ids_batch_inner(sessions, token_ids, true)?
        .ok_or_else(|| InferenceError::new("missing batched next token IDs"))
}

/// Advances several sessions without evaluating their output matrices.
///
/// This is the batched counterpart of
/// [`ScalarLlamaSession::accept_token_context_only`] for non-final prompt
/// tokens. Every session must share the same model.
///
/// # Errors
///
/// Returns an error when the batch is empty, lengths or model identities do
/// not match, a token is out of range, or inference fails.
pub fn accept_token_contexts_batch(
    sessions: &mut [ScalarLlamaSession<'_>],
    token_ids: &[usize],
) -> Result<(), InferenceError> {
    accept_token_ids_batch_inner(sessions, token_ids, false)?;
    Ok(())
}

fn accept_token_ids_batch_inner(
    sessions: &mut [ScalarLlamaSession<'_>],
    token_ids: &[usize],
    evaluate_output: bool,
) -> Result<Option<Vec<usize>>, InferenceError> {
    if sessions.is_empty() {
        return Err(InferenceError::new(
            "batch must contain at least one session",
        ));
    }
    if sessions.len() != token_ids.len() {
        return Err(InferenceError::new(format!(
            "batch has {} sessions but {} token ids",
            sessions.len(),
            token_ids.len()
        )));
    }
    let model = sessions[0].model;
    if sessions
        .iter()
        .any(|session| !std::ptr::eq(session.model, model))
    {
        return Err(InferenceError::new(
            "all sessions in a batch must share the same model",
        ));
    }
    let options = sessions[0].options;
    if sessions.iter().any(|session| session.options != options) {
        return Err(InferenceError::new(
            "all sessions in a batch must share the same execution options",
        ));
    }
    for session in sessions.iter() {
        session.ensure_context_position_available()?;
    }
    for token_id in token_ids {
        if *token_id >= model.config.vocab_size {
            return Err(InferenceError::new(format!(
                "token id {token_id} is out of bounds for vocab size {}",
                model.config.vocab_size
            )));
        }
    }

    let batch = sessions.len();
    let mut hidden = token_ids
        .iter()
        .map(|token_id| model.weights.token_embedding.row_values(*token_id))
        .collect::<Result<Vec<_>, _>>()?;

    for (layer_index, layer) in model.weights.layers.iter().enumerate() {
        let normed = hidden
            .iter()
            .map(|values| rms_norm(values, &layer.attn_norm, model.config.rms_norm_epsilon))
            .collect::<Result<Vec<_>, _>>()?;
        let normed_refs = normed.iter().map(Vec::as_slice).collect::<Vec<_>>();

        let query_options = options.scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::QProj);
        let key_options = options.scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::KProj);
        let value_options = options.scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::VProj);
        let (queries, (keys, values)) = rayon::join(
            || {
                layer
                    .q_proj
                    .mul_vec_batch_with_options(&normed_refs, query_options)
            },
            || {
                rayon::join(
                    || {
                        layer
                            .k_proj
                            .mul_vec_batch_with_options(&normed_refs, key_options)
                    },
                    || {
                        layer
                            .v_proj
                            .mul_vec_batch_with_options(&normed_refs, value_options)
                    },
                )
            },
        );
        let mut queries = queries?;
        let mut keys = keys?;
        let mut values = values?;

        let mut attention_outputs = Vec::with_capacity(batch);
        for (index, session) in sessions.iter_mut().enumerate() {
            add_optional_bias(&mut queries[index], layer.q_bias.as_deref())?;
            add_optional_bias(&mut keys[index], layer.k_bias.as_deref())?;
            add_optional_bias(&mut values[index], layer.v_bias.as_deref())?;

            let position = session.cached_token_count;
            let query = model.apply_rope_to_heads(
                &queries[index],
                position,
                model.config.attention_head_count,
            )?;
            let key = model.apply_rope_to_heads(
                &keys[index],
                position,
                model.config.attention_head_count_kv,
            )?;

            session
                .store
                .push(layer_index, key, std::mem::take(&mut values[index]))?;
            attention_outputs.push(causal_attention(
                &model.config,
                &query,
                &mut session.store,
                layer_index,
            )?);
        }

        let attention_refs = attention_outputs
            .iter()
            .map(Vec::as_slice)
            .collect::<Vec<_>>();
        let projected = layer.o_proj.mul_vec_batch_with_options(
            &attention_refs,
            options.scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::OProj),
        )?;
        for (index, values) in projected.iter().enumerate() {
            add_assign(&mut hidden[index], values)?;
        }

        let ffn_normed = hidden
            .iter()
            .map(|values| rms_norm(values, &layer.ffn_norm, model.config.rms_norm_epsilon))
            .collect::<Result<Vec<_>, _>>()?;
        let ffn_refs = ffn_normed.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let gate_options = options.scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnGate);
        let up_options = options.scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnUp);
        let (gates, ups) = rayon::join(
            || {
                layer
                    .ffn_gate
                    .mul_vec_batch_with_options(&ffn_refs, gate_options)
            },
            || {
                layer
                    .ffn_up
                    .mul_vec_batch_with_options(&ffn_refs, up_options)
            },
        );
        let mut gates = gates?;
        let ups = ups?;
        for (gate, up) in gates.iter_mut().zip(&ups) {
            swiglu_in_place(gate, up)?;
        }
        let activated_refs = gates.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let downs = layer.ffn_down.mul_vec_batch_with_options(
            &activated_refs,
            options.scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnDown),
        )?;
        for (index, values) in downs.iter().enumerate() {
            add_assign(&mut hidden[index], values)?;
        }
    }

    let next_token_ids = if evaluate_output {
        let normed_final = hidden
            .iter()
            .map(|values| {
                rms_norm(
                    values,
                    &model.weights.output_norm,
                    model.config.rms_norm_epsilon,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let output = model
            .weights
            .output
            .logits_matrix(&model.weights.token_embedding);
        let normed_refs = normed_final.iter().map(Vec::as_slice).collect::<Vec<_>>();
        Some(output.argmax_mul_vec_batch_with_options(
            &normed_refs,
            options.scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::Output),
        )?)
    } else {
        None
    };

    for session in sessions.iter_mut() {
        session.cached_token_count += 1;
    }
    Ok(next_token_ids)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Logits,
    TokenIdOnly,
    ContextOnly,
}

#[derive(Debug, PartialEq)]
struct AcceptedToken {
    token_id: Option<usize>,
    logits: Option<Vec<f32>>,
}

fn add_optional_bias(values: &mut [f32], bias: Option<&[f32]>) -> Result<(), InferenceError> {
    let Some(bias) = bias else {
        return Ok(());
    };

    super::math::ensure_len("projection bias", bias, values.len())?;
    if values.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("projection values must be finite"));
    }
    if bias.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("projection bias must be finite"));
    }
    for (value, bias) in values.iter().zip(bias.iter()) {
        let result = *value + *bias;
        if !result.is_finite() {
            return Err(InferenceError::new("projection bias result must be finite"));
        }
    }
    for (value, bias) in values.iter_mut().zip(bias.iter()) {
        *value += *bias;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_optional_bias_rejects_non_finite_results() -> Result<(), InferenceError> {
        let mut values = [f32::MAX];
        let error = match add_optional_bias(&mut values, Some(&[f32::MAX])) {
            Ok(_) => {
                return Err(InferenceError::new(
                    "overflowing projection bias should fail",
                ))
            }
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("projection bias result must be finite"));
        assert_eq!(values, [f32::MAX]);
        Ok(())
    }
}
