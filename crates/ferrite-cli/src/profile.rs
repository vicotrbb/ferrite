use crate::benchmark::BenchmarkTokenProfile;
use ferrite_inference::scalar::{ProfiledNextToken, ScalarMatVecComparison, ScalarProfileEvent};
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ProfileRoleKey {
    role: String,
    storage_kind: &'static str,
    rows: usize,
    cols: usize,
    storage_bytes: u128,
}

#[derive(Clone, Debug)]
struct Q8KComparisonRoleSummary {
    comparisons: usize,
    argmax_mismatches: usize,
    max_abs_diff: f32,
    max_relative_diff: f32,
    min_reference_argmax_margin: f32,
    min_candidate_argmax_margin: f32,
}

impl Q8KComparisonRoleSummary {
    fn new() -> Self {
        Self {
            comparisons: 0,
            argmax_mismatches: 0,
            max_abs_diff: 0.0,
            max_relative_diff: 0.0,
            min_reference_argmax_margin: f32::INFINITY,
            min_candidate_argmax_margin: f32::INFINITY,
        }
    }

    fn observe(&mut self, comparison: &ScalarMatVecComparison) {
        self.comparisons += 1;
        if comparison.reference_argmax_index() != comparison.candidate_argmax_index() {
            self.argmax_mismatches += 1;
        }
        self.max_abs_diff = self.max_abs_diff.max(comparison.max_abs_diff());
        self.max_relative_diff = self.max_relative_diff.max(comparison.max_relative_diff());
        self.min_reference_argmax_margin = self
            .min_reference_argmax_margin
            .min(comparison.reference_argmax_margin());
        self.min_candidate_argmax_margin = self
            .min_candidate_argmax_margin
            .min(comparison.candidate_argmax_margin());
    }
}

pub(crate) fn print_next_token_profile(profile: &ProfiledNextToken) {
    print_profile(
        "profile_next_token",
        profile.total_elapsed(),
        &profile.events,
    );
    print_q8_k_comparisons("profile_next_token", &profile.comparisons);
}

pub(crate) fn print_benchmark_token_profile(profile: &BenchmarkTokenProfile) {
    println!(
        "profile_benchmark_token_input_id={}",
        profile.input_token_id
    );
    println!("profile_benchmark_token_id={}", profile.token.token_id);
    print_profile(
        "profile_benchmark_token",
        profile.token.total_elapsed(),
        &profile.token.events,
    );
    print_q8_k_comparisons("profile_benchmark_token", &profile.token.comparisons);
}

fn print_profile(prefix: &str, total_elapsed: Duration, events: &[ScalarProfileEvent]) {
    println!("{prefix}_total_ns={}", total_elapsed.as_nanos());

    let mut role_totals = BTreeMap::<ProfileRoleKey, u128>::new();
    for event in events {
        println!(
            "{prefix}_op={}:{}",
            event.label(),
            event.elapsed().as_nanos()
        );
        println!(
            "{prefix}_matrix={}:{}:{}:{}:{}",
            event.label(),
            event.storage_kind().as_str(),
            event.rows(),
            event.cols(),
            event.storage_bytes()
        );
        let key = ProfileRoleKey {
            role: profile_role(event.label()).to_owned(),
            storage_kind: event.storage_kind().as_str(),
            rows: event.rows(),
            cols: event.cols(),
            storage_bytes: event.storage_bytes(),
        };
        *role_totals.entry(key).or_default() += event.elapsed().as_nanos();
    }

    for (key, elapsed_ns) in role_totals {
        println!(
            "{prefix}_role={}:{}:{}:{}:{}:{}",
            key.role, key.storage_kind, key.rows, key.cols, key.storage_bytes, elapsed_ns
        );
    }
}

fn print_q8_k_comparisons(prefix: &str, comparisons: &[ScalarMatVecComparison]) {
    let mut role_summaries = BTreeMap::<ProfileRoleKey, Q8KComparisonRoleSummary>::new();
    for comparison in comparisons {
        println!(
            "{prefix}_q8_k_compare={}:{}:{}:{}:{}:{:.6}:{:.6}:{}:{}:{:.6}:{:.6}",
            comparison.label(),
            comparison.storage_kind().as_str(),
            comparison.rows(),
            comparison.cols(),
            comparison.storage_bytes(),
            comparison.max_abs_diff(),
            comparison.max_relative_diff(),
            comparison.reference_argmax_index(),
            comparison.candidate_argmax_index(),
            comparison.reference_argmax_margin(),
            comparison.candidate_argmax_margin()
        );

        let key = ProfileRoleKey {
            role: profile_role(comparison.label()).to_owned(),
            storage_kind: comparison.storage_kind().as_str(),
            rows: comparison.rows(),
            cols: comparison.cols(),
            storage_bytes: comparison.storage_bytes(),
        };
        role_summaries
            .entry(key)
            .or_insert_with(Q8KComparisonRoleSummary::new)
            .observe(comparison);
    }

    for (key, summary) in role_summaries {
        println!(
            "{prefix}_q8_k_compare_role={}:{}:{}:{}:{}:{}:{}:{:.6}:{:.6}:{:.6}:{:.6}",
            key.role,
            key.storage_kind,
            key.rows,
            key.cols,
            key.storage_bytes,
            summary.comparisons,
            summary.argmax_mismatches,
            summary.max_abs_diff,
            summary.max_relative_diff,
            summary.min_reference_argmax_margin,
            summary.min_candidate_argmax_margin
        );
    }
}

fn profile_role(label: &str) -> &str {
    label.rsplit('.').next().unwrap_or(label)
}
