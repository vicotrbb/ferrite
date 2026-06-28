use crate::benchmark::BenchmarkTokenProfile;
use ferrite_inference::scalar::{ProfiledNextToken, ScalarProfileEvent};
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

pub(crate) fn print_next_token_profile(profile: &ProfiledNextToken) {
    print_profile(
        "profile_next_token",
        profile.total_elapsed(),
        &profile.events,
    );
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

fn profile_role(label: &str) -> &str {
    label.rsplit('.').next().unwrap_or(label)
}
