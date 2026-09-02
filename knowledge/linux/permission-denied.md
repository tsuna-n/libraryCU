---
id: linux-permission-denied
language: linux
tool: linux
category: permissions
title: Linux - Permission denied (EACCES)
tags:
  - permissions
  - filesystem
keywords:
  - permission
  - denied
  - eacces
  - chmod
---
# Linux - Permission denied (EACCES)

The process lacks the filesystem permission to read, write, or execute the target. The kernel returns EACCES or EPERM.

Inspect the owner and mode with `ls -l`, then either fix ownership (`sudo chown`), adjust the mode (`chmod +x` for scripts, `chmod 600` for keys), or rerun with elevated rights (`sudo`) when appropriate. Confirm by rerunning the exact failing command.
