---
id: python-recursion-and-memory-errors
kind: troubleshooting
language: python
tool: python
category: resources
title: Python - RecursionError and MemoryError
title_th: Python - แก้ RecursionError และ MemoryError
tags:
  - python
  - recursion
  - memory
keywords:
  - RecursionError
  - maximum recursion depth exceeded
  - infinite recursion
  - MemoryError
  - out of memory
  - killed
  - recursion ไม่จบ
  - memory ไม่พอ
verification_status: unverified
---
<!-- lbc:en -->
# Python - RecursionError and MemoryError

## RecursionError

`RecursionError: maximum recursion depth exceeded` means recursive calls did not reach a base case soon enough. Confirm the input becomes smaller on every call and handle the base case first:

```python
def total(values: list[int]) -> int:
    if not values:
        return 0
    return values[0] + total(values[1:])
```

For large inputs, replace recursion with an explicit loop or stack. Do not raise `sys.setrecursionlimit` until the algorithm is proven bounded.

## MemoryError

`MemoryError` or an OS-level `Killed` often means the process materialized too much data. Read files and query results in chunks, use generators instead of full lists, bound caches and queues, and avoid making unnecessary copies.

Measure the input size and peak resident memory before choosing a limit.

## Verify

Test empty, normal, cyclic, and large inputs while monitoring memory and completion time.

<!-- Source: https://docs.python.org/3/library/exceptions.html#RecursionError -->

<!-- lbc:th -->
# Python - แก้ RecursionError และ MemoryError

## RecursionError

`RecursionError: maximum recursion depth exceeded` หมายถึง recursive call ไปไม่ถึง base case ให้ตรวจว่า input เล็กลงทุกครั้งและจัดการ base case ก่อน:

```python
def total(values: list[int]) -> int:
    if not values:
        return 0
    return values[0] + total(values[1:])
```

ถ้า input ใหญ่ ให้เปลี่ยนเป็น loop หรือ stack ที่จัดการเอง อย่าเพิ่ม `sys.setrecursionlimit` จนกว่าจะพิสูจน์ว่า algorithm มีขอบเขต

## MemoryError

`MemoryError` หรือ process ถูก OS `Killed` มักหมายถึงโหลดข้อมูลเข้า memory มากเกิน ให้แบ่งอ่านไฟล์และ query เป็น chunk ใช้ generator แทน list ทั้งก้อน จำกัด cache/queue และลดการ copy

วัดขนาด input และ peak resident memory ก่อนเลือกลิมิต

## ตรวจผล

ทดสอบ input ว่าง ปกติ มีวงจร และขนาดใหญ่ พร้อมดู memory และเวลาจบงาน

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#RecursionError -->
