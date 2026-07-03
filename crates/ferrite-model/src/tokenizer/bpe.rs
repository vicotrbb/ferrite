use super::{TokenizationControl, TokenizerError};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BpeMetadata {
    token_to_id: BTreeMap<String, usize>,
    merges: Vec<BpeMerge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BpeMerge {
    left: String,
    right: String,
    merged: String,
}

impl BpeMetadata {
    pub(super) fn new(tokens: &[String], merges: &[String]) -> Result<Self, TokenizerError> {
        let token_to_id = tokens
            .iter()
            .enumerate()
            .map(|(id, token)| (token.clone(), id))
            .collect::<BTreeMap<_, _>>();
        let mut parsed_merges = Vec::new();
        for merge in merges {
            let Some((left, right)) = parse_merge(merge) else {
                return Err(TokenizerError::new(format!(
                    "invalid BPE merge rule {merge:?}"
                )));
            };
            let merged = format!("{left}{right}");
            if token_to_id.contains_key(merged.as_str()) {
                parsed_merges.push(BpeMerge {
                    left: left.to_owned(),
                    right: right.to_owned(),
                    merged,
                });
            }
        }
        Ok(Self {
            token_to_id,
            merges: parsed_merges,
        })
    }
}

pub(super) fn decode(ids: &[usize], tokens: &[String]) -> Result<String, TokenizerError> {
    let mut bytes = Vec::new();
    for id in ids {
        let token = tokens
            .get(*id)
            .ok_or_else(|| TokenizerError::new(format!("token id {id} is out of bounds")))?;
        for value in token.chars() {
            bytes.push(unicode_to_byte(value)?);
        }
    }
    String::from_utf8(bytes).map_err(|error| {
        let message = format!("BPE decoded invalid UTF-8: {error}");
        if error.utf8_error().error_len().is_none() {
            TokenizerError::incomplete_utf8(message)
        } else {
            TokenizerError::new(message)
        }
    })
}

pub(super) fn encode_with_cancellation(
    input: &str,
    metadata: &BpeMetadata,
    mut on_cancellation_poll: impl FnMut() -> TokenizationControl,
) -> Result<Vec<usize>, TokenizerError> {
    if on_cancellation_poll() == TokenizationControl::Cancel {
        return Err(TokenizerError::cancelled());
    }
    let mut symbols = seed_symbols(input, &metadata.token_to_id, &mut on_cancellation_poll)?;

    for merge in &metadata.merges {
        if on_cancellation_poll() == TokenizationControl::Cancel {
            return Err(TokenizerError::cancelled());
        }
        apply_merge(&mut symbols, &merge.left, &merge.right, &merge.merged);
    }

    symbols
        .iter()
        .map(|symbol| {
            metadata
                .token_to_id
                .get(symbol.as_str())
                .copied()
                .ok_or_else(|| TokenizerError::new(format!("BPE token {symbol:?} is not in vocab")))
        })
        .collect()
}

fn seed_symbols(
    input: &str,
    token_to_id: &BTreeMap<String, usize>,
    on_cancellation_poll: &mut impl FnMut() -> TokenizationControl,
) -> Result<Vec<String>, TokenizerError> {
    let mut symbols = Vec::with_capacity(input.len());
    for (index, byte) in input.as_bytes().iter().enumerate() {
        if index % 1024 == 0 && on_cancellation_poll() == TokenizationControl::Cancel {
            return Err(TokenizerError::cancelled());
        }
        let symbol = byte_to_unicode(*byte)?.to_string();
        if token_to_id.contains_key(symbol.as_str()) {
            symbols.push(symbol);
        } else {
            return Err(TokenizerError::new(format!(
                "no BPE seed token matches {symbol:?}"
            )));
        }
    }
    Ok(symbols)
}

fn byte_to_unicode(byte: u8) -> Result<char, TokenizerError> {
    let code_point =
        if (33..=126).contains(&byte) || (161..=172).contains(&byte) || (174..=255).contains(&byte)
        {
            return Ok(byte as char);
        } else if byte <= 32 {
            u32::from(byte) + 256
        } else if byte <= 160 {
            u32::from(byte) + 162
        } else {
            323
        };

    char::from_u32(code_point).ok_or_else(|| {
        TokenizerError::new(format!(
            "GPT-2 byte mapping produced invalid code point {code_point}"
        ))
    })
}

fn unicode_to_byte(value: char) -> Result<u8, TokenizerError> {
    let code_point = value as u32;
    let byte = if (33..=126).contains(&code_point)
        || (161..=172).contains(&code_point)
        || (174..=255).contains(&code_point)
    {
        code_point
    } else if (256..=288).contains(&code_point) {
        code_point - 256
    } else if (289..=322).contains(&code_point) {
        code_point - 162
    } else if code_point == 323 {
        173
    } else {
        return Err(TokenizerError::new(format!(
            "BPE token character {value:?} is not in the GPT-2 byte alphabet"
        )));
    };

    u8::try_from(byte).map_err(|error| {
        TokenizerError::new(format!(
            "GPT-2 byte mapping produced invalid byte {byte}: {error}"
        ))
    })
}

fn parse_merge(merge: &str) -> Option<(&str, &str)> {
    let mut parts = merge.split(' ');
    let left = parts.next()?;
    let right = parts.next()?;
    if left.is_empty() || right.is_empty() || parts.next().is_some() {
        return None;
    }
    Some((left, right))
}

fn apply_merge(symbols: &mut Vec<String>, left: &str, right: &str, merged: &str) {
    let mut index = 0usize;
    while index + 1 < symbols.len() {
        if symbols[index] == left && symbols[index + 1] == right {
            symbols[index] = merged.to_owned();
            symbols.remove(index + 1);
        } else {
            index += 1;
        }
    }
}
