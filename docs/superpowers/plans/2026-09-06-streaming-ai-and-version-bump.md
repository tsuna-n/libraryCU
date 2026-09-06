# Streaming AI Responses & Version Bump Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real-time streaming output with a reasoning "Thinking..." indicator for `--ai` in `lbc ask`, `lbc chat`, and `lbc explain`, and bump libraryCube version to `0.3.2-t`.

**Architecture:** Extend `AiProvider` and `AiClient` with a `chat_stream` method accepting a `StreamEvent` callback. Parse SSE lines from `response.chunk().await`, emitting `Thinking` when reasoning deltas appear and `Content(&str)` when content arrives. Integrate streaming into CLI commands with terminal status clearing (`\r\x1b[2K`), while keeping `--json` output intact.

**Tech Stack:** Rust (edition 2024), `reqwest 0.12`, `tokio 1`, `serde_json`, `clap 4`.

## Global Constraints
- Target version: `0.3.2-t`
- All 94 existing tests must remain passing
- Pure offline commands and `--json` outputs must never emit terminal stream formatting
- Secret redaction and bounded buffer protections must be preserved

---

### Task 1: Version Bump to 0.3.2-t

**Files:**
- Modify: `Cargo.toml:1-5`
- Test: `Cargo.lock`, `tests/cli.rs`

**Interfaces:**
- Produces: `librarycube` v`0.3.2-t`

- [ ] **Step 1: Write failing test / check for version**
Run `cargo check` and inspect current version reporting:
Run: `cargo test --test cli help_lists_v01_commands`

- [ ] **Step 2: Update Cargo.toml version**
Modify `Cargo.toml` line 3 from `version = "0.3.1"` to `version = "0.3.2-t"`.

- [ ] **Step 3: Update Cargo.lock and verify compilation**
Run: `cargo check`
Run: `cargo test`

- [ ] **Step 4: Commit**
```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.3.2-t"
```

---

### Task 2: StreamEvent and AiProvider Trait Extension

**Files:**
- Modify: `src/ai/provider.rs:1-120`
- Test: `src/ai/provider.rs` (unit tests)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum StreamEvent<'a> {
      Thinking,
      Content(&'a str),
  }
  ```
- Consumes: `AiRequest`, `AiResponse`

- [ ] **Step 1: Write unit test for stream chunk parsing and events**
In `src/ai/provider.rs` `tests` module:
```rust
#[test]
fn parses_sse_data_lines_correctly() {
    let raw = "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\ndata: [DONE]\n\n";
    let events = parse_sse_events(raw);
    assert_eq!(events, vec![ParsedSse::Content("hello ")]);
}
```

- [ ] **Step 2: Implement StreamEvent, ParsedSse, and AiProvider::chat_stream**
Add `StreamEvent` enum, `parse_sse_line` helper function, and add default `chat_stream` implementation to `AiProvider` and `AiClient`.

- [ ] **Step 3: Run unit tests to verify**
Run: `cargo test ai::provider`

- [ ] **Step 4: Commit**
```bash
git add src/ai/provider.rs
git commit -m "feat(ai): define StreamEvent and extend AiProvider with chat_stream"
```

---

### Task 3: Streaming Implementation for OpenAiCompatProvider

**Files:**
- Modify: `src/ai/openai_compat.rs:1-60`
- Test: `src/ai/openai_compat.rs`, `tests/cli.rs`

**Interfaces:**
- Produces: `OpenAiCompatProvider::chat_stream`
- Increases timeout to 90s for reasoning models

- [ ] **Step 1: Write test for OpenAiCompatProvider streaming**
In `src/ai/openai_compat.rs` tests:
Test mock SSE chunks decoding content and reasoning.

- [ ] **Step 2: Implement chat_stream in OpenAiCompatProvider**
Send `"stream": true` in request body.
Read chunks asynchronously with `response.chunk().await`.
Buffer partial SSE lines across chunks.
Emit `StreamEvent::Thinking` on first `reasoning_content` delta.
Emit `StreamEvent::Content(&text)` on `content` delta.
Accumulate full content and return `AiResponse`.
Update client timeout to 90 seconds.

- [ ] **Step 3: Run tests**
Run: `cargo test ai::openai_compat`

- [ ] **Step 4: Commit**
```bash
git add src/ai/openai_compat.rs
git commit -m "feat(ai): implement SSE streaming and reasoning event support in OpenAiCompatProvider"
```

---

### Task 4: Streaming Implementation for OpenRouterProvider

**Files:**
- Modify: `src/ai/openrouter.rs:1-60`

- [ ] **Step 1: Implement chat_stream in OpenRouterProvider**
Delegate or implement SSE chunk reader with 90s timeout.

- [ ] **Step 2: Run tests**
Run: `cargo test ai::openrouter`

- [ ] **Step 3: Commit**
```bash
git add src/ai/openrouter.rs
git commit -m "feat(ai): implement streaming support in OpenRouterProvider"
```

---

### Task 5: Integrate Streaming in Enhancement & Answer Modules

**Files:**
- Modify: `src/ai/enhance.rs`
- Modify: `src/answer.rs`
- Test: `tests/cli.rs`

**Interfaces:**
- Produces:
  `ai::enhance_with_language_stream(...)`
  `answer::enhance_stream(...)`

- [ ] **Step 1: Add streaming enhance functions**
Allow callers to supply an optional `&mut (dyn FnMut(StreamEvent) + Send)` callback.
When `Some(cb)` is passed, invoke `client.chat_stream(...)`.
When `None` is passed, invoke `client.chat(...)`.

- [ ] **Step 2: Run unit tests**
Run: `cargo test ai::enhance`
Run: `cargo test answer::tests`

- [ ] **Step 3: Commit**
```bash
git add src/ai/enhance.rs src/answer.rs
git commit -m "feat(ai): support optional streaming callback in answer and enhance pipelines"
```

---

### Task 6: CLI Commands Terminal Presentation & Integration

**Files:**
- Modify: `src/cli/commands/ask.rs`
- Modify: `src/cli/commands/explain.rs`
- Modify: `src/cli/commands/chat.rs`

- [ ] **Step 1: Update `lbc ask`**
In non-JSON mode:
Print `AI analysis (provider / model):` header.
Display `Thinking...` on `StreamEvent::Thinking`.
On `StreamEvent::Content(text)`, erase `Thinking...` and stream tokens to stdout with `io::stdout().flush()`.
Print `\n\nAI confidence: ...` upon completion.
In `--json` mode: pass `None` for callback to keep pure JSON output.

- [ ] **Step 2: Update `lbc explain`**
In non-JSON mode: stream AI explanation with `Thinking...` clearing.
In `--json` mode: pass `None` for callback.

- [ ] **Step 3: Update `lbc chat`**
Stream assistant answers interactively in real time.
Ensure full text is pushed to `session_history`.

- [ ] **Step 4: Verify with live command execution**
Test `lbc ask "How to fix Rust E0382?" --ai`
Test `lbc ask "How to fix Rust E0382?" --ai --json`
Test `lbc explain --stdin --ai`
Test `lbc chat --ai`

- [ ] **Step 5: Run full test suite**
Run: `cargo test`

- [ ] **Step 6: Commit**
```bash
git add src/cli/commands/ask.rs src/cli/commands/explain.rs src/cli/commands/chat.rs
git commit -m "feat(cli): stream AI responses with live thinking indicator in ask, explain, and chat"
```

---

### Task 7: Final End-to-End Audit and Git Push

**Files:**
- Test: All repository files and live ZAI connection

- [ ] **Step 1: Run full test suite**
Run: `cargo test`

- [ ] **Step 2: Re-compile and install to ~/.local/bin/lbc**
Run: `./install.sh`
Verify: `lbc --version` reports `0.3.2-t`.

- [ ] **Step 3: Git Push to remote**
Run: `git push origin main`

- [ ] **Step 4: Verify working tree is clean**
Run: `git status`
