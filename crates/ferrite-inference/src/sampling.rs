//! Deterministic, per-request token sampling over model logits.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_TEMPERATURE: f32 = 0.0;
const DEFAULT_TOP_P: f32 = 1.0;
const DEFAULT_MIN_P: f32 = 0.0;
const DEFAULT_REPETITION_PENALTY: f32 = 1.0;
const DEFAULT_FREQUENCY_PENALTY: f32 = 0.0;
const DEFAULT_PRESENCE_PENALTY: f32 = 0.0;
const MAX_TEMPERATURE: f32 = 2.0;
const MAX_API_PENALTY: f32 = 2.0;
const MAX_LOGIT_BIAS: f32 = 100.0;
const SPLITMIX_INCREMENT: u64 = 0x9e37_79b9_7f4a_7c15;
const SPLITMIX_MULTIPLIER_1: u64 = 0xbf58_476d_1ce4_e5b9;
const SPLITMIX_MULTIPLIER_2: u64 = 0x94d0_49bb_1331_11eb;
const UNIT_F64_SCALE: f64 = 1.0 / ((1_u64 << 53) as f64);

static AUTOMATIC_SEED_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Validated controls for selecting the next token from model logits.
///
/// The default is exact greedy decoding. Probability filters are applied only
/// when temperature is greater than zero. Penalties and logit bias still modify
/// greedy selection and therefore require the full logit vector.
#[derive(Clone, Debug, PartialEq)]
pub struct SamplingConfig {
    temperature: f32,
    top_k: Option<usize>,
    top_p: f32,
    min_p: f32,
    repetition_penalty: f32,
    frequency_penalty: f32,
    presence_penalty: f32,
    logit_bias: BTreeMap<usize, f32>,
    seed: Option<i64>,
}

impl SamplingConfig {
    /// Returns the exact greedy policy.
    pub fn greedy() -> Self {
        Self::default()
    }

    /// Sets temperature in the inclusive range `0..=2`.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Sets the optional positive top-k candidate limit.
    #[must_use]
    pub fn with_top_k(mut self, top_k: Option<usize>) -> Self {
        self.top_k = top_k;
        self
    }

    /// Sets nucleus probability in the inclusive range `0..=1`.
    #[must_use]
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p;
        self
    }

    /// Sets the minimum probability ratio in the inclusive range `0..=1`.
    #[must_use]
    pub fn with_min_p(mut self, min_p: f32) -> Self {
        self.min_p = min_p;
        self
    }

    /// Sets the positive multiplicative repetition penalty.
    #[must_use]
    pub fn with_repetition_penalty(mut self, repetition_penalty: f32) -> Self {
        self.repetition_penalty = repetition_penalty;
        self
    }

    /// Sets the frequency penalty in the inclusive range `-2..=2`.
    #[must_use]
    pub fn with_frequency_penalty(mut self, frequency_penalty: f32) -> Self {
        self.frequency_penalty = frequency_penalty;
        self
    }

    /// Sets the presence penalty in the inclusive range `-2..=2`.
    #[must_use]
    pub fn with_presence_penalty(mut self, presence_penalty: f32) -> Self {
        self.presence_penalty = presence_penalty;
        self
    }

    /// Replaces token-ID logit biases. Each value must be in `-100..=100`.
    #[must_use]
    pub fn with_logit_bias(mut self, logit_bias: BTreeMap<usize, f32>) -> Self {
        self.logit_bias = logit_bias;
        self
    }

    /// Sets a stable signed seed. Equal seeds have equal bit patterns on every
    /// supported platform.
    #[must_use]
    pub fn with_seed(mut self, seed: Option<i64>) -> Self {
        self.seed = seed;
        self
    }

    /// Returns the configured temperature.
    pub fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Returns the optional top-k candidate limit.
    pub fn top_k(&self) -> Option<usize> {
        self.top_k
    }

    /// Returns the configured nucleus probability.
    pub fn top_p(&self) -> f32 {
        self.top_p
    }

    /// Returns the configured minimum probability ratio.
    pub fn min_p(&self) -> f32 {
        self.min_p
    }

    /// Returns the configured repetition penalty.
    pub fn repetition_penalty(&self) -> f32 {
        self.repetition_penalty
    }

    /// Returns the configured frequency penalty.
    pub fn frequency_penalty(&self) -> f32 {
        self.frequency_penalty
    }

    /// Returns the configured presence penalty.
    pub fn presence_penalty(&self) -> f32 {
        self.presence_penalty
    }

    /// Returns token-ID logit biases.
    pub fn logit_bias(&self) -> &BTreeMap<usize, f32> {
        &self.logit_bias
    }

    /// Returns the explicitly configured seed.
    pub fn seed(&self) -> Option<i64> {
        self.seed
    }

    /// Returns whether decode can use fused argmax without materializing logits.
    pub fn uses_fused_greedy_path(&self) -> bool {
        self.temperature == 0.0
            && self.repetition_penalty == DEFAULT_REPETITION_PENALTY
            && self.frequency_penalty == DEFAULT_FREQUENCY_PENALTY
            && self.presence_penalty == DEFAULT_PRESENCE_PENALTY
            && self.logit_bias.is_empty()
    }

    /// Validates every sampling control.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-finite or out-of-range control, a zero top-k
    /// value, or an invalid logit bias.
    pub fn validate(&self) -> Result<(), SamplingError> {
        validate_inclusive("temperature", self.temperature, 0.0, MAX_TEMPERATURE)?;
        if self.top_k == Some(0) {
            return Err(SamplingError::new("top_k must be greater than zero"));
        }
        validate_inclusive("top_p", self.top_p, 0.0, 1.0)?;
        validate_inclusive("min_p", self.min_p, 0.0, 1.0)?;
        if !self.repetition_penalty.is_finite() || self.repetition_penalty <= 0.0 {
            return Err(SamplingError::new(
                "repetition_penalty must be finite and greater than zero",
            ));
        }
        validate_inclusive(
            "frequency_penalty",
            self.frequency_penalty,
            -MAX_API_PENALTY,
            MAX_API_PENALTY,
        )?;
        validate_inclusive(
            "presence_penalty",
            self.presence_penalty,
            -MAX_API_PENALTY,
            MAX_API_PENALTY,
        )?;
        for (token_id, bias) in &self.logit_bias {
            if !bias.is_finite() || !(-MAX_LOGIT_BIAS..=MAX_LOGIT_BIAS).contains(bias) {
                return Err(SamplingError::new(format!(
                    "logit bias for token {token_id} must be finite and between -100 and 100"
                )));
            }
        }
        Ok(())
    }
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            temperature: DEFAULT_TEMPERATURE,
            top_k: None,
            top_p: DEFAULT_TOP_P,
            min_p: DEFAULT_MIN_P,
            repetition_penalty: DEFAULT_REPETITION_PENALTY,
            frequency_penalty: DEFAULT_FREQUENCY_PENALTY,
            presence_penalty: DEFAULT_PRESENCE_PENALTY,
            logit_bias: BTreeMap::new(),
            seed: None,
        }
    }
}

/// Per-request deterministic sampler and token-frequency state.
#[derive(Debug)]
pub struct Sampler {
    config: SamplingConfig,
    rng: SplitMix64,
    effective_seed: u64,
    token_counts: BTreeMap<usize, u64>,
}

impl Sampler {
    /// Creates one isolated sampler.
    ///
    /// When no seed is configured, Ferrite assigns a process-local seed. The
    /// assigned generator is still isolated from every other request.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration is invalid.
    pub fn new(config: SamplingConfig) -> Result<Self, SamplingError> {
        config.validate()?;
        let effective_seed = config
            .seed
            .map(|seed| u64::from_ne_bytes(seed.to_ne_bytes()))
            .unwrap_or_else(automatic_seed);
        Ok(Self {
            config,
            rng: SplitMix64::new(effective_seed),
            effective_seed,
            token_counts: BTreeMap::new(),
        })
    }

    /// Returns the seed used by this request's generator.
    pub fn effective_seed(&self) -> u64 {
        self.effective_seed
    }

    /// Records one token for repetition, frequency, and presence penalties.
    pub fn observe(&mut self, token_id: usize) {
        let count = self.token_counts.entry(token_id).or_default();
        *count = count.saturating_add(1);
    }

    /// Records prompt or generated token history in order.
    pub fn observe_all(&mut self, token_ids: &[usize]) {
        for token_id in token_ids {
            self.observe(*token_id);
        }
    }

    /// Selects and records one token from a vocabulary logit vector.
    ///
    /// # Errors
    ///
    /// Returns an error for empty or non-finite logits, out-of-range logit-bias
    /// token IDs, or a policy that removes every candidate.
    pub fn sample(&mut self, logits: &[f32]) -> Result<usize, SamplingError> {
        self.sample_where(logits, |_| true)
    }

    /// Selects and records one token from candidates accepted by `allowed`.
    ///
    /// The constraint is applied before top-k, top-p, and min-p filtering. This
    /// keeps requested sampling semantics within the grammar-allowed token set.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid logits or bias entries, when the constraint
    /// rejects every token, or when probability filters remove every retained
    /// candidate.
    pub fn sample_where(
        &mut self,
        logits: &[f32],
        mut allowed: impl FnMut(usize) -> bool,
    ) -> Result<usize, SamplingError> {
        if logits.is_empty() {
            return Err(SamplingError::new("cannot sample an empty logit vector"));
        }
        if let Some((token_id, _)) = logits
            .iter()
            .enumerate()
            .find(|(_, logit)| !logit.is_finite())
        {
            return Err(SamplingError::new(format!(
                "logit for token {token_id} is not finite"
            )));
        }
        if let Some(token_id) = self
            .config
            .logit_bias
            .keys()
            .find(|token_id| **token_id >= logits.len())
        {
            return Err(SamplingError::new(format!(
                "logit bias token {token_id} is out of bounds for vocabulary size {}",
                logits.len()
            )));
        }

        let mut candidates = logits
            .iter()
            .copied()
            .enumerate()
            .filter(|(token_id, _)| allowed(*token_id))
            .map(|(token_id, logit)| Candidate {
                token_id,
                logit: self.adjusted_logit(token_id, logit),
                weight: 0.0,
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Err(SamplingError::new(
                "token constraint removed every candidate",
            ));
        }
        rank_candidates(&mut candidates);

        let selected = if self.config.temperature == 0.0 {
            candidates[0].token_id
        } else {
            self.sample_candidates(&mut candidates)?
        };
        self.observe(selected);
        Ok(selected)
    }

    fn adjusted_logit(&self, token_id: usize, mut logit: f32) -> f64 {
        let count = self.token_counts.get(&token_id).copied().unwrap_or(0);
        if count > 0 {
            if logit < 0.0 {
                logit *= self.config.repetition_penalty;
            } else {
                logit /= self.config.repetition_penalty;
            }
            logit -= self.config.frequency_penalty * count as f32;
            logit -= self.config.presence_penalty;
        }
        logit += self
            .config
            .logit_bias
            .get(&token_id)
            .copied()
            .unwrap_or(0.0);
        f64::from(logit)
    }

    fn sample_candidates(
        &mut self,
        candidates: &mut Vec<Candidate>,
    ) -> Result<usize, SamplingError> {
        if let Some(top_k) = self.config.top_k {
            candidates.truncate(top_k.min(candidates.len()));
        }

        let temperature = f64::from(self.config.temperature);
        let maximum = candidates[0].logit / temperature;
        for candidate in candidates.iter_mut() {
            candidate.weight = (candidate.logit / temperature - maximum).exp();
        }
        if self.config.min_p > 0.0 {
            let threshold = f64::from(self.config.min_p);
            candidates.retain(|candidate| candidate.weight >= threshold);
        }
        if candidates.is_empty() {
            return Err(SamplingError::new(
                "sampling policy removed every token candidate",
            ));
        }

        let total_weight = candidates
            .iter()
            .map(|candidate| candidate.weight)
            .sum::<f64>();
        let nucleus_target = f64::from(self.config.top_p) * total_weight;
        let mut cumulative = 0.0;
        let mut retained = 0;
        for candidate in candidates.iter() {
            cumulative += candidate.weight;
            retained += 1;
            if cumulative >= nucleus_target {
                break;
            }
        }
        candidates.truncate(retained);

        let retained_weight = candidates
            .iter()
            .map(|candidate| candidate.weight)
            .sum::<f64>();
        let target = self.rng.next_unit_f64() * retained_weight;
        let mut cumulative = 0.0;
        for candidate in candidates.iter() {
            cumulative += candidate.weight;
            if target < cumulative {
                return Ok(candidate.token_id);
            }
        }
        candidates
            .last()
            .map(|candidate| candidate.token_id)
            .ok_or_else(|| SamplingError::new("sampling policy retained no candidates"))
    }
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    token_id: usize,
    logit: f64,
    weight: f64,
}

fn rank_candidates(candidates: &mut [Candidate]) {
    candidates.sort_by(|left, right| {
        right
            .logit
            .total_cmp(&left.logit)
            .then_with(|| left.token_id.cmp(&right.token_id))
    });
}

#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_INCREMENT);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(SPLITMIX_MULTIPLIER_1);
        value = (value ^ (value >> 27)).wrapping_mul(SPLITMIX_MULTIPLIER_2);
        value ^ (value >> 31)
    }

    fn next_unit_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * UNIT_F64_SCALE
    }
}

fn automatic_seed() -> u64 {
    let counter = AUTOMATIC_SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos() as u64);
    let mut mixer = SplitMix64::new(time ^ counter.rotate_left(17));
    mixer.next_u64()
}

fn validate_inclusive(
    name: &str,
    value: f32,
    minimum: f32,
    maximum: f32,
) -> Result<(), SamplingError> {
    if !value.is_finite() || !(minimum..=maximum).contains(&value) {
        return Err(SamplingError::new(format!(
            "{name} must be finite and between {minimum} and {maximum}"
        )));
    }
    Ok(())
}

/// An invalid sampling configuration or sampling input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SamplingError {
    message: String,
}

impl SamplingError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SamplingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SamplingError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_fused_greedy_and_breaks_ties_by_token_id() -> Result<(), SamplingError> {
        let config = SamplingConfig::default();
        assert!(config.uses_fused_greedy_path());
        let mut sampler = Sampler::new(config)?;

        assert_eq!(sampler.sample(&[1.0, 4.0, 4.0, -2.0])?, 1);
        Ok(())
    }

    #[test]
    fn equal_seeds_produce_equal_sequences() -> Result<(), SamplingError> {
        let config = SamplingConfig::default()
            .with_temperature(1.0)
            .with_seed(Some(42));
        let mut left = Sampler::new(config.clone())?;
        let mut right = Sampler::new(config)?;
        let logits = [1.0, 1.5, 2.0, 2.5];

        let left_tokens = (0..32)
            .map(|_| left.sample(&logits))
            .collect::<Result<Vec<_>, _>>()?;
        let right_tokens = (0..32)
            .map(|_| right.sample(&logits))
            .collect::<Result<Vec<_>, _>>()?;

        assert_eq!(left_tokens, right_tokens);
        Ok(())
    }

    #[test]
    fn unrelated_sampler_does_not_perturb_seeded_sequence() -> Result<(), SamplingError> {
        let config = SamplingConfig::default()
            .with_temperature(0.8)
            .with_top_p(0.9)
            .with_seed(Some(7));
        let logits = [1.0, 2.0, 3.0, 4.0];
        let mut expected = Sampler::new(config.clone())?;
        let expected_tokens = (0..16)
            .map(|_| expected.sample(&logits))
            .collect::<Result<Vec<_>, _>>()?;

        let mut actual = Sampler::new(config)?;
        let mut unrelated = Sampler::new(
            SamplingConfig::default()
                .with_temperature(1.0)
                .with_seed(Some(99)),
        )?;
        let mut actual_tokens = Vec::new();
        for _ in 0..16 {
            let _ = unrelated.sample(&logits)?;
            actual_tokens.push(actual.sample(&logits)?);
        }

        assert_eq!(actual_tokens, expected_tokens);
        Ok(())
    }

    #[test]
    fn top_k_one_is_greedy_under_positive_temperature() -> Result<(), SamplingError> {
        let mut sampler = Sampler::new(
            SamplingConfig::default()
                .with_temperature(2.0)
                .with_top_k(Some(1))
                .with_seed(Some(1)),
        )?;

        for _ in 0..16 {
            assert_eq!(sampler.sample(&[-1.0, 3.0, 2.0])?, 1);
        }
        Ok(())
    }

    #[test]
    fn zero_top_p_retains_the_highest_ranked_candidate() -> Result<(), SamplingError> {
        let mut sampler = Sampler::new(
            SamplingConfig::default()
                .with_temperature(1.0)
                .with_top_p(0.0)
                .with_seed(Some(1)),
        )?;

        for _ in 0..16 {
            assert_eq!(sampler.sample(&[-1.0, 3.0, 2.0])?, 1);
        }
        Ok(())
    }

    #[test]
    fn min_p_removes_candidates_far_below_the_maximum() -> Result<(), SamplingError> {
        let mut sampler = Sampler::new(
            SamplingConfig::default()
                .with_temperature(1.0)
                .with_min_p(0.5)
                .with_seed(Some(3)),
        )?;

        for _ in 0..16 {
            assert_eq!(sampler.sample(&[10.0, 0.0, -10.0])?, 0);
        }
        Ok(())
    }

    #[test]
    fn penalties_and_bias_change_greedy_selection() -> Result<(), SamplingError> {
        let mut biases = BTreeMap::new();
        biases.insert(2, 2.0);
        let mut sampler = Sampler::new(
            SamplingConfig::default()
                .with_repetition_penalty(2.0)
                .with_frequency_penalty(1.0)
                .with_presence_penalty(1.0)
                .with_logit_bias(biases),
        )?;
        sampler.observe_all(&[0, 0]);

        assert_eq!(sampler.sample(&[5.0, 4.0, 3.0])?, 2);
        Ok(())
    }

    #[test]
    fn validates_ranges_and_logit_bias_vocabulary() -> Result<(), SamplingError> {
        assert!(SamplingConfig::default()
            .with_temperature(2.1)
            .validate()
            .is_err());
        assert!(SamplingConfig::default()
            .with_top_k(Some(0))
            .validate()
            .is_err());
        assert!(SamplingConfig::default()
            .with_top_p(-0.1)
            .validate()
            .is_err());

        let mut biases = BTreeMap::new();
        biases.insert(4, 1.0);
        let mut sampler = Sampler::new(SamplingConfig::default().with_logit_bias(biases))?;
        assert!(sampler.sample(&[0.0, 1.0]).is_err());
        Ok(())
    }
}
