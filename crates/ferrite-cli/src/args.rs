use std::error::Error;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

pub struct CliArgs {
    pub model_path: PathBuf,
    pub prompt: String,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<CliArgs, Box<dyn Error>> {
    let mut model_path = None;
    let mut prompt = None;
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
        prompt: prompt.ok_or_else(|| io::Error::other("missing --prompt argument"))?,
    })
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

fn usage() -> &'static str {
    "usage: ferrite --model <path.gguf> --prompt <text>"
}
