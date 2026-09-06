---
id: python-task-exception-not-retrieved
kind: troubleshooting
language: python
tool: asyncio
category: async
title: Python - Task exception was never retrieved
title_th: Python - แก้ Task exception was never retrieved
tags:
  - python
  - asyncio
  - task
keywords:
  - Task exception was never retrieved
  - task destroyed but it is pending
  - exception was never retrieved
  - asyncio TaskGroup
  - task exception หาย
  - ไม่ได้ await task
verification_status: unverified
---
<!-- lbc:en -->
# Python - Task exception was never retrieved

## Fix

`Task exception was never retrieved` means a background task failed but no caller awaited or inspected it. Prefer structured concurrency so every task is awaited and failures propagate:

```python
import asyncio
async with asyncio.TaskGroup() as group:
    group.create_task(sync_customer())
    group.create_task(sync_orders())
```

On older Python, keep task references and await `asyncio.gather(*tasks)`. For a deliberately long-lived background task, store the reference, log exceptions in a done callback, and cancel plus await it during shutdown.

Do not silence the warning without handling the task result; the hidden exception is the actual bug.

## Verify

Force one child task to fail and confirm the parent receives or logs that exact exception, then confirm clean shutdown leaves no pending-task warning.

<!-- Source: https://docs.python.org/3/library/asyncio-task.html#task-groups -->

<!-- lbc:th -->
# Python - แก้ Task exception was never retrieved

## วิธีแก้

`Task exception was never retrieved` หมายถึง background task พังแต่ไม่มีผู้เรียก await หรือตรวจผล ให้ใช้ structured concurrency เพื่อให้ทุก task ถูก await และ exception ส่งถึง parent:

```python
import asyncio
async with asyncio.TaskGroup() as group:
    group.create_task(sync_customer())
    group.create_task(sync_orders())
```

ถ้าใช้ Python รุ่นเก่า ให้เก็บ reference ของ task แล้ว await `asyncio.gather(*tasks)` ถ้าตั้งใจให้ task อยู่ยาว ให้เก็บ reference, log exception ใน done callback และ cancel พร้อม await ตอน shutdown

อย่าปิดคำเตือนโดยไม่จัดการผลของ task เพราะ exception ที่ถูกซ่อนคือต้นเหตุจริง

## ตรวจผล

บังคับให้ child task หนึ่งตัวพัง ตรวจว่า parent ได้รับหรือ log exception นั้น แล้วตรวจว่า shutdown ไม่มี pending-task warning

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/asyncio-task.html#task-groups -->
