use crate::args::{self, CliKvBackend, PromptSource};
use crate::benchmark::profile_first_benchmark_token;
use crate::profile::{print_benchmark_token_profile, print_next_token_profile};
use ferrite_inference::scalar::{
    KvBackend, NextToken, ProfiledNextToken, Q8KActivationMatvecPolicy, ScalarExecutionOptions,
    ScalarLlamaModel, ScalarLlamaSession,
};
use ferrite_model::gguf::parse_gguf;
use ferrite_model::tokenizer::GgufTokenizer;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), Box<dyn Error>> {
    let args = args::parse(args)?;
    #[cfg(target_arch = "aarch64")]
    let use_memory_bound_pool = args.experimental_residual_q8_activation_matvec
        && std::arch::is_aarch64_feature_detected!("i8mm");
    #[cfg(not(target_arch = "aarch64"))]
    let use_memory_bound_pool = false;
    let inference_threads = if use_memory_bound_pool {
        ferrite_inference::threading::init_memory_bound_global_pool(args.threads)
    } else {
        ferrite_inference::threading::init_global_pool(args.threads)
    };
    println!("inference_threads={inference_threads}");
    let bytes = fs::read(&args.model_path)?;
    let model_file_bytes = bytes.len();
    let gguf_parse_started = Instant::now();
    let gguf = parse_gguf(&bytes)?;
    let gguf_parse_ns = gguf_parse_started.elapsed().as_nanos();
    let tokenizer_load_started = Instant::now();
    let tokenizer = GgufTokenizer::from_gguf(&gguf)?;
    let tokenizer_load_ns = tokenizer_load_started.elapsed().as_nanos();
    if let Some(runs) = args.benchmark_tokenization_runs {
        return benchmark_tokenization(
            &tokenizer,
            args.prompt,
            runs,
            model_file_bytes,
            TokenizationSetupTiming {
                gguf_parse_ns,
                tokenizer_load_ns,
            },
        );
    }
    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;
    drop(bytes);
    if let Some(sleep_ms) = args.sleep_after_load_ms {
        println!("sleep_after_load_ms={sleep_ms}");
        io::stdout().flush()?;
        thread::sleep(Duration::from_millis(sleep_ms));
    }

    let prompt_token_ids = prompt_token_ids(&tokenizer, args.prompt)?;
    let q8_k_activation_matvec_policy = if args.experimental_residual_q8_activation_matvec {
        Q8KActivationMatvecPolicy::ExperimentalResidualI8mm
    } else if args.experimental_q8_k_activation_matvec {
        Q8KActivationMatvecPolicy::ExperimentalParityScoped
    } else {
        Q8KActivationMatvecPolicy::DefaultOnly
    };
    let mut execution_options = ScalarExecutionOptions::default()
        .with_q8_k_activation_matvec_policy(q8_k_activation_matvec_policy)
        .with_q8_k_activation_matvec_comparison(args.compare_q8_k_activation_matvec);
    if let Some(roles) = args.experimental_q8_k_activation_roles {
        execution_options = execution_options.with_q8_k_activation_matvec_roles(roles);
    }
    let execution_options = match args.kv_backend {
        CliKvBackend::Vec => execution_options,
        CliKvBackend::Locus => {
            // The Locus pool must be sized for the whole workload, not just the
            // requested generation count: prompt tokens are pushed into the KV
            // cache too. Default to prompt length + generated/benchmarked tokens
            // + headroom unless the user gave an explicit override. Over-sizing
            // is cheap (the pool is mmap-backed and lazily resident); under-sizing
            // is the bug this sizing guards against.
            let max_tokens = args.kv_max_tokens.unwrap_or_else(|| {
                prompt_token_ids
                    .len()
                    .saturating_add(args.generate_tokens.unwrap_or(0))
                    .saturating_add(args.benchmark_runs.unwrap_or(0))
                    .saturating_add(16)
            });
            execution_options.with_kv_backend(KvBackend::Locus {
                tokens_per_block: args.kv_tokens_per_block,
                max_tokens,
            })
        }
    };
    let mut session = model.start_session_with_options(execution_options)?;
    let (next, profile) = accept_prompt(&mut session, &prompt_token_ids, args.profile_next_token)?;
    let token = tokenizer.token(next.token_id).ok_or_else(|| {
        io::Error::other(format!(
            "next token id {} is not present in tokenizer vocabulary",
            next.token_id
        ))
    })?;

    println!("prompt_token_ids={}", join_token_ids(&prompt_token_ids));
    println!(
        "experimental_q8_k_activation_matvec={}",
        args.experimental_q8_k_activation_matvec
    );
    println!(
        "experimental_residual_q8_activation_matvec={}",
        args.experimental_residual_q8_activation_matvec
    );
    println!(
        "compare_q8_k_activation_matvec={}",
        args.compare_q8_k_activation_matvec
    );
    println!(
        "q8_k_activation_matvec_policy={}",
        q8_k_activation_matvec_policy.as_str()
    );
    println!(
        "q8_k_activation_matvec_roles={}",
        execution_options.q8_k_activation_matvec_roles_label()
    );
    println!("next_token_id={}", next.token_id);
    println!("next_token={token}");
    if let Some(profile) = &profile {
        print_next_token_profile(profile);
    }
    if let Some(count) = args.top_logits {
        println!("top_logits={}", format_top_logits(&next.logits, count));
    }
    if let Some(count) = args.generate_tokens {
        let generated =
            generate_tokens(&mut session, &tokenizer, next.clone(), count, args.stream)?;
        println!("generated_cached_tokens={}", session.cached_token_count());
        println!(
            "generated_token_ids={}",
            join_token_ids(&generated.token_ids)
        );
        println!("generated_stopped_on_eos={}", generated.stopped_on_eos);
        println!("generated_text={}", tokenizer.decode(&generated.token_ids)?);
        if let Some(expected_token_ids) = args.expected_generated_token_ids {
            println!(
                "expected_generated_token_ids={}",
                join_token_ids(&expected_token_ids)
            );
            let matches = generated.token_ids == expected_token_ids;
            println!("generated_match={matches}");
            if !matches {
                return Err(io::Error::other(format!(
                    "generated token ids {} did not match expected token ids {}",
                    join_token_ids(&generated.token_ids),
                    join_token_ids(&expected_token_ids)
                ))
                .into());
            }
        }
    }
    if let Some(runs) = args.benchmark_runs {
        if let Some(streams) = args.benchmark_batch_streams.filter(|streams| *streams > 1) {
            return benchmark_batched_streams(
                &model,
                &prompt_token_ids,
                next.token_id,
                runs,
                streams,
                execution_options,
            );
        }
        let mut benchmark_token_id = next.token_id;
        let mut benchmark_token_ids = Vec::with_capacity(runs);
        let benchmark_profile = if args.profile_benchmark_token {
            Some(profile_first_benchmark_token(
                &model,
                &prompt_token_ids,
                benchmark_token_id,
                execution_options,
            )?)
        } else {
            None
        };
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
        if let Some(profile) = &benchmark_profile {
            print_benchmark_token_profile(profile);
        }
    }
    println!("model_file_bytes={model_file_bytes}");
    println!("model_file_retained_bytes=0");
    println!("scalar_weight_bytes={}", model.scalar_weight_bytes());
    println!("kv_cache_bytes={}", session.kv_cache_bytes());
    #[cfg(feature = "locus-kv")]
    if let Some(allocations) = session.locus_pool_allocation_count() {
        println!("locus_pool_allocation_count={allocations}");
    }
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

/// Decodes `runs` steps across `streams` sessions with the batched step,
/// so each weight row is streamed once per step for the whole batch.
/// Every stream starts from the same prompt; stream 0's token ids must
/// match a single-session benchmark run of the same length.
fn benchmark_batched_streams(
    model: &ScalarLlamaModel,
    prompt_token_ids: &[usize],
    first_token_id: usize,
    runs: usize,
    streams: usize,
    execution_options: ScalarExecutionOptions,
) -> Result<(), Box<dyn Error>> {
    let mut sessions = Vec::with_capacity(streams);
    for _ in 0..streams {
        let mut session = model.start_session_with_options(execution_options)?;
        session.accept_prompt(prompt_token_ids)?;
        sessions.push(session);
    }
    let mut token_ids = vec![first_token_id; streams];
    let mut stream_zero_ids = Vec::with_capacity(runs);

    let started = Instant::now();
    for _ in 0..runs {
        token_ids = ferrite_inference::scalar::accept_token_ids_batch(&mut sessions, &token_ids)?;
        stream_zero_ids.push(token_ids[0]);
    }
    let total_ns = started.elapsed().as_nanos();

    let total_tokens = runs as u128 * streams as u128;
    println!("benchmark_runs={runs}");
    println!("benchmark_batch_streams={streams}");
    println!(
        "benchmark_cached_tokens={}",
        sessions[0].cached_token_count()
    );
    println!("benchmark_token_ids={}", join_token_ids(&stream_zero_ids));
    println!("benchmark_total_ns={total_ns}");
    println!("benchmark_avg_ns={}", total_ns / runs as u128);
    println!(
        "benchmark_batch_tokens_per_second={:.2}",
        total_tokens as f64 / (total_ns as f64 / 1e9)
    );
    println!("model_file_retained_bytes=0");
    println!("scalar_weight_bytes={}", model.scalar_weight_bytes());
    println!("kv_cache_bytes={}", sessions[0].kv_cache_bytes());
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
) -> Result<GeneratedTokens, Box<dyn Error>> {
    let eos_token_id = tokenizer.eos_token_id();
    let mut token_id = next.token_id;
    let mut token_ids = Vec::with_capacity(count);
    let mut stopped_on_eos = false;

    for _ in 0..count {
        token_ids.push(token_id);
        if stream {
            println!("stream_token_id={token_id}");
            println!("stream_text={}", tokenizer.decode(&[token_id])?);
        }

        if Some(token_id) == eos_token_id {
            stopped_on_eos = true;
            break;
        }

        token_id = session.accept_token_id(token_id)?;
    }
    Ok(GeneratedTokens {
        token_ids,
        stopped_on_eos,
    })
}

struct GeneratedTokens {
    token_ids: Vec<usize>,
    stopped_on_eos: bool,
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

fn benchmark_tokenization(
    tokenizer: &GgufTokenizer,
    prompt: PromptSource,
    runs: usize,
    model_file_bytes: usize,
    setup_timing: TokenizationSetupTiming,
) -> Result<(), Box<dyn Error>> {
    let PromptSource::Text(prompt) = prompt else {
        return Err(io::Error::other(
            "use --benchmark-tokenization-runs with --prompt, not --prompt-token-ids",
        )
        .into());
    };
    let mut token_ids = Vec::new();
    let started = Instant::now();
    for _ in 0..runs {
        token_ids = tokenizer.encode(&prompt)?;
    }
    let total_ns = started.elapsed().as_nanos();
    let avg_ns = total_ns / runs as u128;
    println!("tokenization_benchmark_runs={runs}");
    println!("tokenization_benchmark_prompt_bytes={}", prompt.len());
    println!("tokenization_benchmark_token_count={}", token_ids.len());
    println!(
        "tokenization_benchmark_token_ids_fingerprint={}",
        token_ids_fingerprint(&token_ids)
    );
    println!(
        "tokenization_benchmark_gguf_parse_ns={}",
        setup_timing.gguf_parse_ns
    );
    println!(
        "tokenization_benchmark_tokenizer_load_ns={}",
        setup_timing.tokenizer_load_ns
    );
    println!("tokenization_benchmark_encode_total_ns={total_ns}");
    println!("tokenization_benchmark_encode_avg_ns={avg_ns}");
    println!("tokenization_benchmark_total_ns={total_ns}");
    println!("tokenization_benchmark_avg_ns={avg_ns}");
    println!("model_file_bytes={model_file_bytes}");
    println!("model_file_retained_bytes=0");
    Ok(())
}

struct TokenizationSetupTiming {
    gguf_parse_ns: u128,
    tokenizer_load_ns: u128,
}

fn token_ids_fingerprint(token_ids: &[usize]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for token_id in token_ids {
        for byte in (*token_id as u64).to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("fnv1a64:{hash:016x}")
}

fn join_token_ids(token_ids: &[usize]) -> String {
    token_ids
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
