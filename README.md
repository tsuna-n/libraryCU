# LibraryCU

LibraryCU (`lcu`) is a read-only developer diagnostic toolkit for the terminal. Version 0.2 detects project types, scans repository metadata safely, parses Rust compiler errors, searches local knowledge, and explains known diagnostics without an AI service. When configured, it can optionally extend the deterministic analysis with an AI provider such as OpenRouter or a local OpenAI-compatible server (Ollama, LM Studio, vLLM).

Its diagnostic flow is:

```text
Inspect → Retrieve → Explain → Suggest → Verify
```

## Install

Build from this repository with a current stable Rust toolchain:

```bash
cargo build --release
cargo install --path .
```

The installed command is `lcu`.

## Commands

```bash
lcu --help
lcu scan
lcu scan --tree
lcu search "borrow checker"
lcu explain error.log
cargo check 2>&1 | lcu explain
lcu config show
lcu doctor
```

`scan`, `search`, `explain`, and `doctor` are read-only. They do not modify project files.

### Explain errors

Read an error log file:

```bash
lcu explain error.log
```

Read a Unix pipeline (no flag is required when stdin is piped):

```bash
cargo check 2>&1 | lcu explain
```

LCU parses the diagnostic, detects and scans the project, retrieves matching local knowledge, then reports evidence, cause, suggested fixes, verification commands, and confidence. Known v0.1 Rust rules include E0382, E0432, E0433, and E0499. Unknown errors receive investigation steps instead of an invented fix.

Use detailed or machine-readable output when needed:

```bash
lcu explain error.log --verbose
lcu explain error.log --json
```

Error input is limited to 2 MB. Common API keys, tokens, authorization headers, passwords, database URLs, and private keys are redacted before diagnostics are processed or displayed.

### Scan a project

```bash
lcu scan
lcu scan --tree
lcu scan --path ../another-project --json
```

Detection supports Rust/Cargo, Node.js and TypeScript, Python, Go, Maven, Gradle, Docker, and Docker Compose. Clearly detected stacks can coexist in one repository.

Generated directories such as `.git`, `target`, `node_modules`, `dist`, `build`, `.next`, `coverage`, `vendor`, Python caches, and virtual environments are skipped. LCU uses metadata and file-size limits rather than loading every source file.

### Search local knowledge

```bash
lcu search E0382
lcu search "borrow checker"
lcu search E0432 --json
```

V0.1 ranks exact error codes first, followed by title, metadata, and keyword matches. Built-in documents are under `knowledge/`; a project's own valid Markdown knowledge documents can extend the store without replacing built-in IDs.

### Configuration

The default Linux configuration path is `~/.config/lcu/config.toml`. `XDG_CONFIG_HOME` is respected, and `LCU_CONFIG` can point to a specific file.

```toml
[output]
language = "auto"

[scanner]
max_file_size_kb = 256
ignore_hidden = true

[memory]
mode = "session"
```

A missing file is valid and uses these defaults. View or update supported settings with:

```bash
lcu config show
lcu config show --json
lcu config set scanner.max_file_size_kb 512
lcu config set scanner.ignore_hidden false
```

### AI explanations (optional, v0.2)

The deterministic report is always computed first. With `--ai`, LCU builds a compact context (diagnostic, project stack, evidence, knowledge references, redacted error output) and sends it to the configured provider:

```bash
lcu explain error.log --ai
cargo check 2>&1 | lcu explain --ai
```

Configure the provider in `config.toml`; API keys are read from the environment and are never written to disk:

```toml
[ai]
provider = "openai-compat"          # off | openrouter | openai-compat
model = "qwen2.5-coder:7b"
base_url = "http://localhost:11434/v1"
```

- `openrouter` requires `OPENROUTER_API_KEY`.
- `openai-compat` works with any OpenAI-compatible endpoint; `OPENAI_API_KEY` is optional (local servers such as Ollama need none).
- The provider is asked to end its answer with a `Confidence:` marker, which LCU parses and reports.
- If the AI call fails, LCU prints the reason to stderr and falls back to the deterministic explanation; the command still succeeds. Only titles and paths from local knowledge are sent to the provider, never the knowledge database.

## Development

Required validation for Rust changes:

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

LibraryCU v0.2 does not require an API key, network access, an LLM, or a vector database; AI remains an optional layer.
