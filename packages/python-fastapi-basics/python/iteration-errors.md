---
id: python-iteration-errors
kind: troubleshooting
language: python
tool: python
category: iteration
title: Python - StopIteration and generator RuntimeError
title_th: Python - แก้ StopIteration และ generator RuntimeError
tags:
  - python
  - iterator
  - generator
keywords:
  - StopIteration
  - generator raised StopIteration
  - StopAsyncIteration
  - iterator exhausted
  - next
  - generator หมด
  - iterator ไม่มีค่าต่อ
verification_status: unverified
---
<!-- lbc:en -->
# Python - StopIteration and generator RuntimeError

## Fix

`StopIteration` means an iterator has no next item. In normal application code, iterate with `for` or supply a default to `next`:

```python
item = next(iterator, None)
if item is None:
    handle_empty_input()
```

Inside a generator, use `return` to finish; do not raise `StopIteration` yourself because Python converts that mistake to `RuntimeError: generator raised StopIteration`.

For async iterators, end `__anext__` with `StopAsyncIteration` only when implementing the iterator protocol itself; consumers should use `async for`.

## Verify

Test an empty iterator, one item, several items, and a second read after exhaustion.

<!-- Source: https://docs.python.org/3/library/exceptions.html#StopIteration -->

<!-- lbc:th -->
# Python - แก้ StopIteration และ generator RuntimeError

## วิธีแก้

`StopIteration` หมายถึง iterator ไม่มีสมาชิกถัดไป ใน application code ให้ใช้ `for` หรือกำหนด default ให้ `next`:

```python
item = next(iterator, None)
if item is None:
    handle_empty_input()
```

ภายใน generator ให้ใช้ `return` เพื่อจบ อย่า raise `StopIteration` เอง เพราะ Python จะเปลี่ยนเป็น `RuntimeError: generator raised StopIteration`

สำหรับ async iterator ให้ใช้ `StopAsyncIteration` เฉพาะตอนเขียน protocol `__anext__` เอง ส่วนผู้ใช้ควรวนด้วย `async for`

## ตรวจผล

ทดสอบ iterator ว่าง หนึ่งสมาชิก หลายสมาชิก และอ่านซ้ำหลังหมดแล้ว

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#StopIteration -->
