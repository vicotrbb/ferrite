#[cfg(feature = "locus-kv")]
pub(in crate::scalar) mod locus;

use super::session::ScalarLlamaSessionSnapshot;
use super::InferenceError;

/// KV-cache storage behind a stable interface. `Vec` is the default backend and
/// reproduces the historical nested-`Vec` behavior exactly.
#[derive(Debug)]
pub(in crate::scalar) enum KvCacheStore {
    Vec(VecKvStore),
    #[cfg(feature = "locus-kv")]
    Locus(locus::LocusKvStore),
}

impl KvCacheStore {
    pub(in crate::scalar) fn new_vec(layer_count: usize, head_kv_dim: usize) -> Self {
        KvCacheStore::Vec(VecKvStore::new(layer_count, head_kv_dim))
    }

    pub(in crate::scalar) fn from_backend(
        layer_count: usize,
        head_kv_dim: usize,
        backend: super::options::KvBackend,
    ) -> Result<Self, InferenceError> {
        match backend {
            super::options::KvBackend::Vec => Ok(Self::new_vec(layer_count, head_kv_dim)),
            #[cfg(feature = "locus-kv")]
            super::options::KvBackend::Locus {
                tokens_per_block,
                max_tokens,
            } => Ok(KvCacheStore::Locus(locus::LocusKvStore::new(
                layer_count,
                head_kv_dim,
                tokens_per_block,
                max_tokens,
            )?)),
            #[cfg(not(feature = "locus-kv"))]
            super::options::KvBackend::Locus { .. } => Err(InferenceError::new(
                "locus kv backend requested but the `locus-kv` feature is not enabled",
            )),
        }
    }

    pub(in crate::scalar) fn layer_count(&self) -> usize {
        match self {
            KvCacheStore::Vec(store) => store.layer_count(),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.layer_count(),
        }
    }

    pub(in crate::scalar) fn layer_len(&self, layer: usize) -> usize {
        match self {
            KvCacheStore::Vec(store) => store.layer_len(layer),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.layer_len(layer),
        }
    }

    pub(in crate::scalar) fn push(
        &mut self,
        layer: usize,
        key: &[f32],
        value: &[f32],
    ) -> Result<(), InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.push(layer, key, value),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.push(layer, key, value),
        }
    }

    pub(in crate::scalar) fn key(
        &mut self,
        layer: usize,
        position: usize,
    ) -> Result<&[f32], InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.key(layer, position),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.key(layer, position),
        }
    }

    pub(in crate::scalar) fn value(
        &mut self,
        layer: usize,
        position: usize,
    ) -> Result<&[f32], InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.value(layer, position),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.value(layer, position),
        }
    }

    pub(in crate::scalar) fn truncate(&mut self, token_count: usize) -> Result<(), InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.truncate(token_count),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.truncate(token_count),
        }
    }

    pub(in crate::scalar) fn kv_cache_bytes(&self) -> u128 {
        match self {
            KvCacheStore::Vec(store) => store.kv_cache_bytes(),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.kv_cache_bytes(),
        }
    }

    pub(in crate::scalar) fn snapshot(
        &mut self,
        cached_token_count: usize,
    ) -> Result<ScalarLlamaSessionSnapshot, InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.snapshot(cached_token_count),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.snapshot(cached_token_count),
        }
    }

    pub(in crate::scalar) fn restore(
        &mut self,
        snapshot: &ScalarLlamaSessionSnapshot,
    ) -> Result<(), InferenceError> {
        match self {
            KvCacheStore::Vec(store) => store.restore(snapshot),
            #[cfg(feature = "locus-kv")]
            KvCacheStore::Locus(store) => store.restore(snapshot),
        }
    }
}

/// Nested-`Vec` KV storage: one inner `Vec<f32>` per (layer, position).
#[derive(Debug)]
pub(in crate::scalar) struct VecKvStore {
    head_kv_dim: usize,
    layer_keys: Vec<Vec<Vec<f32>>>,
    layer_values: Vec<Vec<Vec<f32>>>,
}

impl VecKvStore {
    fn new(layer_count: usize, head_kv_dim: usize) -> Self {
        Self {
            head_kv_dim,
            layer_keys: vec![Vec::new(); layer_count],
            layer_values: vec![Vec::new(); layer_count],
        }
    }

    fn layer_count(&self) -> usize {
        self.layer_keys.len()
    }

    fn layer_len(&self, layer: usize) -> usize {
        self.layer_keys.get(layer).map_or(0, Vec::len)
    }

    fn check_dim(&self, label: &str, values: &[f32]) -> Result<(), InferenceError> {
        if values.len() != self.head_kv_dim {
            return Err(InferenceError::new(format!(
                "{label} length {} does not match head_kv_dim {}",
                values.len(),
                self.head_kv_dim
            )));
        }
        Ok(())
    }

    fn push(&mut self, layer: usize, key: &[f32], value: &[f32]) -> Result<(), InferenceError> {
        self.check_dim("key", key)?;
        self.check_dim("value", value)?;
        let keys = self
            .layer_keys
            .get_mut(layer)
            .ok_or_else(|| InferenceError::new(format!("kv layer {layer} out of bounds")))?;
        keys.push(key.to_vec());
        let values = self
            .layer_values
            .get_mut(layer)
            .ok_or_else(|| InferenceError::new(format!("kv layer {layer} out of bounds")))?;
        values.push(value.to_vec());
        Ok(())
    }

    fn key(&mut self, layer: usize, position: usize) -> Result<&[f32], InferenceError> {
        self.layer_keys
            .get(layer)
            .and_then(|layer| layer.get(position))
            .map(Vec::as_slice)
            .ok_or_else(|| {
                InferenceError::new(format!("kv key ({layer},{position}) out of bounds"))
            })
    }

    fn value(&mut self, layer: usize, position: usize) -> Result<&[f32], InferenceError> {
        self.layer_values
            .get(layer)
            .and_then(|layer| layer.get(position))
            .map(Vec::as_slice)
            .ok_or_else(|| {
                InferenceError::new(format!("kv value ({layer},{position}) out of bounds"))
            })
    }

    fn truncate(&mut self, token_count: usize) -> Result<(), InferenceError> {
        for keys in &mut self.layer_keys {
            keys.truncate(token_count);
        }
        for values in &mut self.layer_values {
            values.truncate(token_count);
        }
        Ok(())
    }

    fn kv_cache_bytes(&self) -> u128 {
        super::memory::kv_cache_bytes(&self.layer_keys, &self.layer_values)
    }

    fn snapshot(
        &mut self,
        cached_token_count: usize,
    ) -> Result<ScalarLlamaSessionSnapshot, InferenceError> {
        ScalarLlamaSessionSnapshot::from_layers(
            self.layer_keys.clone(),
            self.layer_values.clone(),
            cached_token_count,
        )
    }

    fn restore(&mut self, snapshot: &ScalarLlamaSessionSnapshot) -> Result<(), InferenceError> {
        if snapshot.layers_len() != self.layer_keys.len() {
            return Err(InferenceError::new(format!(
                "cache snapshot layer count does not match model layer count {}",
                self.layer_keys.len()
            )));
        }
        self.layer_keys = snapshot.layer_keys_owned();
        self.layer_values = snapshot.layer_values_owned();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::KvCacheStore;
    use crate::scalar::InferenceError;

    fn sample(layer: usize, position: usize, dim: usize) -> Vec<f32> {
        (0..dim)
            .map(|d| (layer * 1000 + position * 10 + d) as f32)
            .collect()
    }

    #[test]
    fn vec_store_round_trips_positions() -> Result<(), InferenceError> {
        let dim = 4;
        let mut store = KvCacheStore::new_vec(2, dim);
        for position in 0..3 {
            for layer in 0..2 {
                store.push(
                    layer,
                    &sample(layer, position, dim),
                    &sample(layer + 100, position, dim),
                )?;
            }
        }
        for layer in 0..2 {
            assert_eq!(store.layer_len(layer), 3);
            for position in 0..3 {
                assert_eq!(
                    store.key(layer, position)?,
                    sample(layer, position, dim).as_slice()
                );
                assert_eq!(
                    store.value(layer, position)?,
                    sample(layer + 100, position, dim).as_slice()
                );
            }
        }
        Ok(())
    }

    #[test]
    fn vec_store_truncates() -> Result<(), InferenceError> {
        let dim = 2;
        let mut store = KvCacheStore::new_vec(1, dim);
        for position in 0..4 {
            store.push(0, &sample(0, position, dim), &sample(0, position, dim))?;
        }
        store.truncate(2)?;
        assert_eq!(store.layer_len(0), 2);
        assert!(store.key(0, 2).is_err());
        Ok(())
    }

    #[cfg(feature = "locus-kv")]
    #[test]
    fn build_from_backend_selects_locus() -> Result<(), crate::scalar::InferenceError> {
        use crate::scalar::options::KvBackend;
        let store = KvCacheStore::from_backend(
            2,
            4,
            KvBackend::Locus {
                tokens_per_block: 16,
                max_tokens: 64,
            },
        )?;
        assert!(matches!(store, KvCacheStore::Locus(_)));
        Ok(())
    }

    #[test]
    fn build_from_backend_defaults_to_vec() -> Result<(), crate::scalar::InferenceError> {
        use crate::scalar::options::KvBackend;
        let store = KvCacheStore::from_backend(2, 4, KvBackend::Vec)?;
        assert!(matches!(store, KvCacheStore::Vec(_)));
        Ok(())
    }

    #[test]
    fn vec_store_snapshot_round_trip() -> Result<(), InferenceError> {
        let dim = 3;
        let mut store = KvCacheStore::new_vec(2, dim);
        for position in 0..2 {
            for layer in 0..2 {
                store.push(
                    layer,
                    &sample(layer, position, dim),
                    &sample(layer, position, dim),
                )?;
            }
        }
        let snapshot = store.snapshot(2)?;
        let mut restored = KvCacheStore::new_vec(2, dim);
        restored.restore(&snapshot)?;
        for layer in 0..2 {
            assert_eq!(restored.layer_len(layer), 2);
            for position in 0..2 {
                assert_eq!(restored.key(layer, position)?, store.key(layer, position)?);
            }
        }
        Ok(())
    }
}
