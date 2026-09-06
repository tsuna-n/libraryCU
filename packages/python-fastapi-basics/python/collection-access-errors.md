---
id: python-collection-access-errors
kind: troubleshooting
language: python
tool: python
category: runtime
title: Python - KeyError and IndexError
title_th: Python - แก้ KeyError และ IndexError
tags:
  - python
  - dictionary
  - list
keywords:
  - KeyError
  - missing dictionary key
  - IndexError
  - list index out of range
  - tuple index out of range
  - ไม่มี key
  - index เกิน
verification_status: unverified
---
<!-- lbc:en -->
# Python - KeyError and IndexError

## KeyError

`KeyError` means a dictionary does not contain the requested key. Confirm the actual keys and decide whether absence is an error:

```python
if "user_id" not in payload:
    raise ValueError("payload requires user_id")
user_id = payload["user_id"]
```

Use `payload.get("user_id")` only when `None` or a chosen default is a valid result.

## IndexError

`IndexError: list index out of range` means the index is outside `-len(items)` through `len(items) - 1`. Check length or iterate directly:

```python
if not items:
    raise ValueError("at least one item is required")
first = items[0]
```

Do not catch `IndexError` and continue if an empty collection violates the input contract.

## Verify

Test an existing key/index, a missing key, an empty sequence, and the last valid index.

<!-- Source: https://docs.python.org/3/library/exceptions.html#LookupError -->

<!-- lbc:th -->
# Python - แก้ KeyError และ IndexError

## KeyError

`KeyError` หมายถึง dictionary ไม่มี key ที่เรียก ให้ตรวจ key จริงและตัดสินว่าการไม่มีค่านี้เป็น error หรือไม่:

```python
if "user_id" not in payload:
    raise ValueError("payload requires user_id")
user_id = payload["user_id"]
```

ใช้ `payload.get("user_id")` เฉพาะเมื่อ `None` หรือ default ที่เลือกเป็นผลลัพธ์ที่ยอมรับได้

## IndexError

`IndexError: list index out of range` หมายถึง index อยู่นอกช่วง `-len(items)` ถึง `len(items) - 1` ให้ตรวจความยาวหรือวนสมาชิกโดยตรง:

```python
if not items:
    raise ValueError("at least one item is required")
first = items[0]
```

อย่าจับ `IndexError` แล้วทำงานต่อ ถ้า collection ว่างผิดเงื่อนไขของ input

## ตรวจผล

ทดสอบ key/index ที่มีอยู่, key ที่หาย, sequence ว่าง และ index สุดท้ายที่ใช้ได้

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#LookupError -->
