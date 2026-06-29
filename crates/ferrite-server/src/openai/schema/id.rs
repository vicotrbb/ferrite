use std::sync::atomic::{AtomicU64, Ordering};

static RESPONSE_ID_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn response_id(prefix: &str, created: u64) -> String {
    let sequence = RESPONSE_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-ferrite-{created}-{sequence}")
}
