---
id: python-syntax-error
kind: troubleshooting
language: python
tool: python
category: syntax
error_code: SyntaxError
title: Python - SyntaxError invalid syntax
title_th: Python - แก้ SyntaxError invalid syntax
tags:
  - python
  - syntax
keywords:
  - SyntaxError
  - invalid syntax
  - unexpected EOF
  - was never closed
  - perhaps you forgot a comma
  - ไวยากรณ์ผิด
  - วงเล็บไม่ปิด
verification_status: unverified
---
<!-- lbc:en -->
# Python - SyntaxError invalid syntax

## Fix

`SyntaxError` means Python could not parse the file. Open the reported line and also inspect the line immediately before it; the real cause is often an unclosed quote or bracket, a missing comma, or a missing colon.

```python
items = [
    "book",
    "pen",
]
if items:
    print(items)
```

Match every `()`, `[]`, `{}` and quote. Do not change several lines at once; compile after each small correction.

## Verify

```bash
$ python -m py_compile path/to/file.py
```

<!-- Source: https://docs.python.org/3/tutorial/errors.html#syntax-errors -->

<!-- lbc:th -->
# Python - แก้ SyntaxError invalid syntax

## วิธีแก้

`SyntaxError` หมายถึง Python parse ไฟล์ไม่ได้ ให้เปิดบรรทัดที่แจ้งและตรวจบรรทัดก่อนหน้าด้วย ต้นเหตุมักเป็น quote หรือวงเล็บไม่ปิด, comma หาย หรือ colon หาย

```python
items = [
    "book",
    "pen",
]
if items:
    print(items)
```

จับคู่ `()`, `[]`, `{}` และ quote ให้ครบ แก้ทีละจุดแล้ว compile ใหม่ ไม่ควรเปลี่ยนหลายบรรทัดพร้อมกัน

## ตรวจผล

```bash
$ python -m py_compile path/to/file.py
```

<!-- แหล่งข้อมูล: https://docs.python.org/3/tutorial/errors.html#syntax-errors -->
