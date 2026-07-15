use ferrite_inference::sampling::SamplingConfig;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

const DEFAULT_TEMPERATURE: f64 = 0.0;
const DEFAULT_TOP_P: f64 = 1.0;
const DEFAULT_MIN_P: f64 = 0.0;
const DEFAULT_REPETITION_PENALTY: f64 = 1.0;
const DEFAULT_FREQUENCY_PENALTY: f64 = 0.0;
const DEFAULT_PRESENCE_PENALTY: f64 = 0.0;

#[allow(
    clippy::too_many_arguments,
    reason = "arguments map one-to-one to OpenAI and Ferrite sampling fields"
)]
pub(super) fn sampling_config(
    temperature: &Option<Value>,
    top_k: &Option<Value>,
    top_p: &Option<Value>,
    min_p: &Option<Value>,
    repetition_penalty: &Option<Value>,
    frequency_penalty: &Option<Value>,
    presence_penalty: &Option<Value>,
    logit_bias: &Option<Value>,
    seed: &Option<Value>,
) -> Result<SamplingConfig, SamplingOptionError> {
    let temperature = number_inclusive(temperature, "temperature", DEFAULT_TEMPERATURE, 0.0, 2.0)?;
    let top_k = optional_top_k(top_k)?;
    let top_p = number_inclusive(top_p, "top_p", DEFAULT_TOP_P, 0.0, 1.0)?;
    let min_p = number_inclusive(min_p, "min_p", DEFAULT_MIN_P, 0.0, 1.0)?;
    let repetition_penalty = positive_number(
        repetition_penalty,
        "repetition_penalty",
        DEFAULT_REPETITION_PENALTY,
    )?;
    let frequency_penalty = number_inclusive(
        frequency_penalty,
        "frequency_penalty",
        DEFAULT_FREQUENCY_PENALTY,
        -2.0,
        2.0,
    )?;
    let presence_penalty = number_inclusive(
        presence_penalty,
        "presence_penalty",
        DEFAULT_PRESENCE_PENALTY,
        -2.0,
        2.0,
    )?;
    let logit_bias = parse_logit_bias(logit_bias)?;
    let seed = parse_seed(seed)?;

    let config = SamplingConfig::default()
        .with_temperature(temperature as f32)
        .with_top_k(top_k)
        .with_top_p(top_p as f32)
        .with_min_p(min_p as f32)
        .with_repetition_penalty(repetition_penalty as f32)
        .with_frequency_penalty(frequency_penalty as f32)
        .with_presence_penalty(presence_penalty as f32)
        .with_logit_bias(logit_bias)
        .with_seed(seed);
    config
        .validate()
        .map_err(|error| SamplingOptionError::new("sampling", error.to_string()))?;
    Ok(config)
}

fn optional_top_k(value: &Option<Value>) -> Result<Option<usize>, SamplingOptionError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(value) = value.as_u64() else {
        return Err(SamplingOptionError::new(
            "top_k",
            "top_k must be a non-negative integer",
        ));
    };
    if value == 0 {
        return Ok(None);
    }
    usize::try_from(value)
        .map(Some)
        .map_err(|_error| SamplingOptionError::new("top_k", "top_k is too large for this platform"))
}

fn parse_seed(value: &Option<Value>) -> Result<Option<i64>, SamplingOptionError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number.as_i64().map(Some).ok_or_else(|| {
            SamplingOptionError::new("seed", "seed must be a signed 64-bit integer or null")
        }),
        Some(_) => Err(SamplingOptionError::new(
            "seed",
            "seed must be a signed 64-bit integer or null",
        )),
    }
}

fn parse_logit_bias(value: &Option<Value>) -> Result<BTreeMap<usize, f32>, SamplingOptionError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    if value.is_null() {
        return Ok(BTreeMap::new());
    }
    let object = value.as_object().ok_or_else(|| {
        SamplingOptionError::new(
            "logit_bias",
            "logit_bias must be an object keyed by token ID",
        )
    })?;
    let mut biases = BTreeMap::new();
    for (token_id, bias) in object {
        let token_id = token_id.parse::<usize>().map_err(|_error| {
            SamplingOptionError::new(
                "logit_bias",
                "logit_bias keys must be non-negative integer token IDs",
            )
        })?;
        let bias = bias.as_f64().ok_or_else(|| {
            SamplingOptionError::new(
                "logit_bias",
                "logit_bias values must be numbers between -100 and 100",
            )
        })?;
        if !(-100.0..=100.0).contains(&bias) {
            return Err(SamplingOptionError::new(
                "logit_bias",
                "logit_bias values must be numbers between -100 and 100",
            ));
        }
        biases.insert(token_id, bias as f32);
    }
    Ok(biases)
}

fn positive_number(
    value: &Option<Value>,
    name: &'static str,
    default: f64,
) -> Result<f64, SamplingOptionError> {
    let value = optional_number(value, name)?.unwrap_or(default);
    if value <= 0.0 {
        return Err(SamplingOptionError::new(
            name,
            format!("{name} must be greater than zero"),
        ));
    }
    Ok(value)
}

fn number_inclusive(
    value: &Option<Value>,
    name: &'static str,
    default: f64,
    minimum: f64,
    maximum: f64,
) -> Result<f64, SamplingOptionError> {
    let value = optional_number(value, name)?.unwrap_or(default);
    if !(minimum..=maximum).contains(&value) {
        return Err(SamplingOptionError::new(
            name,
            format!("{name} must be between {minimum} and {maximum}"),
        ));
    }
    Ok(value)
}

fn optional_number(
    value: &Option<Value>,
    name: &'static str,
) -> Result<Option<f64>, SamplingOptionError> {
    match value {
        None => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| SamplingOptionError::new(name, format!("{name} must be a number"))),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SamplingOptionError {
    parameter: &'static str,
    message: String,
}

impl SamplingOptionError {
    fn new(parameter: &'static str, message: impl Into<String>) -> Self {
        Self {
            parameter,
            message: message.into(),
        }
    }

    pub(crate) fn parameter(&self) -> &'static str {
        self.parameter
    }
}

impl fmt::Display for SamplingOptionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SamplingOptionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Default)]
    struct TestValues {
        temperature: Option<Value>,
        top_k: Option<Value>,
        top_p: Option<Value>,
        min_p: Option<Value>,
        repetition_penalty: Option<Value>,
        frequency_penalty: Option<Value>,
        presence_penalty: Option<Value>,
        logit_bias: Option<Value>,
        seed: Option<Value>,
    }

    fn config_with(values: TestValues) -> Result<SamplingConfig, SamplingOptionError> {
        sampling_config(
            &values.temperature,
            &values.top_k,
            &values.top_p,
            &values.min_p,
            &values.repetition_penalty,
            &values.frequency_penalty,
            &values.presence_penalty,
            &values.logit_bias,
            &values.seed,
        )
    }

    #[test]
    fn defaults_to_fused_greedy() -> Result<(), SamplingOptionError> {
        let config = config_with(TestValues::default())?;

        assert!(config.uses_fused_greedy_path());
        assert_eq!(config.temperature(), 0.0);
        Ok(())
    }

    #[test]
    fn parses_all_supported_sampling_controls() -> Result<(), SamplingOptionError> {
        let config = config_with(TestValues {
            temperature: Some(json!(0.8)),
            top_k: Some(json!(40)),
            top_p: Some(json!(0.9)),
            min_p: Some(json!(0.05)),
            repetition_penalty: Some(json!(1.1)),
            frequency_penalty: Some(json!(0.2)),
            presence_penalty: Some(json!(-0.3)),
            logit_bias: Some(json!({"7": 1.5})),
            seed: Some(json!(-42)),
        })?;

        assert_eq!(config.temperature(), 0.8);
        assert_eq!(config.top_k(), Some(40));
        assert_eq!(config.top_p(), 0.9);
        assert_eq!(config.min_p(), 0.05);
        assert_eq!(config.repetition_penalty(), 1.1);
        assert_eq!(config.frequency_penalty(), 0.2);
        assert_eq!(config.presence_penalty(), -0.3);
        assert_eq!(config.logit_bias().get(&7), Some(&1.5));
        assert_eq!(config.seed(), Some(-42));
        Ok(())
    }

    #[test]
    fn reports_the_invalid_parameter() -> Result<(), SamplingOptionError> {
        let error = match config_with(TestValues {
            temperature: Some(json!(3)),
            ..TestValues::default()
        }) {
            Ok(_) => {
                return Err(SamplingOptionError::new(
                    "temperature",
                    "temperature above the maximum should fail",
                ));
            }
            Err(error) => error,
        };

        assert_eq!(error.parameter(), "temperature");
        assert_eq!(error.to_string(), "temperature must be between 0 and 2");
        Ok(())
    }

    #[test]
    fn accepts_zero_top_p() -> Result<(), SamplingOptionError> {
        let config = config_with(TestValues {
            top_p: Some(json!(0)),
            ..TestValues::default()
        })?;

        assert_eq!(config.top_p(), 0.0);
        Ok(())
    }
}
