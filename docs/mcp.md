# Read-only MCP server

Run an optional local stdio server for a coding agent:

```bash
lbc mcp --project /absolute/path/to/project --project-only
```

Use a build from this checkout (`cargo install --path . --locked`). No API key,
AI provider, setup command, network service, or background daemon is required.
The agent starts and stops the process. Standard output contains only newline
delimited JSON-RPC; running it in a terminal waits for protocol messages.

`--project` is required and fixes the exact directory for the server's lifetime.
The server never searches parent repositories. `--project-only` reads just that
directory's `.lbc/knowledge/` and legacy `knowledge/`; omit it to also make your
personal notes, installed packages, and built-in knowledge available. Project
notes from other directories are never included. Use a separate server entry
and explicit path for each project; restart it when changing projects.

## Tools

| Tool | Arguments | Result |
| --- | --- | --- |
| `search_knowledge` | `query`, optional `limit` (1–20, default 5) | Ranked excerpts, source IDs, match reasons, historical verification status |
| `inspect_knowledge` | `source_id`, such as `project:deployment` | Full effective Markdown document and metadata |
| `explain_error` | `text` containing the error log or selection | Offline diagnostic report, retrieved guidance, bounded local evidence |

Search uses the same retrieval core as `lbc search`, `ask`, and `explain`, with
[project context ranking](project-memory.md). Inspect accepts a source-qualified
knowledge ID rather than a file path. Unknown tools and extra arguments are
rejected, including `project`, `file`, `ai`, `apply`, and shell commands.
Search queries are at most 8 KB, error text at most 2 MB, and each encoded
JSON-RPC message at most 4 MB. Oversized messages close the session. Requests run
serially, without persistent caches, history, or knowledge/source writes.

Tool results contain both JSON `structuredContent` and the same JSON in a text
content block. Each successful result includes `verified_for_current_problem:
false` and guidance explaining that recorded checks are historical. Input/output
use the existing credential redactor. Knowledge text remains untrusted reference
material; consuming agents should cite it, inspect applicability, and validate
proposed fixes on the current project.

The three tools cannot invoke `fix`, apply a proposal, edit knowledge or source,
execute verification commands, or contact an AI provider. `explain_error` uses
the standard scanner defaults and existing ignore/path/regular-file checks for
bounded project evidence. LBC's AI configuration is not read by this server.
The consuming coding agent may send tool results to its own configured model;
choose its provider and project scope accordingly. That agent's independent
shell/edit tools retain its own permission policy.

## Codex CLI

Register the local executable with an explicit absolute project path:

```bash
codex mcp add librarycu-project -- lbc mcp --project /absolute/path/to/project --project-only
codex mcp list
```

Alternatively, merge this entry into your Codex configuration without replacing
existing settings:

```toml
[mcp_servers.librarycu-project]
command = "lbc"
args = ["mcp", "--project", "/absolute/path/to/project", "--project-only"]
enabled_tools = ["search_knowledge", "inspect_knowledge", "explain_error"]
```

Use the absolute executable path for `command` if `lbc` is not on the client's
PATH. Restart the client after changing its configuration. Ask it to search
libraryCU for an error, inspect a returned source ID, then explain the log.
These settings follow the [official OpenAI configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference);
the registration syntax was also checked against local `codex mcp add --help`.

## OpenCode

Merge a local server into the project's `opencode.json`. Current OpenCode V2
places names under `mcp.servers`, as documented in the
[official MCP guide](https://opencode.ai/docs/mcp-servers/):

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "librarycu-project": {
        "type": "local",
        "command": ["lbc", "mcp", "--project", "/absolute/path/to/project", "--project-only"]
      }
    }
  }
}
```

OpenCode V1 uses a different shape: place the `librarycu-project` object directly
under `mcp` and add `"enabled": true` inside that object. Use your installed
version's schema; do not mix the V1 and V2 shapes. Run `opencode mcp list`, then
ask the agent to use `search_knowledge`. Keep connections separate per project
and do not enable connection pooling across project directories.

## Protocol check and troubleshooting

This implementation supports MCP `2024-11-05`, `2025-03-26`, `2025-06-18`, and
`2025-11-25`; other requested versions negotiate `2025-11-25`. It implements
initialize, initialized notification, ping, tools/list, and tools/call. It
advertises only tools. Notifications receive no reply; closing stdin ends the
server. See the official [stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports),
[lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle),
and [tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools) contracts.

For a direct shell smoke check, send one JSON message per line:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"manual","version":"1"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search_knowledge","arguments":{"query":"E0308","limit":3}}}' \
  | lbc mcp --project /absolute/path/to/project
```

If there are no results under `--project-only`, capture a reviewed local note
with `lbc learn --project /absolute/path/to/project` or omit that flag to include
shared guidance. A symlinked project root is refused; invalid/symlink knowledge
files are skipped with diagnostics. Malformed requests and unknown tools use
JSON-RPC errors; tool input-validation and read failures return `isError: true`.

Run `cargo test --locked --test mcp --test project_memory --test editor_integration`
for subprocess integration tests. Client configuration examples are documented;
full model sessions in Codex/OpenCode and native Windows/macOS clients have not
been exercised by these local tests.
