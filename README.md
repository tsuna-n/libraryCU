# libraryCube

libraryCube (`lbc`) is a terminal knowledge library. You can save ordinary Markdown notes, find and inspect them, ask questions against retrieved passages, and use the same knowledge when explaining diagnostics. Retrieval and cited offline answers work without an API key, network connection, model, vector database, or Python service. Optional AI expands the retrieved answer; it never replaces retrieval.

The Rust package and crate are named `librarycube`; the executable is `lbc`.

For copy-paste examples covering every command, see the
[complete usage examples](docs/usage-examples.md).
The [0.4.0 release audit](docs/release-0.4.0.md) records verification, recovery,
cross-platform validation, and remaining limitations.
See the [changelog](CHANGELOG.md) for the version's changes.

## Install

Run the installation script to build and install `lbc` to `~/.local/bin`:

```bash
./install.sh
```

Or customize the installation:

```bash
./install.sh --system          # Install system-wide to /usr/local/bin
./install.sh --prefix ~/bin    # Install to custom directory
./install.sh --uninstall       # Remove installed binary
```

Alternatively, build and install manually with Cargo:

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

Existing metadata remains supported: `id`, `title`, `title_th`, `language`, `tool`, `category`, `error_code`, `tags`, and `keywords`. `language` describes technical material; it is separate from the UI language. `title_th` is an optional Thai display title.

Built-in bilingual notes keep each translation in an explicit block:

```markdown
<!-- lbc:en -->
# English title

English guidance.

<!-- lbc:th -->
# ชื่อภาษาไทย

คำแนะนำภาษาไทย
```

When both markers are present, `ask` and `search` select only the configured or automatically detected output language. Notes without these markers remain compatible and are returned as written.

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

Ordinary Python tracebacks and syntax-error frames are parsed too, including
ANSI-colored input and chained exceptions. The last reported frame is retained;
it may belong to a dependency. This does not establish a root cause. Complex
ExceptionGroup tree formatting and arbitrary log prefixes are not fully supported.
TypeScript `tsc` locations (both `(line,column)` and `:line:column` formats),
Go compiler file diagnostics, and Node error headers with a JavaScript/TypeScript
stack frame or explicit Node error code are also parsed. Paths with Windows drive
letters, Unicode, ANSI colors, and CRLF logs are supported. Selected known rules
cover TypeScript type/module errors, Node module resolution, and Go undefined
identifiers. These rules supply general guidance, not a project-verified cause.

## Propose and apply a small patch

```bash
lbc fix build.log --project ./my-project             # Offline guidance
lbc fix build.log --project ./my-project --ai        # Preview a replacement
lbc fix build.log --project ./my-project --ai --apply # Generate and apply
lbc fix --stdin --project ./my-project --ai --json < build.log
lbc fix build.log --project ./my-project --ai --apply --verify "cargo check --offline"
lbc rollback fix-ABC123 --project ./my-project --json # Use the emitted recovery_id
```

`fix --ai` retrieves adequate local knowledge before requesting one exact
`before`/`after` replacement near the first diagnostic's line. The provider cannot
choose another target file. `--apply` requires `--ai`; each invocation generates
a fresh proposal, so an apply invocation can differ from an earlier preview.
Without `--verify`, proposed and applied patches remain `unverified`. Add
`--verify "COMMAND"` with `--ai --apply` to run your chosen executable and arguments
in the exact project directory. Arguments use shell-style quoting on every OS;
quote Windows paths with spaces, preferably using forward slashes. No shell is
launched implicitly, so pipes, redirection, and variable expansion are not shell
operations. On Windows, invoke an executable such as `node.exe` directly; shell
scripts require an explicitly selected interpreter. The provider cannot select
the command. Only use verification commands you trust: they inherit your
environment and can execute project code, access the network, or change files.

The deadline is 120 seconds; `--verify-timeout 30` changes it (1–3600 seconds).
Standard input is closed. Each output stream retains at most 32 KiB, redacts
recognizable secrets, and reports truncation. A passed command records `passed`
only if the patched file still matches. This proves that particular command
exited successfully, not general program correctness. On timeout LBC stops the
direct child; it does not guarantee termination of detached descendant processes.

Every CLI application first saves a private recovery record under
`<project>/.lbc/fixes/<recovery_id>.json`, containing the original and applied
source snapshots. A failed, unavailable, or timed-out verifier triggers rollback
and exits nonzero. `lbc rollback ID --project PATH` also restores an applied patch
in a later invocation without AI. Both forms refuse to overwrite source that no
longer matches the recorded applied content. Only the selected file is restored;
side effects of the verification command are outside the rollback. Records remain
for manual recovery and can be removed after you no longer need them. Keep
`.lbc/fixes/` out of version control; records are local source backups and are not
authenticated against local tampering. Power-loss durability is not guaranteed.

The exact `--project` directory is the boundary. Targets must be regular UTF-8
files of at most 256 KiB; the seven-line excerpt is at most 8 KiB, and provider
patch JSON is at most 8 KiB. Replacements must match uniquely, intersect the
reported line, and keep the resulting file within 256 KiB. Symlinked paths,
hidden/generated targets, recognized `.gitignore` matches, and targets containing
recognizable secrets are refused. Source content is checked again before atomic
replacement, and file permissions are retained. As with note editing, this is
not a guarantee against every concurrent writer or hostile directory race.

JSON reports contain `status` (`offline_guidance`, `proposed`, `applied`,
`verified`, `rolled_back`, or `failed`), `applied`, `verification_status`,
`guidance`, `patch`, `error`, `recovery_id`, `verification`, and `rollback_status`.
Verification status is `unverified`, `passed`, `failed`, `timed_out`, or `error`;
the optional verification object includes exit code, captured stdout/stderr,
truncation, and execution error. `applied` becomes false after successful rollback.
An AI/patch failure preserves offline guidance and exits nonzero; preflight
input/configuration errors go to stderr. Without `--ai`, fix makes no provider
connection and changes no project files.

## Optional AI

No outbound provider call occurs unless `--ai` is present:

```bash
lbc ask "How do I resolve the demo port conflict?" --ai
lbc explain build.log --ai
lbc chat --ai
```

Configure a named provider preset (`openai`, `zai`/`glm`, `ollama`), OpenRouter, or a custom OpenAI-compatible endpoint:

```toml
[ai]
provider = "zai" # off | openai | zai | glm | ollama | openrouter | openai-compat
# Optional for named presets (sensible defaults provided):
# model = "glm-4-flash"
# base_url = "https://open.bigmodel.cn/api/paas/v4"
```

### Supported AI Providers

| Provider | Description | Default Endpoint | Default Model | Environment Variable |
|---|---|---|---|---|
| `openai` | OpenAI Platform | `https://api.openai.com/v1` | `gpt-4o-mini` | `OPENAI_API_KEY` |
| `zai` / `glm` | Zhipu AI (BigModel) | `https://open.bigmodel.cn/api/paas/v4` | `glm-4-flash` | `ZAI_API_KEY` or `GLM_API_KEY` |
| `ollama` | Local Ollama server | `http://localhost:11434/v1` | `llama3.2` | None required (`OLLAMA_API_KEY` optional) |
| `openrouter` | OpenRouter gateway | `https://openrouter.ai/api/v1` | Set via `ai.model` | `OPENROUTER_API_KEY` |
| `openai-compat` | Custom self-hosted server | Configured via `ai.base_url` | Configured via `ai.model` | `OPENAI_API_KEY`, `ZAI_API_KEY`, or `GLM_API_KEY` |

Requests have a timeout and bounded input/output budgets. In terminal mode, `--ai` returns only the concrete edits to make (`Change`/`From`/`To`, or `แก้`/`จาก`/`เป็น`) and does not print the full offline explanation first. Selected note excerpts, source IDs, and bounded project evidence are redacted before sending. Notes are labeled as untrusted data in the prompt. If the provider fails, stderr and JSON expose the failure while the offline answer remains available. Validate your setup anytime with `lbc doctor`.

## English and Thai output

Set the output language explicitly or use automatic selection:

```bash
lbc config set output.language th
lbc ask "แก้ปัญหาพอร์ตของ demo service อย่างไร"
lbc config set output.language auto
```

Supported values are `en`, `th`, and `auto`. `auto` selects Thai when the current question contains Thai characters and otherwise falls back to English. Built-in notes include clean English and Thai sections, so their title and excerpt follow the selected language. Retrieval supports Unicode and matches Thai terms present in a note's title, body, tags, or keywords; unmarked user notes are not translated automatically in offline mode. Commands, paths, error codes, source IDs, and JSON keys remain unchanged.

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

The source checkout includes 40 bilingual Python/FastAPI notes in
`packages/python-fastapi-basics`. Install them explicitly:

```bash
lbc knowledge install ./packages/python-fastapi-basics
lbc search "ModuleNotFoundError"
lbc ask "Python CustomWidgetError failed in worker.py"
```

They are an optional local package, not embedded knowledge. When a Python error
has no adequate specific match, `ask` may cite the generic traceback playbook
with `answer_status: "general_guidance"`. It explicitly states that evidence for
a specific fix is insufficient; `fix --ai` does not use that fallback as adequate
evidence. English/Thai selection applies to the bilingual passages.

Other read-only commands:

```bash
lbc scan --path . --tree
lbc scan --json
lbc config show --json
lbc doctor --json
```

`scan` is a lightweight inventory; it does not imply that every source file was read. JSON commands print one machine-readable value to stdout, while warnings go to stderr. `doctor` exits nonzero when any check fails, including with `--json`; the JSON report is still printed.

## Safety boundaries

`search`, `inspect`, `ask`, `explain`, `scan`, and `doctor` do not change project sources, install packages, run suggested repairs, or execute arbitrary shell commands. Explicit add/edit/package/config/history-clear operations, `fix --ai --apply`, and `rollback` write their selected data targets. `fix --verify` additionally executes the user-selected verification command. Common keys, bearer values, provider tokens, authorization headers, passwords, database URLs, and known token formats are redacted from remote context and persistent history.

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

CircleCI runs these gates for every branch and pull request, packages the tested
Linux release binary, and publishes it to GitHub Releases for matching version
tags. See [CircleCI CI/CD setup](docs/circleci.md) for the one-time token setup
and release procedure.

CLI tests use isolated XDG stores and a local mock HTTP provider; CI needs no API key or live service. Socket-restricted environments must allow loopback for provider tests; those tests fail rather than silently skip. The real timeout regression takes approximately 45 seconds.

To repeat the CLI walkthrough against the release binary, outside the source checkout (the fixtures set their own working directories):

```bash
LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli add_list_inspect_ask_edit_roundtrip_uses_current_content
```

Deferred work includes vector search, embedding services, executable plugin SDKs, model training, automatic repair, autonomous agents, GUI/TUI redesign, npm distribution, and broad compiler coverage.
