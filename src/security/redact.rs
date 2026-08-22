const SENSITIVE_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "access_token",
    "auth_token",
    "password",
    "passwd",
    "database_url",
    "authorization",
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
    for key in SENSITIVE_KEYS {
        let Some(key_position) = lowercase.find(key) else {
            continue;
        };
        let after_key = &line[key_position + key.len()..];
        let Some(separator_offset) = after_key.find(['=', ':']) else {
            continue;
        };
        let value_start = key_position + key.len() + separator_offset + 1;
        return format!("{} [REDACTED]", line[..value_start].trim_end());
    }

    line.split_inclusive(char::is_whitespace)
        .map(|segment| {
            let token = segment.trim_end();
            let whitespace = &segment[token.len()..];
            if token.starts_with("sk-") && token.len() > 12 {
                format!("[REDACTED]{whitespace}")
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
        let input =
            "Authorization: Bearer secret-token\nAPI_KEY=abc123\nerror uses sk-123456789012345";
        let redacted = redact_sensitive(input);
        assert!(!redacted.contains("secret-token"));
        assert!(!redacted.contains("abc123"));
        assert!(!redacted.contains("sk-123456789012345"));
        assert!(redacted.contains("[REDACTED]"));
    }
}
