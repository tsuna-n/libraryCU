---
id: fastapi-cors
kind: troubleshooting
language: python
tool: fastapi
category: networking
title: FastAPI - Browser request blocked by CORS
title_th: FastAPI - Browser บล็อก request ด้วย CORS
tags:
  - python
  - fastapi
  - cors
  - browser
keywords:
  - CORS
  - blocked by CORS policy
  - preflight
  - Access-Control-Allow-Origin
  - browser บล็อก
  - ข้าม origin
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - Browser request blocked by CORS

## Fix

When a browser says a FastAPI request is blocked by CORS, add `CORSMiddleware` with the exact frontend origin. Scheme, host, and port must all match.

```python
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
```

Do not combine credentialed requests with a wildcard origin. Add each production HTTPS origin explicitly.

## Verify

Test the preflight from the same origin as the frontend:

```bash
$ curl -i -X OPTIONS http://127.0.0.1:8000/items \
    -H "Origin: http://localhost:5173" \
    -H "Access-Control-Request-Method: POST"
```

The response should include the matching `access-control-allow-origin` header.

<!-- Source: https://fastapi.tiangolo.com/tutorial/cors/ -->

<!-- lbc:th -->
# FastAPI - Browser บล็อก request ด้วย CORS

## วิธีแก้

เมื่อ browser บล็อก FastAPI request ด้วย CORS ให้เพิ่ม `CORSMiddleware` พร้อม origin ของ frontend ที่ตรงทุกส่วน ทั้ง protocol, host และ port

```python
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
```

อย่าใช้ wildcard origin กับ request ที่ส่ง credentials ให้ระบุ production HTTPS origin แต่ละตัวอย่างชัดเจน

## ตรวจผล

ทดสอบ preflight โดยใช้ origin เดียวกับ frontend:

```bash
$ curl -i -X OPTIONS http://127.0.0.1:8000/items \
    -H "Origin: http://localhost:5173" \
    -H "Access-Control-Request-Method: POST"
```

response ควรมี header `access-control-allow-origin` ที่ตรงกับ origin

<!-- แหล่งข้อมูล: https://fastapi.tiangolo.com/tutorial/cors/ -->
