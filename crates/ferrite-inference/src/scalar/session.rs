use super::{
    attention::causal_attention,
    math::{add_assign, argmax, rms_norm, swiglu},
    profile::{ProfiledNextToken, ScalarProfileEvent},
    InferenceError, Matrix, NextToken, ScalarLlamaModel,
};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct ScalarLlamaSession<'a> {
    model: &'a ScalarLlamaModel,
    layer_keys: Vec<Vec<Vec<f32>>>,
    layer_values: Vec<Vec<Vec<f32>>>,
    cached_token_count: usize,
}

impl<'a> ScalarLlamaSession<'a> {
    pub(super) fn new(model: &'a ScalarLlamaModel) -> Self {
        Self {
            model,
            layer_keys: vec![Vec::<Vec<f32>>::new(); model.weights.layers.len()],
            layer_values: vec![Vec::<Vec<f32>>::new(); model.weights.layers.len()],
            cached_token_count: 0,
        }
    }

    pub fn cached_token_count(&self) -> usize {
        self.cached_token_count
    }

    pub fn kv_cache_bytes(&self) -> u128 {
        super::memory::kv_cache_bytes(&self.layer_keys, &self.layer_values)
    }

    pub fn truncate_cache(&mut self, token_count: usize) -> Result<(), InferenceError> {
        if token_count > self.cached_token_count {
            return Err(InferenceError::new(format!(
                "cannot truncate kv cache from {} tokens to {token_count} tokens",
                self.cached_token_count
            )));
        }

        for keys in &mut self.layer_keys {
            keys.truncate(token_count);
        }
        for values in &mut self.layer_values {
            values.truncate(token_count);
        }
        self.cached_token_count = token_count;
        Ok(())
    }

    pub fn accept_prompt(&mut self, tokens: &[usize]) -> Result<NextToken, InferenceError> {
        if tokens.is_empty() {
            return Err(InferenceError::new(
                "prompt must contain at least one token",
            ));
        }

        let mut next = None;
        for token_id in tokens {
            next = Some(self.accept_token(*token_id)?);
        }

        next.ok_or_else(|| InferenceError::new("prompt must contain at least one token"))
    }

    pub fn accept_token(&mut self, token_id: usize) -> Result<NextToken, InferenceError> {
        let accepted = self.accept_token_inner(token_id, None, OutputMode::Logits)?;
        Ok(NextToken {
            token_id: accepted.token_id,
            logits: accepted
                .logits
                .ok_or_else(|| InferenceError::new("missing logits for next token"))?,
        })
    }

    pub fn accept_token_id(&mut self, token_id: usize) -> Result<usize, InferenceError> {
        Ok(self
            .accept_token_inner(token_id, None, OutputMode::TokenIdOnly)?
            .token_id)
    }

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

    pub fn accept_token_profiled(
        &mut self,
        token_id: usize,
    ) -> Result<ProfiledNextToken, InferenceError> {
        let mut events = Vec::new();
        let accepted = self.accept_token_inner(token_id, Some(&mut events), OutputMode::Logits)?;
        let next_token = NextToken {
            token_id: accepted.token_id,
            logits: accepted
                .logits
                .ok_or_else(|| InferenceError::new("missing logits for profiled next token"))?,
        };
        Ok(ProfiledNextToken { next_token, events })
    }

    fn accept_token_inner(
        &mut self,
        token_id: usize,
        mut profile_events: Option<&mut Vec<ScalarProfileEvent>>,
        output_mode: OutputMode,
    ) -> Result<AcceptedToken, InferenceError> {
        if token_id >= self.model.config.vocab_size {
            return Err(InferenceError::new(format!(
                "token id {token_id} is out of bounds for vocab size {}",
                self.model.config.vocab_size
            )));
        }

        let position = self.cached_token_count;
        let mut hidden = self.model.weights.token_embedding.row_values(token_id)?;

        for (layer_index, layer) in self.model.weights.layers.iter().enumerate() {
            let normed = rms_norm(
                &hidden,
                &layer.attn_norm,
                self.model.config.rms_norm_epsilon,
            )?;
            let mut query = profiled_layer_mul_vec(
                &layer.q_proj,
                &normed,
                layer_index,
                "q_proj",
                profile_events.as_deref_mut(),
            )?;
            add_optional_bias(&mut query, layer.q_bias.as_deref())?;
            let mut key = profiled_layer_mul_vec(
                &layer.k_proj,
                &normed,
                layer_index,
                "k_proj",
                profile_events.as_deref_mut(),
            )?;
            add_optional_bias(&mut key, layer.k_bias.as_deref())?;
            let mut value = profiled_layer_mul_vec(
                &layer.v_proj,
                &normed,
                layer_index,
                "v_proj",
                profile_events.as_deref_mut(),
            )?;
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

            self.layer_keys[layer_index].push(key);
            self.layer_values[layer_index].push(value);

            let attention = causal_attention(
                &self.model.config,
                &query,
                &self.layer_keys[layer_index],
                &self.layer_values[layer_index],
            )?;
            let attention_output = profiled_layer_mul_vec(
                &layer.o_proj,
                &attention,
                layer_index,
                "o_proj",
                profile_events.as_deref_mut(),
            )?;
            add_assign(&mut hidden, &attention_output)?;

            let ffn_normed =
                rms_norm(&hidden, &layer.ffn_norm, self.model.config.rms_norm_epsilon)?;
            let gate = profiled_layer_mul_vec(
                &layer.ffn_gate,
                &ffn_normed,
                layer_index,
                "ffn_gate",
                profile_events.as_deref_mut(),
            )?;
            let up = profiled_layer_mul_vec(
                &layer.ffn_up,
                &ffn_normed,
                layer_index,
                "ffn_up",
                profile_events.as_deref_mut(),
            )?;
            let activated = swiglu(&gate, &up)?;
            let ffn_output = profiled_layer_mul_vec(
                &layer.ffn_down,
                &activated,
                layer_index,
                "ffn_down",
                profile_events.as_deref_mut(),
            )?;
            add_assign(&mut hidden, &ffn_output)?;
        }

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
        let (token_id, logits) = match output_mode {
            OutputMode::Logits => {
                let logits = profiled_mul_vec(output, &normed, "output", profile_events)?;
                (argmax(&logits)?, Some(logits))
            }
            OutputMode::TokenIdOnly => (output.argmax_mul_vec(&normed)?, None),
        };
        self.cached_token_count += 1;

        Ok(AcceptedToken { token_id, logits })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Logits,
    TokenIdOnly,
}

#[derive(Debug, PartialEq)]
struct AcceptedToken {
    token_id: usize,
    logits: Option<Vec<f32>>,
}

fn profiled_layer_mul_vec(
    matrix: &Matrix,
    vector: &[f32],
    layer_index: usize,
    role: &str,
    profile_events: Option<&mut Vec<ScalarProfileEvent>>,
) -> Result<Vec<f32>, InferenceError> {
    if profile_events.is_none() {
        return matrix.mul_vec(vector);
    }
    profiled_mul_vec(
        matrix,
        vector,
        &format!("layer.{layer_index}.{role}"),
        profile_events,
    )
}

fn add_optional_bias(values: &mut [f32], bias: Option<&[f32]>) -> Result<(), InferenceError> {
    let Some(bias) = bias else {
        return Ok(());
    };

    super::math::ensure_len("projection bias", bias, values.len())?;
    for (value, bias) in values.iter_mut().zip(bias.iter()) {
        *value += bias;
    }
    Ok(())
}

fn profiled_mul_vec(
    matrix: &Matrix,
    vector: &[f32],
    label: &str,
    profile_events: Option<&mut Vec<ScalarProfileEvent>>,
) -> Result<Vec<f32>, InferenceError> {
    let Some(events) = profile_events else {
        return matrix.mul_vec(vector);
    };
    let started = Instant::now();
    let output = matrix.mul_vec(vector)?;
    let elapsed = started.elapsed();
    events.push(ScalarProfileEvent::new(
        label,
        nonzero_duration(elapsed),
        matrix,
    ));
    Ok(output)
}

fn nonzero_duration(elapsed: Duration) -> Duration {
    if elapsed.is_zero() {
        Duration::from_nanos(1)
    } else {
        elapsed
    }
}
