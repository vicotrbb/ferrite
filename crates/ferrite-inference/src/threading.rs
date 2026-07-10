//! Global inference thread-pool sizing.
//!
//! Decode-time matvecs fan out through rayon's global pool. On
//! heterogeneous CPUs, spreading fork-join work across every logical
//! core lets the slowest core class set each matvec's tail latency;
//! measured on an Apple M5 Pro (5+10 perflevel split), 15 worker
//! threads decode ~25% slower than 10. The default below sizes the
//! pool to the largest homogeneous performance level instead of the
//! total logical count.
//!
//! Resolution order: explicit caller value (CLI flag), then
//! `FERRITE_THREADS`, then `RAYON_NUM_THREADS` (kept working even
//! though the pool is built eagerly here), then the platform probe.

use std::env;
use std::num::NonZeroUsize;

/// Builds the global rayon pool with the resolved thread count and
/// returns the count. Safe to call more than once: if the pool is
/// already built (e.g. by an earlier caller or a test harness), the
/// existing pool is kept and its size is returned.
pub fn init_global_pool(explicit_threads: Option<usize>) -> usize {
    let threads = resolve_thread_count(explicit_threads);
    build_global_pool(threads)
}

/// Builds a pool sized for bandwidth-bound integer matvecs. Explicit and
/// environment overrides retain precedence; the automatic choice leaves a
/// roughly one third of the widest CPU level idle once additional workers
/// stop increasing memory throughput.
pub fn init_memory_bound_global_pool(explicit_threads: Option<usize>) -> usize {
    let threads = resolve_override_thread_count(explicit_threads)
        .unwrap_or_else(recommended_memory_bound_thread_count);
    build_global_pool(threads)
}

fn build_global_pool(threads: usize) -> usize {
    match rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
    {
        Ok(()) => threads,
        Err(_already_initialized) => rayon::current_num_threads(),
    }
}

/// The thread count that `init_global_pool` would use.
pub fn resolve_thread_count(explicit_threads: Option<usize>) -> usize {
    resolve_override_thread_count(explicit_threads).unwrap_or_else(recommended_thread_count)
}

fn resolve_override_thread_count(explicit_threads: Option<usize>) -> Option<usize> {
    if let Some(threads) = explicit_threads.filter(|threads| *threads > 0) {
        return Some(threads);
    }
    for var in ["FERRITE_THREADS", "RAYON_NUM_THREADS"] {
        if let Some(threads) = env::var(var)
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .filter(|threads| *threads > 0)
        {
            return Some(threads);
        }
    }
    None
}

/// Largest homogeneous performance-level core count on macOS, or the
/// full available parallelism elsewhere (and as the fallback).
pub fn recommended_thread_count() -> usize {
    let fallback = std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1);
    #[cfg(target_os = "macos")]
    {
        if let Some(largest) = largest_perflevel_logicalcpu() {
            return largest.min(fallback);
        }
    }
    fallback
}

/// Conservative default for kernels that saturate memory bandwidth before
/// occupying the full homogeneous CPU level.
pub fn recommended_memory_bound_thread_count() -> usize {
    recommended_thread_count()
        .saturating_mul(2)
        .div_ceil(3)
        .max(1)
}

#[cfg(target_os = "macos")]
fn largest_perflevel_logicalcpu() -> Option<usize> {
    let levels = sysctl_usize("hw.nperflevels")?;
    (0..levels)
        .filter_map(|level| sysctl_usize(&format!("hw.perflevel{level}.logicalcpu")))
        .max()
        .filter(|count| *count > 0)
}

#[cfg(target_os = "macos")]
fn sysctl_usize(name: &str) -> Option<usize> {
    let output = std::process::Command::new("sysctl")
        .arg("-n")
        .arg(name)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_thread_count_wins() {
        assert_eq!(resolve_thread_count(Some(3)), 3);
    }

    #[test]
    fn zero_explicit_thread_count_falls_through() {
        let resolved = resolve_thread_count(Some(0));
        assert!(resolved >= 1);
    }

    #[test]
    fn recommended_thread_count_is_positive_and_bounded() {
        let recommended = recommended_thread_count();
        let logical = std::thread::available_parallelism()
            .map(NonZeroUsize::get)
            .unwrap_or(1);
        assert!(recommended >= 1);
        assert!(recommended <= logical);
    }

    #[test]
    fn memory_bound_thread_count_is_positive_and_bounded() {
        let regular = recommended_thread_count();
        let memory_bound = recommended_memory_bound_thread_count();
        assert!(memory_bound >= 1);
        assert!(memory_bound <= regular);
    }
}
