---
id: python-async-event-loop-errors
kind: troubleshooting
language: python
tool: asyncio
category: async
title: Python - asyncio event loop RuntimeError
title_th: Python - แก้ asyncio event loop RuntimeError
tags:
  - python
  - asyncio
  - event-loop
keywords:
  - asyncio.run cannot be called from a running event loop
  - event loop is already running
  - no running event loop
  - got Future attached to a different loop
  - Event loop is closed
  - event loop พัง
  - มี event loop อยู่แล้ว
verification_status: unverified
---
<!-- lbc:en -->
# Python - asyncio event loop RuntimeError

## Loop already running

`asyncio.run() cannot be called from a running event loop` means an async environment already owns the loop. Inside async code or a notebook with top-level await, call the coroutine directly:

```python
result = await main()
```

In a normal script, use one outer `asyncio.run(main())` and use `await` below it. Do not nest `asyncio.run`.

## No running loop or wrong loop

`no running event loop` means `create_task` was called outside active async execution. Move it inside the coroutine started by `asyncio.run`. `Future attached to a different loop` means an async object was created under another loop; create async clients, locks, and futures inside the current lifespan and do not reuse them across separate loop runs.

## Verify

Run asyncio debug mode and confirm one loop owns resource creation, tasks, and cleanup:

```bash
$ PYTHONASYNCIODEBUG=1 python app.py
```

<!-- Source: https://docs.python.org/3/library/asyncio-runner.html -->

<!-- lbc:th -->
# Python - แก้ asyncio event loop RuntimeError

## มี loop ทำงานอยู่แล้ว

`asyncio.run() cannot be called from a running event loop` หมายถึง environment แบบ async เป็นเจ้าของ loop อยู่แล้ว ถ้าอยู่ใน async code หรือ notebook ที่รองรับ top-level await ให้เรียก coroutine โดยตรง:

```python
result = await main()
```

ใน script ปกติให้มี `asyncio.run(main())` ชั้นนอกเพียงครั้งเดียว แล้วใช้ `await` ภายใน ห้ามซ้อน `asyncio.run`

## ไม่มี loop หรือใช้ผิด loop

`no running event loop` หมายถึงเรียก `create_task` นอก async execution ให้ย้ายเข้า coroutine ที่เริ่มโดย `asyncio.run` ส่วน `Future attached to a different loop` หมายถึง object ถูกสร้างกับอีก loop ให้สร้าง async client, lock และ future ภายใน lifespan ปัจจุบันและอย่าใช้ข้ามการเปิด loop หลายรอบ

## ตรวจผล

เปิด asyncio debug mode แล้วตรวจว่า loop เดียวเป็นเจ้าของการสร้าง resource, task และ cleanup:

```bash
$ PYTHONASYNCIODEBUG=1 python app.py
```

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/asyncio-runner.html -->
