#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GenerationCacheOptions {
    namespace: Option<String>,
}

impl GenerationCacheOptions {
    pub fn from_namespace(namespace: Option<String>) -> Self {
        Self { namespace }
    }

    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
}
