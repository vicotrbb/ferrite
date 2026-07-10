use super::ScalarLlamaSession;
use crate::scalar::{memory, InferenceError};

#[derive(Clone, Debug, PartialEq)]
/// An owned, backend-independent snapshot of a session KV cache.
pub struct ScalarLlamaSessionSnapshot {
    layer_keys: Vec<Vec<Vec<f32>>>,
    layer_values: Vec<Vec<Vec<f32>>>,
    cached_token_count: usize,
}

impl ScalarLlamaSessionSnapshot {
    /// Returns the number of cached token positions in the snapshot.
    pub fn cached_token_count(&self) -> usize {
        self.cached_token_count
    }

    /// Returns a copy truncated to the requested cached-token count.
    ///
    /// # Errors
    ///
    /// Returns an error when `token_count` exceeds the snapshot length.
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

    /// Returns the logical bytes occupied by snapshot key and value vectors.
    pub fn kv_cache_bytes(&self) -> u128 {
        memory::kv_cache_bytes(&self.layer_keys, &self.layer_values)
    }

    pub(in crate::scalar) fn from_layers(
        layer_keys: Vec<Vec<Vec<f32>>>,
        layer_values: Vec<Vec<Vec<f32>>>,
        cached_token_count: usize,
    ) -> Result<Self, InferenceError> {
        if layer_keys.len() != layer_values.len() {
            return Err(InferenceError::new(
                "snapshot key and value layer counts differ",
            ));
        }
        Ok(Self {
            layer_keys,
            layer_values,
            cached_token_count,
        })
    }

    pub(in crate::scalar) fn layers_len(&self) -> usize {
        self.layer_keys.len()
    }

    pub(in crate::scalar) fn layer_keys_owned(&self) -> Vec<Vec<Vec<f32>>> {
        self.layer_keys.clone()
    }

    pub(in crate::scalar) fn layer_values_owned(&self) -> Vec<Vec<Vec<f32>>> {
        self.layer_values.clone()
    }
}

fn truncate_layers(layers: &[Vec<Vec<f32>>], token_count: usize) -> Vec<Vec<Vec<f32>>> {
    layers
        .iter()
        .map(|layer| layer.iter().take(token_count).cloned().collect())
        .collect()
}

impl<'a> ScalarLlamaSession<'a> {
    /// Copies this session's KV state into a backend-independent snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected KV backend cannot produce a
    /// structurally valid snapshot.
    pub fn cache_snapshot(&mut self) -> Result<ScalarLlamaSessionSnapshot, InferenceError> {
        self.store.snapshot(self.cached_token_count)
    }

    /// Replaces this session's KV state from a compatible snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when layer or token counts do not match the model, or
    /// the selected KV backend cannot restore the snapshot.
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

        self.store.restore(snapshot)?;
        self.cached_token_count = snapshot.cached_token_count;
        Ok(())
    }
}
