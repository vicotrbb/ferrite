use super::{TokenizationControl, TokenizerError};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BpeMetadata {
    token_to_id: BTreeMap<String, usize>,
    pair_merges: BTreeMap<String, BTreeMap<String, RankedBpeMerge>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RankedBpeMerge {
    rank: usize,
    merged: String,
}

impl BpeMetadata {
    pub(super) fn new(tokens: &[String], merges: &[String]) -> Result<Self, TokenizerError> {
        let token_to_id = tokens
            .iter()
            .enumerate()
            .map(|(id, token)| (token.clone(), id))
            .collect::<BTreeMap<_, _>>();
        let mut pair_merges: BTreeMap<String, BTreeMap<String, RankedBpeMerge>> = BTreeMap::new();
        for (rank, merge) in merges.iter().enumerate() {
            let Some((left, right)) = parse_merge(merge) else {
                return Err(TokenizerError::new(format!(
                    "invalid BPE merge rule {merge:?}"
                )));
            };
            let merged = format!("{left}{right}");
            if token_to_id.contains_key(merged.as_str()) {
                pair_merges
                    .entry(left.to_owned())
                    .or_default()
                    .entry(right.to_owned())
                    .or_insert(RankedBpeMerge { rank, merged });
            }
        }
        Ok(Self {
            token_to_id,
            pair_merges,
        })
    }
}

pub(super) fn decode(ids: &[usize], tokens: &[String]) -> Result<String, TokenizerError> {
    let bytes = decode_bytes(ids, tokens)?;
    String::from_utf8(bytes).map_err(|error| {
        let message = format!("BPE decoded invalid UTF-8: {error}");
        if error.utf8_error().error_len().is_none() {
            TokenizerError::incomplete_utf8(message)
        } else {
            TokenizerError::new(message)
        }
    })
}

pub(super) fn decode_bytes(ids: &[usize], tokens: &[String]) -> Result<Vec<u8>, TokenizerError> {
    let mut bytes = Vec::new();
    for id in ids {
        let token = tokens
            .get(*id)
            .ok_or_else(|| TokenizerError::new(format!("token id {id} is out of bounds")))?;
        for value in token.chars() {
            bytes.push(unicode_to_byte(value)?);
        }
    }
    Ok(bytes)
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

    while let Some(candidate) = best_active_merge(&symbols, &metadata.pair_merges) {
        if on_cancellation_poll() == TokenizationControl::Cancel {
            return Err(TokenizerError::cancelled());
        }
        apply_merge(
            &mut symbols,
            &candidate.left,
            &candidate.right,
            &candidate.merged,
        );
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveMerge {
    rank: usize,
    left: String,
    right: String,
    merged: String,
}

fn best_active_merge(
    symbols: &[String],
    pair_merges: &BTreeMap<String, BTreeMap<String, RankedBpeMerge>>,
) -> Option<ActiveMerge> {
    let mut best = None;
    for pair in symbols.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        let Some(ranked) = pair_merges
            .get(left.as_str())
            .and_then(|right_merges| right_merges.get(right.as_str()))
        else {
            continue;
        };

        if best
            .as_ref()
            .is_none_or(|current: &ActiveMerge| ranked.rank < current.rank)
        {
            best = Some(ActiveMerge {
                rank: ranked.rank,
                left: left.clone(),
                right: right.clone(),
                merged: ranked.merged.clone(),
            });
        }
    }
    best
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
