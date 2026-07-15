use super::kernels::{KernelDispatch, KernelProvider};

/// Selects the KV-cache storage backend for a scalar session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KvBackend {
    /// Default nested-`Vec` storage (today's behavior).
    #[default]
    Vec,
    /// Locus block-pool storage (requires the `locus-kv` feature at build time).
    Locus {
        /// Tokens stored per fixed-size block.
        tokens_per_block: usize,
        /// Maximum tokens the pool is sized for; exceeding it is an error.
        max_tokens: usize,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Selects the activation quantization policy used by eligible matvec kernels.
pub enum Q8KActivationMatvecPolicy {
    /// Use only the default proven kernel path.
    #[default]
    DefaultOnly,
    /// Enable the parity-scoped experimental `Q8_K` activation path.
    ExperimentalParityScoped,
    /// Enable the experimental residual-Q8 path on supported Arm I8MM hosts.
    ExperimentalResidualI8mm,
}

impl Q8KActivationMatvecPolicy {
    /// Returns the stable command-line and diagnostic label for the policy.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DefaultOnly => "default_only",
            Self::ExperimentalParityScoped => "experimental_parity_scoped",
            Self::ExperimentalResidualI8mm => "experimental_residual_i8mm",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A transformer projection that can be selected for activation experiments.
pub enum Q8KActivationMatvecRole {
    /// Attention query projection.
    QProj,
    /// Attention key projection.
    KProj,
    /// Attention value projection.
    VProj,
    /// Attention output projection.
    OProj,
    /// Feed-forward gate projection.
    FfnGate,
    /// Feed-forward up projection.
    FfnUp,
    /// Feed-forward down projection.
    FfnDown,
    /// Vocabulary output projection.
    Output,
}

impl Q8KActivationMatvecRole {
    const ALL: [Self; 8] = [
        Self::QProj,
        Self::KProj,
        Self::VProj,
        Self::OProj,
        Self::FfnGate,
        Self::FfnUp,
        Self::FfnDown,
        Self::Output,
    ];

    /// Returns the stable command-line label for this projection role.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QProj => "q_proj",
            Self::KProj => "k_proj",
            Self::VProj => "v_proj",
            Self::OProj => "o_proj",
            Self::FfnGate => "ffn_gate",
            Self::FfnUp => "ffn_up",
            Self::FfnDown => "ffn_down",
            Self::Output => "output",
        }
    }

    /// Parses one stable projection-role label.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is not a recognized role label.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "q_proj" => Ok(Self::QProj),
            "k_proj" => Ok(Self::KProj),
            "v_proj" => Ok(Self::VProj),
            "o_proj" => Ok(Self::OProj),
            "ffn_gate" => Ok(Self::FfnGate),
            "ffn_up" => Ok(Self::FfnUp),
            "ffn_down" => Ok(Self::FfnDown),
            "output" => Ok(Self::Output),
            other => Err(format!("unknown Q8_K activation matvec role {other}")),
        }
    }

    /// Parses a comma-separated role list or the `all` alias.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty list, unknown or duplicate roles, empty
    /// items, or combining `all` with another role.
    pub fn parse_list(value: &str) -> Result<Vec<Self>, String> {
        if value == "all" {
            return Ok(Self::ALL.to_vec());
        }
        if value.is_empty() {
            return Err("Q8_K activation matvec role list must not be empty".to_owned());
        }

        let mut roles = Vec::new();
        for part in value.split(',') {
            if part.is_empty() {
                return Err("Q8_K activation matvec role list contains an empty item".to_owned());
            }
            if part == "all" {
                return Err(
                    "Q8_K activation matvec role alias all cannot be combined with other roles"
                        .to_owned(),
                );
            }
            let role = Self::parse(part)?;
            if roles.contains(&role) {
                return Err(format!(
                    "duplicate Q8_K activation matvec role {}",
                    role.as_str()
                ));
            }
            roles.push(role);
        }
        Ok(roles)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Q8KActivationMatvecRoleMask(u16);

impl Q8KActivationMatvecRoleMask {
    const fn all() -> Self {
        Self((1 << Q8KActivationMatvecRole::ALL.len()) - 1)
    }

    fn from_roles(roles: impl IntoIterator<Item = Q8KActivationMatvecRole>) -> Self {
        let mut mask = Self(0);
        for role in roles {
            mask.insert(role);
        }
        assert!(
            mask.0 != 0,
            "Q8_K activation matvec role scope must not be empty"
        );
        mask
    }

    fn insert(&mut self, role: Q8KActivationMatvecRole) {
        self.0 |= role.bit();
    }

    fn contains(self, role: Q8KActivationMatvecRole) -> bool {
        self.0 & role.bit() != 0
    }

    fn label(self) -> String {
        if self == Self::all() {
            return "all".to_owned();
        }

        Q8KActivationMatvecRole::ALL
            .into_iter()
            .filter(|role| self.contains(*role))
            .map(Q8KActivationMatvecRole::as_str)
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl Default for Q8KActivationMatvecRoleMask {
    fn default() -> Self {
        Self::all()
    }
}

impl Q8KActivationMatvecRole {
    const fn bit(self) -> u16 {
        1 << self as u16
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Runtime policy for scalar sessions and matrix dispatch.
///
/// Defaults preserve the proven execution path and the nested-vector KV store.
pub struct ScalarExecutionOptions {
    kernel_provider: KernelProvider,
    q8_k_activation_matvec_policy: Q8KActivationMatvecPolicy,
    q8_k_activation_matvec_roles: Q8KActivationMatvecRoleMask,
    compare_q8_k_activation_matvec: bool,
    kv_backend: KvBackend,
}

impl ScalarExecutionOptions {
    /// Selects the built-in CPU kernel provider.
    #[must_use]
    pub fn with_kernel_provider(mut self, provider: KernelProvider) -> Self {
        self.kernel_provider = provider;
        self
    }

    /// Returns the selected built-in CPU kernel provider.
    pub fn kernel_provider(self) -> KernelProvider {
        self.kernel_provider
    }

    pub(in crate::scalar) fn kernel_dispatch(self) -> KernelDispatch {
        KernelDispatch::detect(self.kernel_provider)
    }

    /// Enables or disables the legacy parity-scoped `Q8_K` activation policy.
    #[must_use]
    pub fn with_q8_k_activation_matvec(mut self, enabled: bool) -> Self {
        self.q8_k_activation_matvec_policy = if enabled {
            Q8KActivationMatvecPolicy::ExperimentalParityScoped
        } else {
            Q8KActivationMatvecPolicy::DefaultOnly
        };
        self
    }

    /// Sets the explicit activation matvec policy.
    #[must_use]
    pub fn with_q8_k_activation_matvec_policy(mut self, policy: Q8KActivationMatvecPolicy) -> Self {
        self.q8_k_activation_matvec_policy = policy;
        self
    }

    /// Restricts the parity-scoped activation experiment to selected roles.
    ///
    /// # Panics
    ///
    /// Panics when `roles` is empty.
    #[must_use]
    pub fn with_q8_k_activation_matvec_roles(
        mut self,
        roles: impl IntoIterator<Item = Q8KActivationMatvecRole>,
    ) -> Self {
        self.q8_k_activation_matvec_roles = Q8KActivationMatvecRoleMask::from_roles(roles);
        self
    }

    /// Enables or disables reference-versus-candidate comparison profiling.
    #[must_use]
    pub fn with_q8_k_activation_matvec_comparison(mut self, enabled: bool) -> Self {
        self.compare_q8_k_activation_matvec = enabled;
        self
    }

    #[cfg_attr(
        not(target_arch = "aarch64"),
        allow(dead_code, reason = "activation policy is implemented only on aarch64")
    )]
    pub(in crate::scalar) fn q8_k_activation_matvec(self) -> bool {
        matches!(
            self.q8_k_activation_matvec_policy,
            Q8KActivationMatvecPolicy::ExperimentalParityScoped
        )
    }

    #[cfg_attr(
        not(target_arch = "aarch64"),
        allow(
            dead_code,
            reason = "residual activation policy is implemented only on aarch64"
        )
    )]
    pub(in crate::scalar) fn residual_q8_activation_matvec(self) -> bool {
        matches!(
            self.q8_k_activation_matvec_policy,
            Q8KActivationMatvecPolicy::ExperimentalResidualI8mm
        )
    }

    #[cfg(test)]
    pub(in crate::scalar) fn q8_k_activation_matvec_for_role(
        self,
        role: Q8KActivationMatvecRole,
    ) -> bool {
        self.q8_k_activation_matvec() && self.q8_k_activation_matvec_roles.contains(role)
    }

    pub(in crate::scalar) fn scoped_to_q8_k_activation_role(
        mut self,
        role: Q8KActivationMatvecRole,
    ) -> Self {
        if !self.q8_k_activation_matvec_roles.contains(role) {
            self.q8_k_activation_matvec_policy = Q8KActivationMatvecPolicy::DefaultOnly;
            self.compare_q8_k_activation_matvec = false;
        }
        self
    }

    pub(in crate::scalar) fn q8_k_activation_matvec_candidate(mut self) -> Self {
        self.q8_k_activation_matvec_policy = Q8KActivationMatvecPolicy::ExperimentalParityScoped;
        self.compare_q8_k_activation_matvec = false;
        self
    }

    /// Returns a stable label for the selected activation experiment roles.
    pub fn q8_k_activation_matvec_roles_label(self) -> String {
        self.q8_k_activation_matvec_roles.label()
    }

    /// Returns the selected activation matvec policy.
    pub fn q8_k_activation_matvec_policy(self) -> Q8KActivationMatvecPolicy {
        self.q8_k_activation_matvec_policy
    }

    pub(in crate::scalar) fn compare_q8_k_activation_matvec(self) -> bool {
        self.compare_q8_k_activation_matvec
    }

    /// Sets the session KV-cache storage backend.
    #[must_use]
    pub fn with_kv_backend(mut self, backend: KvBackend) -> Self {
        self.kv_backend = backend;
        self
    }

    /// Returns the selected session KV-cache storage backend.
    pub fn kv_backend(self) -> KvBackend {
        self.kv_backend
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KernelProvider, Q8KActivationMatvecPolicy, Q8KActivationMatvecRole, ScalarExecutionOptions,
    };

    #[test]
    fn default_kernel_provider_is_automatic_and_can_be_overridden() {
        let default = ScalarExecutionOptions::default();
        let portable = default.with_kernel_provider(KernelProvider::Portable);

        assert_eq!(default.kernel_provider(), KernelProvider::Auto);
        assert_eq!(portable.kernel_provider(), KernelProvider::Portable);
    }

    #[test]
    fn default_policy_keeps_q8_k_activation_matvec_disabled() {
        let options = ScalarExecutionOptions::default();

        assert_eq!(
            options.q8_k_activation_matvec_policy(),
            Q8KActivationMatvecPolicy::DefaultOnly
        );
        assert!(!options.q8_k_activation_matvec());
        assert!(!options.residual_q8_activation_matvec());
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
    fn residual_i8mm_policy_is_distinct_from_legacy_q8_k_policy() {
        let options = ScalarExecutionOptions::default().with_q8_k_activation_matvec_policy(
            Q8KActivationMatvecPolicy::ExperimentalResidualI8mm,
        );

        assert!(!options.q8_k_activation_matvec());
        assert!(options.residual_q8_activation_matvec());
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
        assert_eq!(
            Q8KActivationMatvecPolicy::ExperimentalResidualI8mm.as_str(),
            "experimental_residual_i8mm"
        );
    }

    #[test]
    fn q8_k_activation_role_scope_defaults_to_all_roles() {
        let options = ScalarExecutionOptions::default().with_q8_k_activation_matvec(true);

        assert_eq!(options.q8_k_activation_matvec_roles_label(), "all");
        assert!(options.q8_k_activation_matvec_for_role(Q8KActivationMatvecRole::QProj));
        assert!(options.q8_k_activation_matvec_for_role(Q8KActivationMatvecRole::FfnDown));
        assert!(options.q8_k_activation_matvec_for_role(Q8KActivationMatvecRole::Output));
    }

    #[test]
    fn q8_k_activation_role_scope_limits_experimental_dispatch() {
        let options = ScalarExecutionOptions::default()
            .with_q8_k_activation_matvec(true)
            .with_q8_k_activation_matvec_roles([Q8KActivationMatvecRole::FfnDown]);

        assert_eq!(options.q8_k_activation_matvec_roles_label(), "ffn_down");
        assert!(!options.q8_k_activation_matvec_for_role(Q8KActivationMatvecRole::QProj));
        assert!(options.q8_k_activation_matvec_for_role(Q8KActivationMatvecRole::FfnDown));
    }

    #[test]
    #[should_panic(expected = "Q8_K activation matvec role scope must not be empty")]
    fn q8_k_activation_role_scope_rejects_empty_role_set() {
        let _ = ScalarExecutionOptions::default()
            .with_q8_k_activation_matvec(true)
            .with_q8_k_activation_matvec_roles([]);
    }

    #[test]
    fn q8_k_activation_roles_parse_stable_cli_names() -> Result<(), String> {
        assert_eq!(
            Q8KActivationMatvecRole::parse_list("q_proj,ffn_down,output")?,
            vec![
                Q8KActivationMatvecRole::QProj,
                Q8KActivationMatvecRole::FfnDown,
                Q8KActivationMatvecRole::Output,
            ]
        );
        assert!(Q8KActivationMatvecRole::parse_list("q_proj,,output").is_err());
        assert!(Q8KActivationMatvecRole::parse_list("unknown").is_err());
        Ok(())
    }

    #[test]
    fn q8_k_activation_roles_reject_duplicate_cli_names() -> Result<(), String> {
        let err = match Q8KActivationMatvecRole::parse_list("ffn_up,ffn_up") {
            Ok(_) => return Err("duplicate Q8_K activation role should fail".to_owned()),
            Err(err) => err,
        };

        assert_eq!(err, "duplicate Q8_K activation matvec role ffn_up");
        Ok(())
    }

    #[test]
    fn q8_k_activation_roles_reject_mixed_all_alias() -> Result<(), String> {
        let err = match Q8KActivationMatvecRole::parse_list("all,ffn_up") {
            Ok(_) => return Err("mixed all Q8_K activation role should fail".to_owned()),
            Err(err) => err,
        };

        assert_eq!(
            err,
            "Q8_K activation matvec role alias all cannot be combined with other roles"
        );
        Ok(())
    }

    #[test]
    fn default_kv_backend_is_vec() {
        let options = ScalarExecutionOptions::default();
        assert_eq!(options.kv_backend(), super::KvBackend::Vec);
    }

    #[test]
    fn with_kv_backend_selects_locus() {
        let backend = super::KvBackend::Locus {
            tokens_per_block: 16,
            max_tokens: 256,
        };
        let options = ScalarExecutionOptions::default().with_kv_backend(backend);
        assert_eq!(options.kv_backend(), backend);
    }
}
