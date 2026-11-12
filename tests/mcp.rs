use serde_json::{Value, json};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn session(root: &Path, messages: &[Value], project_only: bool) -> Vec<Value> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lbc"));
    command
        .args(["mcp", "--project"])
        .arg(root.join("project"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env_remove("LBC_CONFIG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if project_only {
        command.arg("--project-only");
    }
    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn initialize() -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"integration-test","version":"1"}
    }})
}
fn ready() -> Value {
    json!({"jsonrpc":"2.0","method":"notifications/initialized"})
}
fn call(id: u64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("project/.lbc/knowledge")).unwrap();
    std::fs::write(root.path().join("project/.lbc/knowledge/note.md"),
        "---\nid: local-note\ntitle: Widget build\nkeywords: [widget]\nverification_status: recorded-check\n---\n# Widget build\nHistorical widget guidance; password=fixture-private-secret\n").unwrap();
    root
}

#[test]
fn stdio_exposes_only_read_only_tools_and_reuses_local_retrieval() {
    let root = fixture();
    let messages = session(
        root.path(),
        &[
            initialize(),
            ready(),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            call(3, "search_knowledge", json!({"query":"widget"})),
            call(
                4,
                "inspect_knowledge",
                json!({"source_id":"project:local-note"}),
            ),
            call(
                5,
                "explain_error",
                json!({"text":"error[E0308]: mismatched types"}),
            ),
        ],
        false,
    );
    assert_eq!(messages.len(), 5);
    assert_eq!(messages[0]["result"]["protocolVersion"], "2025-11-25");
    let tools = messages[1]["result"]["tools"].as_array().unwrap();
    let names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["search_knowledge", "inspect_knowledge", "explain_error"]
    );
    assert!(
        tools
            .iter()
            .all(|t| t["annotations"]["readOnlyHint"] == true)
    );
    let search = &messages[2]["result"]["structuredContent"];
    assert_eq!(search["results"][0]["source_id"], "project:local-note");
    assert_eq!(search["verified_for_current_problem"], false);
    assert_eq!(
        messages[3]["result"]["structuredContent"]["document"]["verification_status"],
        "recorded-check"
    );
    assert_eq!(
        messages[4]["result"]["structuredContent"]["report"]["diagnostic"]["code"],
        "E0308"
    );
    assert!(
        !serde_json::to_string(&messages)
            .unwrap()
            .contains("fixture-private-secret")
    );
    assert!(!root.path().join("config").exists());
    assert!(!root.path().join("cache").exists());
}

#[test]
fn tool_arguments_cannot_widen_scope_read_paths_enable_ai_or_modify_source() {
    let root = fixture();
    let source = root.path().join("project/main.rs");
    std::fs::write(&source, "fn main() {}\n").unwrap();
    std::fs::write(root.path().join("private.log"), "outside-project-marker").unwrap();
    let messages = session(
        root.path(),
        &[
            initialize(),
            ready(),
            call(2, "fix", json!({"apply":true,"text":"replace main.rs"})),
            call(
                3,
                "search_knowledge",
                json!({"query":"widget","project":"../other"}),
            ),
            call(
                4,
                "inspect_knowledge",
                json!({"source_id":"../../private.log"}),
            ),
            call(
                5,
                "inspect_knowledge",
                json!({"source_id":"project:../../private.log"}),
            ),
            call(6, "explain_error", json!({"text":"widget","ai":true})),
            call(7, "explain_error", json!({"file":"../private.log"})),
            call(
                8,
                "explain_error",
                json!({"text":"../private.log:1:1: error: widget"}),
            ),
            call(9, "search_knowledge", json!({"query":"widget","limit":0})),
            call(10, "search_knowledge", json!({"query":"widget","limit":21})),
            call(11, "search_knowledge", json!({"query":" "})),
            call(12, "search_knowledge", json!({"query":"x".repeat(8193)})),
            call(
                13,
                "search_knowledge",
                json!({"query":"widget","limit":"5"}),
            ),
        ],
        true,
    );
    for response in messages.iter().skip(1).filter(|m| m["id"] != 8) {
        if response["id"] == 2 {
            assert_eq!(response["error"]["code"], -32602, "{response}");
        } else {
            assert_eq!(response["result"]["isError"], true, "{response}");
        }
    }
    assert!(
        !serde_json::to_string(&messages)
            .unwrap()
            .contains("outside-project-marker")
    );
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "fn main() {}\n");
    assert!(!root.path().join("project/.lbc/proposals").exists());
    assert!(!root.path().join("data").exists());
}

#[test]
fn mcp_works_without_loading_ai_configuration_or_contacting_a_provider() {
    let root = fixture();
    let config = root.path().join("config/lbc");
    std::fs::create_dir_all(&config).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let settings = format!(
        "[ai]\nprovider = 'openai-compat'\nbase_url = 'http://{}'\nmodel = 'fixture'\n",
        listener.local_addr().unwrap()
    );
    let path = config.join("config.toml");
    for content in [settings.as_str(), "invalid TOML [ secret-never-read"] {
        std::fs::write(&path, content).unwrap();
        let messages = session(
            root.path(),
            &[
                initialize(),
                ready(),
                call(
                    2,
                    "explain_error",
                    json!({"text":"error[E0308]: mismatched types"}),
                ),
            ],
            false,
        );
        assert_eq!(messages[1]["result"]["isError"], false);
        assert!(
            messages[1]["result"]["structuredContent"]["report"]
                .get("ai")
                .is_none()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
}

#[test]
fn project_only_does_not_expose_shared_stores_or_other_project_knowledge() {
    let root = fixture();
    let user = root.path().join("data/lbc/notes");
    std::fs::create_dir_all(&user).unwrap();
    std::fs::write(
        user.join("private.md"),
        "---\nid: global-secret\ntitle: Widget build\n---\nwidget global-store-marker",
    )
    .unwrap();
    let sibling = root.path().join("other/.lbc/knowledge");
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(
        sibling.join("note.md"),
        "---\nid: local-note\n---\nwidget other-project-marker",
    )
    .unwrap();
    let shared = session(
        root.path(),
        &[
            initialize(),
            ready(),
            call(2, "search_knowledge", json!({"query":"widget"})),
        ],
        false,
    );
    assert!(
        serde_json::to_string(&shared)
            .unwrap()
            .contains("global-store-marker")
    );
    let messages = session(
        root.path(),
        &[
            initialize(),
            ready(),
            call(2, "search_knowledge", json!({"query":"widget"})),
            call(
                3,
                "inspect_knowledge",
                json!({"source_id":"user:global-secret"}),
            ),
            call(
                4,
                "inspect_knowledge",
                json!({"source_id":"builtin:rust-e0308"}),
            ),
            call(
                5,
                "inspect_knowledge",
                json!({"source_id":"project:local-note"}),
            ),
        ],
        true,
    );
    assert_eq!(
        messages[1]["result"]["structuredContent"]["results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(messages[2]["result"]["isError"], true);
    assert_eq!(messages[3]["result"]["isError"], true);
    assert_eq!(messages[4]["result"]["isError"], false);
    let text = serde_json::to_string(&messages).unwrap();
    assert!(!text.contains("global-store-marker"));
    assert!(!text.contains("other-project-marker"));
}

#[test]
fn protocol_lifecycle_ids_notifications_and_errors_are_consistent() {
    let root = fixture();
    let mut init = initialize();
    init["params"]["protocolVersion"] = json!("2099-01-01");
    let messages = session(
        root.path(),
        &[
            json!({"jsonrpc":"2.0","id":0,"method":"tools/list"}),
            init,
            ready(),
            json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":42}}),
            json!({"jsonrpc":"2.0","id":"ping-id","method":"ping"}),
            json!({"jsonrpc":"2.0","id":3,"method":"not/a/method"}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"search_knowledge","arguments":[]}}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"search_knowledge","arguments":{"query":"widget"},"_meta":{}}}),
            json!([]),
        ],
        false,
    );
    assert_eq!(messages.len(), 7);
    assert_eq!(messages[0]["error"]["code"], -32600);
    assert_eq!(messages[1]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(messages[2]["id"], "ping-id");
    assert_eq!(messages[2]["result"], json!({}));
    assert_eq!(messages[3]["error"]["code"], -32601);
    assert_eq!(messages[4]["error"]["code"], -32602);
    assert_eq!(messages[5]["result"]["isError"], false);
    assert_eq!(messages[6]["error"]["code"], -32600);
}

#[test]
fn omitted_tool_arguments_are_reported_as_correctable_tool_input_errors() {
    let root = fixture();
    let messages = session(
        root.path(),
        &[
            initialize(),
            ready(),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_knowledge"}}),
        ],
        false,
    );
    assert_eq!(messages[1]["result"]["isError"], true);
}

#[cfg(unix)]
#[test]
fn symlinked_knowledge_and_project_roots_are_refused() {
    let root = fixture();
    let private = root.path().join("private.md");
    std::fs::write(
        &private,
        "---\nid: symlink-note\n---\nwidget outside-project-marker",
    )
    .unwrap();
    std::os::unix::fs::symlink(private, root.path().join("project/.lbc/knowledge/link.md"))
        .unwrap();
    let messages = session(
        root.path(),
        &[
            initialize(),
            ready(),
            call(2, "search_knowledge", json!({"query":"widget"})),
            call(
                3,
                "inspect_knowledge",
                json!({"source_id":"project:symlink-note"}),
            ),
        ],
        true,
    );
    assert!(
        !serde_json::to_string(&messages)
            .unwrap()
            .contains("outside-project-marker")
    );
    assert_eq!(messages[2]["result"]["isError"], true);
    let link = root.path().join("linked-project");
    std::os::unix::fs::symlink(root.path().join("project"), &link).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_lbc"))
        .args(["mcp", "--project"])
        .arg(link)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
