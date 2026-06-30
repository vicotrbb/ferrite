pub(crate) fn q8_k_compare_line_has_argmax_indexes_and_margins(stdout: &str, prefix: &str) -> bool {
    stdout
        .lines()
        .filter(|line| line.starts_with(prefix))
        .any(|line| {
            let parts = line.split(':').collect::<Vec<_>>();
            parts.len() == 11
                && parts[7].parse::<usize>().is_ok()
                && parts[8].parse::<usize>().is_ok()
                && parts[9].parse::<f32>().is_ok()
                && parts[10].parse::<f32>().is_ok()
        })
}

pub(crate) fn q8_k_compare_role_summary_has_drift_fields(stdout: &str, prefix: &str) -> bool {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .any(|line| {
            let parts = line.split(':').collect::<Vec<_>>();
            if parts.len() != 10 {
                return false;
            }
            let Ok(comparisons) = parts[4].parse::<usize>() else {
                return false;
            };
            let Ok(argmax_mismatches) = parts[5].parse::<usize>() else {
                return false;
            };
            comparisons > 0
                && argmax_mismatches <= comparisons
                && parts[1].parse::<usize>().is_ok()
                && parts[2].parse::<usize>().is_ok()
                && parts[3].parse::<u128>().is_ok()
                && parts[6].parse::<f32>().is_ok()
                && parts[7].parse::<f32>().is_ok()
                && parts[8].parse::<f32>().is_ok()
                && parts[9].parse::<f32>().is_ok()
        })
}
