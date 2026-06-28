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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Q8KActivationMatvecRole {
    QProj,
    KProj,
    VProj,
    OProj,
    FfnGate,
    FfnUp,
    FfnDown,
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

    pub fn parse_list(value: &str) -> Result<Vec<Self>, String> {
        if value.is_empty() {
            return Err("Q8_K activation matvec role list must not be empty".to_owned());
        }

        value
            .split(',')
            .map(|part| {
                if part.is_empty() {
                    Err("Q8_K activation matvec role list contains an empty item".to_owned())
                } else {
                    Self::parse(part)
                }
            })
            .collect()
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
pub struct ScalarExecutionOptions {
    q8_k_activation_matvec_policy: Q8KActivationMatvecPolicy,
    q8_k_activation_matvec_roles: Q8KActivationMatvecRoleMask,
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

    pub fn with_q8_k_activation_matvec_roles(
        mut self,
        roles: impl IntoIterator<Item = Q8KActivationMatvecRole>,
    ) -> Self {
        self.q8_k_activation_matvec_roles = Q8KActivationMatvecRoleMask::from_roles(roles);
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
        if !self.q8_k_activation_matvec_for_role(role) {
            self.q8_k_activation_matvec_policy = Q8KActivationMatvecPolicy::DefaultOnly;
            self.compare_q8_k_activation_matvec = false;
        }
        self
    }

    pub fn q8_k_activation_matvec_roles_label(self) -> String {
        self.q8_k_activation_matvec_roles.label()
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
    use super::{Q8KActivationMatvecPolicy, Q8KActivationMatvecRole, ScalarExecutionOptions};

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
}
