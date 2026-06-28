use std::error::Error;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

pub struct CliArgs {
    pub model_path: PathBuf,
    pub prompt: PromptSource,
    pub expected_token_id: Option<usize>,
    pub expected_generated_token_ids: Option<Vec<usize>>,
    pub benchmark_runs: Option<usize>,
    pub generate_tokens: Option<usize>,
    pub top_logits: Option<usize>,
    pub profile_next_token: bool,
    pub profile_benchmark_token: bool,
    pub experimental_q8_k_activation_matvec: bool,
    pub compare_q8_k_activation_matvec: bool,
    pub stream: bool,
}

pub enum PromptSource {
    Text(String),
    TokenIds(Vec<usize>),
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<CliArgs, Box<dyn Error>> {
    let mut model_path = None;
    let mut prompt = None;
    let mut prompt_token_ids = None;
    let mut expected_token_id = None;
    let mut expected_generated_token_ids = None;
    let mut benchmark_runs = None;
    let mut generate_tokens = None;
    let mut top_logits = None;
    let mut profile_next_token = false;
    let mut profile_benchmark_token = false;
    let mut experimental_q8_k_activation_matvec = false;
    let mut compare_q8_k_activation_matvec = false;
    let mut stream = false;
    let mut iter = args.into_iter();
    let _program = iter.next();

    while let Some(arg) = iter.next() {
        let Some(flag) = arg.to_str() else {
            return Err(io::Error::other("arguments must be valid UTF-8").into());
        };

        match flag {
            "--model" => {
                model_path = Some(PathBuf::from(next_value(&mut iter, "--model")?));
            }
            "--prompt" => {
                prompt = Some(os_string_to_string(next_value(&mut iter, "--prompt")?)?);
            }
            "--prompt-token-ids" => {
                prompt_token_ids = Some(parse_token_ids(next_value(
                    &mut iter,
                    "--prompt-token-ids",
                )?)?);
            }
            "--expect-token-id" => {
                expected_token_id = Some(parse_usize(
                    next_value(&mut iter, "--expect-token-id")?,
                    "--expect-token-id",
                )?);
            }
            "--expect-generated-token-ids" => {
                expected_generated_token_ids = Some(parse_token_ids(next_value(
                    &mut iter,
                    "--expect-generated-token-ids",
                )?)?);
            }
            "--benchmark-runs" => {
                benchmark_runs = Some(parse_nonzero_usize(
                    next_value(&mut iter, "--benchmark-runs")?,
                    "--benchmark-runs",
                )?);
            }
            "--generate-tokens" => {
                generate_tokens = Some(parse_nonzero_usize(
                    next_value(&mut iter, "--generate-tokens")?,
                    "--generate-tokens",
                )?);
            }
            "--top-logits" => {
                top_logits = Some(parse_nonzero_usize(
                    next_value(&mut iter, "--top-logits")?,
                    "--top-logits",
                )?);
            }
            "--stream" => {
                stream = true;
            }
            "--profile-next-token" => {
                profile_next_token = true;
            }
            "--profile-benchmark-token" => {
                profile_benchmark_token = true;
            }
            "--experimental-q8-k-activation-matvec" => {
                experimental_q8_k_activation_matvec = true;
            }
            "--compare-q8-k-activation-matvec" => {
                compare_q8_k_activation_matvec = true;
                experimental_q8_k_activation_matvec = true;
            }
            "--help" | "-h" => {
                return Err(io::Error::other(usage()).into());
            }
            other => {
                return Err(
                    io::Error::other(format!("unknown argument {other}\n{}", usage())).into(),
                );
            }
        }
    }

    validate_modes(
        generate_tokens,
        benchmark_runs,
        profile_next_token,
        profile_benchmark_token,
        compare_q8_k_activation_matvec,
        stream,
        expected_generated_token_ids.as_deref(),
    )?;

    Ok(CliArgs {
        model_path: model_path.ok_or_else(|| io::Error::other("missing --model argument"))?,
        prompt: prompt_source(prompt, prompt_token_ids)?,
        expected_token_id,
        expected_generated_token_ids,
        benchmark_runs,
        generate_tokens,
        top_logits,
        profile_next_token,
        profile_benchmark_token,
        experimental_q8_k_activation_matvec,
        compare_q8_k_activation_matvec,
        stream,
    })
}

fn validate_modes(
    generate_tokens: Option<usize>,
    benchmark_runs: Option<usize>,
    profile_next_token: bool,
    profile_benchmark_token: bool,
    compare_q8_k_activation_matvec: bool,
    stream: bool,
    expected_generated_token_ids: Option<&[usize]>,
) -> Result<(), Box<dyn Error>> {
    if generate_tokens.is_some() && benchmark_runs.is_some() {
        return Err(
            io::Error::other("use either --generate-tokens or --benchmark-runs, not both").into(),
        );
    }
    if stream && generate_tokens.is_none() {
        return Err(io::Error::other("use --stream with --generate-tokens").into());
    }
    if expected_generated_token_ids.is_some() && generate_tokens.is_none() {
        return Err(
            io::Error::other("use --expect-generated-token-ids with --generate-tokens").into(),
        );
    }
    if profile_benchmark_token && benchmark_runs.is_none() {
        return Err(io::Error::other("use --profile-benchmark-token with --benchmark-runs").into());
    }
    if compare_q8_k_activation_matvec && !profile_next_token && !profile_benchmark_token {
        return Err(io::Error::other(
            "use --compare-q8-k-activation-matvec with --profile-next-token or --profile-benchmark-token",
        )
        .into());
    }
    Ok(())
}

fn prompt_source(
    prompt: Option<String>,
    prompt_token_ids: Option<Vec<usize>>,
) -> Result<PromptSource, Box<dyn Error>> {
    match (prompt, prompt_token_ids) {
        (Some(prompt), None) => Ok(PromptSource::Text(prompt)),
        (None, Some(token_ids)) => Ok(PromptSource::TokenIds(token_ids)),
        (None, None) => {
            Err(io::Error::other("missing --prompt or --prompt-token-ids argument").into())
        }
        (Some(_), Some(_)) => {
            Err(io::Error::other("use either --prompt or --prompt-token-ids, not both").into())
        }
    }
}

fn next_value(
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<OsString, Box<dyn Error>> {
    iter.next()
        .ok_or_else(|| io::Error::other(format!("missing value for {flag}")).into())
}

fn os_string_to_string(value: OsString) -> Result<String, Box<dyn Error>> {
    value
        .into_string()
        .map_err(|_| io::Error::other("prompt must be valid UTF-8").into())
}

fn parse_usize(value: OsString, flag: &str) -> Result<usize, Box<dyn Error>> {
    let value = os_string_to_string(value)?;
    value
        .parse::<usize>()
        .map_err(|error| io::Error::other(format!("{flag} must be a usize: {error}")).into())
}

fn parse_nonzero_usize(value: OsString, flag: &str) -> Result<usize, Box<dyn Error>> {
    let value = parse_usize(value, flag)?;
    if value == 0 {
        return Err(io::Error::other(format!("{flag} must be greater than zero")).into());
    }
    Ok(value)
}

fn parse_token_ids(value: OsString) -> Result<Vec<usize>, Box<dyn Error>> {
    let value = os_string_to_string(value)?;
    let mut token_ids = Vec::new();
    for part in value.split(',') {
        if part.is_empty() {
            return Err(io::Error::other("token id list contains an empty item").into());
        }
        token_ids.push(part.parse::<usize>().map_err(|error| {
            io::Error::other(format!("prompt token id {part:?} is invalid: {error}"))
        })?);
    }
    if token_ids.is_empty() {
        return Err(io::Error::other("prompt token id list must not be empty").into());
    }
    Ok(token_ids)
}

fn usage() -> &'static str {
    "usage: ferrite --model <path.gguf> (--prompt <text> | --prompt-token-ids <id[,id...]>) [--expect-token-id <id>] [--top-logits <count>] [--profile-next-token] [--generate-tokens <count>] [--expect-generated-token-ids <id[,id...]>] [--stream] [--benchmark-runs <count>] [--profile-benchmark-token] [--experimental-q8-k-activation-matvec] [--compare-q8-k-activation-matvec]"
}
