use locus_alloc::{KvBlockHandle, KvBlockPool, KvBlockPoolError, KvReuseOrder, NodeId};

use crate::scalar::InferenceError;
use crate::scalar::session::ScalarLlamaSessionSnapshot;

const F32_BYTES: usize = std::mem::size_of::<f32>();

/// Locus-backed KV storage: fixed-size, mapped, page-aligned blocks holding
/// `tokens_per_block` positions each, one block list per (layer, K|V).
#[derive(Debug)]
pub(in crate::scalar) struct LocusKvStore {
    pool: KvBlockPool,
    head_kv_dim: usize,
    tokens_per_block: usize,
    key_blocks: Vec<Vec<KvBlockHandle>>,
    value_blocks: Vec<Vec<KvBlockHandle>>,
    layer_len: Vec<usize>,
}

fn map_pool_error(error: KvBlockPoolError) -> InferenceError {
    InferenceError::new(format!("locus kv pool error: {error}"))
}

impl LocusKvStore {
    pub(in crate::scalar) fn new(
        layer_count: usize,
        head_kv_dim: usize,
        tokens_per_block: usize,
        max_tokens: usize,
    ) -> Result<Self, InferenceError> {
        if head_kv_dim == 0 {
            return Err(InferenceError::new("locus kv head_kv_dim must be non-zero"));
        }
        if tokens_per_block == 0 {
            return Err(InferenceError::new(
                "locus kv tokens_per_block must be non-zero",
            ));
        }
        if max_tokens == 0 {
            return Err(InferenceError::new("locus kv max_tokens must be non-zero"));
        }
        let block_size = tokens_per_block
            .checked_mul(head_kv_dim)
            .and_then(|n| n.checked_mul(F32_BYTES))
            .ok_or_else(|| InferenceError::new("locus kv block size overflow"))?;
        let blocks_per_layer = max_tokens.div_ceil(tokens_per_block);
        // Two block lists (K and V) per layer.
        let capacity = blocks_per_layer
            .checked_mul(layer_count)
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| InferenceError::new("locus kv capacity overflow"))?
            .max(1);
        let pool = KvBlockPool::new_mapped(NodeId(0), block_size, capacity, KvReuseOrder::Lifo)
            .map_err(map_pool_error)?;
        Ok(Self {
            pool,
            head_kv_dim,
            tokens_per_block,
            key_blocks: vec![Vec::new(); layer_count],
            value_blocks: vec![Vec::new(); layer_count],
            layer_len: vec![0; layer_count],
        })
    }

    pub(in crate::scalar) fn pool_stats(&self) -> locus_alloc::KvBlockPoolStats {
        self.pool.stats()
    }

    pub(in crate::scalar) fn layer_count(&self) -> usize {
        self.layer_len.len()
    }

    pub(in crate::scalar) fn layer_len(&self, layer: usize) -> usize {
        self.layer_len.get(layer).copied().unwrap_or(0)
    }

    fn byte_range(&self, position: usize) -> (usize, usize) {
        let within = position % self.tokens_per_block;
        let start = within * self.head_kv_dim * F32_BYTES;
        (start, start + self.head_kv_dim * F32_BYTES)
    }

    fn write_block(
        &mut self,
        blocks_are_keys: bool,
        layer: usize,
        position: usize,
        values: &[f32],
    ) -> Result<(), InferenceError> {
        let (start, end) = self.byte_range(position);
        let block_index = position / self.tokens_per_block;
        let handle = {
            let blocks = if blocks_are_keys {
                &self.key_blocks
            } else {
                &self.value_blocks
            };
            let layer_blocks = blocks.get(layer).ok_or_else(|| {
                InferenceError::new(format!("locus kv layer {layer} out of bounds"))
            })?;
            *layer_blocks
                .get(block_index)
                .ok_or_else(|| InferenceError::new("locus kv block index out of bounds"))?
        };
        let bytes = self.pool.block_mut(handle).map_err(map_pool_error)?;
        let slot: &mut [f32] = bytemuck::try_cast_slice_mut(&mut bytes[start..end])
            .map_err(|error| InferenceError::new(format!("locus kv cast error: {error}")))?;
        slot.copy_from_slice(values);
        Ok(())
    }

    fn ensure_block(&mut self, layer: usize, position: usize) -> Result<(), InferenceError> {
        if !position.is_multiple_of(self.tokens_per_block) {
            return Ok(());
        }
        // Validate `layer` for BOTH block lists before allocating anything, so a
        // bad layer index never leaks a pool handle.
        if layer >= self.key_blocks.len() || layer >= self.value_blocks.len() {
            return Err(InferenceError::new(format!(
                "locus kv layer {layer} out of bounds"
            )));
        }
        let key_handle = self.pool.allocate().map_err(map_pool_error)?;
        let value_handle = match self.pool.allocate() {
            Ok(handle) => handle,
            Err(error) => {
                // Roll back the key handle so a failed second allocation never leaks
                // the first (mirrors locus_alloc::KvBlockTable::append_tokens).
                let _ = self.pool.free(key_handle);
                return Err(map_pool_error(error));
            }
        };
        // `layer` was validated above, so these cannot go out of bounds.
        self.key_blocks[layer].push(key_handle);
        self.value_blocks[layer].push(value_handle);
        Ok(())
    }

    pub(in crate::scalar) fn push(
        &mut self,
        layer: usize,
        key: Vec<f32>,
        value: Vec<f32>,
    ) -> Result<(), InferenceError> {
        self.push_slices(layer, &key, &value)
    }

    fn push_slices(
        &mut self,
        layer: usize,
        key: &[f32],
        value: &[f32],
    ) -> Result<(), InferenceError> {
        if key.len() != self.head_kv_dim || value.len() != self.head_kv_dim {
            return Err(InferenceError::new(format!(
                "locus kv push expects head_kv_dim {}, got key {} value {}",
                self.head_kv_dim,
                key.len(),
                value.len()
            )));
        }
        if key.iter().any(|value| !value.is_finite()) {
            return Err(InferenceError::new("cached key must be finite"));
        }
        if value.iter().any(|value| !value.is_finite()) {
            return Err(InferenceError::new("cached value must be finite"));
        }
        let position = self.layer_len(layer);
        self.ensure_block(layer, position)?;
        self.write_block(true, layer, position, key)?;
        self.write_block(false, layer, position, value)?;
        if let Some(len) = self.layer_len.get_mut(layer) {
            *len += 1;
        }
        Ok(())
    }

    fn read_block(
        &mut self,
        blocks_are_keys: bool,
        layer: usize,
        position: usize,
    ) -> Result<&[f32], InferenceError> {
        if position >= self.layer_len(layer) {
            return Err(InferenceError::new(format!(
                "locus kv position {position} out of bounds for layer {layer}"
            )));
        }
        let (start, end) = self.byte_range(position);
        let block_index = position / self.tokens_per_block;
        let handle = {
            let blocks = if blocks_are_keys {
                &self.key_blocks
            } else {
                &self.value_blocks
            };
            *blocks
                .get(layer)
                .and_then(|layer_blocks| layer_blocks.get(block_index))
                .ok_or_else(|| InferenceError::new("locus kv block index out of bounds"))?
        };
        let bytes = self.pool.block_mut(handle).map_err(map_pool_error)?;
        bytemuck::try_cast_slice(&bytes[start..end])
            .map_err(|error| InferenceError::new(format!("locus kv cast error: {error}")))
    }

    pub(in crate::scalar) fn key(
        &mut self,
        layer: usize,
        position: usize,
    ) -> Result<&[f32], InferenceError> {
        self.read_block(true, layer, position)
    }

    pub(in crate::scalar) fn value(
        &mut self,
        layer: usize,
        position: usize,
    ) -> Result<&[f32], InferenceError> {
        self.read_block(false, layer, position)
    }

    pub(in crate::scalar) fn truncate(&mut self, token_count: usize) -> Result<(), InferenceError> {
        let needed_blocks = token_count.div_ceil(self.tokens_per_block);
        for layer in 0..self.layer_count() {
            for blocks in [&mut self.key_blocks, &mut self.value_blocks] {
                if let Some(layer_blocks) = blocks.get_mut(layer) {
                    while layer_blocks.len() > needed_blocks {
                        if let Some(handle) = layer_blocks.pop() {
                            self.pool.free(handle).map_err(map_pool_error)?;
                        }
                    }
                }
            }
            if let Some(len) = self.layer_len.get_mut(layer) {
                *len = (*len).min(token_count);
            }
        }
        Ok(())
    }

    pub(in crate::scalar) fn kv_cache_bytes(&self) -> u128 {
        // Logical f32 bytes, identical semantics to the Vec backend.
        let per_position = (self.head_kv_dim * F32_BYTES) as u128;
        self.layer_len
            .iter()
            .map(|len| *len as u128 * per_position * 2)
            .sum()
    }

    pub(in crate::scalar) fn snapshot(
        &mut self,
        cached_token_count: usize,
    ) -> Result<ScalarLlamaSessionSnapshot, InferenceError> {
        let layer_count = self.layer_count();
        let mut layer_keys = Vec::with_capacity(layer_count);
        let mut layer_values = Vec::with_capacity(layer_count);
        for layer in 0..layer_count {
            let len = self.layer_len(layer);
            let mut keys = Vec::with_capacity(len);
            let mut values = Vec::with_capacity(len);
            for position in 0..len {
                keys.push(self.key(layer, position)?.to_vec());
                values.push(self.value(layer, position)?.to_vec());
            }
            layer_keys.push(keys);
            layer_values.push(values);
        }
        ScalarLlamaSessionSnapshot::from_layers(layer_keys, layer_values, cached_token_count)
    }

    pub(in crate::scalar) fn restore(
        &mut self,
        snapshot: &ScalarLlamaSessionSnapshot,
    ) -> Result<(), InferenceError> {
        if snapshot.layers_len() != self.layer_count() {
            return Err(InferenceError::new(format!(
                "cache snapshot layer count does not match model layer count {}",
                self.layer_count()
            )));
        }
        // Free everything, then copy borrowed snapshot rows directly into the
        // mapped pool. Cloning the full snapshot here would create one
        // prompt-sized heap allocation per restored duplicate session.
        self.truncate(0)?;
        for (layer, (layer_keys, layer_values)) in snapshot
            .layer_keys()
            .iter()
            .zip(snapshot.layer_values())
            .enumerate()
        {
            for (key, value) in layer_keys.iter().zip(layer_values) {
                self.push_slices(layer, key, value)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::LocusKvStore;
    use crate::scalar::InferenceError;

    fn sample(layer: usize, position: usize, dim: usize) -> Vec<f32> {
        (0..dim)
            .map(|d| (layer * 1000 + position * 10 + d) as f32 + 0.5)
            .collect()
    }

    #[test]
    fn locus_store_round_trips_across_block_boundaries() -> Result<(), InferenceError> {
        let dim = 4;
        // tokens_per_block = 2 forces multiple blocks for 5 positions.
        let mut store = LocusKvStore::new(2, dim, 2, 8)?;
        for position in 0..5 {
            for layer in 0..2 {
                store.push(
                    layer,
                    sample(layer, position, dim),
                    sample(layer + 100, position, dim),
                )?;
            }
        }
        for layer in 0..2 {
            assert_eq!(store.layer_len(layer), 5);
            for position in 0..5 {
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
    fn locus_store_truncates_and_frees_blocks() -> Result<(), InferenceError> {
        let dim = 2;
        let mut store = LocusKvStore::new(1, dim, 2, 8)?;
        for position in 0..4 {
            store.push(0, sample(0, position, dim), sample(0, position, dim))?;
        }
        let allocated_before = store.pool_stats().allocated;
        store.truncate(1)?;
        assert_eq!(store.layer_len(0), 1);
        assert!(store.pool_stats().allocated < allocated_before);
        assert!(store.key(0, 1).is_err());
        Ok(())
    }

    #[test]
    fn locus_store_reports_out_of_blocks() -> Result<(), InferenceError> {
        let dim = 2;
        // capacity sized for 2 tokens; pushing a 3rd must error.
        let mut store = LocusKvStore::new(1, dim, 1, 2)?;
        store.push(0, sample(0, 0, dim), sample(0, 0, dim))?;
        store.push(0, sample(0, 1, dim), sample(0, 1, dim))?;
        let error = match store.push(0, sample(0, 2, dim), sample(0, 2, dim)) {
            Ok(()) => return Err(InferenceError::new("expected out-of-blocks error")),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("out of blocks")
                || error.to_string().contains("OutOfBlocks")
        );
        Ok(())
    }

    #[test]
    fn locus_store_snapshot_round_trip() -> Result<(), InferenceError> {
        let dim = 4;
        // tokens_per_block = 2 forces multiple blocks across 5 positions, so
        // restore's re-push path is exercised across block boundaries.
        let mut store = LocusKvStore::new(2, dim, 2, 8)?;
        for position in 0..5 {
            for layer in 0..2 {
                store.push(
                    layer,
                    sample(layer, position, dim),
                    sample(layer + 100, position, dim),
                )?;
            }
        }
        let snapshot = store.snapshot(5)?;
        let mut restored = LocusKvStore::new(2, dim, 2, 8)?;
        restored.restore(&snapshot)?;
        for layer in 0..2 {
            assert_eq!(restored.layer_len(layer), store.layer_len(layer));
            for position in 0..5 {
                assert_eq!(restored.key(layer, position)?, store.key(layer, position)?);
                assert_eq!(
                    restored.value(layer, position)?,
                    store.value(layer, position)?
                );
            }
        }
        Ok(())
    }
}
