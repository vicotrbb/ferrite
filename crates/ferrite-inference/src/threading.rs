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
#[cfg(target_os = "linux")]
use std::{collections::BTreeSet, fs, path::Path};

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

/// Builds the inference pool for an HTTP server. Explicit and environment
/// overrides remain exact. The automatic default leaves one worker-sized CPU
/// slot available for async request handling when the kernel policy would
/// otherwise consume every recommended worker.
pub fn init_server_global_pool(
    explicit_threads: Option<usize>,
    memory_bound_kernels: bool,
) -> usize {
    let threads = resolve_override_thread_count(explicit_threads)
        .unwrap_or_else(|| recommended_server_thread_count(memory_bound_kernels));
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

/// The server thread count after exact overrides or the service-aware default.
pub fn resolve_server_thread_count(
    explicit_threads: Option<usize>,
    memory_bound_kernels: bool,
) -> usize {
    resolve_override_thread_count(explicit_threads)
        .unwrap_or_else(|| recommended_server_thread_count(memory_bound_kernels))
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
    #[cfg(target_os = "linux")]
    {
        if let Some(topology_threads) = linux_topology_thread_count(fallback) {
            return topology_threads;
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

/// Automatic server worker count. Memory-bound policies already leave CPU
/// capacity unused, so the additional service reservation is only applied when
/// the regular provider would otherwise use the full recommendation.
pub fn recommended_server_thread_count(memory_bound_kernels: bool) -> usize {
    let regular = recommended_thread_count();
    let inference = if memory_bound_kernels {
        recommended_memory_bound_thread_count()
    } else {
        regular
    };
    if inference == regular && regular >= 4 {
        regular - 1
    } else {
        inference
    }
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

#[cfg(target_os = "linux")]
fn linux_topology_thread_count(available: usize) -> Option<usize> {
    let cpu_ids = linux_allowed_cpu_ids().or_else(linux_sysfs_cpu_ids)?;
    if cpu_ids.is_empty() {
        return None;
    }

    let capacities = cpu_ids
        .iter()
        .filter_map(|cpu| read_linux_cpu_usize(*cpu, "cpu_capacity"))
        .collect::<Vec<_>>();
    if capacities.len() == cpu_ids.len() {
        let maximum = capacities.iter().copied().max()?;
        let performance_cpus = capacities
            .into_iter()
            .filter(|capacity| *capacity == maximum)
            .count();
        return Some(performance_cpus.clamp(1, available));
    }

    let core_types = cpu_ids
        .iter()
        .filter_map(|cpu| read_linux_cpu_usize(*cpu, "topology/core_type"))
        .collect::<Vec<_>>();
    if core_types.len() == cpu_ids.len() && core_types.iter().any(|kind| *kind != 0) {
        let maximum = core_types.iter().copied().max()?;
        let performance_cpus = core_types
            .into_iter()
            .filter(|kind| *kind == maximum)
            .count();
        return Some(performance_cpus.clamp(1, available));
    }

    let physical_cores = cpu_ids
        .iter()
        .filter_map(|cpu| {
            Some((
                read_linux_cpu_usize(*cpu, "topology/physical_package_id")?,
                read_linux_cpu_usize(*cpu, "topology/core_id")?,
            ))
        })
        .collect::<BTreeSet<_>>();
    if physical_cores.is_empty() {
        None
    } else {
        Some(physical_cores.len().clamp(1, available))
    }
}

#[cfg(target_os = "linux")]
fn linux_allowed_cpu_ids() -> Option<Vec<usize>> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let allowed = status
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))?
        .trim();
    parse_linux_cpu_list(allowed)
}

#[cfg(target_os = "linux")]
fn linux_sysfs_cpu_ids() -> Option<Vec<usize>> {
    let mut cpu_ids = fs::read_dir("/sys/devices/system/cpu")
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()?
                .strip_prefix("cpu")?
                .parse::<usize>()
                .ok()
        })
        .collect::<Vec<_>>();
    cpu_ids.sort_unstable();
    cpu_ids.dedup();
    (!cpu_ids.is_empty()).then_some(cpu_ids)
}

#[cfg(target_os = "linux")]
fn parse_linux_cpu_list(value: &str) -> Option<Vec<usize>> {
    let mut cpu_ids = BTreeSet::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return None;
        }
        if let Some((start, end)) = part.split_once('-') {
            let start = start.parse::<usize>().ok()?;
            let end = end.parse::<usize>().ok()?;
            if start > end || end.saturating_sub(start) > 65_535 {
                return None;
            }
            cpu_ids.extend(start..=end);
        } else {
            cpu_ids.insert(part.parse::<usize>().ok()?);
        }
    }
    (!cpu_ids.is_empty()).then(|| cpu_ids.into_iter().collect())
}

#[cfg(target_os = "linux")]
fn read_linux_cpu_usize(cpu: usize, suffix: &str) -> Option<usize> {
    let path = Path::new("/sys/devices/system/cpu")
        .join(format!("cpu{cpu}"))
        .join(suffix);
    fs::read_to_string(path).ok()?.trim().parse().ok()
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

    #[test]
    fn server_default_reserves_capacity_only_when_needed() {
        let regular = recommended_thread_count();
        let server = recommended_server_thread_count(false);
        let memory_bound = recommended_server_thread_count(true);

        assert!(server >= 1);
        assert!(server <= regular);
        if regular >= 4 {
            assert_eq!(server, regular - 1);
        } else {
            assert_eq!(server, regular);
        }
        assert_eq!(memory_bound, recommended_memory_bound_thread_count());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_linux_allowed_cpu_ranges_strictly() {
        assert_eq!(
            parse_linux_cpu_list("0-2,5,7-8"),
            Some(vec![0, 1, 2, 5, 7, 8])
        );
        assert_eq!(parse_linux_cpu_list("3,3"), Some(vec![3]));
        assert_eq!(parse_linux_cpu_list("4-2"), None);
        assert_eq!(parse_linux_cpu_list("0,,2"), None);
    }
}
