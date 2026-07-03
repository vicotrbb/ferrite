pub(super) fn format_state_capsule_context(capsule: &str, generated_context: &str) -> String {
    format!(
        "Ferrite state capsule:\n{capsule}\n\nGenerated assistant context:\n{generated_context}"
    )
}
