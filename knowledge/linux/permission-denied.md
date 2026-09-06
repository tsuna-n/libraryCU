---
id: linux-permission-denied
kind: troubleshooting
language: linux
tool: linux
category: permissions
title: Linux - Permission denied (EACCES)
title_th: Linux - ไม่มีสิทธิ์เข้าถึง (EACCES)
tags:
  - permissions
  - filesystem
keywords:
  - permission
  - denied
  - eacces
  - chmod
  - ไม่มีสิทธิ์
---
<!-- lbc:en -->
# Linux - Permission denied (EACCES)

## Quick answer

`Permission denied` means the process lacks a required read, write, or execute permission. Inspect the owner and mode first, then grant only the missing permission; do not use broad modes such as `chmod 777`.

## Example

```bash
$ ls -l deploy.sh
$ chmod +x deploy.sh
$ ./deploy.sh
```

## Verify

Run `ls -l deploy.sh` and retry the original command.

<!-- lbc:th -->
# Linux - ไม่มีสิทธิ์เข้าถึง (EACCES)

## คำตอบสั้น ๆ

`Permission denied` หมายถึง process ไม่มีสิทธิ์ read, write หรือ execute ที่จำเป็น ให้ตรวจ owner และ permission ก่อน แล้วเพิ่มเฉพาะสิทธิ์ที่ขาด หลีกเลี่ยงการเปิดกว้างด้วย `chmod 777`

## ตัวอย่าง

```bash
$ ls -l deploy.sh
$ chmod +x deploy.sh
$ ./deploy.sh
```

## ตรวจสอบผล

รัน `ls -l deploy.sh` แล้วลองคำสั่งเดิมอีกครั้ง
