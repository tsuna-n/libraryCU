---
id: python-arithmetic-errors
kind: troubleshooting
language: python
tool: python
category: runtime
title: Python - ZeroDivisionError and OverflowError
title_th: Python - แก้ ZeroDivisionError และ OverflowError
tags:
  - python
  - arithmetic
  - numbers
keywords:
  - ZeroDivisionError
  - division by zero
  - modulo by zero
  - OverflowError
  - math range error
  - numerical result out of range
  - หารด้วยศูนย์
  - ค่าตัวเลขล้น
verification_status: unverified
---
<!-- lbc:en -->
# Python - ZeroDivisionError and OverflowError

## ZeroDivisionError

`ZeroDivisionError` means the divisor became zero. Validate it before division and choose a domain-correct result instead of adding an arbitrary tiny number:

```python
if denominator == 0:
    raise ValueError("denominator must not be zero")
ratio = numerator / denominator
```

## OverflowError

`OverflowError: math range error` usually means a value is outside the range accepted by a math function or C-backed numeric type. Check the input scale, units, and earlier exponential growth:

```python
import math
import sys
if exponent > math.log(sys.float_info.max):
    raise ValueError("exponent is too large")
value = math.exp(exponent)
```

Use `decimal.Decimal` or integer arithmetic only when the required numeric range and precision justify it.

## Verify

Test zero, values near zero, normal values, and the largest supported input.

<!-- Source: https://docs.python.org/3/library/exceptions.html#ArithmeticError -->

<!-- lbc:th -->
# Python - แก้ ZeroDivisionError และ OverflowError

## ZeroDivisionError

`ZeroDivisionError` หมายถึงตัวหารกลายเป็นศูนย์ ให้ validate ก่อนหารและเลือกผลลัพธ์ตามกฎของระบบ อย่าแก้ด้วยการบวกเลขเล็ก ๆ แบบสุ่ม:

```python
if denominator == 0:
    raise ValueError("denominator must not be zero")
ratio = numerator / denominator
```

## OverflowError

`OverflowError: math range error` มักหมายถึงค่าเกินช่วงที่ math function หรือ numeric type ฝั่ง C รับได้ ให้ตรวจ scale, หน่วย และการเพิ่มแบบ exponential ก่อนหน้า:

```python
import math
import sys
if exponent > math.log(sys.float_info.max):
    raise ValueError("exponent is too large")
value = math.exp(exponent)
```

ใช้ `decimal.Decimal` หรือ integer arithmetic เมื่อระบบต้องการช่วงค่าและ precision นั้นจริง

## ตรวจผล

ทดสอบศูนย์ ค่าใกล้ศูนย์ ค่าปกติ และค่าสูงสุดที่ระบบรองรับ

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#ArithmeticError -->
