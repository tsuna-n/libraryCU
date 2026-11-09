use std::{path::Path, process::Command};

fn command(home: &Path) -> Command {
    let binary =
        std::env::var_os("LBC_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_lbc").into());
    let mut command = Command::new(binary);
    command
        .current_dir(home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"));
    for key in [
        "LBC_CONFIG",
        "OPENAI_API_KEY",
        "OPENROUTER_API_KEY",
        "ZAI_API_KEY",
        "GLM_API_KEY",
        "OLLAMA_API_KEY",
    ] {
        command.env_remove(key);
    }
    command
}

fn json(home: &Path, args: &[&str]) -> serde_json::Value {
    let output = command(home)
        .args(args)
        .arg("--project")
        .arg(home)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn install(home: &Path) {
    for package in [
        "packages/python-fastapi-basics",
        "benchmarks/retrieval/corpus",
    ] {
        let output = command(home)
            .args(["knowledge", "install"])
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(package))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn sources(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["source_id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn structured_logs_use_the_same_evidence_in_search_ask_and_explain() {
    let home = tempfile::tempdir().unwrap();
    install(home.path());
    for (input, expected) in [
        (
            "go build ./internal/worker\ninternal/worker/job.go:17:2: undefined: missing",
            "package:retrieval-benchmark:go-undefined",
        ),
        (
            "build setup complete\napp/view.ts:3:9 - error TS2307: cannot find module 'widget'",
            "package:retrieval-benchmark:ts2307",
        ),
        (
            "Traceback (most recent call last):\n  File \"jobs.py\", line 21\njson.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)",
            "package:python-fastapi-basics:python-json-decode-error",
        ),
        ("Node หาไฟล์ไม่เจอ", "package:retrieval-benchmark:node-enoent"),
    ] {
        std::fs::write(home.path().join("input.log"), input).unwrap();
        let search = json(home.path(), &["search", input]);
        let answer = json(home.path(), &["ask", input]);
        let explanation = json(home.path(), &["explain", "input.log"]);
        assert_eq!(sources(&search), [expected], "{input}");
        assert_eq!(sources(&answer["passages"]), sources(&search), "{input}");
        assert_eq!(
            sources(&explanation["knowledge"]),
            sources(&search),
            "{input}"
        );
        assert_eq!(answer["answer_status"], "retrieved_guidance");
    }
}

#[test]
fn known_exception_keeps_specific_support_but_unknown_exception_does_not_borrow_it() {
    let home = tempfile::tempdir().unwrap();
    install(home.path());
    let input = "Traceback (most recent call last):\n  File \"startup.py\", line 42\nImportError: cannot import name from partially initialized module";
    let search = json(home.path(), &["search", input]);
    assert!(
        sources(&search)
            .contains(&"package:python-fastapi-basics:python-circular-import".to_owned())
    );
    let unknown = input.replace("ImportError", "UnrecordedCustomError");
    std::fs::write(home.path().join("input.log"), &unknown).unwrap();
    assert!(sources(&json(home.path(), &["search", &unknown])).is_empty());
    let answer = json(home.path(), &["ask", &unknown]);
    assert_eq!(answer["answer_status"], "general_guidance");
    assert_eq!(
        sources(&answer["passages"]),
        ["package:python-fastapi-basics:python-traceback-playbook"]
    );
    assert!(sources(&json(home.path(), &["explain", "input.log"])["knowledge"]).is_empty());
}

#[test]
fn explain_keeps_large_log_support_and_search_ask_keep_their_query_limit() {
    let home = tempfile::tempdir().unwrap();
    let input = format!(
        "error[E0382]: borrow of moved value\n --> app.rs:2:3\n{}",
        "build context unrelated to the diagnostic\n".repeat(1000)
    );
    std::fs::write(home.path().join("input.log"), &input).unwrap();
    let report = json(home.path(), &["explain", "input.log"]);
    assert!(sources(&report["knowledge"]).contains(&"builtin:rust-e0382".to_owned()));
    for name in ["search", "ask"] {
        let output = command(home.path())
            .args([name, &input, "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("8 KB"));
    }
}

#[test]
fn search_and_ask_keep_multiple_diagnostics_while_explain_reports_the_primary() {
    let home = tempfile::tempdir().unwrap();
    let input = "error[E0308]: mismatched types\n --> a.rs:2:3\nerror[E0382]: borrow of moved value\n --> b.rs:8:4";
    std::fs::write(home.path().join("input.log"), input).unwrap();
    let search = json(home.path(), &["search", input]);
    let answer = json(home.path(), &["ask", input]);
    let ids = sources(&search);
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"builtin:rust-e0308".to_owned()));
    assert!(ids.contains(&"builtin:rust-e0382".to_owned()));
    assert_eq!(sources(&answer["passages"]), ids);
    let explanation = json(home.path(), &["explain", "input.log"]);
    assert_eq!(explanation["diagnostics_detected"], 2);
    assert_eq!(sources(&explanation["knowledge"]), ["builtin:rust-e0308"]);
}
