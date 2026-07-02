use super::{PrefixCacheEntry, PrefixCacheKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefixCacheMetadataStore {
    max_entries: usize,
    max_estimated_kv_bytes: u128,
    estimated_kv_bytes: u128,
    entries: Vec<PrefixCacheEntry>,
}

impl PrefixCacheMetadataStore {
    pub fn new(max_entries: usize, max_estimated_kv_bytes: u128) -> Self {
        Self {
            max_entries,
            max_estimated_kv_bytes,
            estimated_kv_bytes: 0,
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, entry: PrefixCacheEntry) -> Vec<PrefixCacheEntry> {
        if let Some(existing_index) = self
            .entries
            .iter()
            .position(|item| item.key() == entry.key())
        {
            let existing = self.entries.remove(existing_index);
            self.estimated_kv_bytes = self
                .estimated_kv_bytes
                .saturating_sub(existing.estimated_kv_bytes());
        }

        self.estimated_kv_bytes = self
            .estimated_kv_bytes
            .saturating_add(entry.estimated_kv_bytes());
        self.entries.push(entry);
        self.evict_until_within_budget()
    }

    pub fn record_hit(
        &mut self,
        key: &PrefixCacheKey,
        used_at_tick: u64,
    ) -> Option<&PrefixCacheEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.key() == key)?;
        entry.record_use(used_at_tick);
        Some(entry)
    }

    pub fn get(&self, key: &PrefixCacheKey) -> Option<&PrefixCacheEntry> {
        self.entries.iter().find(|entry| entry.key() == key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn estimated_kv_bytes(&self) -> u128 {
        self.estimated_kv_bytes
    }

    fn evict_until_within_budget(&mut self) -> Vec<PrefixCacheEntry> {
        let mut evicted = Vec::new();
        while self.entries.len() > self.max_entries
            || self.estimated_kv_bytes > self.max_estimated_kv_bytes
        {
            let Some(index) = self.least_recent_entry_index() else {
                break;
            };
            let entry = self.entries.remove(index);
            self.estimated_kv_bytes = self
                .estimated_kv_bytes
                .saturating_sub(entry.estimated_kv_bytes());
            evicted.push(entry);
        }
        evicted
    }

    fn least_recent_entry_index(&self) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .min_by_key(|(_, entry)| (entry.last_used_at_tick(), entry.created_at_tick()))
            .map(|(index, _)| index)
    }
}
