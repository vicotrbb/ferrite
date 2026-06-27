use super::{
    math::{add_assign, argmax, rms_norm, swiglu},
    InferenceError, NextToken, ScalarLlamaModel,
};

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
        if token_id >= self.model.config.vocab_size {
            return Err(InferenceError::new(format!(
                "token id {token_id} is out of bounds for vocab size {}",
                self.model.config.vocab_size
            )));
        }

        let position = self.cached_token_count;
        let mut hidden = self.model.weights.token_embedding.row(token_id)?.to_vec();

        for (layer_index, layer) in self.model.weights.layers.iter().enumerate() {
            let normed = rms_norm(
                &hidden,
                &layer.attn_norm,
                self.model.config.rms_norm_epsilon,
            )?;
            let mut query = layer.q_proj.mul_vec(&normed)?;
            let mut key = layer.k_proj.mul_vec(&normed)?;
            let value = layer.v_proj.mul_vec(&normed)?;

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

            let attention = self.model.causal_attention(
                &query,
                &self.layer_keys[layer_index],
                &self.layer_values[layer_index],
            )?;
            let attention_output = layer.o_proj.mul_vec(&attention)?;
            add_assign(&mut hidden, &attention_output)?;

            let ffn_normed =
                rms_norm(&hidden, &layer.ffn_norm, self.model.config.rms_norm_epsilon)?;
            let gate = layer.ffn_gate.mul_vec(&ffn_normed)?;
            let up = layer.ffn_up.mul_vec(&ffn_normed)?;
            let activated = swiglu(&gate, &up)?;
            let ffn_output = layer.ffn_down.mul_vec(&activated)?;
            add_assign(&mut hidden, &ffn_output)?;
        }

        let normed = rms_norm(
            &hidden,
            &self.model.weights.output_norm,
            self.model.config.rms_norm_epsilon,
        )?;
        let logits = self.model.weights.output.mul_vec(&normed)?;
        let token_id = argmax(&logits)?;
        self.cached_token_count += 1;

        Ok(NextToken { token_id, logits })
    }
}
