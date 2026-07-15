use super::ScalarLlamaSession;
use crate::scalar::kv_store::KvCacheStore;
use crate::scalar::{InferenceError, ScalarExecutionOptions, ScalarLlamaModel};

impl<'a> ScalarLlamaSession<'a> {
    pub(in crate::scalar) fn new(model: &'a ScalarLlamaModel) -> Self {
        let head_kv_dim = model.config.attention_head_count_kv * model.config.head_dim;
        Self {
            model,
            store: KvCacheStore::new_vec(model.weights.layers.len(), head_kv_dim),
            cached_token_count: 0,
            options: ScalarExecutionOptions::default(),
        }
    }

    pub(in crate::scalar) fn from_store(
        model: &'a ScalarLlamaModel,
        store: KvCacheStore,
        options: ScalarExecutionOptions,
    ) -> Self {
        Self {
            model,
            store,
            cached_token_count: 0,
            options,
        }
    }

    pub(in crate::scalar) fn new_with_options(
        model: &'a ScalarLlamaModel,
        options: ScalarExecutionOptions,
    ) -> Result<Self, InferenceError> {
        let head_kv_dim = model.config.attention_head_count_kv * model.config.head_dim;
        let store = KvCacheStore::from_backend(
            model.weights.layers.len(),
            head_kv_dim,
            options.kv_backend(),
        )?;
        Ok(Self::from_store(model, store, options))
    }

    /// Returns the number of token positions currently stored in the KV cache.
    pub fn cached_token_count(&self) -> usize {
        self.cached_token_count
    }

    /// Returns the logical bytes occupied by cached key and value vectors.
    pub fn kv_cache_bytes(&self) -> u128 {
        self.store.kv_cache_bytes()
    }

    /// Removes cached positions after `token_count`.
    ///
    /// # Errors
    ///
    /// Returns an error when `token_count` exceeds the current cache length or
    /// the selected KV backend cannot complete the truncation.
    pub fn truncate_cache(&mut self, token_count: usize) -> Result<(), InferenceError> {
        if token_count > self.cached_token_count {
            return Err(InferenceError::new(format!(
                "cannot truncate kv cache from {} tokens to {token_count} tokens",
                self.cached_token_count
            )));
        }
        self.store.truncate(token_count)?;
        self.cached_token_count = token_count;
        Ok(())
    }

    #[cfg(all(feature = "locus-kv", unix))]
    /// Returns the Locus pool allocation count when this session uses Locus.
    pub fn locus_pool_allocation_count(&self) -> Option<u64> {
        match &self.store {
            crate::scalar::kv_store::KvCacheStore::Locus(store) => {
                Some(store.pool_stats().allocation_count)
            }
            crate::scalar::kv_store::KvCacheStore::Vec(_) => None,
        }
    }
}
