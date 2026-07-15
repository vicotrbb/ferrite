use super::{TokenType, TokenizationControl, TokenizerError};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};

const ESCAPED_SPACE: &str = "\u{2581}";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SpmMetadata {
    token_to_id: BTreeMap<String, usize>,
    score_bits: Vec<u32>,
    byte_to_token: Vec<Option<usize>>,
}

impl SpmMetadata {
    pub(super) fn new(
        tokens: &[String],
        token_types: &[TokenType],
        scores: &[f32],
    ) -> Result<Self, TokenizerError> {
        if scores.len() != tokens.len() {
            return Err(TokenizerError::new(format!(
                "tokenizer score length {} does not match tokens length {}",
                scores.len(),
                tokens.len()
            )));
        }
        if token_types.len() != tokens.len() {
            return Err(TokenizerError::new(format!(
                "token type length {} does not match tokens length {}",
                token_types.len(),
                tokens.len()
            )));
        }

        let mut token_to_id = BTreeMap::new();
        let mut byte_to_token = vec![None; 256];
        for (id, token) in tokens.iter().enumerate() {
            token_to_id.entry(token.clone()).or_insert(id);
            if token_types[id] == TokenType::Byte {
                let byte = parse_byte_token(token).ok_or_else(|| {
                    TokenizerError::new(format!(
                        "SentencePiece byte token {token:?} does not use <0xXX> syntax"
                    ))
                })?;
                let slot = &mut byte_to_token[usize::from(byte)];
                if slot.replace(id).is_some() {
                    return Err(TokenizerError::new(format!(
                        "SentencePiece vocabulary maps byte 0x{byte:02X} more than once"
                    )));
                }
            }
        }

        Ok(Self {
            token_to_id,
            score_bits: scores.iter().map(|score| score.to_bits()).collect(),
            byte_to_token,
        })
    }

    fn score(&self, token_id: usize) -> f32 {
        f32::from_bits(self.score_bits[token_id])
    }
}

pub(super) fn encode_ordinary_with_cancellation(
    input: &str,
    metadata: &SpmMetadata,
    mut on_cancellation_poll: impl FnMut() -> TokenizationControl,
) -> Result<Vec<usize>, TokenizerError> {
    if on_cancellation_poll() == TokenizationControl::Cancel {
        return Err(TokenizerError::cancelled());
    }
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let normalized = input.replace(' ', ESCAPED_SPACE);
    let mut symbols = seed_symbols(&normalized, &mut on_cancellation_poll)?;
    let mut work_queue = BinaryHeap::new();
    let mut reverse_merges = BTreeMap::new();
    for right in 1..symbols.len() {
        try_add_bigram(
            &normalized,
            &symbols,
            right - 1,
            right,
            metadata,
            &mut work_queue,
            &mut reverse_merges,
        );
    }

    while let Some(bigram) = work_queue.pop() {
        if on_cancellation_poll() == TokenizationControl::Cancel {
            return Err(TokenizerError::cancelled());
        }
        let left_len = symbols[bigram.left].len;
        let right_len = symbols[bigram.right].len;
        if left_len == 0 || right_len == 0 || left_len + right_len != bigram.size {
            continue;
        }

        let left_previous = symbols[bigram.left].previous;
        let right_next = symbols[bigram.right].next;
        symbols[bigram.left].len += right_len;
        symbols[bigram.left].next = right_next;
        symbols[bigram.right].len = 0;
        symbols[bigram.right].previous = None;
        symbols[bigram.right].next = None;
        if let Some(next) = right_next {
            symbols[next].previous = Some(bigram.left);
        }

        if let Some(previous) = left_previous {
            try_add_bigram(
                &normalized,
                &symbols,
                previous,
                bigram.left,
                metadata,
                &mut work_queue,
                &mut reverse_merges,
            );
        }
        if let Some(next) = right_next {
            try_add_bigram(
                &normalized,
                &symbols,
                bigram.left,
                next,
                metadata,
                &mut work_queue,
                &mut reverse_merges,
            );
        }
    }

    let mut output = Vec::new();
    let mut current = Some(0usize);
    while let Some(index) = current {
        let symbol = &symbols[index];
        let text = &normalized[symbol.start..symbol.start + symbol.len];
        resegment(text, metadata, &reverse_merges, &mut output)?;
        current = symbol.next;
    }
    Ok(output)
}

pub(super) fn decode_token_bytes(
    token_id: usize,
    tokens: &[String],
    token_types: &[TokenType],
) -> Result<Vec<u8>, TokenizerError> {
    let token = tokens
        .get(token_id)
        .ok_or_else(|| TokenizerError::new(format!("token id {token_id} is out of bounds")))?;
    let token_type = token_types
        .get(token_id)
        .copied()
        .ok_or_else(|| TokenizerError::new(format!("token id {token_id} has no token type")))?;
    match token_type {
        TokenType::Normal => Ok(token.replace(ESCAPED_SPACE, " ").into_bytes()),
        TokenType::Byte => parse_byte_token(token).map_or_else(
            || {
                Err(TokenizerError::new(format!(
                    "SentencePiece byte token {token:?} does not use <0xXX> syntax"
                )))
            },
            |byte| Ok(vec![byte]),
        ),
        _ => Ok(token.as_bytes().to_vec()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Symbol {
    start: usize,
    len: usize,
    previous: Option<usize>,
    next: Option<usize>,
}

fn seed_symbols(
    input: &str,
    on_cancellation_poll: &mut impl FnMut() -> TokenizationControl,
) -> Result<Vec<Symbol>, TokenizerError> {
    let mut symbols = Vec::with_capacity(input.chars().count());
    for (index, (start, character)) in input.char_indices().enumerate() {
        if index % 1024 == 0 && on_cancellation_poll() == TokenizationControl::Cancel {
            return Err(TokenizerError::cancelled());
        }
        symbols.push(Symbol {
            start,
            len: character.len_utf8(),
            previous: index.checked_sub(1),
            next: None,
        });
        if index > 0 {
            symbols[index - 1].next = Some(index);
        }
    }
    Ok(symbols)
}

#[derive(Clone, Debug)]
struct Bigram {
    left: usize,
    right: usize,
    score: f32,
    size: usize,
}

impl PartialEq for Bigram {
    fn eq(&self, other: &Self) -> bool {
        self.left == other.left
            && self.right == other.right
            && self.score.to_bits() == other.score.to_bits()
            && self.size == other.size
    }
}

impl Eq for Bigram {}

impl PartialOrd for Bigram {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Bigram {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| other.left.cmp(&self.left))
            .then_with(|| other.right.cmp(&self.right))
            .then_with(|| other.size.cmp(&self.size))
    }
}

fn try_add_bigram(
    text: &str,
    symbols: &[Symbol],
    left: usize,
    right: usize,
    metadata: &SpmMetadata,
    work_queue: &mut BinaryHeap<Bigram>,
    reverse_merges: &mut BTreeMap<String, (String, String)>,
) {
    let left_symbol = symbols[left];
    let right_symbol = symbols[right];
    if left_symbol.len == 0 || right_symbol.len == 0 {
        return;
    }
    let size = left_symbol.len + right_symbol.len;
    let combined = &text[left_symbol.start..left_symbol.start + size];
    let Some(token_id) = metadata.token_to_id.get(combined).copied() else {
        return;
    };
    let left_text = &text[left_symbol.start..left_symbol.start + left_symbol.len];
    let right_text = &text[right_symbol.start..right_symbol.start + right_symbol.len];
    reverse_merges.insert(
        combined.to_owned(),
        (left_text.to_owned(), right_text.to_owned()),
    );
    work_queue.push(Bigram {
        left,
        right,
        score: metadata.score(token_id),
        size,
    });
}

fn resegment(
    text: &str,
    metadata: &SpmMetadata,
    reverse_merges: &BTreeMap<String, (String, String)>,
    output: &mut Vec<usize>,
) -> Result<(), TokenizerError> {
    if let Some(token_id) = metadata.token_to_id.get(text).copied() {
        output.push(token_id);
        return Ok(());
    }
    if let Some((left, right)) = reverse_merges.get(text) {
        resegment(left, metadata, reverse_merges, output)?;
        resegment(right, metadata, reverse_merges, output)?;
        return Ok(());
    }
    for byte in text.as_bytes() {
        let token_id = metadata.byte_to_token[usize::from(*byte)].ok_or_else(|| {
            TokenizerError::new(format!(
                "SentencePiece vocabulary has no fallback token for byte 0x{byte:02X}"
            ))
        })?;
        output.push(token_id);
    }
    Ok(())
}

fn parse_byte_token(token: &str) -> Option<u8> {
    let hex = token.strip_prefix("<0x")?.strip_suffix('>')?;
    if hex.len() != 2 {
        return None;
    }
    u8::from_str_radix(hex, 16).ok()
}
