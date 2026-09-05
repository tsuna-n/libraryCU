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

#[test]
fn provider_errors_and_answers_do_not_echo_credentials() {
    let payload = r#"{"error":{"message":"invalid API_KEY=fixture-secret"}}"#;
    let error =
        librarycube::ai::provider::parse_chat_response(payload, "fixture-model").unwrap_err();
    assert!(!error.to_string().contains("fixture-secret"));
    let payload = r#"{"choices":[{"message":{"content":"Your password=fixture-secret"}}]}"#;
    let response =
        librarycube::ai::provider::parse_chat_response(payload, "fixture-model").unwrap();
    assert!(!response.content.contains("fixture-secret"));
}

#[test]
fn large_chat_history_cannot_displace_retrieved_knowledge() {
    use librarycube::answer::{AnswerPassage, AnswerReport, build_ai_request};
    let report = AnswerReport {
        question: "ก".repeat(50_000),
        language: "th".into(),
        answer_status: "retrieved_guidance".into(),
        passages: vec![AnswerPassage {
            source_id: "user:port-note".into(),
            title: "Port note".into(),
            source_locator: "fixture.md".into(),
            excerpt: "Use UNIQUE-RECOVERY-STEP then verify port 4429".into(),
            match_reason: "title match".into(),
            score: 100,
            verification_status: "unverified".into(),
        }],
        offline_answer: String::new(),
        project_evidence: vec![],
        warnings: vec![],
        ai: None,
        ai_error: None,
    };
    let history = vec!["ก".repeat(50_000); 20];
    let request = build_ai_request(&report, "mock", &history);
    assert!(request.user.chars().count() <= librarycube::ai::prompt::MAX_PROMPT_CHARS);
    assert!(request.user.contains("user:port-note"));
    assert!(request.user.contains("UNIQUE-RECOVERY-STEP"));
    assert!(request.user.contains("[truncated]"));
}
