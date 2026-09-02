---
id: git-detached-head
language: git
tool: git
category: version-control
title: Git - Detached HEAD
tags:
  - checkout
  - head
keywords:
  - detached
  - head
  - checkout
---
# Git - Detached HEAD

HEAD points directly to a commit instead of a branch, so new commits are not recorded on any branch and can be lost when switching away.

Create a branch at the current commit with `git switch -c <name>` (or `git checkout -b <name>`), or return to a branch with `git switch <branch>`. Use `git reflog` to find commits made while detached.
