use ferrite_inference::scalar::{ProfiledTokenId, ScalarExecutionOptions, ScalarLlamaModel};
use std::error::Error;
use std::io;

pub(crate) struct BenchmarkTokenProfile {
    pub(crate) input_token_id: usize,
    pub(crate) token: ProfiledTokenId,
}

pub(crate) fn profile_first_benchmark_token(
    model: &ScalarLlamaModel,
    prompt_token_ids: &[usize],
    input_token_id: usize,
    options: ScalarExecutionOptions,
) -> Result<BenchmarkTokenProfile, Box<dyn Error>> {
    let mut session = model.start_session_with_options(options);
    let replayed_next = session.accept_prompt(prompt_token_ids)?;
    if replayed_next.token_id != input_token_id {
        return Err(io::Error::other(format!(
            "benchmark profile replay produced token id {} but benchmark input token id is {input_token_id}",
            replayed_next.token_id
        ))
        .into());
    }

    let token = session.accept_token_id_profiled(input_token_id)?;
    Ok(BenchmarkTokenProfile {
        input_token_id,
        token,
    })
}
