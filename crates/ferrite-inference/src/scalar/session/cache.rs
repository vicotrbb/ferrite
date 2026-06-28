use super::ScalarLlamaSession;
use crate::scalar::{InferenceError, ScalarLlamaModel};

impl<'a> ScalarLlamaSession<'a> {
    pub(in crate::scalar) fn new(model: &'a ScalarLlamaModel) -> Self {
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
        crate::scalar::memory::kv_cache_bytes(&self.layer_keys, &self.layer_values)
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
}
