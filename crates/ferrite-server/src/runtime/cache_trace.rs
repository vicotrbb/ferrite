#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptCacheLookup {
    Disabled,
    Miss,
    ExactHit,
    PrefixHit,
    SharedPrefixHit,
}

impl PromptCacheLookup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Miss => "miss",
            Self::ExactHit => "exact_hit",
            Self::PrefixHit => "prefix_hit",
            Self::SharedPrefixHit => "shared_prefix_hit",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptCacheTrace {
    enabled: bool,
    namespace: Option<String>,
    prompt_token_count: usize,
    prompt_token_hash: u64,
    lookup: PromptCacheLookup,
    selected_entry_token_count: Option<usize>,
    selected_entry_token_hash: Option<u64>,
    shared_prefix_tokens: usize,
}

impl PromptCacheTrace {
    pub fn new(
        enabled: bool,
        namespace: Option<String>,
        prompt_token_count: usize,
        prompt_token_hash: u64,
        lookup: PromptCacheLookup,
    ) -> Self {
        Self {
            enabled,
            namespace,
            prompt_token_count,
            prompt_token_hash,
            lookup,
            selected_entry_token_count: None,
            selected_entry_token_hash: None,
            shared_prefix_tokens: 0,
        }
    }

    pub fn with_selected_entry(mut self, token_count: usize, token_hash: u64) -> Self {
        self.selected_entry_token_count = Some(token_count);
        self.selected_entry_token_hash = Some(token_hash);
        self
    }

    pub fn with_shared_prefix_tokens(mut self, token_count: usize) -> Self {
        self.shared_prefix_tokens = token_count;
        self
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    pub fn prompt_token_count(&self) -> usize {
        self.prompt_token_count
    }

    pub fn prompt_token_hash(&self) -> u64 {
        self.prompt_token_hash
    }

    pub fn lookup(&self) -> PromptCacheLookup {
        self.lookup
    }

    pub fn selected_entry_token_count(&self) -> Option<usize> {
        self.selected_entry_token_count
    }

    pub fn selected_entry_token_hash(&self) -> Option<u64> {
        self.selected_entry_token_hash
    }

    pub fn shared_prefix_tokens(&self) -> usize {
        self.shared_prefix_tokens
    }
}
