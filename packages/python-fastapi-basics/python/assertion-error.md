---
id: python-assertion-error
kind: troubleshooting
language: python
tool: python
category: testing
error_code: AssertionError
title: Python - AssertionError
title_th: Python - แก้ AssertionError
tags:
  - python
  - assertion
  - testing
keywords:
  - AssertionError
  - assert failed
  - expected actual
  - test failed
  - assertion พัง
  - ค่าจริงไม่ตรงค่าที่คาด
verification_status: unverified
---
<!-- lbc:en -->
# Python - AssertionError

## Fix

`AssertionError` means an asserted condition was false. Read the failing expression and compare expected and actual values at the first failed assertion:

```python
assert result.status == "ready", repr(result)
```

If behavior changed intentionally, update the test only after verifying the new requirement. Otherwise fix the production value or setup that made the assertion false.

Do not use `assert` for validating untrusted runtime input or enforcing permissions because optimized Python can remove assertions. Raise `ValueError`, `TypeError`, or a domain exception instead.

## Verify

Run the single failing test first, then its file, then the full suite.

<!-- Source: https://docs.python.org/3/reference/simple_stmts.html#the-assert-statement -->

<!-- lbc:th -->
# Python - แก้ AssertionError

## วิธีแก้

`AssertionError` หมายถึงเงื่อนไขใน assert เป็นเท็จ ให้อ่าน expression ที่พังและเปรียบเทียบ expected กับ actual ที่ assertion แรก:

```python
assert result.status == "ready", repr(result)
```

ถ้าพฤติกรรมเปลี่ยนโดยตั้งใจ ให้อัปเดต test หลังยืนยัน requirement ใหม่แล้วเท่านั้น มิฉะนั้นให้แก้ค่าจาก production code หรือ setup ที่ทำให้ assertion เป็นเท็จ

อย่าใช้ `assert` ตรวจ untrusted runtime input หรือบังคับ permission เพราะ optimized Python อาจตัด assertion ออก ให้ raise `ValueError`, `TypeError` หรือ domain exception

## ตรวจผล

รัน test ที่พังก่อน จากนั้นรันทั้งไฟล์และ full suite

<!-- แหล่งข้อมูล: https://docs.python.org/3/reference/simple_stmts.html#the-assert-statement -->
