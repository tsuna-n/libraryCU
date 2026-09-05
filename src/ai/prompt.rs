use crate::security::redact_sensitive;

/// Total user-context budget, including headings, identifiers and history.
pub const MAX_PROMPT_CHARS: usize = 32_000;
const TRUNCATED: &str = "\n... [truncated]";

pub(crate) struct Prompt {
    text: String,
    remaining: usize,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            remaining: MAX_PROMPT_CHARS,
        }
    }

    pub fn push(&mut self, text: &str, budget: usize) {
        let text = bounded_redacted(text, budget.min(self.remaining));
        self.remaining -= text.chars().count();
        self.text.push_str(&text);
    }

    pub fn finish(self) -> String {
        self.text
    }
}

/// Redact first: truncation must not turn a credential into an unrecognized
/// prefix, or remove a private-key marker before it reaches the filter.
pub(crate) fn bounded_redacted(text: &str, budget: usize) -> String {
    let redacted = redact_sensitive(text);
    if redacted.chars().count() <= budget {
        return redacted;
    }
    let suffix_len = TRUNCATED.chars().count();
    if budget < suffix_len {
        return TRUNCATED.chars().take(budget).collect();
    }
    let mut result: String = redacted.chars().take(budget - suffix_len).collect();
    result.push_str(TRUNCATED);
    result
}
