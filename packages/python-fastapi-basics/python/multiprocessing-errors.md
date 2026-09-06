---
id: python-multiprocessing-errors
kind: troubleshooting
language: python
tool: multiprocessing
category: concurrency
title: Python - multiprocessing spawn and pickling errors
title_th: Python - แก้ multiprocessing spawn และ pickling error
tags:
  - python
  - multiprocessing
  - process
keywords:
  - An attempt has been made to start a new process
  - bootstrapping phase
  - freeze_support
  - cannot pickle
  - pickle local object
  - cannot get attribute
  - multiprocessing RuntimeError
  - spawn process ไม่ได้
  - pickle function ไม่ได้
verification_status: unverified
---
<!-- lbc:en -->
# Python - multiprocessing spawn and pickling errors

## Bootstrapping error

The multiprocessing bootstrapping phase `RuntimeError: An attempt has been made to start a new process before bootstrapping` means process creation runs during module import. Put it behind the main guard:

```python
from multiprocessing import Process
def worker() -> None:
    print("working")
if __name__ == "__main__":
    process = Process(target=worker)
    process.start()
    process.join()
```

Use `freeze_support()` inside the guard for a frozen executable.

## Pickling error

`cannot pickle local object` or `cannot get attribute` means the child cannot import the target. Define worker functions and classes at module top level, pass serializable arguments, and run from a real module rather than defining targets only in a REPL or notebook.

## Verify

Run the script as a fresh process on the same operating system and start method used in deployment.

<!-- Source: https://docs.python.org/3/library/multiprocessing.html#the-spawn-and-forkserver-start-methods -->

<!-- lbc:th -->
# Python - แก้ multiprocessing spawn และ pickling error

## Bootstrapping error

multiprocessing bootstrapping phase `RuntimeError: An attempt has been made to start a new process before bootstrapping` หมายถึงสร้าง process ตอน import module ให้ย้ายไว้หลัง main guard:

```python
from multiprocessing import Process
def worker() -> None:
    print("working")
if __name__ == "__main__":
    process = Process(target=worker)
    process.start()
    process.join()
```

ถ้าเป็น executable ที่ freeze แล้ว ให้ใช้ `freeze_support()` ภายใน guard

## Pickling error

`cannot pickle local object` หรือ `cannot get attribute` หมายถึง child import target ไม่ได้ ให้ประกาศ worker function/class ที่ระดับบนสุดของ module ส่ง argument ที่ serialize ได้ และรันจาก module จริงแทนการสร้าง target เฉพาะใน REPL หรือ notebook

## ตรวจผล

รัน script เป็น process ใหม่บนระบบปฏิบัติการและ start method เดียวกับ deployment

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/multiprocessing.html#the-spawn-and-forkserver-start-methods -->
