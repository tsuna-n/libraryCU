---
id: fastapi-body-vs-query
kind: troubleshooting
language: python
tool: fastapi
category: requests
title: FastAPI - Request value read from query instead of JSON body
title_th: FastAPI - ค่าถูกอ่านจาก query แทน JSON body
tags:
  - python
  - fastapi
  - request-body
  - query
keywords:
  - body vs query
  - missing query parameter
  - request body
  - query parameter
  - ส่ง body แล้วหา query
  - ค่าอยู่ผิดตำแหน่ง
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - Request value read from query instead of JSON body

## Fix

A plain scalar parameter such as `name: str` is read from the query string. Use a Pydantic model when the client sends a JSON object:

```python
from pydantic import BaseModel
class SearchIn(BaseModel):
    name: str
    limit: int = 10
@app.post("/search")
async def search(payload: SearchIn):
    return payload
```

Then send:

```json
{"name": "book", "limit": 5}
```

If the API intentionally accepts one scalar JSON value, declare it explicitly with `Body()`:

```python
from typing import Annotated
from fastapi import Body
async def set_enabled(enabled: Annotated[bool, Body()]):
    return {"enabled": enabled}
```

## Verify

Check `/docs`. The value should now appear under Request body, not Parameters.

<!-- Source: https://fastapi.tiangolo.com/tutorial/body/ -->

<!-- lbc:th -->
# FastAPI - ค่าถูกอ่านจาก query แทน JSON body

## วิธีแก้

parameter แบบ scalar เช่น `name: str` จะถูกอ่านจาก query string ถ้า client ส่ง JSON object ให้ใช้ Pydantic model:

```python
from pydantic import BaseModel
class SearchIn(BaseModel):
    name: str
    limit: int = 10
@app.post("/search")
async def search(payload: SearchIn):
    return payload
```

แล้วส่ง:

```json
{"name": "book", "limit": 5}
```

ถ้า API ตั้งใจรับ JSON scalar เพียงค่าเดียว ให้ระบุด้วย `Body()`:

```python
from typing import Annotated
from fastapi import Body
async def set_enabled(enabled: Annotated[bool, Body()]):
    return {"enabled": enabled}
```

## ตรวจผล

ดู `/docs` ค่านี้ควรแสดงใต้ Request body ไม่ใช่ Parameters

<!-- แหล่งข้อมูล: https://fastapi.tiangolo.com/tutorial/body/ -->
