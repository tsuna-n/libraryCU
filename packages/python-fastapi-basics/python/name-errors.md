---
id: python-name-errors
kind: troubleshooting
language: python
tool: python
category: runtime
title: Python - NameError and UnboundLocalError
title_th: Python - แก้ NameError และ UnboundLocalError
tags:
  - python
  - names
  - scope
keywords:
  - NameError
  - name is not defined
  - UnboundLocalError
  - local variable referenced before assignment
  - cannot access local variable
  - ตัวแปรไม่ถูกประกาศ
  - หา variable ไม่เจอ
verification_status: unverified
---
<!-- lbc:en -->
# Python - NameError and UnboundLocalError

## NameError

`NameError: name ... is not defined` means the name is unavailable at that line. Fix spelling and capitalization, import the symbol, or assign it before use in every control-flow branch.

```python
from app.settings import settings
print(settings.database_url)
```

## UnboundLocalError

`UnboundLocalError: cannot access local variable ... where it is not associated with a value` means a function treats a name as local because it assigns to it, but reads it before that assignment. Pass the value in and return the new value instead of relying on hidden global state:

```python
def increment(count: int) -> int:
    return count + 1
count = increment(count)
```

Use `global` or `nonlocal` only when shared mutable scope is intentional.

## Verify

Run the branch that previously skipped assignment and the normal branch.

<!-- Source: https://docs.python.org/3/library/exceptions.html#NameError -->

<!-- lbc:th -->
# Python - แก้ NameError และ UnboundLocalError

## NameError

`NameError: name ... is not defined` หมายถึงชื่อนั้นไม่มีใน scope ของบรรทัดนี้ ให้ตรวจตัวสะกดและตัวพิมพ์ใหญ่เล็ก import symbol หรือกำหนดค่าก่อนใช้ในทุก branch

```python
from app.settings import settings
print(settings.database_url)
```

## UnboundLocalError

`UnboundLocalError: cannot access local variable ...` หมายถึงฟังก์ชันมองชื่อนี้เป็น local เพราะมีการ assign แต่กลับอ่านก่อน assign ให้รับค่าเข้าฟังก์ชันและ return ค่าใหม่แทนการพึ่ง global state:

```python
def increment(count: int) -> int:
    return count + 1
count = increment(count)
```

ใช้ `global` หรือ `nonlocal` เฉพาะเมื่อออกแบบให้แก้ shared state จริง

## ตรวจผล

รันทั้ง branch ที่เคยข้ามการ assign และ branch ปกติ

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#NameError -->
