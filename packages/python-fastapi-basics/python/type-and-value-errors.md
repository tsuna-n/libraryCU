---
id: python-type-and-value-errors
kind: troubleshooting
language: python
tool: python
category: runtime
title: Python - TypeError and ValueError
title_th: Python - แก้ TypeError และ ValueError
tags:
  - python
  - types
  - validation
keywords:
  - TypeError
  - unsupported operand type
  - unexpected keyword argument
  - missing required positional argument
  - not callable
  - ValueError
  - invalid literal
  - too many values to unpack
  - not enough values to unpack
  - ชนิดข้อมูลผิด
  - ค่าไม่ถูกต้อง
verification_status: unverified
---
<!-- lbc:en -->
# Python - TypeError and ValueError

## TypeError

`TypeError: unsupported operand type` means an operation received incompatible objects, such as `int` and `str`, while other TypeError messages can indicate a wrong call signature. Inspect `type(value)`, then convert at the input boundary or correct the function arguments:

```python
count = int(raw_count)
total = count + 1
```

For `unexpected keyword argument` or `missing required positional argument`, inspect the installed signature:

```python
import inspect
print(inspect.signature(target_function))
```

## ValueError

`ValueError: invalid literal for int() with base 10` means the type can be passed to `int` but its text is not an integer. More generally, ValueError means the type is acceptable but its content is invalid. Validate before conversion and report the rejected value:

```python
try:
    port = int(raw_port)
except ValueError as exc:
    raise ValueError(f"PORT must be an integer, got {raw_port!r}") from exc
```

For unpacking errors, make the number of target names match the actual sequence length.

## Verify

Test one valid value, the original invalid value, and a boundary value.

<!-- Source: https://docs.python.org/3/library/exceptions.html#TypeError -->

<!-- lbc:th -->
# Python - แก้ TypeError และ ValueError

## TypeError

`TypeError: unsupported operand type` หมายถึง operation ได้ object ที่ใช้ร่วมกันไม่ได้ เช่น `int` กับ `str` ส่วน TypeError แบบอื่นอาจเกิดจาก signature ผิด ให้ตรวจ `type(value)` แล้วแปลงชนิดที่ขอบเขตรับข้อมูลหรือแก้ argument:

```python
count = int(raw_count)
total = count + 1
```

ถ้าเป็น `unexpected keyword argument` หรือ `missing required positional argument` ให้ดู signature ของตัวที่ติดตั้งจริง:

```python
import inspect
print(inspect.signature(target_function))
```

## ValueError

`ValueError: invalid literal for int() with base 10` หมายถึงส่งชนิดที่ `int` รับได้แต่ข้อความไม่ใช่เลขจำนวนเต็ม โดยทั่วไป ValueError หมายถึงชนิดข้อมูลใช้ได้แต่เนื้อหาของค่าไม่ถูกต้อง ให้ validate ก่อนแปลงและบอกค่าที่ถูกปฏิเสธ:

```python
try:
    port = int(raw_port)
except ValueError as exc:
    raise ValueError(f"PORT must be an integer, got {raw_port!r}") from exc
```

ถ้าเป็น unpacking error ให้จำนวนชื่อตัวแปรตรงกับจำนวนสมาชิกจริง

## ตรวจผล

ทดสอบค่าที่ถูกต้อง ค่าที่เคยพัง และค่าขอบเขต

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#TypeError -->
