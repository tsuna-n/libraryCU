# Design Specification: Streaming AI Responses & Version Bump to 0.3.2-t

## Overview
This specification details the addition of streaming Server-Sent Events (SSE) support for AI-enhanced queries in `libraryCube` (`lbc`), providing real-time token-by-token terminal output and a "Thinking..." status indicator during reasoning model execution, along with a project version bump to `0.3.2-t`.

## Goals
1. **Incremental Streaming Output**: Stream tokens in real time via `--ai` in `lbc ask`, `lbc chat`, and `lbc explain`, rather than waiting for the entire completion to finish.
2. **Thinking / Reasoning Status**: Display a clean `Thinking...` status indicator when reasoning models (such as GLM on Z.AI) are emitting `reasoning_content`, and erase/transition the status once final content begins streaming.
3. **Structured JSON Compatibility**: Suppress streaming output when `--json` is supplied to ensure machine-readable JSON integrity.
4. **Resilience & Fallback**: Retain full response accumulation so confidence scoring (`Confidence: high/medium/low`), chat session history, and offline fallback mechanisms continue to function.
5. **Timeout Safety**: Increase client HTTP timeout from 45 seconds to 90 seconds to avoid prematurely aborting deep reasoning loops.
6. **Version Bump**: Update crate version in `Cargo.toml` to `0.3.2-t`.

---

## Architecture & Interfaces

### 1. Stream Event Abstraction (`src/ai/provider.rs`)
Define a lightweight event enum for callbacks:
```rust
pub enum StreamEvent<'a> {
    Thinking,
    Content(&'a str),
}
```

Extend `AiProvider` trait with streaming capability:
```rust
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn chat(
        &self,
        request: AiRequest,
    ) -> impl std::future::Future<Output = anyhow::Result<AiResponse>> + Send;

    fn chat_stream<'a>(
        &'a self,
        request: AiRequest,
        on_event: &'a mut (dyn FnMut(StreamEvent) + Send),
    ) -> impl std::future::Future<Output = anyhow::Result<AiResponse>> + Send {
        // Default implementation delegates to chat without streaming callbacks
        self.chat(request)
    }
}
```

### 2. OpenAI-Compatible SSE Streaming (`src/ai/openai_compat.rs`)
- Request payload sets `"stream": true`.
- Uses `reqwest::Response::bytes_stream()` to read incoming chunks.
- Parses incoming SSE lines:
  - Lines starting with `data: [DONE]` signal completion.
  - Lines starting with `data: ` are deserialized into chunk structures containing `delta`.
  - If `delta.reasoning_content` is present and non-empty, emit `StreamEvent::Thinking` on the first encounter.
  - If `delta.content` is present and non-empty, emit `StreamEvent::Content(content_str)`.
- Accumulates all `content` into an internal string buffer.
- At the end of the stream, parses confidence marker and returns the complete `AiResponse`.
- Increase `Client` timeout from 45s to 90s.

### 3. OpenRouter Provider (`src/ai/openrouter.rs`)
- Implement `chat_stream` similarly for OpenRouter SSE stream format.
- Increase `Client` timeout to 90s.

### 4. Enhancement Integration Layer (`src/ai/enhance.rs` & `src/answer.rs`)
- Add streaming variants:
  - `ai::enhance_with_language_stream(...)`
  - `answer::enhance_stream(...)`
- Accepts an optional callback `Option<&mut dyn FnMut(StreamEvent)>`.
- In `src/cli/commands/ask.rs`, `src/cli/commands/chat.rs`, and `src/cli/commands/explain.rs`:
  - When `--json` is false:
    - Print header (e.g. `\nAI analysis (provider / model):`).
    - Create a stateful printer:
      - On `StreamEvent::Thinking`: if not already thinking, print `Thinking...` on stderr or stdout.
      - On `StreamEvent::Content(str)`: if thinking, clear `Thinking...` (`\r\x1b[2K`) and print `str`, flushing stdout immediately.
    - At completion, print the parsed `AI confidence: ...`.
  - When `--json` is true: pass `None` for stream callback so output remains pure JSON.

---

## Edge Cases & Error Handling
1. **Network Drop During Stream**: If a network failure occurs midway, catch the error and fall back gracefully to the deterministic offline report.
2. **Empty Content / Reasoning Only**: Ensure models that output only reasoning or empty content fail cleanly without panics.
3. **ANSI Terminal Capabilities**: If stdout is not a TTY or colors are disabled (`NO_COLOR`), avoid escape code corruption when clearing the `Thinking...` line.
4. **Session Chat Persistence**: In `lbc chat`, the accumulated text is appended to `session_history` identically to non-streaming mode.

---

## Verification Plan
1. **Unit Tests**:
   - Test SSE chunk parsing logic with mocked byte streams (handling split lines, partial chunks, reasoning deltas, content deltas).
   - Verify `chat_stream` accumulator produces identical `AiResponse` structure and confidence parsing.
2. **Integration Tests**:
   - Test `lbc ask --ai` with live ZAI provider, observing `Thinking...` transition to real-time token stream.
   - Test `lbc ask --ai --json` confirming pure JSON output.
   - Test `lbc chat --ai` multi-turn interactive stream.
   - Test `lbc explain --stdin --ai` stream.
3. **Full Regression Test**:
   - Run `cargo test` ensuring all 94+ tests pass.
