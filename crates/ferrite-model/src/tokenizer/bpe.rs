use super::TokenizerError;
use std::collections::BTreeMap;

pub(super) fn encode(
    input: &str,
    tokens: &[String],
    merges: &[String],
) -> Result<Vec<usize>, TokenizerError> {
    let token_to_id = token_to_id(tokens);
    let mut symbols = seed_symbols(input, &token_to_id)?;

    for merge in merges {
        let Some((left, right)) = parse_merge(merge) else {
            return Err(TokenizerError::new(format!(
                "invalid BPE merge rule {merge:?}"
            )));
        };
        let merged = format!("{left}{right}");
        if !token_to_id.contains_key(merged.as_str()) {
            continue;
        }
        apply_merge(&mut symbols, left, right, &merged);
    }

    symbols
        .iter()
        .map(|symbol| {
            token_to_id
                .get(symbol.as_str())
                .copied()
                .ok_or_else(|| TokenizerError::new(format!("BPE token {symbol:?} is not in vocab")))
        })
        .collect()
}

fn token_to_id(tokens: &[String]) -> BTreeMap<&str, usize> {
    tokens
        .iter()
        .enumerate()
        .map(|(id, token)| (token.as_str(), id))
        .collect()
}

fn seed_symbols(
    input: &str,
    token_to_id: &BTreeMap<&str, usize>,
) -> Result<Vec<String>, TokenizerError> {
    input
        .as_bytes()
        .iter()
        .map(|byte| {
            let symbol = byte_to_unicode(*byte)?.to_string();
            if token_to_id.contains_key(symbol.as_str()) {
                Ok(symbol)
            } else {
                Err(TokenizerError::new(format!(
                    "no BPE seed token matches {symbol:?}"
                )))
            }
        })
        .collect()
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
