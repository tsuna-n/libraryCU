---
id: python-indentation-error
kind: troubleshooting
language: python
tool: python
category: syntax
error_code: IndentationError
title: Python - IndentationError or TabError
title_th: Python - แก้ IndentationError หรือ TabError
tags:
  - python
  - indentation
  - syntax
keywords:
  - IndentationError
  - TabError
  - unexpected indent
  - unindent does not match
  - เยื้องไม่ตรง
  - tab กับ space
verification_status: unverified
---
<!-- lbc:en -->
# Python - IndentationError or TabError

## Fix

Open the first file and line named in the traceback. Replace tabs with spaces and align the block with its surrounding code. Use four spaces for each indentation level.

```python
def greet(name: str) -> str:
    if name:
        return f"Hello {name}"
    return "Hello"
```

Configure the editor to insert spaces when pressing Tab. Check the whole file for mixed indentation, not only the reported line.

## Verify

```bash
$ python -m tabnanny path/to/file.py
$ python -m py_compile path/to/file.py
```

<!-- Source: https://docs.python.org/3/reference/lexical_analysis.html#indentation -->

<!-- lbc:th -->
# Python - แก้ IndentationError หรือ TabError

## วิธีแก้

เปิดไฟล์และบรรทัดแรกที่ traceback ระบุ เปลี่ยน tab เป็น space แล้วจัดบล็อกให้ตรงกับโค้ดรอบข้าง ใช้ 4 spaces ต่อหนึ่งระดับ

```python
def greet(name: str) -> str:
    if name:
        return f"Hello {name}"
    return "Hello"
```

ตั้ง editor ให้ปุ่ม Tab ใส่ spaces และตรวจทั้งไฟล์ว่ามี tab กับ space ปนกันหรือไม่

## ตรวจผล

```bash
$ python -m tabnanny path/to/file.py
$ python -m py_compile path/to/file.py
```

<!-- แหล่งข้อมูล: https://docs.python.org/3/reference/lexical_analysis.html#indentation -->
