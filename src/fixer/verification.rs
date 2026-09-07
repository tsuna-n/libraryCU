//! Only the CLI user chooses the executable and arguments. No implicit shell,
//! automatic project command discovery, or provider supplied command execution.
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

pub const MAX_OUTPUT_BYTES: usize = 32 * 1024;
const MAX_COMMAND_BYTES: usize = 8 * 1024;

pub fn parse(command: &str) -> Result<Vec<String>> {
    ensure!(
        command.len() <= MAX_COMMAND_BYTES,
        "verification command exceeds 8 KiB"
    );
    ensure!(
        !command.chars().any(char::is_control),
        "verification command contains control characters"
    );
    let args = shell_words::split(command).context("invalid verification command quoting")?;
    ensure!(
        args.first().is_some_and(|s| !s.is_empty()),
        "verification command is empty"
    );
    Ok(args)
}

#[derive(Debug, Serialize)]
pub struct Verification {
    pub status: &'static str,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub output_truncated: bool,
    pub error: Option<String>,
    pub process_stopped: bool,
}

async fn capture(mut stream: impl AsyncRead + Unpin) -> std::io::Result<(String, bool)> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let mut truncated = false;
    loop {
        let count = stream.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        let keep = count.min(MAX_OUTPUT_BYTES - bytes.len());
        bytes.extend_from_slice(&buffer[..keep]);
        truncated |= keep < count;
    }
    // Discard a truncated final line: it may end in the middle of a credential.
    if truncated {
        let end = bytes.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        bytes.truncate(end);
    }
    let output = crate::security::redact_sensitive(&String::from_utf8_lossy(&bytes));
    let mut output: String = output
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .collect();
    if output.len() > MAX_OUTPUT_BYTES {
        let mut end = MAX_OUTPUT_BYTES;
        while !output.is_char_boundary(end) {
            end -= 1;
        }
        output.truncate(end);
        truncated = true;
    }
    Ok((output, truncated))
}

pub fn run(root: &Path, args: &[String], timeout: u64) -> Verification {
    let mut report = Verification {
        status: "error",
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        output_truncated: false,
        error: None,
        process_stopped: true,
    };
    let outcome = (|| -> Result<()> {
        ensure!(
            !args.is_empty() && (1..=3600).contains(&timeout),
            "invalid verification arguments or timeout"
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let mut child = Command::new(&args[0])
                .args(&args[1..])
                .current_dir(root)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .context("cannot start verification command")?;
            report.process_stopped = false;
            let stdout = child.stdout.take().context("missing verification stdout")?;
            let stderr = child.stderr.take().context("missing verification stderr")?;
            let result = tokio::time::timeout(Duration::from_secs(timeout), async {
                tokio::try_join!(child.wait(), capture(stdout), capture(stderr))
            })
            .await;
            match result {
                Ok(Ok((status, (stdout, out_cut), (stderr, err_cut)))) => {
                    report.process_stopped = true;
                    report.status = if status.success() { "passed" } else { "failed" };
                    report.exit_code = status.code();
                    report.stdout = stdout;
                    report.stderr = stderr;
                    report.output_truncated = out_cut || err_cut;
                }
                Ok(Err(error)) => {
                    report.process_stopped = child.kill().await.is_ok();
                    return Err(error.into());
                }
                Err(_) => {
                    report.process_stopped = child.kill().await.is_ok();
                    report.status = "timed_out";
                    report.error = Some("verification deadline exceeded".into());
                }
            }
            Ok(())
        })
    })();
    if let Err(error) = outcome {
        report.error = Some(crate::security::redact_sensitive(&format!("{error:#}")));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parsing_has_no_shell_expansion_and_bounds_bytes() {
        assert_eq!(
            parse(r#"'C:\Program Files\tool.exe' 'งาน path' '$HOME' '&&'"#).unwrap(),
            vec![r"C:\Program Files\tool.exe", "งาน path", "$HOME", "&&"]
        );
        for size in [
            MAX_COMMAND_BYTES - 1,
            MAX_COMMAND_BYTES,
            MAX_COMMAND_BYTES + 1,
        ] {
            assert_eq!(parse(&"x".repeat(size)).is_ok(), size <= MAX_COMMAND_BYTES);
        }
        for invalid in ["", "  ", "''", "'unfinished", "echo\nother"] {
            assert!(parse(invalid).is_err());
        }
    }

    #[test]
    fn output_capture_bounds_unicode_and_redacts_before_reporting() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        for size in [MAX_OUTPUT_BYTES - 1, MAX_OUTPUT_BYTES, MAX_OUTPUT_BYTES + 1] {
            let text = format!("{}\n", "x".repeat(size - 1));
            let (output, truncated) = runtime.block_on(capture(text.as_bytes())).unwrap();
            assert!(output.len() <= MAX_OUTPUT_BYTES);
            assert_eq!(truncated, size > MAX_OUTPUT_BYTES);
        }
        let text = format!("safe งาน\npassword={}\n", "ก".repeat(MAX_OUTPUT_BYTES));
        let (output, truncated) = runtime.block_on(capture(text.as_bytes())).unwrap();
        assert!(truncated);
        assert_eq!(output, "safe งาน\n");
        let (output, _) = runtime
            .block_on(capture(&b"API_KEY=hidden-value\n"[..]))
            .unwrap();
        assert!(!output.contains("hidden-value"));
    }
}
