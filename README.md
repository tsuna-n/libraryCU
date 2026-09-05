# libraryCube

libraryCube (`lbc`) is a terminal knowledge library. You can save ordinary Markdown notes, find and inspect them, ask questions against retrieved passages, and use the same knowledge when explaining diagnostics. Retrieval and cited offline answers work without an API key, network connection, model, vector database, or Python service. Optional AI expands the retrieved answer; it never replaces retrieval.

The Rust package and crate are named `librarycube`; the executable is `lbc`.

For copy-paste examples covering every command, see the
[complete usage examples](docs/usage-examples.md).

## Install

Use a compatible stable Rust toolchain:

```bash
cargo build --release
cargo install --path .
lbc --help
```

## Add, find, inspect, and edit knowledge

Import a plain Markdown file into the user store:

```bash
lbc add --title "Demo service port conflict" --file ./port-note.md
lbc add --id demo-port --title "Demo service port conflict" \
  --kind troubleshooting --file ./port-note.md
printf '%s\n' 'Use port 4318, then verify the listener.' | \
  lbc add --id demo-stdin --title "Demo port" --stdin
```

`--kind` accepts `note`, `concept`, or `troubleshooting`. An omitted ID is generated from the title and remains stable after storage. Existing entries are never overwritten by `add`.

Use `--project PATH` with `add` to write explicitly to `PATH/.lbc/knowledge` instead of the user store. Then retrieve or update entries:

```bash
lbc list
lbc search "demo service port conflict"
lbc inspect user:demo-port
lbc ask "How do I resolve the demo service port conflict?"
lbc edit user:demo-port --file ./updated-port-note.md
lbc index
```

`search` shows ranked excerpts, match reasons, source-qualified IDs, and locators. `inspect` shows the full body, including embedded built-ins. `edit` validates the replacement and atomically replaces a writable entry; without `--file`, it starts `VISUAL` or `EDITOR` directly without invoking a shell. A failed editor or invalid replacement preserves the original.

### Markdown format

Stored entries use YAML frontmatter followed by a nonempty Markdown body:

```markdown
---
id: demo-port
title: Demo service port conflict
kind: troubleshooting
tags: [demo, port]
keywords: [address-in-use]
---
# Demo service port conflict

Change the development port to 4318, restart the development process, and
confirm that it listens on 4318.
```

Existing metadata remains supported: `id`, `title`, `language`, `tool`, `category`, `error_code`, `tags`, and `keywords`. `language` describes technical material; it is separate from the UI language.

Troubleshooting notes can use headings for symptoms/environment, cause, suggested solution, verification steps, and references. Optional `verification_status` accepts `unverified`, `user-reported`, or `recorded-check`. The latter two describe what the note author recorded; libraryCube never presents them as a fresh check performed by the current command.

### Sources, collisions, and overrides

The sources are:

- `builtin:ID`: knowledge embedded in the executable.
- `package:NAME:ID`: installed package knowledge under `$XDG_DATA_HOME/lbc/knowledge` (fallback `~/.local/share/lbc/knowledge`).
- `user:ID`: user notes under `$XDG_DATA_HOME/lbc/notes` (fallback `~/.local/share/lbc/notes`).
- `project:ID`: explicit `<project>/.lbc/knowledge` and compatible `<project>/knowledge` documents.

IDs that collide across sources remain distinct; a bare ambiguous ID produces an error with candidates. Duplicate IDs inside one source are validation errors. Installed packages do not silently replace built-ins.

Create an intentional user override without modifying the binary:

```bash
lbc inspect builtin:rust-e0308
lbc edit builtin:rust-e0308 --override --file ./our-e0308.md
```

The override records its target. A project override has priority over a user override in that project. External Markdown changes are reloaded by the next command and each chat question; `index` reports that it rebuilds in memory and does not pretend a persistent cache exists.

## Ask and explain offline

```bash
lbc ask "What causes a detached HEAD?"
printf '%s\n' 'error[E0308]: mismatched types' | lbc explain
lbc explain build.log --project ./my-project --verbose
```

Answers include actual retrieved passages and resolvable source IDs. Retrieved suggestions are labeled unverified against the current project. If no adequate match exists, libraryCube says so and offers investigation steps instead of inventing a fix.

`explain` parses compiler output, retrieves through the same index as `search` and `ask`, and reads only bounded relevant project excerpts: the diagnostic file near its reported line and small recognized manifests. JSON distinguishes inventory counts from file contents actually used as evidence. Paths outside the project, symlinks, inaccessible files, and oversized evidence are excluded with warnings. libraryCube only suggests verification commands; analysis commands never execute them.

Multiple diagnostics are counted explicitly. The current text report expands the first diagnostic and states how many additional diagnostics were detected.

## Optional AI

No outbound provider call occurs unless `--ai` is present:

```bash
lbc ask "How do I resolve the demo port conflict?" --ai
lbc explain build.log --ai
lbc chat --ai
```

Configure OpenRouter or an OpenAI-compatible endpoint:

```toml
[ai]
provider = "openai-compat" # off | openrouter | openai-compat
model = "qwen2.5-coder:7b"
base_url = "http://localhost:11434/v1"
```

OpenRouter reads `OPENROUTER_API_KEY`. OpenAI-compatible endpoints optionally read `GLM_API_KEY`, `ZAI_API_KEY`, then `OPENAI_API_KEY`; local endpoints such as Ollama can run without a key. Requests have a timeout and bounded input/output budgets. Selected note excerpts, source IDs, and bounded project evidence are redacted before sending. Notes are labeled as untrusted data in the prompt. If the provider fails, stderr and JSON expose the failure while the offline answer remains available.

## English and Thai output

Set the output language explicitly or use automatic selection:

```bash
lbc config set output.language th
lbc ask "แก้ปัญหาพอร์ตของ demo service อย่างไร"
lbc config set output.language auto
```

Supported values are `en`, `th`, and `auto`. `auto` selects Thai when the current question contains Thai characters and otherwise falls back to English. Retrieval supports Unicode and matches Thai terms present in a note's title, body, tags, or keywords; it does not claim cross-language semantic translation. Commands, paths, error codes, source IDs, and JSON keys remain unchanged.

## Chat memory

The default is honest session-only memory:

```toml
[memory]
mode = "session"
```

`lbc chat` keeps at most 12 messages and a bounded character budget inside that process. `/clear` clears it; `/exit` or EOF ends the session. Default chat writes no history file, and independent `lbc ask` invocations are stateless. Knowledge added with `lbc add` is separate and is not deleted when chat exits.

Persistent history is explicit opt-in:

```bash
lbc config set memory.mode persistent
lbc history show
lbc history clear
```

Persistent mode saves only bounded, redacted chat messages under `$XDG_DATA_HOME/lbc/history/default.json`, restores them in a new chat process, and lets `/clear` or `lbc history clear` remove them. Project file contents are not persisted by default. `lbc config show` and `lbc doctor` display the effective mode and history location.

## Packages and other commands

Existing Markdown packages remain supported:

```bash
lbc knowledge install ./team-rules
lbc knowledge list
lbc knowledge remove team-rules
```

A package contains `package.toml` with `name`, `version`, and optional `description`, plus valid Markdown documents. Installation validates every document before copying.

Other read-only commands:

```bash
lbc scan --path . --tree
lbc scan --json
lbc config show --json
lbc doctor --json
```

`scan` is a lightweight inventory; it does not imply that every source file was read. JSON commands print one machine-readable value to stdout, while warnings go to stderr. `doctor` exits nonzero when any check fails, including with `--json`; the JSON report is still printed.

## Safety boundaries

`search`, `inspect`, `ask`, `explain`, `scan`, and `doctor` do not change project sources, install packages, run suggested repairs, or execute arbitrary shell commands. Only explicit add/edit/package/config/history-clear operations write their selected data targets. Common keys, bearer values, provider tokens, authorization headers, passwords, database URLs, and known token formats are redacted from remote context and persistent history.

Redaction handles multiple credentials per line, quoted structured keys, URI user information, and Unicode prefixes. Private-key blocks are redacted before passage selection while preserving line numbers. It is pattern-based, not a guarantee that arbitrary secrets are detected: review sensitive notes before opting into remote AI. Config display masks recognizable credentials without changing the saved value.

Input limits: questions/search queries are at most 8 KiB; notes are at most 256 KiB **including serialized metadata**; error-log input is at most 2 MiB. AI user context is limited to 32,000 Unicode characters, with separate per-field limits and retrieved knowledge prioritized ahead of chat history. Each provider request has a 45-second timeout and a 2 MiB response limit.

Note creation publishes a fully written temporary file without overwriting an existing target. Edits atomically replace a validated document; failed editor launches and cancellations clean up temporary files. Builtin overrides preserve retrieval metadata, and changed bodies reset verification to `unverified`. Editor arguments support quoting without launching a shell implicitly. Bounded text readers reject special files and symlinked path components (including parent directories); use real, non-symlinked storage paths.

Production readiness remains **under audit**, especially ignored-file handling, concurrent writers, and complete Thai diagnostic content. See [the readiness audit](docs/readiness-audit.md) before using project context with sensitive repositories.

## Development

```bash
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
```

CLI tests use isolated XDG stores and a local mock HTTP provider; CI needs no API key or live service. Socket-restricted environments must allow loopback for provider tests; those tests fail rather than silently skip. The real timeout regression takes approximately 45 seconds.

To repeat the CLI walkthrough against the release binary, outside the source checkout (the fixtures set their own working directories):

```bash
LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli add_list_inspect_ask_edit_roundtrip_uses_current_content
```

Deferred work includes vector search, embedding services, executable plugin SDKs, model training, automatic repair, autonomous agents, GUI/TUI redesign, npm distribution, and broad compiler coverage.
