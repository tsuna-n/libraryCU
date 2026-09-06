# Multi-AI Provider Support, Version 0.3.1, and Documentation Updates Design Specification

## Overview
This specification details the enhancements to libraryCube (`lbc`) to support first-class named presets for multiple AI providers (OpenAI, Zhipu AI / ZAI GLM, Ollama, alongside existing OpenRouter and generic OpenAI-compatible endpoints), bump the crate version to `0.3.1`, and provide comprehensive documentation in both English and Thai.

## Goals
1. **Named AI Provider Presets**: Allow users to configure `ai.provider` with `openai`, `zai` (alias `glm`), and `ollama` with zero-friction sensible defaults (endpoint and default model) without manually typing long base URLs.
2. **Flexible Configuration & Validation**: Fix configuration validation order so setting `ai.provider` first does not fail when `base_url` or `model` has not yet been explicitly typed. Implement `effective_base_url()` and `effective_model()` fallbacks.
3. **Provider-Aware Key Resolution & Diagnostics**:
   - `openai`: Reads `OPENAI_API_KEY`.
   - `zai` / `glm`: Reads `ZAI_API_KEY` or `GLM_API_KEY`.
   - `ollama`: No API key required by default (optional `OLLAMA_API_KEY` if present).
   - `openrouter`: Reads `OPENROUTER_API_KEY`.
   - `openai-compat`: Custom `base_url` with optional keys.
   - `lbc doctor`: Reports vendor-specific status and endpoint details.
4. **Version Bump to 0.3.1**: Update `Cargo.toml`, `Cargo.lock`, and documentation references from `0.3.0` to `0.3.1`.
5. **Documentation Improvements**: Update `README.md` and `docs/usage-examples.md` with explicit guides for each AI provider, configuration recipes, environment variable setup, and Thai translations/notes.

## Architecture and Design

### 1. Configuration (`src/config/settings.rs`)
Add helper methods on `AiConfig`:
- `pub fn effective_base_url(&self) -> &str`
  - Returns `base_url` if not empty.
  - Defaults:
    - `"openai"` => `"https://api.openai.com/v1"`
    - `"zai"` | `"glm"` => `"https://open.bigmodel.cn/api/paas/v4"`
    - `"ollama"` => `"http://localhost:11434/v1"`
    - Others => `""`
- `pub fn effective_model(&self) -> &str`
  - Returns `model` if not empty.
  - Defaults:
    - `"openai"` => `"gpt-4o-mini"`
    - `"zai"` | `"glm"` => `"glm-4-flash"`
    - `"ollama"` => `"llama3.2"`
    - Others => `""`
- Update `validate(&Config)`:
  - Valid providers: `"off"`, `"openai"`, `"zai"`, `"glm"`, `"ollama"`, `"openrouter"`, `"openai-compat"`.
  - Check `effective_model()` and `effective_base_url()` instead of raw fields for validation, allowing presets to validate cleanly without manual overrides.

### 2. Provider Resolution (`src/ai/mod.rs`)
In `resolve_client_with_env`:
- Handle `"openai"`: Read `OPENAI_API_KEY`, use `ai.effective_base_url()`, return `AiClient::OpenAiCompat`.
- Handle `"zai" | "glm"`: Read `ZAI_API_KEY` or `GLM_API_KEY`, use `ai.effective_base_url()`, return `AiClient::OpenAiCompat`.
- Handle `"ollama"`: Read optional `OLLAMA_API_KEY`, use `ai.effective_base_url()`, return `AiClient::OpenAiCompat`.
- Handle `"openrouter"`: Read `OPENROUTER_API_KEY`.
- Handle `"openai-compat"`: Custom base_url, optional keys.
- Update error messages to list all supported providers.

### 3. AI Enhancement & Execution (`src/ai/enhance.rs`)
- When building `AiRequest`, pass `ai.effective_model()` so default models are passed if `ai.model` was left blank in config.

### 4. Health Check Diagnostics (`src/cli/commands/doctor.rs`)
- Update `ai_check` to report vendor-specific status for `openai`, `zai` / `glm`, `ollama`, `openrouter`, and `openai-compat`.

### 5. Terminal Display (`src/output/terminal.rs`)
- Display `effective_model()` and `effective_base_url()` in `lbc config list`.

### 6. Versioning and Documentation
- Bump version to `0.3.1` in `Cargo.toml`.
- Update `README.md` and `docs/usage-examples.md`.
