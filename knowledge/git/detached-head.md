---
id: git-detached-head
kind: troubleshooting
language: git
tool: git
category: version-control
title: Git - Detached HEAD
title_th: Git - HEAD ไม่ได้ชี้ไปที่ branch
tags:
  - checkout
  - head
keywords:
  - detached
  - head
  - checkout
  - detached head
  - ไม่ได้อยู่บน branch
---
<!-- lbc:en -->
# Git - Detached HEAD

## Quick answer

Detached HEAD means `HEAD` points directly to a commit instead of a branch. Create a branch before making work you want to keep, or switch back to an existing branch.

## Keep work from the current commit

```bash
$ git switch -c new-feature-branch
```

## Leave without keeping new work

```bash
$ git switch main
```

Use `git reflog` to find a commit if you switched away before creating a branch.

<!-- lbc:th -->
# Git - HEAD ไม่ได้ชี้ไปที่ branch

## คำตอบสั้น ๆ

Detached HEAD หมายถึง `HEAD` ชี้ตรงไปยัง commit แทนที่จะชี้ไปที่ branch หากต้องการเก็บงานให้สร้าง branch จากตำแหน่งปัจจุบัน หรือสลับกลับไป branch เดิมเมื่อไม่ต้องการเก็บงานใหม่

## เก็บงานจาก commit ปัจจุบัน

```bash
$ git switch -c new-feature-branch
```

## ออกโดยไม่เก็บงานใหม่

```bash
$ git switch main
```

หากสลับออกไปก่อนสร้าง branch ให้ใช้ `git reflog` เพื่อหา commit เดิม
