use super::RuntimeError;
use ferrite_inference::sampling::Sampler;
use ferrite_model::tokenizer::{GgufTokenizer, TokenType};

const MAX_JSON_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_JSON_NESTING_DEPTH: usize = 64;

#[derive(Debug)]
pub(super) struct JsonObjectConstraint {
    output: Vec<u8>,
    token_pieces: Vec<Option<Vec<u8>>>,
    allowed: Vec<bool>,
}

impl JsonObjectConstraint {
    pub(super) fn new(tokenizer: &GgufTokenizer) -> Result<Self, RuntimeError> {
        let mut token_pieces = Vec::with_capacity(tokenizer.len());
        for token_id in 0..tokenizer.len() {
            let piece = if tokenizer.is_end_of_generation_token(token_id) {
                None
            } else if matches!(
                tokenizer.token_type(token_id),
                Some(TokenType::Normal | TokenType::Byte)
            ) {
                let bytes = tokenizer.token_bytes(token_id).map_err(|error| {
                    RuntimeError::new(format!(
                        "failed to decode JSON grammar token {token_id}: {error}"
                    ))
                })?;
                (!bytes.is_empty()).then_some(bytes)
            } else {
                None
            };
            token_pieces.push(piece);
        }
        Ok(Self {
            output: Vec::new(),
            allowed: vec![false; tokenizer.len()],
            token_pieces,
        })
    }

    pub(super) fn select_token(
        &mut self,
        sampler: &mut Sampler,
        logits: &[f32],
    ) -> Result<usize, RuntimeError> {
        if logits.len() != self.token_pieces.len() {
            return Err(RuntimeError::new(format!(
                "JSON grammar vocabulary {} does not match logits {}",
                self.token_pieces.len(),
                logits.len()
            )));
        }
        let mut candidate = self.output.clone();
        for (token_id, piece) in self.token_pieces.iter().enumerate() {
            let Some(piece) = piece else {
                self.allowed[token_id] = false;
                continue;
            };
            candidate.truncate(self.output.len());
            if candidate.len().saturating_add(piece.len()) > MAX_JSON_OUTPUT_BYTES {
                self.allowed[token_id] = false;
                continue;
            }
            candidate.extend_from_slice(piece);
            self.allowed[token_id] =
                classify_json_object_prefix(&candidate) != PrefixStatus::Invalid;
        }
        let selected = sampler
            .sample_where(logits, |token_id| {
                self.allowed.get(token_id).copied().unwrap_or(false)
            })
            .map_err(|error| {
                RuntimeError::new(format!("failed to select JSON grammar token: {error}"))
            })?;
        let piece = self
            .token_pieces
            .get(selected)
            .and_then(Option::as_deref)
            .ok_or_else(|| RuntimeError::new("JSON grammar selected a token without text"))?;
        self.output.extend_from_slice(piece);
        if classify_json_object_prefix(&self.output) == PrefixStatus::Invalid {
            return Err(RuntimeError::new(
                "JSON grammar accepted a token that invalidated its output",
            ));
        }
        Ok(selected)
    }

    pub(super) fn is_complete(&self) -> bool {
        classify_json_object_prefix(&self.output) == PrefixStatus::Complete
    }

    #[cfg(test)]
    fn from_test_pieces(pieces: &[&[u8]]) -> Self {
        Self {
            output: Vec::new(),
            token_pieces: pieces.iter().map(|piece| Some((*piece).to_vec())).collect(),
            allowed: vec![false; pieces.len()],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrefixStatus {
    Invalid,
    Incomplete,
    Complete,
}

fn classify_json_object_prefix(bytes: &[u8]) -> PrefixStatus {
    if bytes.len() > MAX_JSON_OUTPUT_BYTES || !has_valid_utf8_prefix(bytes) {
        return PrefixStatus::Invalid;
    }
    let mut parser = PrefixParser::new(bytes);
    parser.skip_whitespace();
    if parser.is_at_end() {
        return PrefixStatus::Incomplete;
    }
    match parser.parse_object(1) {
        Ok(()) => {
            parser.skip_whitespace();
            if !parser.is_at_end() {
                return PrefixStatus::Invalid;
            }
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(serde_json::Value::Object(_)) => PrefixStatus::Complete,
                Ok(_) | Err(_) => PrefixStatus::Invalid,
            }
        }
        Err(ParseFailure::NeedMore) => PrefixStatus::Incomplete,
        Err(ParseFailure::Invalid) => PrefixStatus::Invalid,
    }
}

fn has_valid_utf8_prefix(bytes: &[u8]) -> bool {
    match std::str::from_utf8(bytes) {
        Ok(_) => true,
        Err(error) => error.error_len().is_none(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParseFailure {
    NeedMore,
    Invalid,
}

struct PrefixParser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> PrefixParser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn is_at_end(&self) -> bool {
        self.cursor == self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.cursor).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let value = self.peek()?;
        self.cursor += 1;
        Some(value)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.cursor += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), ParseFailure> {
        match self.next() {
            Some(actual) if actual == expected => Ok(()),
            Some(_) => Err(ParseFailure::Invalid),
            None => Err(ParseFailure::NeedMore),
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<(), ParseFailure> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(depth),
            Some(b'[') => self.parse_array(depth),
            Some(b'"') => self.parse_string(),
            Some(b't') => self.parse_literal(b"true"),
            Some(b'f') => self.parse_literal(b"false"),
            Some(b'n') => self.parse_literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(_) => Err(ParseFailure::Invalid),
            None => Err(ParseFailure::NeedMore),
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<(), ParseFailure> {
        if depth > MAX_JSON_NESTING_DEPTH {
            return Err(ParseFailure::Invalid);
        }
        self.expect(b'{')?;
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.cursor += 1;
            return Ok(());
        }
        loop {
            self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            self.parse_value(depth + 1)?;
            self.skip_whitespace();
            match self.next() {
                Some(b',') => {
                    self.skip_whitespace();
                }
                Some(b'}') => return Ok(()),
                Some(_) => return Err(ParseFailure::Invalid),
                None => return Err(ParseFailure::NeedMore),
            }
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<(), ParseFailure> {
        if depth > MAX_JSON_NESTING_DEPTH {
            return Err(ParseFailure::Invalid);
        }
        self.expect(b'[')?;
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.cursor += 1;
            return Ok(());
        }
        loop {
            self.parse_value(depth + 1)?;
            self.skip_whitespace();
            match self.next() {
                Some(b',') => {
                    self.skip_whitespace();
                }
                Some(b']') => return Ok(()),
                Some(_) => return Err(ParseFailure::Invalid),
                None => return Err(ParseFailure::NeedMore),
            }
        }
    }

    fn parse_string(&mut self) -> Result<(), ParseFailure> {
        self.expect(b'"')?;
        loop {
            match self.next() {
                Some(b'"') => return Ok(()),
                Some(b'\\') => match self.next() {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {}
                    Some(b'u') => {
                        for _ in 0..4 {
                            match self.next() {
                                Some(value) if value.is_ascii_hexdigit() => {}
                                Some(_) => return Err(ParseFailure::Invalid),
                                None => return Err(ParseFailure::NeedMore),
                            }
                        }
                    }
                    Some(_) => return Err(ParseFailure::Invalid),
                    None => return Err(ParseFailure::NeedMore),
                },
                Some(0..=0x1f) => return Err(ParseFailure::Invalid),
                Some(_) => {}
                None => return Err(ParseFailure::NeedMore),
            }
        }
    }

    fn parse_literal(&mut self, literal: &[u8]) -> Result<(), ParseFailure> {
        for expected in literal {
            match self.next() {
                Some(actual) if actual == *expected => {}
                Some(_) => return Err(ParseFailure::Invalid),
                None => return Err(ParseFailure::NeedMore),
            }
        }
        Ok(())
    }

    fn parse_number(&mut self) -> Result<(), ParseFailure> {
        if self.peek() == Some(b'-') {
            self.cursor += 1;
            if self.is_at_end() {
                return Err(ParseFailure::NeedMore);
            }
        }
        match self.peek() {
            Some(b'0') => {
                self.cursor += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(ParseFailure::Invalid);
                }
            }
            Some(b'1'..=b'9') => {
                self.cursor += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.cursor += 1;
                }
            }
            Some(_) => return Err(ParseFailure::Invalid),
            None => return Err(ParseFailure::NeedMore),
        }
        if self.peek() == Some(b'.') {
            self.cursor += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return if self.is_at_end() {
                    Err(ParseFailure::NeedMore)
                } else {
                    Err(ParseFailure::Invalid)
                };
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.cursor += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.cursor += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.cursor += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return if self.is_at_end() {
                    Err(ParseFailure::NeedMore)
                } else {
                    Err(ParseFailure::Invalid)
                };
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.cursor += 1;
            }
        }
        match self.peek() {
            None | Some(b' ' | b'\n' | b'\r' | b'\t' | b',' | b'}' | b']') => Ok(()),
            Some(_) => Err(ParseFailure::Invalid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrite_inference::sampling::SamplingConfig;

    #[test]
    fn accepts_only_complete_json_objects() {
        for prefix in [
            "",
            " ",
            "{",
            "{\"name\"",
            "{\"name\":",
            "{\"name\":[true,null,-1.2e+3]",
            "{\"unicode\":\"é",
        ] {
            assert_eq!(
                classify_json_object_prefix(prefix.as_bytes()),
                PrefixStatus::Incomplete,
                "prefix {prefix:?}"
            );
        }
        for complete in [
            "{}",
            " {\"name\":\"Ferrite\"} ",
            "{\"values\":[true,false,null,-1.2e+3]}",
            "{\"nested\":{\"escaped\":\"\\u263a\"}}",
        ] {
            assert_eq!(
                classify_json_object_prefix(complete.as_bytes()),
                PrefixStatus::Complete,
                "object {complete:?}"
            );
        }
    }

    #[test]
    fn rejects_invalid_or_non_object_json() {
        for invalid in [
            "[]",
            "null",
            "{]",
            "{\"x\":01}",
            "{\"x\":1.}",
            "{\"x\":truX}",
            "{\"x\":\"\\q\"}",
            "{} trailing",
        ] {
            assert_eq!(
                classify_json_object_prefix(invalid.as_bytes()),
                PrefixStatus::Invalid,
                "input {invalid:?}"
            );
        }
    }

    #[test]
    fn rejects_nesting_beyond_the_bound() {
        let input = format!("{{\"x\":{}}}", "[".repeat(MAX_JSON_NESTING_DEPTH + 1));
        assert_eq!(
            classify_json_object_prefix(input.as_bytes()),
            PrefixStatus::Invalid
        );
    }

    #[test]
    fn every_byte_prefix_of_valid_objects_remains_viable() {
        for object in [
            "{}",
            " {\"name\":\"Ferrite\",\"unicode\":\"aé水\"} ",
            "{\"values\":[true,false,null,-1.2e+3,0,42]}",
            "{\"nested\":{\"array\":[{\"escaped\":\"\\u263a\"}]}}",
        ] {
            for end in 0..object.len() {
                assert_ne!(
                    classify_json_object_prefix(&object.as_bytes()[..end]),
                    PrefixStatus::Invalid,
                    "valid object {object:?} has invalid byte prefix ending at {end}"
                );
            }
            assert_eq!(
                classify_json_object_prefix(object.as_bytes()),
                PrefixStatus::Complete,
                "object {object:?}"
            );
        }
    }

    #[test]
    fn nesting_limit_accepts_boundary_and_rejects_next_level() {
        let arrays_at_limit = MAX_JSON_NESTING_DEPTH - 1;
        let boundary = format!(
            "{{\"x\":{}0{}}}",
            "[".repeat(arrays_at_limit),
            "]".repeat(arrays_at_limit)
        );
        assert_eq!(
            classify_json_object_prefix(boundary.as_bytes()),
            PrefixStatus::Complete
        );

        let beyond = format!(
            "{{\"x\":{}0{}}}",
            "[".repeat(arrays_at_limit + 1),
            "]".repeat(arrays_at_limit + 1)
        );
        assert_eq!(
            classify_json_object_prefix(beyond.as_bytes()),
            PrefixStatus::Invalid
        );
    }

    #[test]
    fn constraint_masks_invalid_high_logit_tokens_until_object_completion()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut constraint = JsonObjectConstraint::from_test_pieces(&[b"garbage", b"{", b"}"]);
        let mut sampler = Sampler::new(SamplingConfig::greedy())?;

        assert_eq!(
            constraint.select_token(&mut sampler, &[100.0, 10.0, 1.0])?,
            1
        );
        assert!(!constraint.is_complete());
        assert_eq!(
            constraint.select_token(&mut sampler, &[100.0, 10.0, 1.0])?,
            2
        );
        assert!(constraint.is_complete());
        Ok(())
    }
}
