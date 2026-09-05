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
    "bearer",
    "token",
    "secret",
];

pub fn redact_sensitive(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_private_key = false;
    for line in input.lines() {
        let lowercase = line.to_lowercase();
        if lowercase.contains("-----begin") && lowercase.contains("private key-----") {
            inside_private_key = true;
            output.push_str("[REDACTED PRIVATE KEY]\n");
            continue;
        }
        if inside_private_key {
            if lowercase.contains("-----end") && lowercase.contains("private key-----") {
                inside_private_key = false;
            }
            continue;
        }
        output.push_str(&redact_line(line, &lowercase));
        output.push('\n');
    }
    if !input.ends_with('\n') {
        output.pop();
    }
    output
}

fn redact_line(line: &str, lowercase: &str) -> String {
    if let Some(position) = lowercase.find("bearer ") {
        let value_start = position + "bearer ".len();
        let value_end = line[value_start..]
            .find(char::is_whitespace)
            .map_or(line.len(), |offset| value_start + offset);
        if value_end > value_start {
            return format!("{}[REDACTED]{}", &line[..value_start], &line[value_end..]);
        }
    }
    for key in SENSITIVE_KEYS {
        let Some(key_position) = lowercase.find(key) else {
            continue;
        };
        // Boundary before key: must be start of string or preceded by non-alphanumeric
        if key_position > 0 {
            let prev = line.as_bytes()[key_position - 1];
            if prev.is_ascii_alphanumeric() {
                continue;
            }
        }
        let after_key = &line[key_position + key.len()..];
        let Some(separator_offset) = after_key.find(['=', ':']) else {
            continue;
        };
        // Boundary after key: only optional whitespace before '=' or ':'
        if !after_key[..separator_offset].trim().is_empty() {
            continue;
        }
        let value_start = key_position + key.len() + separator_offset + 1;
        return format!("{} [REDACTED]", line[..value_start].trim_end());
    }

    line.split_inclusive(|character: char| character.is_whitespace() || character == '=')
        .map(|segment| {
            let token = segment
                .trim_end_matches(|character: char| character.is_whitespace() || character == '=');
            let delimiter = &segment[token.len()..];
            let is_known_token = (token.starts_with("sk-")
                || token.starts_with("ghp_")
                || token.starts_with("glpat-")
                || token.starts_with("xoxb-")
                || token.starts_with("xoxp-"))
                && token.len() > 12;
            if is_known_token {
                format!("[REDACTED]{delimiter}")
            } else {
                segment.to_owned()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secret_forms() {
        let input = "Authorization: Bearer secret-token\nAPI_KEY=abc123\nerror uses sk-123456789012345\nGITHUB_TOKEN=ghp_1234567890abcdef\nSECRET_KEY=supersecret123";
        let redacted = redact_sensitive(input);
        assert!(!redacted.contains("secret-token"));
        assert!(!redacted.contains("abc123"));
        assert!(!redacted.contains("sk-123456789012345"));
        assert!(!redacted.contains("ghp_1234567890abcdef"));
        assert!(!redacted.contains("supersecret123"));
        assert!(redacted.contains("[REDACTED]"));
    }
}
