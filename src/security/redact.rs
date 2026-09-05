const SENSITIVE_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "access_token",
    "auth_token",
    "secret_key",
    "secret_token",
    "client_secret",
    "private_key",
    "password",
    "passwd",
    "database_url",
    "authorization",
    "token",
    "secret",
];

/// Mask recognizable credentials while preserving line count and UTF-8 boundaries.
/// Run on complete input before extracting passages or truncating context.
pub fn redact_sensitive(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_private_key = false;
    for line in input.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let ending = &line[body.len()..];
        // ASCII folding leaves byte offsets unchanged for arbitrary Unicode.
        let lowercase = body.to_ascii_lowercase();
        if lowercase.contains("-----begin") && lowercase.contains("private key-----") {
            inside_private_key = true;
        }
        if inside_private_key {
            output.push_str("[REDACTED PRIVATE KEY]");
            if lowercase.contains("-----end") && lowercase.contains("private key-----") {
                inside_private_key = false;
            }
        } else {
            output.push_str(&redact_line(body));
        }
        output.push_str(ending);
    }
    output
}

fn redact_line(line: &str) -> String {
    let mut spans = Vec::new();
    // URI user information is sensitive even without a DATABASE_URL label.
    for (offset, _) in line.match_indices("://") {
        let start = offset + 3;
        let rest = &line[start..];
        let end = rest
            .find(|ch: char| ch.is_whitespace() || matches!(ch, '/' | '?' | '#' | '\'' | '"' | '`'))
            .unwrap_or(rest.len());
        if let Some(at) = rest[..end].rfind('@')
            && &rest[..at] != "[REDACTED]"
        {
            spans.push((start, start + at));
        }
    }
    let mut cursor = 0;
    while cursor < line.len() {
        let character = line[cursor..].chars().next().expect("character boundary");
        if !character.is_ascii_alphanumeric() && character != '_' {
            cursor += character.len_utf8();
            continue;
        }
        let start = cursor;
        while cursor < line.len() && identifier_byte(line.as_bytes()[cursor]) {
            cursor += 1;
        }
        let word = &line[start..cursor];
        let key = word.to_ascii_lowercase().replace('-', "_");
        if is_sensitive_key(&key) {
            let mut value_start = cursor;
            // Quoted keys in JSON, YAML, TOML, and source object literals.
            if matches!(line.as_bytes().get(value_start), Some(b'\'' | b'"')) {
                value_start += 1;
            }
            value_start = skip_space(line, value_start);
            if matches!(line.as_bytes().get(value_start), Some(b'=' | b':')) {
                value_start = skip_space(line, value_start + 1);
                if let Some(span) = value_span(line, value_start) {
                    spans.push(span);
                }
            }
        }
        if key == "bearer" || key == "basic" {
            let value_start = skip_space(line, cursor);
            if value_start > cursor
                && let Some(span) = value_span(line, value_start)
            {
                spans.push(span);
            }
        }
        if ["sk-", "ghp_", "github_pat_", "glpat-", "xoxb-", "xoxp-"]
            .iter()
            .any(|prefix| word.starts_with(prefix))
            && word.len() > 12
        {
            spans.push((start, cursor));
        }
    }

    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in spans {
        if let Some(previous) = merged.last_mut()
            && start <= previous.1
        {
            previous.1 = previous.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    let mut output = String::with_capacity(line.len());
    let mut copied = 0;
    for (start, end) in merged {
        output.push_str(&line[copied..start]);
        output.push_str("[REDACTED]");
        copied = end;
    }
    output.push_str(&line[copied..]);
    output
}

fn identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

fn is_sensitive_key(key: &str) -> bool {
    SENSITIVE_KEYS.contains(&key)
        || SENSITIVE_KEYS.iter().any(|suffix| {
            key.strip_suffix(suffix)
                .is_some_and(|prefix| prefix.ends_with('_'))
        })
}

fn skip_space(line: &str, start: usize) -> usize {
    start
        + line[start..]
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>()
}

fn value_span(line: &str, start: usize) -> Option<(usize, usize)> {
    let rest = &line[start..];
    if rest.is_empty() || rest.starts_with("[REDACTED]") {
        return None;
    }
    if let Some(quote @ ('\'' | '"')) = rest.chars().next() {
        let mut escaped = false;
        for (offset, character) in rest[1..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if character == quote {
                let end = start + 1 + offset;
                return (end > start + 1 && &line[start + 1..end] != "[REDACTED]")
                    .then_some((start + 1, end));
            }
        }
        return Some((start + 1, line.len()));
    }
    // Authorization includes a scheme followed by its credential.
    if rest
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("bearer "))
        || rest
            .get(..6)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("basic "))
    {
        return value_span(line, skip_space(line, start + rest.find(' ').unwrap_or(0)));
    }
    let end = rest
        .char_indices()
        .find(|(_, ch)| {
            ch.is_whitespace() || matches!(ch, ',' | ';' | '}' | ']' | '\'' | '"' | '`')
        })
        .map_or(line.len(), |(offset, _)| start + offset);
    (end > start).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacts_common_secret_forms() {
        let input = "Authorization: Bearer secret-token\nAPI_KEY=abc123\nerror uses sk-123456789012345\nGITHUB_TOKEN=ghp_1234567890abcdef\nSECRET_KEY=supersecret123";
        let redacted = redact_sensitive(input);
        for value in [
            "secret-token",
            "abc123",
            "sk-123456789012345",
            "ghp_1234567890abcdef",
            "supersecret123",
        ] {
            assert!(!redacted.contains(value));
        }
        assert!(redacted.contains("[REDACTED]"));
    }
}
