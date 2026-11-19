# Five-minute quick start

[Install lbc](installation.md), then open a terminal in a project directory.
The workflow runs offline.

## Set up once

```bash
lbc setup
```

Setup creates the current directory’s `.lbc/knowledge` and personal notes
directories and shows the config path. For unattended setup:

```bash
lbc setup --yes
```

Use `--project PATH` to initialize a specific project.

## Three commands: save, find, reuse

Run these in the same project directory:

```bash
lbc learn --yes --new --problem "Rust E0308 String to &str mismatch" --solution "Borrow the String with &value" --verification "cargo check passed after borrowing"
lbc search E0308
lbc explain --stdin
```

The first command records an example solution in project Markdown. The supplied
check is historical text (`recorded-check`), not a command executed by `learn`.
Only record checks you actually performed when capturing your real work.
This example deliberately uses `--new` because built-in E0308 knowledge already
exists. Prefer interactive `lbc learn` to review similar entries for real work.

Search should show the new `project:` entry among the E0308 results, with an
excerpt and ranking reasons. Explain reads a diagnostic: paste
`error[E0308]: mismatched types` and end input with Ctrl-D on Linux/macOS, or
Ctrl-Z followed by Enter on Windows. It shows local guidance, source IDs, and
available previous solutions and checks. It does not run a repair.

Prefer a log file? Create this tiny sample first, then run `lbc explain build.log`:

Linux/macOS:

```bash
printf '%s\n' 'error[E0308]: mismatched types' > build.log
lbc explain build.log
```

Windows PowerShell:

```powershell
Set-Content -Path build.log -Encoding utf8 -Value 'error[E0308]: mismatched types'
lbc explain build.log
```

Replace `build.log` with your own compiler/runtime output next time. An inadequate
knowledge match is reported; a retrieved historical check is not a fresh test.

## Keep useful knowledge

After solving a diagnostic, run `lbc learn --from build.log`. Supported Rust/Cargo,
TypeScript, Python, Node, and Go diagnostics prefill the problem, error code,
language/tool hints, and sanitized log context offline. Unknown logs ask you to
describe the observed problem. You must supply the real solution; LBC never
invents one.

Interactive capture asks only for missing problem/solution text, then shows any
similar entries. Choose `new` or a listed writable number to replace that entry's
body. Built-ins and packages are read-only. Replacement keeps its metadata and ID;
changed content becomes `unverified`. The current and replacement bodies are
previewed before confirmation; there is no automatic merge or overwrite.

Review the preview and answer `y` at `Save [y/N]` to save. The default is cancel;
`:cancel`, EOF, or Ctrl-C before saving creates no draft or knowledge store.
Use `--user` for personal notes, or `--project PATH` for project notes; no
destination question is asked. Optional cause/check text uses `--cause` and
`--verification` only. Notes remain portable Markdown.

Logs must be regular UTF-8 files of at most 2 MiB; fenced context is truncated to
12 KiB. Shared secret recognition redacts known patterns, not every possible
secret, so review the preview.

Use the actual qualified ID printed by capture or search with `lbc inspect ID`.
`lbc ask "How do I resolve E0308?"` answers from adequate local evidence.
[More examples](usage-examples.md) cover importing Markdown and editing notes.

## Use knowledge in another tool

Use `lbc search E0308 --json` in scripts. External coding agents can use the
same JSON or connect to the [read-only MCP server](mcp.md).

## If something fails

- `lbc` not found: open a new Windows terminal, or add `~/.local/bin` to Unix `PATH`.
- Capture missing in search: use the same working directory or `--project PATH`.
- Permission/storage error: use real, non-symlinked directories owned by your user.
- Run `lbc doctor` for local checks or `lbc COMMAND --help` for flags.

[Detailed behavior and limits](cli-reference.md).
