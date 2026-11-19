use librarycube::security::redact_sensitive;

#[test]
fn structured_and_multiple_secrets_are_all_redacted() {
    for (input, secrets) in [
        (
            r#"{"api_key":"json-key-value","password":"json-password-value"}"#,
            vec!["json-key-value", "json-password-value"],
        ),
        (
            "Authorization: Bearer first-token password=second-password",
            vec!["first-token", "second-password"],
        ),
        (
            "request: 'ghp_12345678901234567890' and (sk-12345678901234567890)",
            vec!["ghp_12345678901234567890", "sk-12345678901234567890"],
        ),
        (
            "password_hint is safe; password=actual-password",
            vec!["actual-password"],
        ),
        ("İİİ Bearer unicode-token", vec!["unicode-token"]),
        ("İİİ API_KEY=unicode-key", vec!["unicode-key"]),
        (
            "https://fixture-user:uri-password@example.invalid/v1",
            vec!["fixture-user", "uri-password"],
        ),
        (
            "base_url=https://password=uri-password@example.invalid/v1",
            vec!["uri-password"],
        ),
        (
            "database_url = postgres://user:db-password@localhost/db",
            vec!["db-password"],
        ),
    ] {
        let redacted = redact_sensitive(input);
        for secret in secrets {
            assert!(
                !redacted.contains(secret),
                "a fixture secret survived redaction"
            );
        }
        assert!(redacted.contains("[REDACTED]"));
        assert_eq!(
            redact_sensitive(&redacted),
            redacted,
            "redaction must be idempotent"
        );
    }
}

#[test]
fn unicode_prefixes_never_corrupt_offsets() {
    for prefix in ["İİİ", "KKK", "ภาษาไทย", "🔑", "é"] {
        let result = redact_sensitive(&format!("{prefix} Bearer test-token"));
        assert!(result.starts_with(prefix));
        assert!(!result.contains("test-token"));
    }
}

#[test]
fn private_key_redaction_keeps_source_line_numbers() {
    let text =
        "before\n-----BEGIN PRIVATE KEY-----\nprivate-material\n-----END PRIVATE KEY-----\nafter\n";
    let redacted = redact_sensitive(text);
    assert_eq!(redacted.lines().count(), text.lines().count());
    assert_eq!(redacted.lines().nth(4), Some("after"));
    assert!(!redacted.contains("private-material"));
}

#[test]
fn retrieved_passages_cannot_expose_an_interior_private_key_line() {
    let body = format!(
        "---\nid: credential-note\ntitle: Credential fixture\n---\n-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        "UNIQUE-PRIVATE-MATERIAL\n\n".repeat(30)
    );
    let document = librarycube::knowledge::loader::parse_document("fixture.md", &body).unwrap();
    let results = librarycube::knowledge::search(&[document], "PRIVATE MATERIAL");
    assert!(!results.is_empty());
    assert!(!results[0].excerpt.contains("UNIQUE-PRIVATE-MATERIAL"));
}
