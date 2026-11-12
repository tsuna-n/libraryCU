# Project memory

Project notes live in `<project>/.lbc/knowledge/*.md`. The legacy
`<project>/knowledge/` directory remains supported. Both use the ordinary
knowledge Markdown format and existing `add` / `learn` workflows;
there is no separate memory database or background indexing service.

Pass an explicit `--project /path/to/project` when working across repositories.
The selected directory is the boundary: libraryCube does not walk upward to a
parent repository or read sibling projects. A nested directory therefore has its
own scope. Symlink project roots and symlink knowledge sources are rejected, and
symlink documents are skipped with a warning.

Search, ask, and explain include project notes alongside user notes, installed
packages, and built-in guidance by default. Use `--project-only` to read only the
selected project's two knowledge directories. Excluded stores are not loaded,
and their invalid entries or paths are not included in warnings. The MCP server
supports the same project scope; see the [MCP guide](mcp.md) for its invocation.
This flag limits retrieved documents; deterministic diagnostic rules and bounded
project-file evidence still apply when explaining an error.

```bash
lbc search "deployment" --project /path/to/project --project-only
lbc ask "How do we deploy this service?" --project /path/to/project --project-only
lbc explain build.log --project /path/to/project --project-only
```

IDs are scoped by source. Two different projects may each have
`project:deployment` without sharing their content. Inside one project, duplicate
IDs across the two project directories remain invalid. A project note that
declares `overrides: builtin:rust-e0382` (or another valid external source ID)
remains usable in project-only mode without inspecting the excluded target.
Conflicting overrides, cycles, invalid targets, and absent `project:` targets
still produce validation warnings. External target existence cannot be checked
in project-only mode.

Retrieval uses the existing bounded project detector to recognize languages,
build tools, containers, and supported frameworks from filenames and small
manifests. Matching `language`, `tool`, and `framework` metadata adds a bounded
preference to already relevant notes. JSON results explain these preferences in
`ranking_reasons`, such as `project language: Rust` or `project framework: Axum`.
Context does not create a match, turn weak overlap into adequate evidence, or
outrank exact diagnostic codes and explicit knowledge IDs. Existing source
preference and specific lexical evidence continue to apply. This detector is
limited to its supported manifests and frameworks; it does not infer a complete
stack from source code or scan the project automatically for new knowledge.

Historical notes remain guidance. `user-reported` describes the author's report,
and `recorded-check` describes a previously recorded check. Neither confirms a
fix for the current problem. Answers label recorded verification as not rerun;
retrieved explanations remain unverified against the current project. Optional
AI receives the same distinction and treats notes as untrusted evidence.
