# Complete libraryCube usage examples

This guide covers every command available in libraryCube 0.3.0. Commands run
offline unless `--ai` is explicitly supplied. Paths, scores, and document counts
in the sample output will vary by machine. An ellipsis (`...`) means that only a
relevant part of a longer result is shown.

## 1. Install and get help

Build and install `lbc` from a source checkout:

```bash
cargo build --release
cargo install --path . --locked
lbc --version
lbc --help
```

Example output:

```text
lbc 0.3.0

libraryCube - terminal knowledge library

Usage: lbc <COMMAND>

Commands:
  add        Add a Markdown entry to your knowledge library
  list       List effective knowledge entries
  inspect    Show a complete knowledge entry
  edit       Replace the body of a writable knowledge entry
  index      Validate and rebuild the in-memory retrieval index
  ask        Answer a question from retrieved local knowledge
  chat       Start a bounded interactive question session
  history    Inspect or clear explicitly persisted chat history
  scan       Inspect the current project
  explain    Explain compiler or runtime errors
  search     Search local technical knowledge
  config     View or modify LBC configuration
  doctor     Check the LBC environment
  knowledge  Manage installable knowledge packages
```

If the binary is not installed on `PATH`, replace `lbc` with
`./target/release/lbc`. Every command has its own help page:

```bash
lbc add --help
lbc explain --help
lbc knowledge --help
```

## 2. End-to-end note workflow

Create a plain Markdown note:

```bash
printf '%s\n' \
  'Change the demo service development port to 4318.' \
  'Restart the development process and confirm it listens on 4318.' \
  > port-note.md
```

Add the note:

```bash
lbc add --id demo-port \
  --title "Demo service port conflict" \
  --kind troubleshooting \
  --file ./port-note.md
```

```text
Added user:demo-port
  Title: Demo service port conflict
  Kind: troubleshooting
  Stored at: /home/alice/.local/share/lbc/notes/demo-port.md
```

List, search, inspect, and ask:

```bash
lbc list
lbc search "demo service port conflict"
lbc inspect user:demo-port
lbc ask "How do I resolve the demo service port conflict?"
```

The first answer includes `4318` and cites `user:demo-port`. Replace the body:

```bash
printf '%s\n' \
  'Change the demo service development port to 4429.' \
  'Restart the development process and confirm it listens on 4429.' \
  > updated-port-note.md

lbc edit user:demo-port --file ./updated-port-note.md
lbc ask "How do I resolve the demo service port conflict?"
```

```text
Updated user:demo-port
  /home/alice/.local/share/lbc/notes/demo-port.md

libraryCube answer

Retrieved guidance:

[user:demo-port] Demo service port conflict
Change the demo service development port to 4429. Restart the development
process and confirm it listens on 4429.
Status: retrieved guidance; not verified against this project
```

## 3. `add`: import knowledge

The supported kinds are `note`, `concept`, and `troubleshooting`.

Let `lbc` derive an ID from the title:

```bash
lbc add --title "Rust ownership basics" \
  --kind concept \
  --file ./ownership.md
```

Provide an ID for deterministic scripts:

```bash
lbc add --id rust-ownership \
  --title "Rust ownership basics" \
  --kind concept \
  --file ./ownership.md
```

Read the body from standard input:

```bash
printf '%s\n' 'Use cargo check to type-check without producing a release binary.' |
  lbc add --id cargo-check \
    --title "Check Rust with cargo check" \
    --stdin
```

Store a note for one project. The project directory must already exist:

```bash
mkdir -p ./demo-project
lbc add --id local-port \
  --title "Port used by this project" \
  --file ./port-note.md \
  --project ./demo-project
```

```text
Added project:local-port
  Title: Port used by this project
  Kind: note
  Stored at: /work/demo-project/.lbc/knowledge/local-port.md
```

Request JSON output:

```bash
lbc add --id json-note \
  --title "JSON example" \
  --file ./port-note.md \
  --json
```

Selected fields from the output:

```json
{
  "metadata": {"id":"json-note","kind":"note","title":"JSON example"},
  "title": "JSON example",
  "body": "Change the demo service development port to 4318.\n...",
  "source": "user",
  "source_id": "user:json-note",
  "kind": "note",
  "verification_status": "unverified",
  "writable": true,
  "effective": true
}
```

The actual JSON also includes the source locator. `add` never overwrites an
existing target and rejects duplicate IDs within the same source.

## 4. Markdown metadata

A structured entry uses YAML frontmatter followed by a nonempty Markdown body:

```markdown
---
id: demo-port
title: Demo service port conflict
kind: troubleshooting
language: en
tool: docker
category: networking
error_code: EADDRINUSE
tags: [demo, port]
keywords: [address-in-use, port-conflict]
verification_status: unverified
---
# Symptoms

The service reports that its port is already in use.

# Cause

Another process may be listening on the same port.

# Suggested solution

Change the development port to 4318.

# Verification steps

Use the project's own tooling to confirm that the service listens on 4318.

# References

Add a URL or document that the note author actually reviewed.
```

Supported metadata includes `id`, `title`, `kind`, `language`, `tool`,
`category`, `error_code`, `tags`, `keywords`, `overrides`, and
`verification_status`. Verification values mean:

- `unverified`: no recorded successful check.
- `user-reported`: the author recorded a user's successful result.
- `recorded-check`: the author says a check and its evidence were recorded.

These labels do not claim that `lbc` ran a command. Editing the body resets its
verification status to `unverified`.

## 5. `list` and `inspect`: browse entries

List effective entries, optionally including project sources:

```bash
lbc list
lbc list --project ./demo-project
lbc list --project ./demo-project --json
```

```text
libraryCube entries

user:demo-port
  Demo service port conflict [troubleshooting; unverified]
  /home/alice/.local/share/lbc/notes/demo-port.md

builtin:rust-e0308
  Rust E0308 - Mismatched types [troubleshooting; unverified]
  embedded:rust/E0308.md
```

Inspect the complete body and provenance:

```bash
lbc inspect user:demo-port
lbc inspect project:local-port --project ./demo-project
lbc inspect builtin:rust-e0308
lbc inspect user:demo-port --json
```

```text
user:demo-port
Title: Demo service port conflict
Kind: troubleshooting
Source: user
Locator: /home/alice/.local/share/lbc/notes/demo-port.md
Verification status: unverified
Writable: true
Effective: true

Change the demo service development port to 4429.
```

Source-qualified IDs have these forms:

- `builtin:ID`: knowledge embedded in the executable.
- `user:ID`: a user note.
- `project:ID`: a project note.
- `package:NAME:ID`: a note from an installed package.

A bare ID such as `lbc inspect demo-port` works only when unambiguous. If an ID
exists in multiple sources, `lbc` reports the candidates instead of silently
choosing one.

## 6. `edit` and built-in overrides

Replace a writable entry's body from a file or open an editor:

```bash
lbc edit user:demo-port --file ./updated-port-note.md
lbc edit user:demo-port --file ./updated-port-note.md --json

export EDITOR=nvim
lbc edit user:demo-port

export EDITOR='code --wait'
lbc edit user:demo-port
```

Edit a project entry:

```bash
lbc edit project:local-port \
  --project ./demo-project \
  --file ./updated-port-note.md
```

Built-ins are read-only. Create an explicit user override:

```bash
lbc inspect builtin:rust-e0308
lbc edit builtin:rust-e0308 --override --file ./our-e0308.md
lbc inspect user:rust-e0308
```

```text
Updated user:rust-e0308
  /home/alice/.local/share/lbc/notes/rust-e0308.md
```

The override records its target and preserves retrieval metadata. An edit is
validated before atomic replacement; an invalid body or failed editor leaves the
original entry intact.

## 7. `search` and `index`: retrieve and validate

```bash
lbc search "demo service port conflict"
lbc search "E0308 mismatched types"
lbc search "port conflict" --project ./demo-project
lbc search "E0308" --json
```

```text
Knowledge Search

Query
  E0308 mismatched types

Results

1. Rust E0308 - Mismatched types
   Match: exact error code
   Source: builtin:rust-e0308
   Locator: embedded:rust/E0308.md
   Excerpt: E0308 occurs when an expression has a type different from the
   expected type. Read the `expected ... found ...` pair in the diagnostic...
```

Validate documents and rebuild retrieval state in memory:

```bash
lbc index
lbc index --project ./demo-project
lbc index --project ./demo-project --json
```

```text
Knowledge index
  Documents: 14
  Effective: 13
  Invalid: 0
  Retrieval state: rebuilt in memory (no persistent cache)
```

If invalid Markdown exists, `index` lists each path and exits nonzero. Valid
documents remain available. External Markdown edits are loaded by the next
command; `index` does not create a persistent cache.

## 8. `ask`: answer from local knowledge

```bash
lbc ask "What causes a detached HEAD?"
lbc ask "How do I fix E0308 mismatched types?"
lbc ask "Why does src/main.rs report a type mismatch?" \
  --project ./demo-project
lbc ask "E0308 mismatched types" --json
```

```text
libraryCube answer

Retrieved guidance:

[builtin:rust-e0308] Rust E0308 - Mismatched types
E0308 occurs when an expression has a type different from the expected type.
Read the `expected ... found ...` pair and make both sides agree...
Status: retrieved guidance; not verified against this project
```

`ask` retrieves before answering and cites the passages it used. When no adequate
match exists, it provides investigation steps instead of inventing a fix. Without
`--ai`, it makes no provider request. Separate `ask` invocations do not share
chat history.

## 9. `explain`: analyze compiler or runtime errors

Read a diagnostic from a pipeline:

```bash
printf '%s\n' 'error[E0308]: mismatched types' |
  lbc explain --stdin
```

Read a log file and optionally use bounded project evidence:

```bash
lbc explain ./build.log
lbc explain ./build.log --project ./demo-project
lbc explain ./build.log --project ./demo-project --verbose
lbc explain ./build.log --project ./demo-project --json
```

Abbreviated output:

```text
libraryCube diagnostic

✗ E0308 - mismatched types

Project
  Rust / Cargo

Evidence
  - The inventory scan counted 8 eligible project files; this does not mean
    their contents were analyzed.

Cause
  No hardcoded diagnostic rule matched. The guidance below comes from retrieved
  local knowledge and has not been verified against this project.

Suggested fix
  1. [builtin:rust-e0308] Read the expected/found pair and make both sides agree...

Knowledge
  Rust E0308 - Mismatched types [builtin:rust-e0308; status: unverified]
    E0308 occurs when an expression has a type different from the expected type...

Confidence
  Retrieved knowledge (unverified)
```

`explain` parses diagnostics, uses the same retrieval path as `search` and `ask`,
and selects bounded source excerpts and recognized manifests. It never runs
`cargo`, `npm`, Docker, or a suggested repair command. With multiple diagnostics,
the current text view expands the first and reports how many others were found.

## 10. `scan`: inventory a project

```bash
lbc scan --path .
lbc scan --path ./demo-project --tree
lbc scan --path ./demo-project --json
```

```text
Project detected

Language
  Rust

Build system
  Cargo

Source directories
  src/

Project Tree

demo-project
├── Cargo.toml
└── src/
    └── main.rs

Scan Summary

Project root
  /work/demo-project

Eligible files inventoried
  2

File contents read
  1

Rust source files
  1

Configuration files
  1

Knowledge documents
  0
```

The scanner detects common languages, build systems, containers, and a limited
set of frameworks. Inventory counts do not imply that every file body was read.

## 11. Optional AI

No outbound call occurs unless `--ai` is present:

```bash
lbc ask "How do I resolve the demo port conflict?" --ai
lbc explain ./build.log --project ./demo-project --ai
lbc chat --project ./demo-project --ai
```

### Ollama or another OpenAI-compatible endpoint

```bash
lbc config set ai.provider openai-compat
lbc config set ai.model qwen2.5-coder:7b
lbc config set ai.base_url http://127.0.0.1:11434/v1
```

Prepare and start the provider separately, for example:

```bash
ollama pull qwen2.5-coder:7b
ollama serve
```

An AI-enhanced answer adds output like:

```text
AI analysis (openai-compat / qwen2.5-coder:7b)
The retrieved note identifies a type mismatch. Compare the expected and found
types, then choose an explicit conversion that preserves the intended ownership.

AI confidence: medium
```

### OpenRouter

```bash
lbc config set ai.provider openrouter
lbc config set ai.model openai/gpt-4o-mini
export OPENROUTER_API_KEY='YOUR_API_KEY'
lbc ask "Explain Rust ownership" --ai
```

Do not commit API keys. Selected passages and evidence are bounded and redacted,
but pattern-based redaction cannot recognize every secret. If a provider fails,
the offline answer remains available and JSON includes `ai_error`:

```text
! AI unavailable: failed to reach the OpenAI-compatible endpoint; showing the
offline answer
```

## 12. `chat`, session memory, and history

```bash
lbc chat
lbc chat --project ./demo-project
lbc chat --project ./demo-project --ai
```

```text
libraryCube chat — /clear clears this session; /exit ends it
> What does E0308 mean?
Retrieved guidance:

[builtin:rust-e0308] Rust E0308 - Mismatched types
...
> /clear
Session context cleared.
> /exit
```

`/clear` clears context, `/exit` ends the session, and EOF (`Ctrl-D` on Unix)
also exits. Session history is bounded by message count and characters.

The default memory mode exists only inside the active process:

```bash
lbc config set memory.mode session
```

Enable disk persistence explicitly, then inspect or clear it:

```bash
lbc config set memory.mode persistent
lbc chat
lbc history show
lbc history show --json
lbc history clear
lbc history clear --json
```

```text
Persistent history
  Path: /home/alice/.local/share/lbc/history/default.json
  Exists: true
  Messages: 4

Cleared persistent history at /home/alice/.local/share/lbc/history/default.json
```

JSON status:

```json
{"path":"/home/alice/.local/share/lbc/history/default.json","exists":true,"messages":4}
```

Persistent messages are bounded and redacted. Project contents are not persisted
by default. `/clear` in persistent mode also deletes stored history. Knowledge
added with `add` is separate and is never removed when chat exits.

## 13. Language and configuration

Select English, Thai, or automatic output:

```bash
lbc config set output.language en
lbc config set output.language th
lbc config set output.language auto
```

```text
Proposed configuration change
  output.language = en

✓ Set output.language

Config file
  /home/alice/.config/lbc/config.toml
```

`auto` chooses Thai UI text when the current question contains Thai characters,
and otherwise chooses English. It does not translate a note's body. Commands,
paths, error codes, source IDs, and JSON keys remain unchanged.

Configure scanner behavior and view effective settings:

```bash
lbc config set scanner.max_file_size_kb 512
lbc config set scanner.ignore_hidden true
lbc config show
lbc config show --json
LBC_CONFIG=./test-config.toml lbc config show
```

```text
libraryCube configuration

Config file
  /home/alice/.config/lbc/config.toml

Output
  Language:        en

Scanner
  Max file size:   512 KB
  Ignore hidden:   yes

Memory
  Mode:            session
  History:         /home/alice/.local/share/lbc/history/default.json
  Persistence:     disabled; chat context exists only in the active process

AI
  Provider:        off (deterministic mode)
```

Supported `config set` keys:

```text
output.language
scanner.max_file_size_kb
scanner.ignore_hidden
memory.mode
ai.provider
ai.model
ai.base_url
```

Disable AI configuration with `lbc config set ai.provider off`. The default
configuration path is `$XDG_CONFIG_HOME/lbc/config.toml`, falling back to
`~/.config/lbc/config.toml`.

## 14. `doctor`: check the environment

```bash
lbc doctor
lbc doctor --json
```

Example healthy output:

```text
libraryCube doctor

✓ Configuration
  using safe defaults

✓ Project detection
  Rust / Cargo at /work/demo-project

✓ Local knowledge
  12 documents available

✓ AI provider
  off (deterministic mode); set ai.provider to enable AI explanations

✓ Session memory
  session-only mode; no history is written to /home/alice/.local/share/lbc/history/default.json

Status
  healthy
```

`doctor` returns a nonzero status if any check fails, even though `--json` still
prints a complete report:

```bash
if lbc doctor; then
  printf '%s\n' 'libraryCube is ready'
else
  printf '%s\n' 'libraryCube needs attention'
fi
```

## 15. `knowledge`: installable packages

A package contains `package.toml` and one or more valid Markdown documents:

```text
team-rules/
├── package.toml
├── rust.md
└── docker.md
```

Example `team-rules/package.toml`:

```toml
name = "team-rules"
version = "1.0.0"
description = "Shared team knowledge"
```

Example `team-rules/rust.md`:

```markdown
---
id: team-rust-check
title: Team Rust checks
kind: troubleshooting
tags: [rust]
---
Use `cargo check --locked` before producing a release build.
```

Install, list, search, inspect, and remove:

```bash
lbc knowledge install ./team-rules
lbc knowledge list
lbc search "cargo check locked"
lbc inspect package:team-rules:team-rust-check
lbc knowledge remove team-rules
```

```text
Installed knowledge package

  Name:        team-rules
  Version:     1.0.0
  Documents:   2
  Location:    /home/alice/.local/share/lbc/knowledge/team-rules

Knowledge Packages

Data directory
  /home/alice/.local/share/lbc/knowledge

Installed
  team-rules 1.0.0 (2 documents)
    Shared team knowledge

✓ Removed knowledge package team-rules

Location
  /home/alice/.local/share/lbc/knowledge/team-rules
```

Every document is validated before installation. Packages live under
`$XDG_DATA_HOME/lbc/knowledge`, with a fallback under `~/.local/share/lbc`.

## 16. JSON in scripts

`add`, `list`, `inspect`, `edit`, `index`, `ask`, `search`, `explain`, `scan`,
`config show`, `doctor`, and `history` support JSON through their respective
flags. Examples with `jq`:

```bash
lbc search "E0308" --json |
  jq '.[0] | {source_id, title, score, excerpt}'
```

```json
{
  "source_id": "builtin:rust-e0308",
  "title": "Rust E0308 - Mismatched types",
  "score": 1231,
  "excerpt": "E0308 occurs when an expression has a type different..."
}
```

```bash
lbc ask "E0308" --json |
  jq '{status: .answer_status, sources: [.passages[].source_id]}'
```

```json
{
  "status": "retrieved_guidance",
  "sources": ["builtin:rust-e0308"]
}
```

```bash
lbc doctor --json |
  jq '.checks[] | {name, ok, detail}'
```

A JSON command writes one object or array to stdout. Warnings and explicit AI
fallback notices may be written to stderr.

## 17. Storage, limits, and safety boundaries

| Data | Default location |
| --- | --- |
| User notes | `$XDG_DATA_HOME/lbc/notes` |
| Installed packages | `$XDG_DATA_HOME/lbc/knowledge` |
| Persistent history | `$XDG_DATA_HOME/lbc/history/default.json` |
| Project notes | `<project>/.lbc/knowledge` |
| Configuration | `$XDG_CONFIG_HOME/lbc/config.toml` |

When XDG variables are absent, user data falls back to `~/.local/share/lbc` and
configuration falls back to `~/.config/lbc`.

Important limits:

- `ask` and `search` query: 8 KiB.
- Knowledge document including metadata: 256 KiB.
- Diagnostic log: 2 MiB.
- AI user context: 32,000 Unicode characters.
- Provider response: 2 MiB.
- Provider timeout: 45 seconds.

Special files and symlinked components in knowledge paths are rejected. `search`,
`inspect`, `ask`, `explain`, `scan`, and `doctor` do not modify project sources or
execute suggested commands. Data can leave the machine only after explicit
`--ai` use. Review sensitive content before using a remote provider because
redaction is pattern-based rather than a universal secret detector.

See the [readiness audit](readiness-audit.md) for known limitations.
