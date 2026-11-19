use super::*;
use std::path::Path;
use std::thread;

const SOURCE: &str = "fn main() { let value: i32 = \"3\"; }\n";
const LOG: &str = "error[E0308]: mismatched types\n --> main.rs:1:28\n";

// The same Rust executable supplies verification fixtures on all three OSes.
#[test]
fn verification_child() {
    let Ok(mode) = std::env::var("LBC_VERIFY_FIXTURE") else {
        return;
    };
    assert_eq!(
        std::fs::read_to_string("main.rs").unwrap(),
        SOURCE.replace("\"3\"", "3")
    );
    match mode.as_str() {
        "pass" => println!("checked locally"),
        "fail" => {
            eprintln!("API_KEY=private-verifier-marker");
            std::process::exit(7);
        }
        "timeout" => thread::sleep(std::time::Duration::from_secs(30)),
        "edit" => std::fs::write("main.rs", "user edit\n").unwrap(),
        "noisy" => {
            for _ in 0..10000 {
                println!("bounded output งาน");
                eprintln!("password=private-verifier-marker");
            }
        }
        _ => panic!("unknown fixture"),
    }
}

fn verifier_command() -> String {
    format!(
        "{} --exact fix::verification_child --nocapture",
        shell_words::quote(std::env::current_exe().unwrap().to_str().unwrap())
    )
}

#[test]
fn verify_success_and_explicit_rollback_work_across_processes() {
    let home = fixture();
    let id = saved_proposal(home.path());
    let output = isolated_lbc(home.path())
        .args([
            "apply-proposal",
            &id,
            "--json",
            "--verify",
            &verifier_command(),
        ])
        .env("LBC_VERIFY_FIXTURE", "pass")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "verified");
    assert_eq!(report["verification_status"], "passed");
    assert_eq!(report["verification"]["exit_code"], 0);
    assert!(
        report["verification"]["stdout"]
            .as_str()
            .unwrap()
            .contains("checked locally")
    );
    let id = report["recovery_id"].as_str().unwrap();
    let output = isolated_lbc(home.path())
        .args(["rollback", id, "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["status"],
        "rolled_back"
    );
    assert_eq!(
        std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
        SOURCE
    );
    let again = isolated_lbc(home.path())
        .args(["rollback", id, "--json"])
        .output()
        .unwrap();
    assert!(!again.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&again.stdout).unwrap()["status"],
        "failed"
    );
}

#[test]
fn verification_failures_restore_source_but_preserve_later_edits() {
    for mode in ["fail", "timeout", "missing", "edit", "noisy"] {
        let home = fixture();
        let id = saved_proposal(home.path());
        let command = if mode == "missing" {
            "lbc-nonexistent-verifier-492718".into()
        } else {
            verifier_command()
        };
        let output = isolated_lbc(home.path())
            .args([
                "apply-proposal",
                &id,
                "--json",
                "--verify",
                &command,
                "--verify-timeout",
                if mode == "timeout" { "1" } else { "30" },
            ])
            .env("LBC_VERIFY_FIXTURE", mode)
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.success(), mode == "noisy", "{mode}: {report}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-verifier-marker"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-verifier-marker"));
        if mode == "noisy" {
            assert_eq!(report["verification"]["output_truncated"], true);
            assert!(output.stdout.len() < 90_000);
        } else if mode == "edit" {
            assert_eq!(report["rollback_status"], "failed");
            assert_eq!(
                std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
                "user edit\n"
            );
        } else {
            assert_eq!(report["status"], "rolled_back", "{mode}: {report}");
            assert_eq!(report["applied"], false);
            assert_eq!(
                report["verification_status"],
                match mode {
                    "fail" => "failed",
                    "timeout" => "timed_out",
                    _ => "error",
                }
            );
            assert_eq!(
                std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
                SOURCE
            );
        }
    }
}

fn fixture() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
    // Give scanner commands a local root marker even when /tmp has ancestors
    // with project markers in a sandbox or a CI image.
    std::fs::write(home.path().join("README.md"), "Isolated fixture\n").unwrap();
    std::fs::write(home.path().join("main.rs"), SOURCE).unwrap();
    std::fs::write(home.path().join("error.log"), LOG).unwrap();
    home
}

fn saved_proposal(home: &Path) -> String {
    let diagnostic = librarycube::diagnostics::parse_primary(LOG).unwrap();
    let target = librarycube::fixer::Target::read(home, &diagnostic).unwrap();
    let patch = target
        .validate_response(r#"{"before":"\"3\"","after":"3"}"#)
        .unwrap();
    librarycube::fixer::proposal::save(&target, &patch).unwrap()
}

#[test]
fn saved_proposal_applies_exact_preview_and_preserves_later_edits() {
    let home = fixture();
    let id = saved_proposal(home.path());
    let output = isolated_lbc(home.path())
        .args(["apply-proposal", &id, "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "applied");
    assert_eq!(report["patch"]["after"], "3");
    assert_eq!(report["verification_status"], "unverified");
    let replay = isolated_lbc(home.path())
        .args(["apply-proposal", &id, "--json"])
        .output()
        .unwrap();
    assert!(!replay.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&replay.stdout).unwrap()["status"],
        "failed"
    );
    std::fs::write(home.path().join("main.rs"), "later edit\n").unwrap();
    let output = isolated_lbc(home.path())
        .args([
            "rollback",
            report["recovery_id"].as_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
        "later edit\n"
    );
}

#[test]
fn saved_proposal_works_even_with_invalid_configuration() {
    for mode in ["pass", "fail", "timeout", "edit"] {
        let home = fixture();
        let id = saved_proposal(home.path());
        std::fs::write(home.path().join("config/lbc/config.toml"), "invalid config").unwrap();
        let output = isolated_lbc(home.path())
            .args([
                "apply-proposal",
                &id,
                "--json",
                "--verify",
                &verifier_command(),
                "--verify-timeout",
                "1",
            ])
            .env("LBC_VERIFY_FIXTURE", mode)
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.success(), mode == "pass", "{report}");
        assert_eq!(
            report["status"],
            match mode {
                "pass" => "verified",
                "edit" => "failed",
                _ => "rolled_back",
            }
        );
        assert_eq!(
            std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
            match mode {
                "pass" => SOURCE.replace("\"3\"", "3"),
                "edit" => "user edit\n".into(),
                _ => SOURCE.into(),
            }
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-verifier-marker"));
    }
}

#[test]
fn fix_shows_guidance_without_modifying_source() {
    let home = fixture();
    let output = isolated_lbc(home.path())
        .args(["fix", "error.log", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "offline_guidance");
    assert_eq!(report["applied"], false);
    assert!(report["patch"].is_null());
    assert!(
        !report["guidance"]["passages"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let output = isolated_lbc(home.path())
        .args(["fix", "error.log", "--apply"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
        SOURCE
    );
    assert!(!home.path().join("data/lbc/history").exists());
}

#[test]
fn fix_help_and_input_failures_are_explicit() {
    let home = fixture();
    let output = isolated_lbc(home.path())
        .args(["fix", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for flag in ["--json", "--project", "--stdin"] {
        assert!(help.contains(flag));
    }
    for args in [
        vec!["fix", "missing.log"],
        vec!["fix", "error.log", "--stdin"],
        vec!["fix", "--stdin"],
    ] {
        let output = isolated_lbc(home.path())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn fix_stdin_auto_language_uses_the_complete_current_input() {
    let home = fixture();
    let mut child = isolated_lbc(home.path())
        .args(["fix", "--stdin", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{LOG}คำอธิบายเพิ่มเติม\n").as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["guidance"]["language"], "th");
    assert_eq!(report["applied"], false);
}

#[test]
fn python_package_install_search_explain_and_fix_work_together() {
    let home = fixture();
    let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/python-fastapi-basics");
    let output = isolated_lbc(home.path())
        .args(["knowledge", "install"])
        .arg(&package)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(home.path().join("main.py"), "print(missing_name)\n").unwrap();
    std::fs::write(home.path().join("error.log"), "Traceback (most recent call last):\n  File \"main.py\", line 1, in <module>\nNameError: name 'missing_name' is not defined\n").unwrap();
    let output = isolated_lbc(home.path())
        .args(["explain", "error.log", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["diagnostic"]["source"], "python");
    assert_eq!(report["diagnostic"]["file"], "main.py");
    assert!(
        report["project_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"] == "main.py")
    );
    assert!(!report["knowledge"].as_array().unwrap().is_empty());
    let output = isolated_lbc(home.path())
        .args(["fix", "error.log", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        report["guidance"]["passages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|passage| passage["source_id"]
                .as_str()
                .unwrap()
                .starts_with("package:python-fastapi-basics:"))
    );
    assert_eq!(
        std::fs::read_to_string(home.path().join("main.py")).unwrap(),
        "print(missing_name)\n"
    );
    for (language, query) in [
        ("en", "CustomWidgetError frobnication worker.py"),
        (
            "th",
            "Python ข้อผิดพลาด CustomWidgetError frobnication worker.py",
        ),
    ] {
        let output = isolated_lbc(home.path())
            .args(["config", "set", "output.language", language])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = isolated_lbc(home.path())
            .args(["ask", query, "--json"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["answer_status"], "general_guidance");
        assert_eq!(report["language"], language);
        assert!(
            report["offline_answer"]
                .as_str()
                .unwrap()
                .contains(if language == "th" {
                    "ยังไม่พบหลักฐานเพียงพอ"
                } else {
                    "insufficient evidence"
                })
        );
    }
    let output = isolated_lbc(home.path())
        .args(["search", "ModuleNotFoundError", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("package:python-fastapi-basics:"));
}
