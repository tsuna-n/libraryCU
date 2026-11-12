# Five-minute quick start

[Install lbc](installation.md), then open a terminal in a project directory.
The following workflow uses no AI, API key, or network connection.

## Set up once

```bash
lbc setup
```

Choose offline mode. Setup creates the current directory’s `.lbc/knowledge` and
personal notes directories and shows the config location. It never asks for or
saves API keys; it only checks their presence in the process environment and
makes no network request. Existing config is preserved unless
you explicitly choose changes. For unattended offline defaults on a fresh install:

```bash
lbc setup --yes
```

An existing provider setting is retained; normal knowledge commands still make
no provider request without `--ai`. Use `--provider off` to explicitly disable AI.

## Three commands: save, find, reuse

Run these in the same project directory:

```bash
lbc learn --yes --problem "Rust E0308 String to &str mismatch" --solution "Borrow the String with &value" --verification "cargo check passed after borrowing"
lbc search E0308
lbc explain --stdin
```

The first command records an example solution in project Markdown. The supplied
check is historical text (`recorded-check`), not a command executed by `learn`.
Only record checks you actually performed when capturing your real work.

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

`lbc learn` without flags asks for the problem, optional cause, solution, and
optional recorded check, then previews the destination before confirmation.
Use `--user` for personal notes, or `--project PATH` to choose an exact project.
Capture never overwrites an existing entry. Notes remain portable Markdown.

Use the actual qualified ID printed by capture or search with `lbc inspect ID`.
`lbc ask "How do I resolve E0308?"` answers from adequate local evidence.
[More examples](usage-examples.md) cover importing Markdown and editing notes.

## Optional AI

Offline mode is the default for a fresh config. To opt in later:

```bash
lbc setup --provider openai
```

Setup configures a provider/model, not credentials. Load `OPENAI_API_KEY` into your
process environment using your secret manager or a masked prompt; never put a
real key in setup arguments, shell history, config, or committed files.
For example, Bash can read a key without echoing or storing it in shell history:

```bash
read -r -s -p 'OpenAI API key: ' OPENAI_API_KEY
export OPENAI_API_KEY
printf '\n'
lbc ask "How do I resolve E0308?" --ai
unset OPENAI_API_KEY
```

Windows PowerShell (masked input, current process only):

```powershell
$credential = Get-Credential -UserName 'api' -Message 'Enter your API key as the password'
$env:OPENAI_API_KEY = $credential.GetNetworkCredential().Password
lbc ask "How do I resolve E0308?" --ai
Remove-Item Env:OPENAI_API_KEY
Remove-Variable credential
```

`--ai` sends bounded, redacted retrieved passages and project evidence to the
selected provider. Pattern-based redaction cannot detect every secret: review
sensitive content first. Ollama is optional local AI; no provider is needed for
core workflows. See [provider options](cli-reference.md#optional-ai).

## If something fails

- `lbc` not found: open a new Windows terminal, or add `~/.local/bin` to Unix `PATH`.
- Capture missing in search: use the same working directory or `--project PATH`.
- Permission/storage error: use real, non-symlinked directories owned by your user.
- Run `lbc doctor` for local checks or `lbc COMMAND --help` for flags.

Normal `doctor` does not probe providers. Connectivity checks require the
explicit `--connectivity` flag. [Detailed behavior and limits](cli-reference.md).
