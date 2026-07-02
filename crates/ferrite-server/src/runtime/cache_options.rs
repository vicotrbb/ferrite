#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GenerationCacheOptions {
    namespace: Option<String>,
    prefix_cache_enabled: bool,
}

impl GenerationCacheOptions {
    pub fn from_namespace(namespace: Option<String>) -> Self {
        Self {
            namespace,
            prefix_cache_enabled: false,
        }
    }

    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    pub fn with_prefix_cache_enabled(mut self, enabled: bool) -> Self {
        self.prefix_cache_enabled = enabled;
        self
    }

    pub fn prefix_cache_enabled(&self) -> bool {
        self.prefix_cache_enabled
    }
}
