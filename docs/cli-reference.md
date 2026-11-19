# libraryCube CLI reference

Detailed behavior, compatibility, and safety limits. Start with the
[quick start](quick-start.md) for the daily workflow and
[installation guide](installation.md) for binary installation.
Run `lbc COMMAND --help` for the complete flag list.

## Developer tools and project memory

`lbc mcp --project PATH [--project-only]` starts an optional read-only MCP stdio
server. Its three tools search, inspect, and explain using the same local
retrieval and diagnostic code as the CLI. The project directory is required and
fixed for the session. See [MCP, Codex CLI, and OpenCode](mcp.md).

`search`, `ask`, and `explain` accept `--project-only` to exclude all shared
knowledge stores. Default scope remains project plus user/package/built-in
knowledge. Matching project language/tool/framework metadata helps rank already
relevant results; historical verification is not verification of a new problem.
See [project memory](project-memory.md).

`lbc explain --text TEXT [--project PATH]` accepts selected editor text as one
literal argument. It conflicts with a positional log file and `--stdin`, rejects
empty input, and retains the 2 MB input bound. For large or sensitive text,
prefer stdin or a saved log over process arguments. Optional [VS Code and Neovim
examples](editor-integration.md) send selections and logs without shell evaluation.

## Output and troubleshooting

`--quiet` (`-q`) hides optional warnings, keeping command results,
errors, previews and confirmation prompts. `--no-color` disables generated color
and terminal escape sequences; the presence of `NO_COLOR` does the same. These
flags work before or after a subcommand. Results go to stdout. Warnings, errors,
and chat prompts go to stderr.

`search --limit N` and `list --limit N` return at most N matches/entries, where N
is a positive integer. Defaults are unchanged: search text shows at most 10
matches; search JSON and list return all entries. Limiting JSON keeps the existing
array and entry fields, without adding a wrapper or truncation metadata.

```bash
lbc search E0308 --limit 3 --no-color
lbc --quiet list --limit 5 --json > entries.json
lbc doctor --json > doctor.json
```

JSON keeps warning and error fields even with `--quiet`. Runtime errors exit 1,
invalid CLI arguments exit 2, and help/version exit 0. A failing doctor still
writes its JSON report before exiting 1. Failures before a report can be built
leave stdout empty and explain the cause, fix and next command on stderr.
See the [upgrade notes](#upgrading-to-the-offline-cli) for removed JSON fields.

Error guidance and doctor use `output.language = "en"` or `"th"`; `"auto"` uses
`LC_ALL`, `LC_MESSAGES`, then `LANG` for these UI messages. For example,
`lbc config set output.language th` selects Thai guidance (`สาเหตุ`, `วิธีแก้`,
`คำสั่งถัดไป`) while keeping command names and JSON keys stable. Invalid config
falls back to English. TOML/YAML parse diagnostics omit source values to avoid
printing credentials; YAML diagnostics retain the error location. Recognizable
credentials are masked in error messages.

`doctor` checks the configuration, readable knowledge store directories,
document validity, effective Git ignore rules and tracked private `.lbc` paths
without creating stores, changing notes, opening persistent
history, or repairing files. Missing optional stores are normal on first use;
follow `lbc setup` guidance when you want to create them. Invalid paths,
permissions, symlinks and metadata receive repair instructions. Doctor JSON
reports `healthy`, `health_scope`, and `checks`.

## Setup

`lbc setup` shows the selected paths and asks for confirmation.
`lbc setup --yes` skips confirmation, uses defaults for a fresh installation,
and preserves existing settings. It creates the exact project directory’s
`.lbc/knowledge` and the personal notes directory.
It also creates `.lbc/.gitignore` with private defaults when that file is absent;
existing ignore rules are preserved. See [Git safety](#git-safety).

Use `--project PATH` to select a directory. Use `--yes --json` for unattended
setup with machine-readable output. Setup JSON reports `os`, `project`,
`stack`, `config_path`, `project_knowledge`, and `personal_knowledge`.

Setup displays the resolved knowledge and configuration paths. `LBC_CONFIG`,
`XDG_CONFIG_HOME` and `XDG_DATA_HOME` keep their existing meanings. Without these
or `HOME`, a fresh Windows installation uses `.config/lbc` and `.local/share/lbc`
under `USERPROFILE`, so personal notes follow you between projects. Existing
working-directory fallback stores are preserved in place; no files are moved.
Configuration updates preserve unknown TOML keys, though comments/formatting may
be rewritten.

## Capture a solved problem

Run `lbc learn` for guided capture, or `lbc learn --from FILE` to prefill a supported
Rust/Cargo, TypeScript, Python, Node, or Go diagnostic offline. Log parsing supplies
the observed problem, error code, language/tool hints, and sanitized fenced
context. Unknown diagnostics require an observed problem from you. The solution
must describe the fix that actually worked.

```bash
lbc learn
lbc learn --from build.log
lbc learn --yes --new --problem 'Rust E0308 String to &str mismatch' \
  --cause 'Function expected &str but database value was String' \
  --solution 'Borrowed the database value using &value' \
  --verification 'cargo test' --project ./backend
lbc search E0308 --project ./backend
```

Interactive capture asks for Problem only when missing, then Solution only when
missing, then a duplicate choice when matches exist. It previews the content and
destination before `Save [y/N]`; saving requires `y`, and the default cancels.
`:cancel` at any prompt, EOF, or Ctrl-C before saving creates no draft or store.
The default destination is the current directory's `.lbc/knowledge`;
`--project PATH` selects a project and `--user` selects personal notes. There is no
destination prompt.

Up to five effective matches are ranked by normalized exact title/problem,
meaningful token overlap, then error code with compatible language. Built-ins
and packages remain read-only suggestions. Choose `new` to save separately, or a
listed writable number to explicitly replace that entry's body. Replacement
preserves its metadata, ID, and creation time; a changed body becomes `unverified`.
Both current and replacement bodies are previewed and require `y`. Nothing is
automatically merged or overwritten.

`--cause` and `--verification` are optional flags, never extra questions. A
supplied check is `recorded-check`, otherwise `unverified`; checks are never run.
Other capture flags include `--title`, `--id`, `--language`, `--tool`, `--framework`,
`--error-code`, `--tag`, `--symptoms`, `--context`, and `--reference`. `ask` and
`explain` show recorded problem/solution/check sections with sources.

Scripts require `--yes --solution ...` and either `--problem ...` or `--from FILE`.
An unknown log also needs `--problem`. Similar knowledge stops scripted capture
unless `--new` explicitly requests a separate entry. The E0308 example needs
`--new` because built-in knowledge matches; prefer interactive review for normal
use. `--json` requires `--yes`; stdout contains JSON, while previews and duplicate
suggestions go to stderr.

Log files must be regular UTF-8 files no larger than 2 MiB. Context is fenced and
truncated to at most 12 KiB. Shared secret recognition removes known patterns but
cannot guarantee every secret is removed. Review the preview before saving.

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

Optional `created_at_unix` and `updated_at_unix` record UTC seconds since the Unix
epoch; older notes need no changes. Optional `framework` describes technical context.
Existing metadata remains supported: `id`, `title`, `title_th`, `language`, `tool`, `category`, `error_code`, `tags`, and `keywords`. `language` describes technical material; it is separate from the UI language. `title_th` is an optional Thai display title.

`add` and `learn` date new notes. An edit preserves the ID and creation date and
records a new update date when the body changes. Editing an undated legacy note
adds only its update date; its original creation time remains unknown. An
unchanged body leaves the original file and dates untouched. Custom YAML fields
are preserved as values; YAML formatting and comments may change during an edit.

Before an edit or confirmed import/restore replaces a note, its exact previous
Markdown is saved privately under `.lbc/knowledge-history/<id>.revisions/` for
project notes, or `$XDG_DATA_HOME/lbc/knowledge-history/<id>.revisions/` for
personal notes (the usual `~/.local/share` fallback applies). Files are named by
the SHA-256 of their contents. Repeated snapshots reuse the same file; revisions
are never automatically removed. A failed revision write prevents replacement.
These local revisions are separate from portable backups and are excluded from
knowledge loading and export. Use `backup create` for a portable current snapshot;
copy revision directories separately if their history is needed elsewhere.

### Validate knowledge and find review candidates

```bash
lbc knowledge validate --project .
lbc knowledge validate --json
lbc knowledge review --max-age-days 180 --json
```

Validation reports warnings for malformed/missing YAML frontmatter, invalid field
types, IDs, empty bodies, duplicate source IDs, broken overrides, and invalid,
missing or ambiguous metadata references. These commands do not repair, delete,
rewrite, index or install anything and return success after reporting warnings.
Operational failures, such as an invalid project path, still return nonzero.
`lbc index` retains its existing nonzero result for invalid documents.

Optional references are local knowledge IDs:

```yaml
references: [builtin:rust-e0308, 'package:python-fastapi-basics:python-none-type-error']
```

Use the actual IDs shown by `lbc list` or `lbc inspect`. A bare ID must match
exactly one entry across all sources; a source-qualified ID also disambiguates
overridden entries. References to superseded documents remain valid. External
URLs and Markdown hyperlinks are not fetched or checked.

`review` shows missing dates, dates in the future, update-before-creation dates,
and entries whose last update (or creation date when no update exists) is more
than `--max-age-days` old. The default is 365 days. These are review hints, not a
claim that the content is incorrect; undated older Markdown remains supported.

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

When relevance is comparable, source preference is project → user → installed
package → builtin. Exact error codes and entry IDs take priority over weaker
lexical overlap. Ranking reasons appear in text and JSON. The sources are:

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

## Back up and transfer knowledge

```bash
lbc backup create ./project-knowledge.json
lbc backup restore ./project-knowledge.json
lbc export ./personal-knowledge.json --user
lbc import ./legacy-knowledge --project ./backend
```

`backup create FILE` and `export FILE` perform the same operation;
`backup restore FILE_OR_DIRECTORY` and `import FILE_OR_DIRECTORY` share the same
restore behavior. The default scope is the exact current project directory.
`--project PATH` selects another existing project; `--user` selects personal
notes instead and cannot be combined with `--project`.
File arguments are relative to the current working directory.

Project exports include Markdown from `.lbc/knowledge` and legacy `knowledge/`,
including nested notes. Personal exports include only the personal notes store.
Bundles exclude recovery keys, history, proposals, fixes, configuration, installed
packages and built-in knowledge. Export writes a new file with private access
and refuses to overwrite an existing output.

The version 1 JSON bundle contains each note's ID, SHA-256 checksum and exact
UTF-8 Markdown, including unknown YAML fields, timestamps and CRLF line endings.
It contains no source filesystem paths. Checksums detect corruption; they do not
authenticate the sender or encrypt the notes. Export preserves note content
without redaction, so review it before sharing and keep backups private.
Limits are 4,096 entries, 256 KiB per Markdown document, 16 MiB of Markdown in
total and 32 MiB for the encoded JSON bundle.

Import also accepts a legacy `.md` file or directory of Markdown with explicit
YAML `id` values. Select the actual knowledge directory, such as
`./old-project/.lbc/knowledge` or `./old-project/knowledge`, rather than private
application storage. A selected knowledge store includes its nested notes;
when scanning a general directory, Git's `.git` and private `.lbc` directories
are skipped. Hidden note directories and ordinary folders named `runtime` are
included. Symlinks, unsafe destinations, malformed notes,
invalid or duplicate IDs, unsupported bundle versions and checksum failures
are refused.

Incoming Markdown and `metadata.id` are preserved. New project notes go to
`.lbc/knowledge`; personal notes go to the user store. Source-qualified IDs follow
the selected destination: importing ID `demo-port` into a project yields
`project:demo-port`, while `--user` yields `user:demo-port`.

The command validates the entire input, displays a plan and asks `Apply this
plan? [y/N]`. The input and every planned destination are validated before note
writes. Same ID and identical Markdown is skipped, making retries idempotent.
Same ID with different Markdown is a conflict that stops the whole plan by
default, including unrelated additions. Similar notes with different IDs are
shown as suggestions; they are never automatically merged.

```bash
lbc import ./team-knowledge.json --project ./backend
lbc import ./team-knowledge.json --project ./backend --replace
lbc import ./team-knowledge.json --project ./backend --replace --yes --json
```

`--replace` permits replacing conflicting notes only after confirmation;
`--replace --yes` explicitly confirms replacement for scripts. `--yes` alone
confirms additions and identical skips, and still refuses conflicts. Import JSON
requires `--yes`; the report contains `added`, `replaced`, `duplicates`, `similar`
and `conflicts`, while the preview goes to stderr. Export supports `--json` and
reports its output `path` and entry count. An empty response, `n`, `:cancel`, or
EOF at confirmation writes no notes.

Locks coordinate writers in the current, legacy and nested stores. A change
since the preview aborts the stale plan so you can review again. Each note is
published atomically, but the whole batch is not a transaction: interruption
can leave a complete subset applied. Rerun the same command to skip identical
completed IDs and resume; conflicting replacements still need `--replace` and
confirmation. Unrelated existing notes are never deleted.

## Git safety

Setup, project capture/import, recovery-record creation and backups stored under
`.lbc` create `.lbc/.gitignore` if it is absent. Its default
rules ignore all `.lbc` contents except the ignore file itself, including
knowledge until you choose individual notes to share. Existing rules are never
overwritten. To share one reviewed Markdown note, append these rules to
`.lbc/.gitignore`:

```gitignore
!/knowledge/
/knowledge/*
!/knowledge/shared.md
```

For a nested note, reopen each parent directory, ignore its children, then
reopen the selected file. If the repository's root rules ignore `.lbc/`, append
`!/.lbc/` there so the inner rules can apply. Review the result before staging:

```bash
git check-ignore --no-index .lbc/recovery.key
git status --short --untracked-files=all
lbc doctor
git add .lbc/.gitignore .lbc/knowledge/shared.md
```

`doctor` checks actual Git ignore behavior and the index, including already
tracked private records that ignore rules cannot protect. It reports unsafe
paths without reading record contents. Only selected knowledge Markdown and
`.lbc/.gitignore` should be tracked under `.lbc`; recovery keys, fixes, proposals,
history and future runtime records must stay local. Non-Git projects need no Git
installation for this check.

Doctor never stages, untracks, removes files, installs hooks or changes ignore
rules. Correct unsafe rules and review any index cleanup yourself. Ignore rules
cannot prevent `git add --force`, and adding rules does not remove tracked files
or past commits. If a credential or recovery key has been exposed, assess its
use, rotate the affected credential and address repository history separately.
These protections retain the [ownership and ACL policy](shared-store-security.md).

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

## Diagnostic guidance and saved proposals

`fix` reads a diagnostic and reports local guidance without writing source.
Its JSON retains the former offline shape with `status: "offline_guidance"`,
`applied: false`, `verification_status: "unverified"`, and the `guidance`
report. Patch, proposal, verification, and recovery fields remain null.

```bash
lbc fix build.log --project ./app
lbc fix --stdin --project ./app --json < build.log
```

Use `apply-proposal` for a reviewed, authenticated proposal saved by an earlier
version. It revalidates the source, records recovery before writing, and can
run an optional verification command. A successful application without a
verification command is marked `unverified`.

```bash
lbc apply-proposal proposal-ID --project ./app --verify "cargo check --offline" --json
lbc rollback fix-ID --project ./app --json
```

Use the saved proposal's ID and the `recovery_id` from the application.
Records under `<project>/.lbc/proposals/` bind the ID, canonical project,
diagnostic target and line, original SHA-256, and exact before/after replacement.
Proposals expire after 30 days. Application refuses tampered records, moves to
another project, changed source, unsafe paths, and unsafe permissions. All
platforms enforce the owner and ACL policy. On Unix, directories use mode 0700
and records and keys use mode 0600. Keep the directory and recovery key private.

An unchanged proposal can be applied again after rollback within its validity
window.

`--verify "COMMAND"` runs the selected executable and arguments in the exact
project directory. Arguments use shell-style quoting on every OS. Quote Windows
paths that contain spaces, preferably using forward slashes. LBC launches the
executable directly. Pipes, redirection, and variable expansion require an
explicitly selected shell. The command inherits your environment and can execute
project code, access the network, or change files.

The deadline is 120 seconds; `--verify-timeout 30` changes it (1–3600 seconds).
Standard input is closed. Each output stream retains at most 32 KiB, redacts
recognizable secrets, and reports truncation. A passed command records `passed`
only if the patched file still matches. This proves that particular command
exited successfully, not general program correctness. On timeout LBC stops the
direct child; it does not guarantee termination of detached descendant processes.

Every CLI application first saves a private recovery record under
`<project>/.lbc/fixes/<recovery_id>.json`, containing the original and applied
source snapshots. Version 2 records are authenticated with HMAC-SHA256 using a
random project-local key at `<project>/.lbc/recovery.key`; altered records and
keys with unsafe ownership, permissions, or ACL grants are refused. A failed, unavailable,
or timed-out verifier triggers rollback and exits nonzero. `lbc rollback ID
--project PATH` also restores an applied patch in a later invocation.
Both forms refuse to overwrite source that no longer matches the recorded applied
content. Only the selected file is restored; side effects of the verification
command are outside the rollback. LBC keeps at most 100 records and removes
records older than 30 days when preparing a new patch. Version 1 records from
older releases are unauthenticated and must be inspected and recovered manually.
Keep `.lbc/fixes/` and `.lbc/recovery.key` out of version control. Power-loss
durability is not guaranteed.

The exact `--project` directory is the boundary. Targets must be regular UTF-8
files of at most 256 KiB. Replacements must match uniquely, intersect the
reported line, and keep the resulting file within 256 KiB. Symlinked paths,
hidden/generated targets, recognized `.gitignore` matches, and targets containing
recognizable secrets are refused. Source content is checked again before atomic
replacement, and file permissions are retained. On Unix, final publication is
anchored to an opened parent-directory descriptor and rejects a replaced parent;
other platforms retain portable component checks. Advisory locks still require
cooperating writers.

`apply-proposal --json` reports status, patch, proposal and recovery IDs,
verification, and error. Verification states are `unverified`, `passed`,
`failed`, `timed_out`, or `error`. When verification runs, its report includes
the exit code, retained stdout and stderr, truncation status, and execution error.
Refused applications exit nonzero.

## Upgrading to the offline CLI

- Remove `--ai` from `ask`, `explain`, `chat`, and `fix`. `fix` now offers
  diagnostic guidance, and its previous generation, application, save, and
  verification flags (`--apply`, `--save-proposal`, `--verify`, and
  `--verify-timeout`) are removed from `fix`. `apply-proposal` still accepts
  the two verification flags.
- Remove `--provider`, `--model`, and `--base-url` from `setup` and
  `--connectivity` and `--connectivity-timeout` from `doctor`. Update JSON
  consumers that use `provider`, `environment_variables`, `credentials`, or
  `connectivity` from setup, or `provider_connectivity` or the AI provider
  check from doctor. Ordinary offline `ask` and `explain` JSON retains its
  existing shape.
- The `ai.provider`, `ai.model`, and `ai.base_url` config setters are removed.
  `config show` omits the `ai` field. LBC ignores a legacy `[ai]` table and
  preserves it when updating supported settings. Provider credential lookup
  has been removed.
- Existing Markdown and YAML metadata, stores, installed packages, backups,
  and qualified IDs work in place. Keep an existing authenticated proposal's
  project location and private recovery key if you still need to apply it.
- Give external agents access through [CLI JSON or read-only MCP](mcp.md).

## English and Thai output

Set the output language explicitly or use automatic selection:

```bash
lbc config set output.language th
lbc ask "แก้ปัญหาพอร์ตของ demo service อย่างไร"
lbc config set output.language auto
```

Supported values are `en`, `th`, and `auto`. `auto` selects Thai when the
current question contains Thai characters and otherwise falls back to English.
Built-in note titles and excerpts follow the selected language. Retrieval
supports Unicode and matches Thai terms in titles, bodies, tags, or keywords.
User note text appears as written. Commands, paths, error codes, source IDs,
and JSON keys remain unchanged.

## Optional chat and history

The default memory mode is `session`.

```toml
[memory]
mode = "session"
```

`lbc chat` retrieves evidence independently for each question. The default mode
stores no transcript. `/exit` or EOF ends the session. Knowledge saved with
`lbc add` is stored separately.

Persistent history keeps at most 12 messages within a bounded character budget.
`/clear` clears that history. Set `memory.mode` to `persistent` to enable it.

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
lbc knowledge list --json
lbc knowledge remove team-rules --yes
```

A package contains `package.toml` with `name`, `version`, and optional
`description`, plus valid Markdown documents. Installation validates a complete
snapshot, stages and syncs it under a process lock, records `SHA256SUMS`, and
publishes the directory atomically after checking that the target is absent.
If the source supplies `SHA256SUMS`, installation verifies it against the
validated snapshot before publishing anything. Invalid documents, duplicate
IDs/paths (including case collisions), malformed checksums, missing/extra files
in checksummed packages, and duplicate installed package names are rejected.
Removal requires `--yes` because it deletes the entire package directory,
including local changes. Installing never replaces an existing package or user
note; upgrade an installed package only after saving any local changes.
Unix publication is anchored to an opened parent directory; Linux also uses a
kernel no-replace rename so a concurrently created target is preserved. `lbc
knowledge list` reports `verified`, `unverified`
(legacy install), or `corrupt`; corrupt packages are not loaded.

The binary includes an optional package of 40 bilingual Python/FastAPI notes.
List and install it without a source checkout or internet access:

```bash
lbc knowledge list --bundled
lbc knowledge install --bundled python-fastapi-basics
lbc search "ModuleNotFoundError"
lbc ask "Python CustomWidgetError failed in worker.py"
```

The files also remain in `packages/python-fastapi-basics` for local directory
installation. No package command downloads anything; URL sources are rejected.
Checksums detect corruption, not publisher authenticity. Legacy packages without
checksums are listed as `unverified`. When a Python error
has no adequate specific match, `ask` may cite the generic traceback playbook
with `answer_status: "general_guidance"`. It explicitly states that evidence for
a specific fix is insufficient. English/Thai selection applies to the bilingual
passages.

Other read-only commands:

```bash
lbc scan --path . --tree
lbc scan --json
lbc config show --json
lbc doctor --json
```

`scan` is a lightweight inventory; it does not imply that every source file was
read. `--project` and `--path` use the exact supplied directory and never ascend
to a parent project root. Git-compatible `.gitignore` rules and nested ignore
files are applied to inventory and diagnostic evidence. JSON commands print one
machine-readable value to stdout, while warnings go to stderr. `doctor` exits
nonzero when any check fails, including with `--json`; the JSON report is still
printed.

## Release status and integrity

The [2026-11-08 release audit](release-readiness-2026-11-08.md) records fresh
GitHub/CircleCI findings, local fixes, validation and the remaining owner gates.
No production release was published by this work.

The current source version is `0.7.1`. Publishing production assets requires
hosted CI, native signing, and owner approval. [Changelog](../CHANGELOG.md).
As rechecked on 2026-11-12, GitHub does expose an empty public `v0.5.0`
Release despite its DRAFT title/body. This is a release-state inconsistency,
not a production release; there are no verified production assets to install.

The Rust package and crate are named `librarycube`; the executable is `lbc`.

For copy-paste examples covering every command, see the
[complete usage examples](usage-examples.md).
The [v0.5 roadmap checkpoint](roadmap-progress.md) records current
verification, external release gates, and remaining limitations.
The [v0.5 acceptance/evidence ledger](v0.5-completion-checklist.md) separates
repository checks from external production gates; `ROADMAP.md` remains canonical.
See the [changelog](../CHANGELOG.md) for the version's changes.

After every hosted and repository gate passes, the version-tag release pipeline is
configured to publish every archive, checksum, SBOM, and provenance file with a
detached `.asc` signature. Download
`lbc-release-signing-key.asc`, compare its full
fingerprint with the value published by the maintainer through a separate trusted
channel, then verify before installation:

Use the [clean-keyring full-manifest verification procedure](circleci.md#reproducible-independent-verification)
to verify all 25 files, trusted fingerprints, checksums, SBOM and exact
source/version/workflow provenance before extracting or installing anything.
The key shipped alongside the assets is not itself an independent trust anchor.

The tag workflow now requires Windows Authenticode and macOS Developer ID,
hardened runtime, notarization and a stapled DMG before production packaging.
These executable jobs have not run with production credentials. Candidates
remain unsigned on Windows and ad-hoc signed on macOS; no production native
trust or valid production `v0.7.1` release is claimed. See the
[native signing and independent verification runbook](native-code-signing.md).
The DMG is the offline-ticket delivery path; a raw tar cannot carry a stapled
ticket. Certificate trust does not guarantee absence of SmartScreen prompts.

## Safety boundaries

`search`, `inspect`, `ask`, `explain`, `scan`, `fix`, and `doctor` report
results without changing project sources or executing suggested commands.
`apply-proposal` and `rollback` can write the selected source file.
`apply-proposal --verify` executes the user-selected verification command.

Redaction covers common credentials in errors, diagnostic context, and
persistent history. It handles multiple credentials per line, quoted keys, URI
user information, and Unicode prefixes. Private-key blocks are redacted before
passage selection while preserving line numbers. Review sensitive text before
sharing CLI or MCP output, since redaction recognizes specific patterns.

Questions and search queries are limited to 8 KiB, notes to 256 KiB including
serialized metadata, and diagnostic input to 2 MiB.

Note creation publishes a fully written temporary file without overwriting an existing target. Edits atomically replace a validated document; failed editor launches and cancellations clean up temporary files. Builtin overrides preserve retrieval metadata, and changed bodies reset verification to `unverified`. Editor arguments support quoting without launching a shell implicitly. Bounded text readers reject special files and symlinked path components (including parent directories); use real, non-symlinked storage paths.

Mutable config, history, knowledge, and package stores use advisory process locks
and atomic file/directory publication. Unix file replacement anchors validation
and rename to an opened directory descriptor to prevent parent-symlink redirects,
and rechecks the target digest so a same-size concurrent edit is not silently
overwritten during publication. Mutable stores follow the fail-closed
[single-owner ownership and ACL policy](shared-store-security.md): unsafe
owners, shared writable directories, extended/default ACL grants, and Windows
reparse points are refused. Locks and private key/history data require private
access; owner-controlled public configuration and notes remain supported.
These controls require cooperating LBC processes; shared writable collaboration
is not supported. Production readiness remains under
audit, especially non-Unix directory races, and
the final target-creation race on non-Linux platforms, and complete Thai
diagnostic content.

The fixed [retrieval benchmark](retrieval-benchmark.md) evaluates 40 bilingual
cases against embedded notes, the optional Python package and seven explicit
reference fixtures. Top-1 improved 28/31 → 31/31 and Top-3 30/31 → 31/31;
strict citation relevance remains 33/40. This measures that corpus, not product
accuracy or successful resolution of arbitrary incidents.

## Development

```bash
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
cargo audit
cargo deny check
cargo cyclonedx --format json --all-features
bash .circleci/test-release-scripts.sh
```

CircleCI runs these gates for every branch and pull request. The dependency job
also exercises provenance and signing with a disposable one-day test key.
Linux, universal macOS, and Windows x86-64 jobs test and package every candidate
branch and pull request as well as matching version tags. Only matching version
tags can publish artifacts to GitHub Releases, after the remote names, sizes,
states, and SHA-256 digests match the canonical manifest. See
[CircleCI CI/CD setup](circleci.md) for the one-time token setup and release
procedure.

CLI tests use isolated XDG stores and local fixtures.

To repeat the CLI walkthrough against the release binary, outside the source checkout (the fixtures set their own working directories):

```bash
LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli add_list_inspect_ask_edit_roundtrip_uses_current_content
```

Deferred work includes vector search, embedding services, executable plugin SDKs, model training, automatic repair, autonomous agents, GUI/TUI redesign, npm distribution, and broad compiler coverage.
