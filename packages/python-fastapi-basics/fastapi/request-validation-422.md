---
id: fastapi-request-validation-422
kind: troubleshooting
language: python
tool: fastapi
category: validation
error_code: 422
title: FastAPI - 422 Unprocessable Entity request validation
title_th: FastAPI - แก้ 422 Request Validation Error
tags:
  - python
  - fastapi
  - pydantic
  - validation
keywords:
  - 422
  - Unprocessable Entity
  - RequestValidationError
  - field required
  - input should be
  - validation error
  - ข้อมูลไม่ผ่าน validation
  - ฟิลด์หาย
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - 422 Unprocessable Entity request validation

## Fix

A FastAPI `422 Unprocessable Entity` or `field required` response means request validation failed. Read the `detail` array; each `loc` shows whether the bad value came from `body`, `query`, `path`, or `header` and names the field to fix.

Make the sent JSON match the Pydantic model:

```python
from pydantic import BaseModel
class ItemIn(BaseModel):
    name: str
    price: float
@app.post("/items")
async def create_item(item: ItemIn):
    return item
```

```bash
$ curl -i -X POST http://127.0.0.1:8000/items \
    -H "Content-Type: application/json" \
    -d '{"name":"pen","price":12.5}'
```

If a field may be omitted, give it a default such as `description: str | None = None`. A union with `None` but no default is still required.

## Verify

Repeat the request and confirm a 2xx response. Check `/docs` for the exact generated request schema.

<!-- Source: https://fastapi.tiangolo.com/tutorial/body/ -->

<!-- lbc:th -->
# FastAPI - แก้ 422 Request Validation Error

## วิธีแก้

FastAPI `422 Unprocessable Entity` หรือ `field required` หมายถึง request ไม่ผ่าน validation ให้อ่าน array `detail` โดย `loc` จะบอกว่าค่าที่ผิดมาจาก `body`, `query`, `path` หรือ `header` และบอกชื่อ field ที่ต้องแก้

แก้ JSON ที่ส่งให้ตรงกับ Pydantic model:

```python
from pydantic import BaseModel
class ItemIn(BaseModel):
    name: str
    price: float
@app.post("/items")
async def create_item(item: ItemIn):
    return item
```

```bash
$ curl -i -X POST http://127.0.0.1:8000/items \
    -H "Content-Type: application/json" \
    -d '{"name":"pen","price":12.5}'
```

ถ้า field ไม่จำเป็นต้องส่ง ให้กำหนด default เช่น `description: str | None = None` การใส่ union กับ `None` แต่ไม่ใส่ default ยังถือว่า field นั้นจำเป็น

## ตรวจผล

ส่ง request เดิมอีกครั้งและตรวจว่าได้ status 2xx ดู schema ที่ระบบสร้างให้จาก `/docs`

<!-- แหล่งข้อมูล: https://fastapi.tiangolo.com/tutorial/body/ -->
