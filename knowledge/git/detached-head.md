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

HEAD ชี้ตรงไปยัง commit แทนที่จะชี้ไปที่ branch ทำให้ commit ใหม่ที่สร้างขึ้นไม่ได้อยู่บน branch ใดๆ และอาจสูญหายเมื่อ switch ไปที่อื่น

## ก่อนแก้ (Before - Detached State)
```bash
$ git checkout a1b2c3d
Note: switching to 'a1b2c3d'.
You are in 'detached HEAD' state...
$ git status
HEAD detached at a1b2c3d
```

## แก้แล้ว (After - Success)
```bash
# สร้างและย้ายไปทำงานบน branch ใหม่จาก commit นี้
$ git switch -c new-feature-branch
# SUCCESS: Switched to a new branch 'new-feature-branch'

# หรือหากต้องการกลับไปยัง branch เดิม
$ git switch main
```

## การทำงาน (How it works)
- ปกติ HEAD จะชี้ที่ชื่อ branch แต่ในสภาวะ Detached HEAD ตัวชี้ HEAD จะชี้ตรงไปที่ commit hash
- ใช้ `git switch -c <name>` (หรือ `git checkout -b <name>`) เพื่อตั้งชื่อ branch ให้กับ commit ปัจจุบัน
- หากเผลอทำ commit หลุดไป สามารถใช้ `git reflog` ดูประวัติ commit เพื่อกู้คืนได้
