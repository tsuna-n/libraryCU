---
id: git-merge-conflict
language: git
tool: git
category: version-control
title: Git - Merge conflict
tags:
  - merge
  - conflict
keywords:
  - conflict
  - merge
  - rebase
---
# Git - Merge conflict

Both branches changed the same lines, so Git cannot merge them automatically and marks the file with `<<<<<<<`, `=======`, and `>>>>>>>` markers.

Run `git status` to list conflicted files, edit each one to keep the intended content and remove the markers, then `git add <file>` the resolved files and finish with `git commit` (or `git rebase --continue` during a rebase). `git diff --check` confirms no markers remain.
