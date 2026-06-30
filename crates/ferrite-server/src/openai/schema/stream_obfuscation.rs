use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static STREAM_OBFUSCATION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn stream_obfuscation() -> String {
    let counter = STREAM_OBFUSCATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{nanos:032x}{counter:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_obfuscation_is_non_empty() {
        assert!(!stream_obfuscation().is_empty());
    }
}
