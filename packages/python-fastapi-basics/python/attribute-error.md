---
id: python-attribute-error
kind: troubleshooting
language: python
tool: python
category: runtime
error_code: AttributeError
title: Python - AttributeError object has no attribute
title_th: Python - แก้ AttributeError object has no attribute
tags:
  - python
  - attribute
  - object
keywords:
  - AttributeError
  - object has no attribute
  - module has no attribute
  - partially initialized module has no attribute
  - ไม่มี attribute
  - เรียก method ไม่เจอ
verification_status: unverified
---
<!-- lbc:en -->
# Python - AttributeError object has no attribute

## Fix

`AttributeError: ... has no attribute ...` means the runtime object is not the type or version the code expects. At the failing line, print the object type and the module source:

```python
print(type(value), repr(value))
print(type(value).__module__)
```

Correct the attribute spelling, construct the expected object, handle `None` before access, or update the code to the installed library API. If the object is a module, check that a local file is not shadowing the real package:

```bash
$ python -c "import PACKAGE; print(PACKAGE.__file__)"
```

## Verify

Add a test that asserts the input type and calls the corrected attribute.

<!-- Source: https://docs.python.org/3/library/exceptions.html#AttributeError -->

<!-- lbc:th -->
# Python - แก้ AttributeError object has no attribute

## วิธีแก้

`AttributeError: ... has no attribute ...` หมายถึง object ตอนรันไม่ใช่ชนิดหรือ version ที่โค้ดคาดไว้ ให้ดูชนิดและแหล่ง module ที่บรรทัดซึ่งพัง:

```python
print(type(value), repr(value))
print(type(value).__module__)
```

แก้ชื่อ attribute, สร้าง object ให้ถูกชนิด, จัดการ `None` ก่อนเรียก หรือปรับโค้ดให้ตรงกับ API ของ library version ที่ติดตั้ง ถ้า object เป็น module ให้ตรวจว่าไฟล์ในโปรเจกต์ไม่ได้บัง package จริง:

```bash
$ python -c "import PACKAGE; print(PACKAGE.__file__)"
```

## ตรวจผล

เพิ่ม test ที่ตรวจชนิด input และเรียก attribute ที่แก้แล้ว

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#AttributeError -->
