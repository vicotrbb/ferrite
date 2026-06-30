use super::{
    attention::causal_attention,
    math::{add_assign, argmax, rms_norm, swiglu},
    profile::{ProfiledNextToken, ProfiledTokenId, ScalarMatVecComparison, ScalarProfileEvent},
    InferenceError, NextToken, Q8KActivationMatvecRole, ScalarExecutionOptions, ScalarLlamaModel,
};

mod cache;
mod profiling;

use profiling::{profiled_argmax_mul_vec, profiled_layer_mul_vec, profiled_mul_vec};

#[derive(Debug)]
pub struct ScalarLlamaSession<'a> {
    model: &'a ScalarLlamaModel,
    layer_keys: Vec<Vec<Vec<f32>>>,
    layer_values: Vec<Vec<Vec<f32>>>,
    cached_token_count: usize,
    options: ScalarExecutionOptions,
}

impl<'a> ScalarLlamaSession<'a> {
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
        let accepted = self.accept_token_inner(token_id, None, None, OutputMode::Logits)?;
        Ok(NextToken {
            token_id: accepted.token_id,
            logits: accepted
                .logits
                .ok_or_else(|| InferenceError::new("missing logits for next token"))?,
        })
    }

    pub fn accept_token_id(&mut self, token_id: usize) -> Result<usize, InferenceError> {
        Ok(self
            .accept_token_inner(token_id, None, None, OutputMode::TokenIdOnly)?
            .token_id)
    }

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
        )?;
        Ok(ProfiledTokenId {
            token_id: accepted.token_id,
            events,
            comparisons,
        })
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
        let mut comparisons = Vec::new();
        let accepted = self.accept_token_inner(
            token_id,
            Some(&mut events),
            Some(&mut comparisons),
            OutputMode::Logits,
        )?;
        let next_token = NextToken {
            token_id: accepted.token_id,
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
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::QProj),
            )?;
            add_optional_bias(&mut query, layer.q_bias.as_deref())?;
            let mut key = profiled_layer_mul_vec(
                &layer.k_proj,
                &normed,
                layer_index,
                "k_proj",
                profile_events.as_deref_mut(),
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::KProj),
            )?;
            add_optional_bias(&mut key, layer.k_bias.as_deref())?;
            let mut value = profiled_layer_mul_vec(
                &layer.v_proj,
                &normed,
                layer_index,
                "v_proj",
                profile_events.as_deref_mut(),
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::VProj),
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
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::OProj),
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
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnGate),
            )?;
            let up = profiled_layer_mul_vec(
                &layer.ffn_up,
                &ffn_normed,
                layer_index,
                "ffn_up",
                profile_events.as_deref_mut(),
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnUp),
            )?;
            let activated = swiglu(&gate, &up)?;
            let ffn_output = profiled_layer_mul_vec(
                &layer.ffn_down,
                &activated,
                layer_index,
                "ffn_down",
                profile_events.as_deref_mut(),
                comparison_events.as_deref_mut(),
                self.options
                    .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::FfnDown),
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
                let logits = profiled_mul_vec(
                    output,
                    &normed,
                    "output",
                    profile_events,
                    comparison_events,
                    self.options
                        .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::Output),
                )?;
                (argmax(&logits)?, Some(logits))
            }
            OutputMode::TokenIdOnly => {
                let token_id = profiled_argmax_mul_vec(
                    output,
                    &normed,
                    "output",
                    profile_events,
                    comparison_events,
                    self.options
                        .scoped_to_q8_k_activation_role(Q8KActivationMatvecRole::Output),
                )?;
                (token_id, None)
            }
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
