---
id: python-circular-import
kind: troubleshooting
language: python
tool: python
category: imports
title: Python - Partially initialized module or circular import
title_th: Python - แก้ partially initialized module หรือ circular import
tags:
  - python
  - import
  - circular-import
keywords:
  - circular import
  - partially initialized module
  - most likely due to a circular import
  - import วนกัน
  - circular dependency
verification_status: unverified
---
<!-- lbc:en -->
# Python - Partially initialized module or circular import

## Fix

If `a.py` imports `b.py` and `b.py` imports `a.py`, move the shared type or function into a third module:

```text
app/
  common.py
  a.py
  b.py
```

Then import `common` from both files. If the import is needed only while a function runs, a local import can break the startup cycle:

```python
def build_service():
    from app.service import Service
    return Service()
```

Also check that the current file does not have the same name as the package being imported.

## Verify

```bash
$ python -c "import app.a; import app.b"
```

<!-- Source: https://docs.python.org/3/reference/import.html -->

<!-- lbc:th -->
# Python - แก้ partially initialized module หรือ circular import

## วิธีแก้

ถ้า `a.py` import `b.py` และ `b.py` import `a.py` ให้ย้าย type หรือ function ที่ใช้ร่วมกันไป module ที่สาม:

```text
app/
  common.py
  a.py
  b.py
```

จากนั้นให้ทั้งสองไฟล์ import จาก `common` ถ้าต้องใช้ import เฉพาะตอนเรียกฟังก์ชัน สามารถย้าย import เข้าไปในฟังก์ชันเพื่อตัดวงจรตอนเริ่มโปรแกรม:

```python
def build_service():
    from app.service import Service
    return Service()
```

ตรวจด้วยว่าไฟล์ปัจจุบันไม่ได้ตั้งชื่อชนกับ package ที่ต้องการ import

## ตรวจผล

```bash
$ python -c "import app.a; import app.b"
```

<!-- แหล่งข้อมูล: https://docs.python.org/3/reference/import.html -->
