---
id: fastapi-lifespan
kind: troubleshooting
language: python
tool: fastapi
category: lifecycle
title: FastAPI - Initialize and clean up shared resources with lifespan
title_th: FastAPI - เปิดและปิด shared resource ด้วย lifespan
tags:
  - python
  - fastapi
  - lifespan
  - startup
  - shutdown
keywords:
  - lifespan
  - startup event
  - shutdown event
  - database pool
  - resource cleanup
  - เปิด connection ตอนเริ่ม
  - ปิด resource
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - Initialize and clean up shared resources with lifespan

## Fix

Use FastAPI lifespan for startup and shutdown resource handling. In the lifespan, create application-wide resources during startup before `yield` and always release them during shutdown after `yield`.

```python
from contextlib import asynccontextmanager
from fastapi import FastAPI
@asynccontextmanager
async def lifespan(app: FastAPI):
    app.state.pool = await create_pool()
    try:
        yield
    finally:
        await app.state.pool.close()
app = FastAPI(lifespan=lifespan)
```

Use lifespan for application-wide resources such as a database pool or model. Use a dependency with `yield` for per-request resources such as a database session.

When `lifespan=...` is supplied, keep startup and shutdown work in that lifespan instead of also relying on old `startup` and `shutdown` event handlers.

## Verify

Start and stop the server once. Confirm that startup completes before the first request and the cleanup message appears during graceful shutdown.

<!-- Source: https://fastapi.tiangolo.com/advanced/events/ -->

<!-- lbc:th -->
# FastAPI - เปิดและปิด shared resource ด้วย lifespan

## วิธีแก้

ใช้ FastAPI lifespan จัดการ resource ตอน startup และ shutdown โดยสร้าง shared resource ระดับทั้งแอปในช่วง startup ก่อน `yield` และปิดในช่วง shutdown หลัง `yield` เสมอ:

```python
from contextlib import asynccontextmanager
from fastapi import FastAPI
@asynccontextmanager
async def lifespan(app: FastAPI):
    app.state.pool = await create_pool()
    try:
        yield
    finally:
        await app.state.pool.close()
app = FastAPI(lifespan=lifespan)
```

ใช้ lifespan กับ resource ระดับทั้งแอป เช่น database pool หรือ model ส่วน resource ต่อ request เช่น database session ให้ใช้ dependency ที่มี `yield`

เมื่อกำหนด `lifespan=...` แล้ว ให้รวมงาน startup และ shutdown ไว้ที่ lifespan ไม่ควรพึ่ง event handler แบบเก่าควบคู่กัน

## ตรวจผล

เริ่มและหยุด server หนึ่งรอบ ตรวจว่า startup เสร็จก่อน request แรก และมีการ cleanup ตอน graceful shutdown

<!-- แหล่งข้อมูล: https://fastapi.tiangolo.com/advanced/events/ -->
