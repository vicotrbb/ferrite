use super::{PrefixCacheEntry, PrefixCacheKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A cache hit that reports the common token count between two compatible keys.
pub struct PrefixCacheSharedPrefixHit<'a> {
    entry: &'a PrefixCacheEntry,
    shared_prefix_token_count: usize,
}

impl<'a> PrefixCacheSharedPrefixHit<'a> {
    /// Returns the cache entry that produced the hit.
    pub fn entry(&self) -> &'a PrefixCacheEntry {
        self.entry
    }

    /// Returns the number of leading tokens shared with the request key.
    pub fn shared_prefix_token_count(&self) -> usize {
        self.shared_prefix_token_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A bounded in-memory store for prefix-cache metadata.
///
/// The store evicts least-recently-used entries until both the entry count and
/// estimated KV byte budgets are satisfied. It does not own KV tensors.
pub struct PrefixCacheMetadataStore {
    max_entries: usize,
    max_estimated_kv_bytes: u128,
    estimated_kv_bytes: u128,
    entries: Vec<PrefixCacheEntry>,
}

impl PrefixCacheMetadataStore {
    /// Creates an empty store with entry-count and estimated-byte limits.
    pub fn new(max_entries: usize, max_estimated_kv_bytes: u128) -> Self {
        Self {
            max_entries,
            max_estimated_kv_bytes,
            estimated_kv_bytes: 0,
            entries: Vec::new(),
        }
    }

    /// Inserts or replaces an entry and returns entries evicted by the budgets.
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

    /// Records an exact-key hit and returns the updated entry.
    pub fn record_hit(
        &mut self,
        key: &PrefixCacheKey,
        used_at_tick: u64,
    ) -> Option<&PrefixCacheEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.key() == key)?;
        entry.record_use(used_at_tick);
        Some(entry)
    }

    /// Records the longest compatible cached prefix of the request key.
    pub fn record_longest_prefix_hit(
        &mut self,
        key: &PrefixCacheKey,
        used_at_tick: u64,
    ) -> Option<&PrefixCacheEntry> {
        let index = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| is_compatible_prefix(entry.key(), key))
            .max_by_key(|(_, entry)| entry.key().prefix_token_count())
            .map(|(index, _)| index)?;
        self.entries[index].record_use(used_at_tick);
        Some(&self.entries[index])
    }

    /// Records the compatible entry with the longest nonempty shared prefix.
    pub fn record_longest_shared_prefix_hit(
        &mut self,
        key: &PrefixCacheKey,
        used_at_tick: u64,
    ) -> Option<PrefixCacheSharedPrefixHit<'_>> {
        let (index, shared_prefix_token_count) = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let shared_prefix_token_count = shared_prefix_token_count(entry.key(), key);
                (shared_prefix_token_count > 0).then_some((index, shared_prefix_token_count))
            })
            .max_by_key(|(_, shared_prefix_token_count)| *shared_prefix_token_count)?;
        self.entries[index].record_use(used_at_tick);
        Some(PrefixCacheSharedPrefixHit {
            entry: &self.entries[index],
            shared_prefix_token_count,
        })
    }

    /// Returns an entry with an exactly matching key.
    pub fn get(&self, key: &PrefixCacheKey) -> Option<&PrefixCacheEntry> {
        self.entries.iter().find(|entry| entry.key() == key)
    }

    /// Returns the number of entries in the store.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when the store contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the estimated KV bytes charged by all entries.
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

fn is_compatible_prefix(cached: &PrefixCacheKey, requested: &PrefixCacheKey) -> bool {
    cached.fingerprints() == requested.fingerprints()
        && cached.namespace() == requested.namespace()
        && requested
            .prefix_tokens()
            .starts_with(cached.prefix_tokens())
}

fn shared_prefix_token_count(cached: &PrefixCacheKey, requested: &PrefixCacheKey) -> usize {
    if cached.fingerprints() != requested.fingerprints()
        || cached.namespace() != requested.namespace()
    {
        return 0;
    }
    cached
        .prefix_tokens()
        .iter()
        .zip(requested.prefix_tokens())
        .take_while(|(cached, requested)| cached == requested)
        .count()
}
