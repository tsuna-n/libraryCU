---
id: python-traceback-playbook
kind: troubleshooting
language: python
tool: python
category: diagnostics
title: Python - Troubleshoot any traceback or unknown exception
title_th: Python - ไล่แก้ traceback หรือ exception ที่ยังไม่รู้จัก
tags:
  - python
  - traceback
  - error
  - exception
keywords:
  - Python error
  - traceback
  - exception
  - unknown error
  - custom exception
  - BaseException
  - SystemExit
  - KeyboardInterrupt
  - GeneratorExit
  - BufferError
  - EOFError
  - ReferenceError
  - SystemError
  - ExceptionGroup
  - BrokenPipeError
  - ChildProcessError
  - ProcessLookupError
  - InterruptedError
  - NotImplementedError
  - StopAsyncIteration
  - FloatingPointError
  - UnicodeTranslateError
  - ไล่ traceback
  - แก้ Python error
  - exception ไม่รู้จัก
verification_status: unverified
---
<!-- lbc:en -->
# Python - Troubleshoot any traceback or unknown exception

## Fix

For any Python traceback, read the final `ExceptionType: message` first, then move upward to the lowest frame that belongs to your code. Inspect the values used on that exact line; do not start by changing the library frame at the top.

If the traceback says `The above exception was the direct cause` or `During handling`, fix the first underlying exception before the later wrapper error.

For `EOFError: EOF when reading a line`, the input stream ended before `input()` received data; pass the expected stdin in automation or catch EOF and choose an explicit no-input result. For `NotImplementedError`, implement the required subclass method or instantiate a concrete class. For `SystemError`, reproduce on a supported Python and dependency version because the failure commonly comes from interpreter or extension-module internals.

Reproduce with the smallest input and capture a complete traceback:

```bash
$ python -X dev path/to/script.py
$ python -m pdb path/to/script.py
```

Inside `pdb`, use `where`, `up`, `down` and `p variable_name`. Compare the failing value, its type, relevant configuration, active Python, and dependency versions with a working run.

Catch only an error the program can recover from. Log context and re-raise unexpected errors; avoid `except Exception: pass` because it removes the evidence needed to fix the cause.

## Verify

Add a regression test for the smallest failing input, run it before and after the change, then run the surrounding test suite.

<!-- Source: https://docs.python.org/3/tutorial/errors.html -->

<!-- lbc:th -->
# Python - ไล่แก้ traceback หรือ exception ที่ยังไม่รู้จัก

## วิธีแก้

ใช้ได้กับ Python traceback ทุกชนิด: อ่านบรรทัดสุดท้าย `ExceptionType: message` ก่อน แล้วไล่ขึ้นไปหา frame ล่างสุดที่เป็นโค้ดของเรา ตรวจค่าที่ใช้ในบรรทัดนั้น อย่าเริ่มจากแก้ library frame ด้านบน

ถ้ามีข้อความ `The above exception was the direct cause` หรือ `During handling` ให้แก้ exception ต้นเหตุที่เกิดก่อน แล้วค่อยดู wrapper error ตัวหลัง

ถ้าเป็น `EOFError: EOF when reading a line` แปลว่า input stream จบก่อน `input()` ได้ข้อมูล ให้ส่ง stdin ที่ต้องการใน automation หรือจับ EOF แล้วกำหนดผลลัพธ์กรณีไม่มี input ถ้าเป็น `NotImplementedError` ให้ implement method ของ subclass หรือสร้าง concrete class ส่วน `SystemError` ให้ทำซ้ำบน Python และ dependency version ที่รองรับ เพราะมักมาจาก interpreter หรือ extension module ภายใน

ทำให้เกิดซ้ำด้วย input ที่เล็กที่สุดและเก็บ traceback ให้ครบ:

```bash
$ python -X dev path/to/script.py
$ python -m pdb path/to/script.py
```

ใน `pdb` ใช้ `where`, `up`, `down` และ `p variable_name` เปรียบเทียบค่าที่พัง ชนิดข้อมูล config, Python ที่กำลังใช้ และ version ของ dependency กับรอบที่ทำงานได้

จับเฉพาะ error ที่โปรแกรมฟื้นตัวได้ ถ้าเป็น error ที่ไม่คาดคิดให้ log context แล้ว raise ต่อ หลีกเลี่ยง `except Exception: pass` เพราะจะทำให้หลักฐานต้นเหตุหาย

## ตรวจผล

เพิ่ม regression test ด้วย input ที่เล็กที่สุด รันก่อนและหลังแก้ แล้วรัน test suite ส่วนที่เกี่ยวข้อง

<!-- แหล่งข้อมูล: https://docs.python.org/3/tutorial/errors.html -->
