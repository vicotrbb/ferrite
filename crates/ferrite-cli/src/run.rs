use crate::args::{self, PromptSource};
use ferrite_inference::scalar::ScalarLlamaModel;
use ferrite_model::gguf::parse_gguf;
use ferrite_model::tokenizer::GgufTokenizer;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::time::Instant;

pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), Box<dyn Error>> {
    let args = args::parse(args)?;
    let bytes = fs::read(&args.model_path)?;
    let model_file_bytes = bytes.len();
    let gguf = parse_gguf(&bytes)?;
    let tokenizer = GgufTokenizer::from_gguf(&gguf)?;
    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    drop(bytes);

    let prompt_token_ids = prompt_token_ids(&tokenizer, args.prompt)?;
    let mut session = model.start_session();
    let next = session.accept_prompt(&prompt_token_ids)?;
    let token = tokenizer.token(next.token_id).ok_or_else(|| {
        io::Error::other(format!(
            "next token id {} is not present in tokenizer vocabulary",
            next.token_id
        ))
    })?;

    println!("prompt_token_ids={}", join_token_ids(&prompt_token_ids));
    println!("next_token_id={}", next.token_id);
    println!("next_token={token}");
    if let Some(count) = args.generate_tokens {
        let generated_token_ids = generate_tokens(&mut session, next.clone(), count)?;
        println!("generated_cached_tokens={}", session.cached_token_count());
        println!(
            "generated_token_ids={}",
            join_token_ids(&generated_token_ids)
        );
        println!("generated_text={}", tokenizer.decode(&generated_token_ids)?);
    }
    if let Some(runs) = args.benchmark_runs {
        let mut benchmark_next = next.clone();
        let mut benchmark_token_ids = Vec::with_capacity(runs);
        let started = Instant::now();
        for _ in 0..runs {
            benchmark_next = session.accept_token(benchmark_next.token_id)?;
            benchmark_token_ids.push(benchmark_next.token_id);
        }
        let total_ns = started.elapsed().as_nanos();
        let avg_ns = total_ns / runs as u128;
        println!("benchmark_runs={runs}");
        println!("benchmark_cached_tokens={}", session.cached_token_count());
        println!(
            "benchmark_token_ids={}",
            join_token_ids(&benchmark_token_ids)
        );
        println!("benchmark_total_ns={total_ns}");
        println!("benchmark_avg_ns={avg_ns}");
    }
    println!("model_file_bytes={model_file_bytes}");
    println!("model_file_retained_bytes=0");
    println!("scalar_weight_bytes={}", model.scalar_weight_bytes());
    println!("kv_cache_bytes={}", session.kv_cache_bytes());
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

fn generate_tokens(
    session: &mut ferrite_inference::scalar::ScalarLlamaSession<'_>,
    next: ferrite_inference::scalar::NextToken,
    count: usize,
) -> Result<Vec<usize>, Box<dyn Error>> {
    let mut next = next;
    let mut generated_token_ids = Vec::with_capacity(count);
    for _ in 0..count {
        generated_token_ids.push(next.token_id);
        next = session.accept_token(next.token_id)?;
    }
    Ok(generated_token_ids)
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
