---
id: python-coroutine-not-awaited
kind: troubleshooting
language: python
tool: asyncio
category: async
title: Python - Coroutine was never awaited
title_th: Python - แก้ Coroutine was never awaited
tags:
  - python
  - asyncio
  - async
  - await
keywords:
  - coroutine was never awaited
  - RuntimeWarning
  - await
  - ลืม await
  - coroutine
verification_status: unverified
---
<!-- lbc:en -->
# Python - Coroutine was never awaited

## Fix

`Coroutine was never awaited` means an async function was called without running it. Inside an `async def` function, add `await` before that coroutine call:

```python
async def load_page() -> str:
    response = await client.get("/items")
    return response.text
```

From a normal script entry point, start the event loop once:

```python
import asyncio
async def main() -> None:
    await load_page()
asyncio.run(main())
```

Use `asyncio.create_task(...)` only when the task should run concurrently, and keep or await the returned task so exceptions are observed.

## Verify

Run with asyncio debug mode and confirm that no never-awaited warning remains:

```bash
$ PYTHONASYNCIODEBUG=1 python app.py
```

<!-- Source: https://docs.python.org/3/library/asyncio-dev.html#detect-never-awaited-coroutines -->

<!-- lbc:th -->
# Python - แก้ Coroutine was never awaited

## วิธีแก้

`Coroutine was never awaited` หมายถึงเรียก async function แต่ไม่ได้รัน coroutine ภายในฟังก์ชัน `async def` ให้เติม `await` หน้าจุดที่เรียก:

```python
async def load_page() -> str:
    response = await client.get("/items")
    return response.text
```

ถ้าเริ่มจาก script ปกติ ให้สร้าง event loop เพียงครั้งเดียว:

```python
import asyncio
async def main() -> None:
    await load_page()
asyncio.run(main())
```

ใช้ `asyncio.create_task(...)` เฉพาะเมื่อต้องการให้ทำงานพร้อมกัน และเก็บหรือ await task ที่ได้กลับมาเพื่อไม่ให้ exception หายไป

## ตรวจผล

เปิด asyncio debug mode แล้วตรวจว่าไม่มีคำเตือน never-awaited:

```bash
$ PYTHONASYNCIODEBUG=1 python app.py
```

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/asyncio-dev.html#detect-never-awaited-coroutines -->
