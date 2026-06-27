use crate::args;
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
    let model = ScalarLlamaModel::from_gguf_unquantized(&gguf, &bytes)?;
    let next = model.next_token_for_text_prompt(&tokenizer, &args.prompt)?;
    let token = tokenizer.token(next.token_id).ok_or_else(|| {
        io::Error::other(format!(
            "next token id {} is not present in tokenizer vocabulary",
            next.token_id
        ))
    })?;

    println!("next_token_id={}", next.token_id);
    println!("next_token={token}");
    Ok(())
}
