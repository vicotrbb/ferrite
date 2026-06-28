#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScalarExecutionOptions {
    q8_k_activation_matvec: bool,
    compare_q8_k_activation_matvec: bool,
}

impl ScalarExecutionOptions {
    pub fn with_q8_k_activation_matvec(mut self, enabled: bool) -> Self {
        self.q8_k_activation_matvec = enabled;
        self
    }

    pub fn with_q8_k_activation_matvec_comparison(mut self, enabled: bool) -> Self {
        self.compare_q8_k_activation_matvec = enabled;
        self
    }

    #[cfg(target_arch = "aarch64")]
    pub(in crate::scalar) fn q8_k_activation_matvec(self) -> bool {
        self.q8_k_activation_matvec
    }

    pub(in crate::scalar) fn compare_q8_k_activation_matvec(self) -> bool {
        self.compare_q8_k_activation_matvec
    }
}
