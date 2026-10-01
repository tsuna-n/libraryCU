use super::*;

#[test]
fn streaming_http_preserves_unicode_redacts_before_display_and_bounds_reasoning() {
    for oversized in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
        std::fs::write(
            home.path().join("config/lbc/config.toml"),
            format!(
                "[ai]\nprovider='openai-compat'\nmodel='fixture'\nbase_url='http://{}/v1'\n",
                listener.local_addr().unwrap()
            ),
        )
        .unwrap();
        let server = thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(std::time::Instant::now() < deadline);
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0 && bytes.len() < 256 * 1024);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                    let length: usize = String::from_utf8_lossy(&bytes[..end])
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")?
                                .trim()
                                .parse()
                                .ok()
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        let request: serde_json::Value =
                            serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap();
                        assert_eq!(request["stream"], true);
                        break;
                    }
                }
            }
            if oversized {
                let body = format!(
                    "data: {{\"choices\":[{{\"delta\":{{\"reasoning_content\":\"{}\"}}}}]}}\n",
                    "x".repeat(2 * 1024 * 1024)
                );
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
                // Rejection may close the socket before all adversarial bytes are written.
                let _ = stream.write_all(body.as_bytes());
            } else {
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
                for content in [
                    "Change: งานไทย 🦀\nAPI_",
                    "KEY=private-stream-marker\n-----BEGIN PRIVATE KEY-----\n",
                    "private-interior-marker\n-----END PRIVATE KEY-----\nFrom: old\nTo: new\n\nConfi",
                    "dence: high",
                ] {
                    let body = format!(
                        "data: {}\n\n",
                        serde_json::json!({"choices":[{"delta":{"content":content}}]})
                    );
                    for chunk in body.as_bytes().chunks(7) {
                        write!(stream, "{:x}\r\n", chunk.len()).unwrap();
                        stream.write_all(chunk).unwrap();
                        stream.write_all(b"\r\n").unwrap();
                    }
                }
                let done = b"data: [DONE]\n\n";
                write!(stream, "{:x}\r\n", done.len()).unwrap();
                stream.write_all(done).unwrap();
                stream.write_all(b"\r\n").unwrap();
                // A client must finish at [DONE], not wait for HTTP EOF/timeout.
                thread::sleep(std::time::Duration::from_secs(3));
            }
        });
        let started = std::time::Instant::now();
        let output = isolated_lbc(home.path())
            .args(["ask", "E0308", "--ai"])
            .output()
            .unwrap();
        let elapsed = started.elapsed();
        server.join().unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        if oversized {
            // Optional AI failure still leaves the offline answer available.
            assert!(stdout.contains("E0308"));
            assert!(stderr.contains("stream exceeds 2 MB"));
        } else {
            assert!(output.status.success(), "{stderr}");
            assert!(
                elapsed < std::time::Duration::from_secs(2),
                "waited after DONE: {elapsed:?}"
            );
            assert!(stdout.contains("งานไทย 🦀"), "{stdout}");
            assert!(!stdout.contains('�'));
            assert!(stdout.contains("[REDACTED"));
            assert!(!stdout.contains("private-stream-marker"));
            assert!(!stdout.contains("private-interior-marker"));
            assert!(!stdout.contains("Confidence:"));
        }
    }
}
