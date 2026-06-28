#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Q8KActivationMatvecPolicy {
    #[default]
    DefaultOnly,
    ExperimentalParityScoped,
}

impl Q8KActivationMatvecPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DefaultOnly => "default_only",
            Self::ExperimentalParityScoped => "experimental_parity_scoped",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScalarExecutionOptions {
    q8_k_activation_matvec_policy: Q8KActivationMatvecPolicy,
    compare_q8_k_activation_matvec: bool,
}

impl ScalarExecutionOptions {
    pub fn with_q8_k_activation_matvec(mut self, enabled: bool) -> Self {
        self.q8_k_activation_matvec_policy = if enabled {
            Q8KActivationMatvecPolicy::ExperimentalParityScoped
        } else {
            Q8KActivationMatvecPolicy::DefaultOnly
        };
        self
    }

    pub fn with_q8_k_activation_matvec_policy(mut self, policy: Q8KActivationMatvecPolicy) -> Self {
        self.q8_k_activation_matvec_policy = policy;
        self
    }

    pub fn with_q8_k_activation_matvec_comparison(mut self, enabled: bool) -> Self {
        self.compare_q8_k_activation_matvec = enabled;
        self
    }

    #[cfg_attr(not(target_arch = "aarch64"), allow(dead_code))]
    pub(in crate::scalar) fn q8_k_activation_matvec(self) -> bool {
        matches!(
            self.q8_k_activation_matvec_policy,
            Q8KActivationMatvecPolicy::ExperimentalParityScoped
        )
    }

    pub fn q8_k_activation_matvec_policy(self) -> Q8KActivationMatvecPolicy {
        self.q8_k_activation_matvec_policy
    }

    pub(in crate::scalar) fn compare_q8_k_activation_matvec(self) -> bool {
        self.compare_q8_k_activation_matvec
    }
}

#[cfg(test)]
mod tests {
    use super::{Q8KActivationMatvecPolicy, ScalarExecutionOptions};

    #[test]
    fn default_policy_keeps_q8_k_activation_matvec_disabled() {
        let options = ScalarExecutionOptions::default();

        assert_eq!(
            options.q8_k_activation_matvec_policy(),
            Q8KActivationMatvecPolicy::DefaultOnly
        );
        assert!(!options.q8_k_activation_matvec());
    }

    #[test]
    fn parity_scoped_policy_enables_q8_k_activation_matvec() {
        let options = ScalarExecutionOptions::default().with_q8_k_activation_matvec_policy(
            Q8KActivationMatvecPolicy::ExperimentalParityScoped,
        );

        assert_eq!(
            options.q8_k_activation_matvec_policy(),
            Q8KActivationMatvecPolicy::ExperimentalParityScoped
        );
        assert!(options.q8_k_activation_matvec());
    }

    #[test]
    fn legacy_bool_adapter_maps_to_explicit_q8_k_policy() {
        let options = ScalarExecutionOptions::default().with_q8_k_activation_matvec(true);

        assert_eq!(
            options.q8_k_activation_matvec_policy(),
            Q8KActivationMatvecPolicy::ExperimentalParityScoped
        );
        assert!(options.q8_k_activation_matvec());
    }

    #[test]
    fn q8_k_activation_matvec_policy_has_stable_output_names() {
        assert_eq!(
            Q8KActivationMatvecPolicy::DefaultOnly.as_str(),
            "default_only"
        );
        assert_eq!(
            Q8KActivationMatvecPolicy::ExperimentalParityScoped.as_str(),
            "experimental_parity_scoped"
        );
    }
}
