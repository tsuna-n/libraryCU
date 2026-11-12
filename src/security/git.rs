use std::{
    collections::HashSet,
    fs,
    io::{ErrorKind, Write},
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, ensure};

/// Deny future runtime records too. Knowledge is private until individually selected.
pub const DEFAULT_GITIGNORE: &str =
    "# libraryCube: private runtime records and knowledge stay local by default.
*
!/.gitignore

# To share a reviewed Markdown note, append these rules (replace shared.md):
# !/knowledge/
# /knowledge/*
# !/knowledge/shared.md
# For nested notes, reopen each parent and ignore its children before selecting a file.
# If root rules ignore .lbc/, append !/.lbc/ there so these rules can apply.
# Ignore rules never protect records already tracked by Git.
";

/// Publish once; existing user rules are never overwritten.
pub fn ensure_gitignore(project: &Path) -> Result<()> {
    let directory = project.join(".lbc");
    let target = directory.join(".gitignore");
    super::permissions::validate_directory(&directory)?;
    super::files::reject_symlinks(&target)?;
    match fs::symlink_metadata(&target) {
        Ok(metadata) => {
            ensure!(metadata.is_file(), ".lbc/.gitignore must be a regular file");
            return super::permissions::validate_path(&target, false);
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    super::storage::atomic_create(&target, DEFAULT_GITIGNORE.as_bytes(), true)
        .context("failed to create .lbc/.gitignore without overwriting")
}

pub struct GitSafetyReport {
    pub ok: bool,
    pub detail: String,
}

/// Inspect index paths and effective ignore rules, never private file contents.
pub fn check(project: &Path) -> Result<GitSafetyReport> {
    let project = project
        .canonicalize()
        .context("cannot inspect project directory")?;
    if !project
        .ancestors()
        .any(|ancestor| ancestor.join(".git").exists())
    {
        return Ok(healthy("not a Git worktree; no Git changes made"));
    }
    let inside = command(&project)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .context(
            "cannot run Git safety checks; install Git or inspect .lbc ignore rules manually",
        )?;
    ensure!(
        inside.status.success(),
        "Git repository could not be inspected; review repository configuration"
    );
    if inside.stdout != b"true\n" {
        return Ok(healthy("not a Git worktree; no Git changes made"));
    }

    let tracked = output(&project, &["ls-files", "--stage", "-z", "--", ".lbc"])?;
    let mut problems = Vec::new();
    for record in tracked
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let separator = record
            .iter()
            .position(|byte| *byte == b'\t')
            .context("invalid Git index path response")?;
        let (mode, path) = record.split_at(separator);
        let path = &path[1..];
        if !shareable(path) || mode.starts_with(b"120000 ") || mode.starts_with(b"160000 ") {
            problems.push(format!("already tracked private record {}", filename(path)));
        }
    }

    let directory = project.join(".lbc");
    super::files::reject_symlinks(&directory)?;
    if directory.exists() {
        ensure!(directory.is_dir(), ".lbc must be a directory");
        let exposed = output(
            &project,
            &[
                "ls-files",
                "--others",
                "--exclude-standard",
                "-z",
                "--",
                ".lbc",
            ],
        )?;
        for path in exposed
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
        {
            let regular = std::str::from_utf8(path).ok().is_some_and(|path| {
                fs::symlink_metadata(project.join(path)).is_ok_and(|metadata| metadata.is_file())
            });
            if !shareable(path) || !regular {
                problems.push(format!("unignored private record {}", filename(path)));
            }
        }
        // Probe absent paths too, so today's empty stores cannot hide unsafe defaults.
        let probes = [
            ".lbc/recovery.key",
            ".lbc/fixes/git-safety-probe.json",
            ".lbc/proposals/git-safety-probe.json",
            ".lbc/history/git-safety-probe.json",
            ".lbc/git-safety-future-private-record",
        ];
        let mut child = command(&project)
            .args(["check-ignore", "--no-index", "--stdin", "-z"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("cannot inspect effective Git ignore rules")?;
        {
            let mut input = child.stdin.take().context("cannot send Git ignore paths")?;
            for probe in probes {
                input.write_all(probe.as_bytes())?;
                input.write_all(&[0])?;
            }
        }
        let ignored = child.wait_with_output()?;
        ensure!(
            matches!(ignored.status.code(), Some(0 | 1)),
            "Git ignore rules could not be inspected"
        );
        let ignored: HashSet<&[u8]> = ignored.stdout.split(|byte| *byte == 0).collect();
        if probes
            .iter()
            .any(|probe| !ignored.contains(probe.as_bytes()))
        {
            problems.push(
                ".lbc private runtime paths are not fully ignored (including future records)"
                    .into(),
            );
        }
    }
    if problems.is_empty() {
        Ok(healthy(
            "private .lbc records are ignored and untracked; only selected knowledge Markdown and .gitignore may be versioned",
        ))
    } else {
        Ok(GitSafetyReport {
            ok: false,
            detail: format!(
                "{}; Fix: review .lbc/.gitignore and root ignore rules; use deny-all defaults and explicitly select reviewed knowledge Markdown; already tracked records require manual index cleanup and exposed credentials require rotation; Next: git ls-files -- .lbc; git check-ignore --no-index .lbc/recovery.key; lbc doctor",
                problems.join("; ")
            ),
        })
    }
}

fn healthy(detail: &str) -> GitSafetyReport {
    GitSafetyReport {
        ok: true,
        detail: detail.into(),
    }
}

fn command(project: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(project)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR");
    command
}

fn output(project: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = command(project)
        .args(args)
        .output()
        .context("cannot inspect Git paths")?;
    ensure!(output.status.success(), "Git paths could not be inspected");
    Ok(output.stdout)
}

fn shareable(path: &[u8]) -> bool {
    path == b".lbc/.gitignore" || (path.starts_with(b".lbc/knowledge/") && path.ends_with(b".md"))
}

fn filename(path: &[u8]) -> String {
    super::redact_sensitive(&format!("{:?}", String::from_utf8_lossy(path)))
}
