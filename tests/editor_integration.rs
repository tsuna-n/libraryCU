use std::path::Path;
use std::process::Command;

fn isolated_lbc(project: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lbc"));
    command
        .current_dir(project)
        .env("XDG_CONFIG_HOME", project.join("config"))
        .env("XDG_DATA_HOME", project.join("data"))
        .env("XDG_CACHE_HOME", project.join("cache"))
        .env_remove("LBC_CONFIG");
    command
}

#[test]
fn vscode_process_tasks_keep_selection_and_paths_as_literal_arguments() {
    let tasks: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("examples/editors/vscode/tasks.json").unwrap(),
    )
    .unwrap();
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("project 'quoted' $literal");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("README.md"), "fixture").unwrap();
    let text =
        "app.ts(2,3): error TS2322: incompatible type\n'\" $(touch injected) `touch injected`";
    let log = project.join("error 'quoted' $(touch injected).log");
    std::fs::write(&log, text).unwrap();

    for task in tasks["tasks"].as_array().unwrap() {
        assert_eq!(task["type"], "process");
        assert_eq!(task["command"], "lbc");
        assert_eq!(task["options"]["cwd"], "${workspaceFolder}");
        let args: Vec<String> = task["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| match arg.as_str().unwrap() {
                "${workspaceFolder}" => project.to_str().unwrap().to_owned(),
                "${selectedText}" => text.to_owned(),
                "${file}" => log.to_str().unwrap().to_owned(),
                value => value.to_owned(),
            })
            .collect();
        assert!(args.contains(&"--project-only".to_owned()));
        if let Some(index) = args.iter().position(|arg| arg == "--text") {
            assert_eq!(args[index + 1], text);
        }
        let output = isolated_lbc(&project).args(args).output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("TS2322"));
        assert!(!project.join("injected").exists());
    }
}

#[test]
fn selected_text_is_literal_and_exclusive_with_other_input_sources() {
    let project = tempfile::tempdir().unwrap();
    let output = isolated_lbc(project.path())
        .args([
            "explain",
            "--text",
            "--error starts with a hyphen",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        report["diagnostic"]["message"]
            .as_str()
            .unwrap()
            .contains("--error starts")
    );
    for args in [
        vec!["explain", "--text", "error", "--stdin"],
        vec!["explain", "--text", "error", "error.log"],
        vec!["explain", "--text", " "],
    ] {
        let output = isolated_lbc(project.path()).args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn neovim_example_sends_visual_text_without_a_shell_and_preserves_source_buffer() {
    let source = std::fs::read_to_string("examples/editors/nvim/lbc.lua").unwrap();
    assert!(source.contains("vim.system"));
    assert!(!source.contains("vim.fn.system("));
    let Ok(version) = Command::new("nvim").arg("--version").output() else {
        eprintln!("nvim unavailable; checked example statically only");
        return;
    };
    assert!(version.status.success());
    let version = String::from_utf8_lossy(&version.stdout);
    let mut numbers = version
        .trim_start_matches("NVIM v")
        .split('.')
        .take(2)
        .map(|part| part.parse::<u32>().unwrap_or(0));
    if numbers.next().unwrap_or(0) == 0 && numbers.next().unwrap_or(0) < 10 {
        eprintln!("Neovim is older than 0.10; checked example statically only");
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("project 'quoted' $literal");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("README.md"), "fixture").unwrap();
    let script = home.path().join("check.lua");
    std::fs::write(
        &script,
        r#"
local function check()
  local m = dofile(vim.env.LBC_EDITOR_EXAMPLE)
  local text = vim.env.LBC_EDITOR_TEXT
  local original = vim.api.nvim_get_current_buf()
  local lines = vim.split(text, '\n', { plain = true })
  vim.api.nvim_buf_set_lines(original, 0, -1, false, lines)
  vim.g.lbc_project_root = vim.env.LBC_EDITOR_PROJECT
  local real_system = vim.system
  local called = 0
  vim.system = function(argv, opts, callback)
    called = called + 1
    assert(vim.deep_equal(argv, {
      'lbc', 'explain', '--stdin', '--project', vim.env.LBC_EDITOR_PROJECT,
      '--project-only', '--no-color',
    }), vim.inspect(argv))
    assert(opts.stdin == text, vim.inspect(opts.stdin))
    assert(opts.cwd == vim.env.LBC_EDITOR_PROJECT)
    argv[1] = vim.env.LBC_EDITOR_BINARY
    return real_system(argv, opts, callback)
  end
  m.explain('  \n')
  assert(called == 0)
  vim.cmd('normal! gg0vG$')
  m.explain_selection()
  assert(vim.wait(10000, function() return vim.b.lbc_exit_code ~= nil end))
  assert(vim.b.lbc_exit_code == 0)
  assert(table.concat(vim.api.nvim_buf_get_lines(0, 0, -1, false), '\n'):find('TS2322', 1, true))
  assert(vim.deep_equal(vim.api.nvim_buf_get_lines(original, 0, -1, false), lines))
  assert(vim.bo.buftype == 'nofile' and not vim.bo.modifiable)
  assert(vim.fn.filereadable(vim.env.LBC_EDITOR_PROJECT .. '/injected') == 0)
  vim.g.lbc_project_root = nil
  vim.api.nvim_cmd({ cmd = 'tcd', args = { vim.env.LBC_EDITOR_PROJECT } }, {})
  vim.api.nvim_cmd({ cmd = 'lcd', args = { vim.fn.fnamemodify(vim.env.LBC_EDITOR_PROJECT, ':h') } }, {})
  vim.system = function(_, opts, callback)
    assert(opts.cwd == vim.env.LBC_EDITOR_PROJECT)
    callback({ code = 7, signal = 0, stdout = '', stderr = 'fixture failure' })
  end
  local previous = vim.api.nvim_get_current_buf()
  m.explain('error')
  assert(vim.wait(1000, function() return vim.api.nvim_get_current_buf() ~= previous end))
  assert(vim.b.lbc_exit_code == 7)
  assert(table.concat(vim.api.nvim_buf_get_lines(0, 0, -1, false), '\n'):find('fixture failure', 1, true))
  local notice
  vim.notify = function(message) notice = message end
  vim.system = function() error('fixture launch error') end
  m.explain('error')
  assert(notice:find('fixture launch error', 1, true))
end
local ok, err = pcall(check)
if not ok then
  io.stderr:write(tostring(err) .. '\n')
  vim.cmd('cquit 1')
end
vim.cmd('qa!')
"#,
    )
    .unwrap();
    let output = Command::new("nvim")
        .args(["--headless", "-u", "NONE", "-i", "NONE", "-n", "-l"])
        .arg(script)
        .current_dir(&project)
        .env("LBC_EDITOR_PROJECT", &project)
        .env("LBC_EDITOR_BINARY", env!("CARGO_BIN_EXE_lbc"))
        .env(
            "LBC_EDITOR_EXAMPLE",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/editors/nvim/lbc.lua"),
        )
        .env(
            "LBC_EDITOR_TEXT",
            "app.ts(2,3): error TS2322: incompatible type\n'\" $(touch injected) `touch injected`",
        )
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_CACHE_HOME", home.path().join("cache"))
        .env_remove("LBC_CONFIG")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}
