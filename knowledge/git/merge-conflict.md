---
id: git-merge-conflict
kind: troubleshooting
language: git
tool: git
category: version-control
title: Git - Merge conflict
title_th: Git - ไฟล์ขัดแย้งตอนรวม branch
tags:
  - merge
  - conflict
keywords:
  - conflict
  - merge
  - rebase
  - merge conflict
  - ขัดแย้ง
  - รวม branch
---
<!-- lbc:en -->
# Git - Merge conflict

## Quick answer

A merge conflict occurs when Git cannot combine competing changes automatically. Use `git status`, edit each conflicted file to keep the intended content, remove all conflict markers, then stage the resolved files and continue the merge or rebase.

## Resolve a merge

```bash
$ git status
$ git diff --check
$ git add app.js
$ git merge --continue
```

During a rebase, use `git rebase --continue` for the final command instead.

<!-- lbc:th -->
# Git - ไฟล์ขัดแย้งตอนรวม branch

## คำตอบสั้น ๆ

Merge conflict เกิดเมื่อ Git รวมการแก้ไขที่ขัดกันโดยอัตโนมัติไม่ได้ ให้ใช้ `git status` หาไฟล์ แก้เนื้อหาและลบ conflict markers ทั้งหมด จากนั้น stage ไฟล์แล้วทำ merge หรือ rebase ต่อ

## แก้ conflict จาก merge

```bash
$ git status
$ git diff --check
$ git add app.js
$ git merge --continue
```

ถ้าเกิดระหว่าง rebase ให้ใช้ `git rebase --continue` เป็นคำสั่งสุดท้าย
