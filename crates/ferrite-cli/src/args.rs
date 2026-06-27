use std::error::Error;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

pub struct CliArgs {
    pub model_path: PathBuf,
    pub prompt: PromptSource,
    pub expected_token_id: Option<usize>,
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

    Ok(CliArgs {
        model_path: model_path.ok_or_else(|| io::Error::other("missing --model argument"))?,
        prompt: prompt_source(prompt, prompt_token_ids)?,
        expected_token_id,
    })
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
    "usage: ferrite --model <path.gguf> (--prompt <text> | --prompt-token-ids <id[,id...]>) [--expect-token-id <id>]"
}
