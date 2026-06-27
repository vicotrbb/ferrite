use crate::args::{self, PromptSource};
use ferrite_inference::scalar::ScalarLlamaModel;
use ferrite_model::gguf::parse_gguf;
use ferrite_model::tokenizer::GgufTokenizer;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io;

pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), Box<dyn Error>> {
    let args = args::parse(args)?;
    let bytes = fs::read(&args.model_path)?;
    let gguf = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&gguf)?;
    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    let prompt_token_ids = prompt_token_ids(&tokenizer, args.prompt)?;
    let next = model.next_token_for_prompt(&prompt_token_ids)?;
    let token = tokenizer.token(next.token_id).ok_or_else(|| {
        io::Error::other(format!(
            "next token id {} is not present in tokenizer vocabulary",
            next.token_id
        ))
    })?;

    println!("prompt_token_ids={}", join_token_ids(&prompt_token_ids));
    println!("next_token_id={}", next.token_id);
    println!("next_token={token}");
    if let Some(expected_token_id) = args.expected_token_id {
        println!("expected_token_id={expected_token_id}");
        let matches = next.token_id == expected_token_id;
        println!("match={matches}");
        if !matches {
            return Err(io::Error::other(format!(
                "next token id {} did not match expected token id {expected_token_id}",
                next.token_id
            ))
            .into());
        }
    }
    Ok(())
}

fn prompt_token_ids(
    tokenizer: &GgufTokenizer,
    prompt: PromptSource,
) -> Result<Vec<usize>, Box<dyn Error>> {
    match prompt {
        PromptSource::Text(prompt) => Ok(tokenizer.encode(&prompt)?),
        PromptSource::TokenIds(token_ids) => Ok(token_ids),
    }
}

fn join_token_ids(token_ids: &[usize]) -> String {
    token_ids
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
