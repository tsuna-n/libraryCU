---
id: python-none-type-error
kind: troubleshooting
language: python
tool: python
category: runtime
title: Python - NoneType is not subscriptable or iterable
title_th: Python - แก้ NoneType is not subscriptable หรือ iterable
tags:
  - python
  - none
  - typeerror
keywords:
  - NoneType object is not subscriptable
  - NoneType object is not iterable
  - cannot unpack non-iterable NoneType
  - ค่าเป็น None
  - NoneType
verification_status: unverified
---
<!-- lbc:en -->
# Python - NoneType is not subscriptable or iterable

## Fix

`NoneType is not subscriptable` or `NoneType is not iterable` means the value is `None` at the last traceback frame. Check it immediately before indexing or iterating, then handle the missing value deliberately:

```python
user = find_user(user_id)
if user is None:
    raise LookupError(f"user {user_id} was not found")
name = user["name"]
```

If the value comes from your function, make every intended branch return a value:

```python
def find_user(user_id: int) -> dict | None:
    if user_id in users:
        return users[user_id]
    return None
```

Do not hide an unexpected `None` with `or {}` unless an empty value is truly valid.

## Verify

Run the failing case and a missing-value case. Both should now return a result or a deliberate error.

<!-- Source: https://docs.python.org/3/library/exceptions.html#TypeError -->

<!-- lbc:th -->
# Python - แก้ NoneType is not subscriptable หรือ iterable

## วิธีแก้

`NoneType is not subscriptable` หรือ `NoneType is not iterable` หมายถึงค่าใน frame สุดท้ายของ traceback เป็น `None` ให้ตรวจก่อนนำไป index หรือวน loop แล้วจัดการกรณีไม่มีค่าอย่างชัดเจน:

```python
user = find_user(user_id)
if user is None:
    raise LookupError(f"user {user_id} was not found")
name = user["name"]
```

ถ้าค่ามาจากฟังก์ชันของเรา ให้ทุก branch ที่ตั้งใจไว้ return ค่าให้ชัดเจน:

```python
def find_user(user_id: int) -> dict | None:
    if user_id in users:
        return users[user_id]
    return None
```

อย่ากลบ `None` ที่ไม่ควรเกิดด้วย `or {}` เว้นแต่ empty value เป็นค่าที่ถูกต้องจริง

## ตรวจผล

ลองทั้งกรณีเดิมที่พังและกรณีหาไม่พบ ทั้งสองกรณีควรได้ผลลัพธ์หรือ error ที่ตั้งใจไว้

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#TypeError -->
