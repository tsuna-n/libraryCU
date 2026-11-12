# Send editor text to lbc

These optional examples call the installed `lbc` executable. They require no
extension or plugin manager and do not install or change your editor settings.
Use a build that supports `lbc explain --text` and `--project-only`.

Both examples pass an explicit project directory and use `--project-only` to
retrieve from that project's `.lbc/knowledge` and legacy `knowledge/` directories
only. Initialize or populate that
project store separately with `lbc setup --project /path/to/project` and
`lbc learn --project /path/to/project`. They run the normal
offline explanation path; adding `--ai` would opt into your configured provider.
Neither example edits the source buffer or applies fixes.

## VS Code

Merge the tasks from [tasks.json](../examples/editors/vscode/tasks.json) into your
project's `.vscode/tasks.json`. Keep existing tasks. Open the intended project as
a workspace folder, ensure `lbc` is on VS Code's process PATH (or set `command`
to the executable's absolute path), then use **Terminal → Run Task**:

- **lbc: Explain selection** sends the active editor selection, including
  multiple lines. Select text first; empty input produces a CLI error. To use
  terminal or Problems-panel text, paste it into an untitled editor and select it.
- **lbc: Explain saved log** reads the active file from disk. Open and save the
  error log first; unsaved changes are not included.

Results appear in the task terminal. `${workspaceFolder}` supplies both `cwd`
and `--project`; in a multi-root workspace, put the tasks in the intended folder
or replace both values with `${workspaceFolder:your-folder-name}`.

Keep `type: "process"` and separate `args` entries. VS Code documents the
[process task type](https://code.visualstudio.com/docs/debugtest/tasks#_custom-tasks)
and the [`selectedText`, `file`, and `workspaceFolder` variables](https://code.visualstudio.com/docs/reference/variables-reference).
The selected text is one literal argument: quotes, newlines, `$()` and backticks
are not shell code. Do not add quotes around the variable or embed it in a shell
command. Selected text in arguments may be visible to local process inspection;
use a saved log for sensitive or very large input, and remove secrets first.
Process argument limits also apply, especially on Windows.

## Neovim 0.10 or newer

Copy [lbc.lua](../examples/editors/nvim/lbc.lua) to your Neovim configuration's
`lua/lbc.lua`, then add `require('lbc')` to `init.lua`. This is ordinary Lua
configuration; no plugin installation is needed. It defines two mappings:

- Visual mode `<leader>le`: explain the selected characters, lines, or block.
- Normal mode `<leader>le`: explain the current error-log buffer, including
  unsaved text.

Before using it, set the current tab's project directory with `:tcd /path/to/project`.
Alternatively, configure `vim.g.lbc_project_root = '/absolute/path/to/project'`
before loading the file. The example uses that configured root or the current
tab's directory exactly; it does not walk up to a parent repository. A window's
`:lcd` does not override the tab's project. Change the mappings if these keys are
already assigned. `lbc` must be on Neovim's process PATH.

The explanation runs asynchronously and opens a disposable, read-only scratch
split. Close it with `:q`; the log buffer is preserved. Empty text is rejected
before starting a process. Launch errors are reported with `vim.notify`, while
a failed `lbc` process shows its exit status and stderr in the result split.

The example sends text through stdin using an argument list, without a shell or
temporary log file. Neovim documents [`vim.system`](https://neovim.io/doc/user/lua/#vim.system())
and its `cwd`, `stdin`, and completion callback options. Selection extraction
uses `getregion`, which is present in the
[Neovim 0.10 reference](https://github.com/neovim/neovim/blob/v0.10.0/runtime/doc/builtin.txt),
so multiline, reverse and block selections follow Neovim's own selection rules.
