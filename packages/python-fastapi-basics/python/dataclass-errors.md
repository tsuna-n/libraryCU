---
id: python-dataclass-errors
kind: troubleshooting
language: python
tool: python
category: data-model
title: Python - dataclass mutable default and field order errors
title_th: Python - แก้ dataclass mutable default และลำดับ field
tags:
  - python
  - dataclass
  - defaults
keywords:
  - mutable default
  - default_factory
  - mutable default is not allowed
  - non-default argument follows default argument
  - dataclass ValueError
  - ค่า default แก้ไขได้
  - ลำดับ field dataclass
verification_status: unverified
---
<!-- lbc:en -->
# Python - dataclass mutable default and field order errors

## Mutable default

`mutable default ... is not allowed: use default_factory` means instances would share one list, dict, or set. Create a fresh value for each instance:

```python
from dataclasses import dataclass, field
@dataclass
class Basket:
    items: list[str] = field(default_factory=list)
```

## Field order

`non-default argument ... follows default argument` means a required field appears after a defaulted field, including through inheritance. Put required fields first, give the later field a default, or make fields keyword-only when appropriate.

## Verify

Create two instances, mutate one collection, and confirm the other is unchanged. Instantiate the class with all required fields.

<!-- Source: https://docs.python.org/3/library/dataclasses.html#mutable-default-values -->

<!-- lbc:th -->
# Python - แก้ dataclass mutable default และลำดับ field

## Mutable default

`mutable default ... is not allowed: use default_factory` หมายถึงแต่ละ instance จะใช้ list, dict หรือ set ก้อนเดียวกัน ให้สร้างค่าใหม่ต่อ instance:

```python
from dataclasses import dataclass, field
@dataclass
class Basket:
    items: list[str] = field(default_factory=list)
```

## ลำดับ field

`non-default argument ... follows default argument` หมายถึง required field อยู่หลัง field ที่มี default รวมถึง field จาก inheritance ให้ย้าย required field ขึ้นก่อน กำหนด default ให้ field หลัง หรือใช้ keyword-only เมื่อเหมาะสม

## ตรวจผล

สร้างสอง instance แก้ collection ของตัวหนึ่ง แล้วตรวจว่าอีกตัวไม่เปลี่ยน จากนั้นลองสร้าง class พร้อม required fields ครบ

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/dataclasses.html#mutable-default-values -->
