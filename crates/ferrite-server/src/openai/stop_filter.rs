pub(super) fn apply_stop_sequences(
    generated: crate::runtime::GeneratedText,
    stop_sequences: &[String],
) -> crate::runtime::GeneratedText {
    let Some(stop_index) = first_stop_index(generated.text(), stop_sequences) else {
        return generated;
    };
    let text = generated.text()[..stop_index].to_owned();
    let token_texts = if text.is_empty() {
        Vec::new()
    } else {
        vec![text.clone()]
    };
    crate::runtime::GeneratedText::with_finish_reason(
        text,
        generated.prompt_tokens(),
        generated.completion_tokens(),
        token_texts,
        crate::runtime::GenerationFinishReason::Stop,
    )
}

fn first_stop_index(text: &str, stop_sequences: &[String]) -> Option<usize> {
    stop_sequences
        .iter()
        .filter(|stop| !stop.is_empty())
        .filter_map(|stop| text.find(stop))
        .min()
}

pub(super) struct StopSequenceFilter {
    stop_sequences: Vec<String>,
    pending: String,
    stopped: bool,
}

impl StopSequenceFilter {
    pub(super) fn new(stop_sequences: Vec<String>) -> Self {
        Self {
            stop_sequences,
            pending: String::new(),
            stopped: false,
        }
    }

    pub(super) fn push(&mut self, piece: &str) -> Vec<String> {
        if self.stopped {
            return Vec::new();
        }
        if self.stop_sequences.is_empty() {
            return vec![piece.to_owned()];
        }

        self.pending.push_str(piece);
        if let Some(stop_index) = first_stop_index(&self.pending, &self.stop_sequences) {
            let visible = self.pending[..stop_index].to_owned();
            self.pending.clear();
            self.stopped = true;
            if visible.is_empty() {
                Vec::new()
            } else {
                vec![visible]
            }
        } else {
            let retained = stop_prefix_suffix_len(&self.pending, &self.stop_sequences);
            if retained == self.pending.len() {
                return Vec::new();
            }
            let split_index = self.pending.len() - retained;
            let visible = self.pending[..split_index].to_owned();
            self.pending = self.pending[split_index..].to_owned();
            vec![visible]
        }
    }

    pub(super) fn stopped(&self) -> bool {
        self.stopped
    }

    pub(super) fn finish(self) -> Vec<String> {
        if self.stopped || self.pending.is_empty() {
            Vec::new()
        } else {
            vec![self.pending]
        }
    }
}

fn stop_prefix_suffix_len(text: &str, stop_sequences: &[String]) -> usize {
    let mut longest = 0;
    for stop in stop_sequences.iter().filter(|stop| !stop.is_empty()) {
        for prefix_len in stop_prefix_lens(stop) {
            if text.ends_with(&stop[..prefix_len]) {
                longest = longest.max(prefix_len);
            }
        }
    }
    longest
}

fn stop_prefix_lens(stop: &str) -> impl Iterator<Item = usize> + '_ {
    stop.char_indices()
        .map(|(index, character)| index + character.len_utf8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_prefix_suffix_len_retains_only_possible_stop_prefix() {
        assert_eq!(stop_prefix_suffix_len("winner", &[String::from("zzz")]), 0);
        assert_eq!(
            stop_prefix_suffix_len("winner n", &[String::from("ner")]),
            1
        );
        assert_eq!(
            stop_prefix_suffix_len("hello μ", &[String::from("μ-stop")]),
            "μ".len()
        );
    }
}
