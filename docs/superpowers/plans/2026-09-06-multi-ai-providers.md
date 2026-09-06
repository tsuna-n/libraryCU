# Multi-AI Provider Support, Version 0.3.1, and Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement first-class support for multiple AI providers (`openai`, `zai`/`glm`, `ollama`), bump crate version to `0.3.1`, and enrich documentation.

**Architecture:** Extend `AiConfig` with `effective_base_url()` and `effective_model()` fallbacks; update `resolve_client_with_env` to route named providers to appropriate default endpoints and API key environment variables; enhance `lbc doctor` to report vendor-specific diagnostic status; update version to `0.3.1` in `Cargo.toml`; and update `README.md` and `docs/usage-examples.md`.

**Tech Stack:** Rust (edition 2024), Cargo, Clap, Reqwest, Tokio.

## Global Constraints
- Do not introduce breaking changes to existing `openrouter` or `openai-compat` workflows.
- All offline operations (`ask`, `search`, `list`, `inspect`) must remain strictly offline and zero-network.
- Preserve deterministic fallback when AI fails or is disabled.
- Follow existing codebase patterns: safe error handling via `anyhow::Result`, redact sensitive URLs/keys.

---

### Task 1: Add Provider Defaults and Relax Validation in `src/config/settings.rs`

**Files:**
- Modify: `src/config/settings.rs`
- Test: `src/config/settings.rs` (unit tests)

**Interfaces:**
- `AiConfig::effective_base_url(&self) -> &str`
- `AiConfig::effective_model(&self) -> &str`
- `validate(&Config) -> Result<()>` accepting `openai`, `zai`, `glm`, `ollama`, `openrouter`, `openai-compat`, `off`.

- [ ] **Step 1: Write failing unit tests for named providers in `src/config/settings.rs`**
- [ ] **Step 2: Run `cargo test --lib config::settings::tests` and verify failure**
- [ ] **Step 3: Implement `effective_base_url`, `effective_model`, and update `validate`**
- [ ] **Step 4: Run `cargo test --lib config::settings::tests` and verify pass**
- [ ] **Step 5: Commit changes**

### Task 2: Provider Resolution in `src/ai/mod.rs` & Model Usage in `src/ai/enhance.rs`

**Files:**
- Modify: `src/ai/mod.rs`
- Modify: `src/ai/enhance.rs`
- Test: `src/ai/mod.rs` (unit tests)

**Interfaces:**
- `resolve_client_with_env(ai: &AiConfig, env_value: impl Fn(&str) -> Option<String>) -> anyhow::Result<AiClient>`
- Handles `openai` with `OPENAI_API_KEY`
- Handles `zai` / `glm` with `ZAI_API_KEY` or `GLM_API_KEY`
- Handles `ollama` with optional `OLLAMA_API_KEY`

- [ ] **Step 1: Write failing unit tests in `src/ai/mod.rs` for `openai`, `zai`, `ollama` resolution**
- [ ] **Step 2: Run `cargo test --lib ai::tests` and verify failure**
- [ ] **Step 3: Implement provider resolution and update `enhance_with_language` to use `effective_model()`**
- [ ] **Step 4: Run `cargo test --lib ai::tests` and verify pass**
- [ ] **Step 5: Commit changes**

### Task 3: Doctor Diagnostics and Terminal Output

**Files:**
- Modify: `src/cli/commands/doctor.rs`
- Modify: `src/output/terminal.rs`
- Test: `tests/cli.rs`

**Interfaces:**
- `ai_check(ai: &AiConfig) -> DoctorCheck`
- `print_config(loaded: &LoadedConfig)`

- [ ] **Step 1: Write CLI integration test in `tests/cli.rs` checking `lbc doctor` with `openai` and `zai`**
- [ ] **Step 2: Run test and verify failure**
- [ ] **Step 3: Update `ai_check` in `doctor.rs` and `print_config` in `terminal.rs`**
- [ ] **Step 4: Run test and verify pass**
- [ ] **Step 5: Commit changes**

### Task 4: Bump Version to 0.3.1 and Update Documentation

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `README.md`
- Modify: `docs/usage-examples.md`

- [ ] **Step 1: Bump version to 0.3.1 in `Cargo.toml` and update `Cargo.lock`**
- [ ] **Step 2: Update `README.md` and `docs/usage-examples.md` with multi-AI provider setup and examples**
- [ ] **Step 3: Run full test suite `cargo test`**
- [ ] **Step 4: Commit changes and push to git origin**
