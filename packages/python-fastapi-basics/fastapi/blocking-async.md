---
id: fastapi-blocking-async
kind: troubleshooting
language: python
tool: fastapi
category: performance
title: FastAPI - Requests stall because async endpoint runs blocking code
title_th: FastAPI - Request ค้างเพราะ async endpoint รัน blocking code
tags:
  - python
  - fastapi
  - async
  - blocking
keywords:
  - blocking event loop
  - slow concurrent requests
  - async def blocking
  - time.sleep
  - request ค้าง
  - event loop ถูกบล็อก
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - Requests stall because async endpoint runs blocking code

## Fix

Use `async def` when the libraries provide awaitable operations:

```python
@app.get("/items")
async def read_items():
    return await async_repository.list_items()
```

If the whole endpoint uses a blocking library, declare it with normal `def` so FastAPI can run it outside the event loop:

```python
@app.get("/legacy")
def read_legacy():
    return blocking_client.fetch()
```

For one blocking call inside an async endpoint, move that call to a worker thread:

```python
from anyio import to_thread
result = await to_thread.run_sync(blocking_client.fetch)
```

Replace `time.sleep(...)` inside async code with `await asyncio.sleep(...)`.

## Verify

Send several concurrent requests. A slow request should no longer prevent an unrelated fast endpoint from responding.

<!-- Source: https://fastapi.tiangolo.com/async/ -->

<!-- lbc:th -->
# FastAPI - Request ค้างเพราะ async endpoint รัน blocking code

## วิธีแก้

ใช้ `async def` เมื่อ library มี operation ที่ await ได้:

```python
@app.get("/items")
async def read_items():
    return await async_repository.list_items()
```

ถ้าทั้ง endpoint ใช้ blocking library ให้ประกาศด้วย `def` ปกติ เพื่อให้ FastAPI นำไปรันนอก event loop:

```python
@app.get("/legacy")
def read_legacy():
    return blocking_client.fetch()
```

ถ้ามี blocking call เพียงจุดเดียวใน async endpoint ให้ย้าย call นั้นไปรันใน worker thread:

```python
from anyio import to_thread
result = await to_thread.run_sync(blocking_client.fetch)
```

เปลี่ยน `time.sleep(...)` ใน async code เป็น `await asyncio.sleep(...)`

## ตรวจผล

ส่งหลาย request พร้อมกัน request ที่ช้าไม่ควรทำให้ endpoint อื่นที่เร็วตอบไม่ได้

<!-- แหล่งข้อมูล: https://fastapi.tiangolo.com/async/ -->
