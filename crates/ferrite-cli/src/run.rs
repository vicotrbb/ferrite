use crate::args::{self, PromptSource};
use crate::profile::print_next_token_profile;
use ferrite_inference::scalar::{
    NextToken, ProfiledNextToken, ScalarLlamaModel, ScalarLlamaSession,
};
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
    let (next, profile) = accept_prompt(&mut session, &prompt_token_ids, args.profile_next_token)?;
    let token = tokenizer.token(next.token_id).ok_or_else(|| {
        io::Error::other(format!(
            "next token id {} is not present in tokenizer vocabulary",
            next.token_id
        ))
    })?;

    println!("prompt_token_ids={}", join_token_ids(&prompt_token_ids));
    println!("next_token_id={}", next.token_id);
    println!("next_token={token}");
    if let Some(profile) = &profile {
        print_next_token_profile(profile);
    }
    if let Some(count) = args.top_logits {
        println!("top_logits={}", format_top_logits(&next.logits, count));
    }
    if let Some(count) = args.generate_tokens {
        let generated_token_ids =
            generate_tokens(&mut session, &tokenizer, next.clone(), count, args.stream)?;
        println!("generated_cached_tokens={}", session.cached_token_count());
        println!(
            "generated_token_ids={}",
            join_token_ids(&generated_token_ids)
        );
        println!("generated_text={}", tokenizer.decode(&generated_token_ids)?);
        if let Some(expected_token_ids) = args.expected_generated_token_ids {
            println!(
                "expected_generated_token_ids={}",
                join_token_ids(&expected_token_ids)
            );
            let matches = generated_token_ids == expected_token_ids;
            println!("generated_match={matches}");
            if !matches {
                return Err(io::Error::other(format!(
                    "generated token ids {} did not match expected token ids {}",
                    join_token_ids(&generated_token_ids),
                    join_token_ids(&expected_token_ids)
                ))
                .into());
            }
        }
    }
    if let Some(runs) = args.benchmark_runs {
        let mut benchmark_token_id = next.token_id;
        let mut benchmark_token_ids = Vec::with_capacity(runs);
        let started = Instant::now();
        for _ in 0..runs {
            benchmark_token_id = session.accept_token_id(benchmark_token_id)?;
            benchmark_token_ids.push(benchmark_token_id);
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

fn accept_prompt(
    session: &mut ScalarLlamaSession<'_>,
    tokens: &[usize],
    profile_next_token: bool,
) -> Result<(NextToken, Option<ProfiledNextToken>), Box<dyn Error>> {
    if tokens.is_empty() {
        return Err(io::Error::other("prompt must contain at least one token").into());
    }
    if !profile_next_token {
        return Ok((session.accept_prompt(tokens)?, None));
    }

    for token_id in &tokens[..tokens.len() - 1] {
        session.accept_token(*token_id)?;
    }
    let profiled = session.accept_token_profiled(tokens[tokens.len() - 1])?;
    Ok((profiled.next_token.clone(), Some(profiled)))
}

fn generate_tokens(
    session: &mut ScalarLlamaSession<'_>,
    tokenizer: &GgufTokenizer,
    next: NextToken,
    count: usize,
    stream: bool,
) -> Result<Vec<usize>, Box<dyn Error>> {
    let mut next = next;
    let mut generated_token_ids = Vec::with_capacity(count);
    for _ in 0..count {
        if stream {
            println!("stream_token_id={}", next.token_id);
            println!("stream_text={}", tokenizer.decode(&[next.token_id])?);
        }
        generated_token_ids.push(next.token_id);
        next = session.accept_token(next.token_id)?;
    }
    Ok(generated_token_ids)
}

fn format_top_logits(logits: &[f32], count: usize) -> String {
    let mut ranked = logits
        .iter()
        .copied()
        .enumerate()
        .collect::<Vec<(usize, f32)>>();
    ranked.sort_by(|(left_id, left), (right_id, right)| {
        right.total_cmp(left).then_with(|| left_id.cmp(right_id))
    });
    ranked
        .into_iter()
        .take(count)
        .map(|(token_id, logit)| format!("{token_id}:{logit:.6}"))
        .collect::<Vec<_>>()
        .join(",")
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
