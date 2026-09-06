---
id: fastapi-response-validation-error
kind: troubleshooting
language: python
tool: fastapi
category: validation
error_code: ResponseValidationError
title: FastAPI - ResponseValidationError
title_th: FastAPI - แก้ ResponseValidationError
tags:
  - python
  - fastapi
  - pydantic
  - response-model
keywords:
  - ResponseValidationError
  - response validation
  - missing response field
  - invalid response
  - response ไม่ตรง model
  - field ใน response หาย
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - ResponseValidationError

## Fix

Read `loc` in the server traceback. It points to the missing or invalid response field. Make the returned value match the declared return type or `response_model`:

```python
from pydantic import BaseModel
class ItemOut(BaseModel):
    id: int
    name: str
@app.get("/items/{item_id}", response_model=ItemOut)
async def read_item(item_id: int):
    row = await repository.get(item_id)
    return {"id": row.id, "name": row.name}
```

If the endpoint can have no item, raise a deliberate 404 instead of returning `None`:

```python
from fastapi import HTTPException
if row is None:
    raise HTTPException(status_code=404, detail="Item not found")
```

Do not disable response validation merely to hide a mismatched return shape.

## Verify

Test success and not-found cases. The success JSON must match the schema in `/docs`.

<!-- Source: https://fastapi.tiangolo.com/tutorial/response-model/ -->

<!-- lbc:th -->
# FastAPI - แก้ ResponseValidationError

## วิธีแก้

อ่าน `loc` ใน server traceback เพื่อหา response field ที่หายหรือชนิดไม่ถูก แล้วแก้ค่าที่ return ให้ตรงกับ return type หรือ `response_model`:

```python
from pydantic import BaseModel
class ItemOut(BaseModel):
    id: int
    name: str
@app.get("/items/{item_id}", response_model=ItemOut)
async def read_item(item_id: int):
    row = await repository.get(item_id)
    return {"id": row.id, "name": row.name}
```

ถ้า endpoint อาจหา item ไม่พบ ให้ตอบ 404 อย่างตั้งใจแทนการ return `None`:

```python
from fastapi import HTTPException
if row is None:
    raise HTTPException(status_code=404, detail="Item not found")
```

อย่าปิด response validation เพียงเพื่อซ่อนรูปข้อมูลที่ไม่ตรง

## ตรวจผล

ทดสอบทั้งกรณีสำเร็จและหาไม่พบ JSON ของกรณีสำเร็จต้องตรงกับ schema ใน `/docs`

<!-- แหล่งข้อมูล: https://fastapi.tiangolo.com/tutorial/response-model/ -->
