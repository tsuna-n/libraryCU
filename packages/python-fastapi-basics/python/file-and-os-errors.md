---
id: python-file-and-os-errors
kind: troubleshooting
language: python
tool: python
category: filesystem
error_code: PermissionError
title: Python - FileNotFoundError PermissionError and path errors
title_th: Python - แก้ FileNotFoundError PermissionError และ path error
tags:
  - python
  - filesystem
  - path
  - permissions
keywords:
  - FileNotFoundError
  - No such file or directory
  - PermissionError
  - Permission denied
  - FileExistsError
  - IsADirectoryError
  - NotADirectoryError
  - OSError
  - errno 2
  - errno 13
  - ไม่พบไฟล์
  - ไม่มีสิทธิ์
verification_status: unverified
---
<!-- lbc:en -->
# Python - FileNotFoundError PermissionError and path errors

## FileNotFoundError

`FileNotFoundError` means the resolved path does not exist. Print the working directory and absolute path, then build paths relative to a known base rather than the shell location:

```python
from pathlib import Path
base = Path(__file__).resolve().parent
config_path = base / "config.json"
print(Path.cwd(), config_path)
```

Create a missing parent only when the program owns that directory: `target.parent.mkdir(parents=True, exist_ok=True)`.

## PermissionError

`PermissionError` means the process lacks access to the file, directory, or port. Inspect the target owner and mode, choose a user-writable location, and grant only the missing permission. Do not use `chmod 777`.

## Other path errors

For `IsADirectoryError` or `NotADirectoryError`, inspect every path component and use `is_file()` or `is_dir()` before the operation. For `FileExistsError`, decide explicitly whether overwrite, reuse, or a unique filename is correct.

## Verify

Run with the same user and working directory as production, then test missing, unreadable, and valid paths.

<!-- Source: https://docs.python.org/3/library/exceptions.html#OSError -->

<!-- lbc:th -->
# Python - แก้ FileNotFoundError PermissionError และ path error

## FileNotFoundError

`FileNotFoundError` หมายถึง path หลัง resolve แล้วไม่มีอยู่จริง ให้พิมพ์ working directory และ absolute path แล้วประกอบ path จาก base ที่แน่นอนแทนตำแหน่ง shell:

```python
from pathlib import Path
base = Path(__file__).resolve().parent
config_path = base / "config.json"
print(Path.cwd(), config_path)
```

สร้าง parent ที่หายด้วย `target.parent.mkdir(parents=True, exist_ok=True)` เฉพาะเมื่อโปรแกรมเป็นเจ้าของ directory นั้น

## PermissionError

`PermissionError` หมายถึง process ไม่มีสิทธิ์กับไฟล์ directory หรือ port ให้ตรวจ owner/mode เลือกตำแหน่งที่ user เขียนได้ และเพิ่มเฉพาะสิทธิ์ที่ขาด ห้ามแก้แบบกว้างด้วย `chmod 777`

## Path error อื่น

ถ้าเป็น `IsADirectoryError` หรือ `NotADirectoryError` ให้ตรวจ path ทุกส่วนและใช้ `is_file()` หรือ `is_dir()` ก่อนทำงาน ถ้าเป็น `FileExistsError` ให้เลือกชัดเจนว่าจะเขียนทับ ใช้ของเดิม หรือสร้างชื่อใหม่

## ตรวจผล

รันด้วย user และ working directory เดียวกับ production แล้วทดสอบ path ที่หาย ไม่มีสิทธิ์ และ path ที่ถูกต้อง

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#OSError -->
