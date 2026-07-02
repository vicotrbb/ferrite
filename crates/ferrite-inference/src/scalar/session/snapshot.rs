use super::ScalarLlamaSession;
use crate::scalar::{memory, InferenceError};

#[derive(Clone, Debug, PartialEq)]
pub struct ScalarLlamaSessionSnapshot {
    layer_keys: Vec<Vec<Vec<f32>>>,
    layer_values: Vec<Vec<Vec<f32>>>,
    cached_token_count: usize,
}

impl ScalarLlamaSessionSnapshot {
    pub fn cached_token_count(&self) -> usize {
        self.cached_token_count
    }

    pub fn truncate_to_cached_token_count(
        &self,
        token_count: usize,
    ) -> Result<Self, InferenceError> {
        if token_count > self.cached_token_count {
            return Err(InferenceError::new(format!(
                "cannot truncate cache snapshot with {} cached tokens to {token_count} tokens",
                self.cached_token_count
            )));
        }
        if token_count == self.cached_token_count {
            return Ok(self.clone());
        }
        Ok(Self {
            layer_keys: truncate_layers(&self.layer_keys, token_count),
            layer_values: truncate_layers(&self.layer_values, token_count),
            cached_token_count: token_count,
        })
    }

    pub fn kv_cache_bytes(&self) -> u128 {
        memory::kv_cache_bytes(&self.layer_keys, &self.layer_values)
    }
}

fn truncate_layers(layers: &[Vec<Vec<f32>>], token_count: usize) -> Vec<Vec<Vec<f32>>> {
    layers
        .iter()
        .map(|layer| layer.iter().take(token_count).cloned().collect())
        .collect()
}

impl<'a> ScalarLlamaSession<'a> {
    pub fn cache_snapshot(&self) -> ScalarLlamaSessionSnapshot {
        ScalarLlamaSessionSnapshot {
            layer_keys: self.layer_keys.clone(),
            layer_values: self.layer_values.clone(),
            cached_token_count: self.cached_token_count,
        }
    }

    pub fn restore_cache_snapshot(
        &mut self,
        snapshot: &ScalarLlamaSessionSnapshot,
    ) -> Result<(), InferenceError> {
        let expected_layers = self.model.weights.layers.len();
        if snapshot.layer_keys.len() != expected_layers
            || snapshot.layer_values.len() != expected_layers
        {
            return Err(InferenceError::new(format!(
                "cache snapshot layer count does not match model layer count {expected_layers}"
            )));
        }

        for (layer_index, (keys, values)) in snapshot
            .layer_keys
            .iter()
            .zip(snapshot.layer_values.iter())
            .enumerate()
        {
            if keys.len() != snapshot.cached_token_count
                || values.len() != snapshot.cached_token_count
            {
                return Err(InferenceError::new(format!(
                    "cache snapshot layer {layer_index} token count does not match cached token count {}",
                    snapshot.cached_token_count
                )));
            }
        }

        self.layer_keys = snapshot.layer_keys.clone();
        self.layer_values = snapshot.layer_values.clone();
        self.cached_token_count = snapshot.cached_token_count;
        Ok(())
    }
}
