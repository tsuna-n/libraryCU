---
id: fastapi-file-upload
kind: troubleshooting
language: python
tool: fastapi
category: requests
title: FastAPI - File upload needs python-multipart
title_th: FastAPI - อัปโหลดไฟล์ต้องใช้ python-multipart
tags:
  - python
  - fastapi
  - upload
  - multipart
keywords:
  - python-multipart
  - Form data requires
  - multipart form
  - UploadFile
  - อัปโหลดไฟล์ไม่ได้
  - ต้องติดตั้ง python-multipart
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - File upload needs python-multipart

## Fix

`Form data requires "python-multipart"` means the upload parser is missing. Install `python-multipart` in the same environment as FastAPI:

```bash
$ python -m pip install python-multipart
```

Use `UploadFile` for uploaded files, especially large ones:

```python
from fastapi import FastAPI, UploadFile
app = FastAPI()
@app.post("/files")
async def upload(file: UploadFile):
    return {
        "filename": file.filename,
        "content_type": file.content_type,
    }
```

Send `multipart/form-data` and use the same field name as the function parameter:

```bash
$ curl -i -F "file=@example.pdf" http://127.0.0.1:8000/files
```

## Verify

The request should return the filename without a `Form data requires "python-multipart"` startup error or a missing-field 422.

<!-- Source: https://fastapi.tiangolo.com/tutorial/request-files/ -->

<!-- lbc:th -->
# FastAPI - อัปโหลดไฟล์ต้องใช้ python-multipart

## วิธีแก้

error `Form data requires "python-multipart"` หมายถึงยังไม่มี upload parser ให้ติดตั้ง `python-multipart` ใน environment เดียวกับ FastAPI:

```bash
$ python -m pip install python-multipart
```

ใช้ `UploadFile` รับไฟล์ โดยเฉพาะไฟล์ขนาดใหญ่:

```python
from fastapi import FastAPI, UploadFile
app = FastAPI()
@app.post("/files")
async def upload(file: UploadFile):
    return {
        "filename": file.filename,
        "content_type": file.content_type,
    }
```

ส่งแบบ `multipart/form-data` และใช้ชื่อ field ให้ตรงกับ parameter:

```bash
$ curl -i -F "file=@example.pdf" http://127.0.0.1:8000/files
```

## ตรวจผล

request ควรคืนชื่อไฟล์ โดยไม่มี error `Form data requires "python-multipart"` ตอนเริ่มแอปหรือ 422 เพราะหา field ไม่พบ

<!-- แหล่งข้อมูล: https://fastapi.tiangolo.com/tutorial/request-files/ -->
