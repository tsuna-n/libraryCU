use std::process::Command;

fn lcu() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lcu"))
}

#[test]
fn help_lists_v01_commands() {
    let output = lcu().arg("--help").output().expect("lcu should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in ["scan", "explain", "search", "config", "doctor"] {
        assert!(stdout.contains(command), "help did not list {command}");
    }
    assert!(stdout.contains("LibraryCU - Developer Diagnostic Toolkit"));
}

#[test]
fn explain_help_documents_pipeline_options() {
    let output = lcu()
        .args(["explain", "--help"])
        .output()
        .expect("lcu explain --help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for option in ["--stdin", "--verbose", "--json"] {
        assert!(stdout.contains(option), "help did not list {option}");
    }
    assert!(stdout.contains("[FILE]"));
}
