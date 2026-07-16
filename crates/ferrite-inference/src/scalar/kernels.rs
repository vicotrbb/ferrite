//! CPU capability detection and built-in kernel-provider selection.
//!
//! Optimized kernels remain isolated in their architecture modules. This file
//! owns the runtime feature probes and the policy that decides whether those
//! modules may be entered. A provider can disable optimized code for parity
//! and portability checks, but it can never force a feature that the CPU did
//! not report.

use std::sync::OnceLock;

static DETECTED_CAPABILITIES: OnceLock<CpuKernelCapabilities> = OnceLock::new();

/// Selects the built-in CPU kernel provider used by a scalar session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KernelProvider {
    /// Detect CPU features at runtime and use a proven optimized kernel when
    /// the current operation has one.
    #[default]
    Auto,
    /// Use only architecture-neutral reference kernels.
    Portable,
}

impl KernelProvider {
    /// Returns the stable CLI and diagnostic label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Portable => "portable",
        }
    }

    /// Parses a stable provider label.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is not `auto` or `portable`.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "portable" => Ok(Self::Portable),
            other => Err(format!(
                "kernel provider must be one of auto, portable (got {other})"
            )),
        }
    }
}

/// Runtime-detected CPU features relevant to Ferrite's current and candidate
/// kernel families.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuKernelCapabilities {
    neon: bool,
    dotprod: bool,
    i8mm: bool,
    sve2: bool,
    avx2: bool,
    f16c: bool,
    avx_vnni: bool,
    avx512_vnni: bool,
}

impl CpuKernelCapabilities {
    /// Detects supported features for the current process and architecture.
    ///
    /// Unsupported architectures return an all-false capability set. Ferrite
    /// uses the same standard-library runtime probes that guard each
    /// `#[target_feature]` kernel entry.
    pub fn detect() -> Self {
        *DETECTED_CAPABILITIES.get_or_init(Self::detect_uncached)
    }

    fn detect_uncached() -> Self {
        let mut capabilities = Self::default();
        #[cfg(target_arch = "aarch64")]
        {
            capabilities.neon = std::arch::is_aarch64_feature_detected!("neon");
            capabilities.dotprod = std::arch::is_aarch64_feature_detected!("dotprod");
            capabilities.i8mm = std::arch::is_aarch64_feature_detected!("i8mm");
            capabilities.sve2 = std::arch::is_aarch64_feature_detected!("sve2");
        }
        #[cfg(target_arch = "x86_64")]
        {
            capabilities.avx2 = std::arch::is_x86_feature_detected!("avx2");
            capabilities.f16c = std::arch::is_x86_feature_detected!("f16c");
            capabilities.avx_vnni = std::arch::is_x86_feature_detected!("avxvnni");
            capabilities.avx512_vnni = std::arch::is_x86_feature_detected!("avx512vnni");
        }
        capabilities
    }

    /// Returns whether Arm Advanced SIMD (NEON) is available.
    pub fn neon(self) -> bool {
        self.neon
    }

    /// Returns whether Arm `FEAT_DotProd` is available.
    pub fn dotprod(self) -> bool {
        self.dotprod
    }

    /// Returns whether Arm `FEAT_I8MM` is available.
    pub fn i8mm(self) -> bool {
        self.i8mm
    }

    /// Returns whether Arm `FEAT_SVE2` is available.
    pub fn sve2(self) -> bool {
        self.sve2
    }

    /// Returns whether x86 AVX2 is available.
    pub fn avx2(self) -> bool {
        self.avx2
    }

    /// Returns whether x86 F16C conversion instructions are available.
    pub fn f16c(self) -> bool {
        self.f16c
    }

    /// Returns whether x86 AVX-VNNI is available.
    pub fn avx_vnni(self) -> bool {
        self.avx_vnni
    }

    /// Returns whether x86 AVX512-VNNI is available.
    pub fn avx512_vnni(self) -> bool {
        self.avx512_vnni
    }

    /// Returns a stable comma-separated feature list for diagnostics.
    pub fn detected_feature_labels(self) -> String {
        let features = [
            ("neon", self.neon),
            ("dotprod", self.dotprod),
            ("i8mm", self.i8mm),
            ("sve2", self.sve2),
            ("avx2", self.avx2),
            ("f16c", self.f16c),
            ("avx_vnni", self.avx_vnni),
            ("avx512_vnni", self.avx512_vnni),
        ];
        let labels = features
            .into_iter()
            .filter_map(|(label, detected)| detected.then_some(label))
            .collect::<Vec<_>>();
        if labels.is_empty() {
            "none".to_owned()
        } else {
            labels.join(",")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::scalar) struct KernelDispatch {
    provider: KernelProvider,
    capabilities: CpuKernelCapabilities,
}

impl KernelDispatch {
    pub(in crate::scalar) fn detect(provider: KernelProvider) -> Self {
        Self {
            provider,
            capabilities: CpuKernelCapabilities::detect(),
        }
    }

    fn optimized_allowed(self) -> bool {
        matches!(self.provider, KernelProvider::Auto)
    }

    #[cfg(any(target_arch = "aarch64", test))]
    pub(in crate::scalar) fn neon(self) -> bool {
        self.optimized_allowed() && self.capabilities.neon()
    }

    #[cfg(any(target_arch = "aarch64", test))]
    pub(in crate::scalar) fn dotprod(self) -> bool {
        self.optimized_allowed() && self.capabilities.dotprod()
    }

    #[cfg(any(target_arch = "aarch64", test))]
    pub(in crate::scalar) fn i8mm(self) -> bool {
        self.optimized_allowed() && self.capabilities.i8mm()
    }

    #[cfg(any(target_arch = "x86_64", test))]
    pub(in crate::scalar) fn avx2(self) -> bool {
        self.optimized_allowed() && self.capabilities.avx2()
    }

    #[cfg(any(target_arch = "x86_64", test))]
    pub(in crate::scalar) fn f16c(self) -> bool {
        self.optimized_allowed() && self.capabilities.f16c()
    }
}

#[cfg(test)]
mod tests {
    use super::{CpuKernelCapabilities, KernelDispatch, KernelProvider};

    #[test]
    fn provider_labels_round_trip() {
        for provider in [KernelProvider::Auto, KernelProvider::Portable] {
            assert_eq!(KernelProvider::parse(provider.as_str()), Ok(provider));
        }
        assert!(KernelProvider::parse("native").is_err());
    }

    #[test]
    fn portable_provider_cannot_enter_optimized_kernels() {
        let dispatch = KernelDispatch {
            provider: KernelProvider::Portable,
            capabilities: CpuKernelCapabilities {
                neon: true,
                dotprod: true,
                i8mm: true,
                sve2: true,
                avx2: true,
                f16c: true,
                avx_vnni: true,
                avx512_vnni: true,
            },
        };

        assert!(!dispatch.neon());
        assert!(!dispatch.dotprod());
        assert!(!dispatch.i8mm());
        assert!(!dispatch.avx2());
        assert!(!dispatch.f16c());
    }

    #[test]
    fn automatic_provider_never_invents_cpu_features() {
        let detected = CpuKernelCapabilities::detect();
        let dispatch = KernelDispatch::detect(KernelProvider::Auto);

        assert_eq!(dispatch.neon(), detected.neon());
        assert_eq!(dispatch.dotprod(), detected.dotprod());
        assert_eq!(dispatch.i8mm(), detected.i8mm());
        assert_eq!(dispatch.avx2(), detected.avx2());
        assert_eq!(dispatch.f16c(), detected.f16c());
    }

    #[test]
    fn capability_diagnostics_are_stable_and_nonempty() {
        assert!(
            !CpuKernelCapabilities::detect()
                .detected_feature_labels()
                .is_empty()
        );
    }
}
