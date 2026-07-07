#![cfg(feature = "locus-kv")]

use ferrite_fixtures::scalar_llama_f32_gguf_fixture;
use ferrite_inference::scalar::{
    InferenceError, KvBackend, ScalarExecutionOptions, ScalarLlamaModel,
};
use ferrite_model::gguf::parse_gguf;

// `scalar_llama_f32_gguf_fixture` wires identity Q/K/V/O and FFN-gate/up
// projections, so cached keys and values genuinely mix across positions
// (unlike the quantized fixtures, whose attention/FFN weights are zero and
// would make cache reads a no-op). The prompt spans past a 4-token block
// boundary (`tokens_per_block: 4` below) so the Locus backend must span
// multiple blocks to match the Vec backend.
const PROMPT: &[usize] = &[0, 1, 2, 0, 1];

#[test]
fn locus_backend_matches_vec_logits() -> Result<(), InferenceError> {
    let bytes = scalar_llama_f32_gguf_fixture();
    let gguf = parse_gguf(&bytes)?;
    let model = ScalarLlamaModel::from_gguf_scalar(&gguf, &bytes)?;

    let mut vec_session = model.start_session_with_options(ScalarExecutionOptions::default())?;
    let vec_next = vec_session.accept_prompt(PROMPT)?;

    let locus_options = ScalarExecutionOptions::default().with_kv_backend(KvBackend::Locus {
        tokens_per_block: 4,
        max_tokens: 64,
    });
    let mut locus_session = model.start_session_with_options(locus_options)?;
    let locus_next = locus_session.accept_prompt(PROMPT)?;

    assert_eq!(vec_next.token_id, locus_next.token_id);
    assert_eq!(vec_next.logits, locus_next.logits);
    assert_eq!(vec_session.kv_cache_bytes(), locus_session.kv_cache_bytes());
    Ok(())
}
