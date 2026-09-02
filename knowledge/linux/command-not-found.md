---
id: linux-command-not-found
language: linux
tool: linux
category: shell
title: Linux - Command not found
tags:
  - shell
  - path
keywords:
  - command
  - not-found
  - path
---
# Linux - Command not found

The shell could not find an executable with that name in any directory listed in `PATH`, or the binary is not installed.

Check `which <command>` and `echo $PATH`. Install the missing package with the system package manager, add the tool's directory to `PATH`, or fix the spelling. Verify with `which <command>` returning a path.
