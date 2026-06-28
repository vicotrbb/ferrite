use ferrite_inference::scalar::ProfiledNextToken;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ProfileRoleKey {
    role: String,
    storage_kind: &'static str,
    rows: usize,
    cols: usize,
    storage_bytes: u128,
}

pub(crate) fn print_next_token_profile(profile: &ProfiledNextToken) {
    println!(
        "profile_next_token_total_ns={}",
        profile.total_elapsed().as_nanos()
    );

    let mut role_totals = BTreeMap::<ProfileRoleKey, u128>::new();
    for event in &profile.events {
        println!(
            "profile_next_token_op={}:{}",
            event.label(),
            event.elapsed().as_nanos()
        );
        println!(
            "profile_next_token_matrix={}:{}:{}:{}:{}",
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
            "profile_next_token_role={}:{}:{}:{}:{}:{}",
            key.role, key.storage_kind, key.rows, key.cols, key.storage_bytes, elapsed_ns
        );
    }
}

fn profile_role(label: &str) -> &str {
    label.rsplit('.').next().unwrap_or(label)
}
