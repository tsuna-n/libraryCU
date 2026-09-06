---
id: fastapi-port-in-use
kind: troubleshooting
language: python
tool: uvicorn
category: networking
error_code: EADDRINUSE
title: FastAPI - Address already in use
title_th: FastAPI - พอร์ตถูกใช้งานอยู่แล้ว
tags:
  - python
  - fastapi
  - uvicorn
  - port
keywords:
  - address already in use
  - EADDRINUSE
  - Errno 98
  - port 8000
  - พอร์ตชน
  - พอร์ตถูกใช้
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - Address already in use

## Fix

Find the process listening on port 8000:

```bash
$ ss -ltnp | grep ':8000'
```

If it is an old development server, stop it normally in its terminal with Ctrl+C. Otherwise use a different development port:

```bash
$ python -m uvicorn app.main:app --reload --port 8001
```

Do not kill a process until its PID and purpose are known.

## Verify

```bash
$ curl -i http://127.0.0.1:8001/docs
```

<!-- Source: https://www.uvicorn.org/settings/#socket-binding -->

<!-- lbc:th -->
# FastAPI - พอร์ตถูกใช้งานอยู่แล้ว

## วิธีแก้

หา process ที่ listen port 8000:

```bash
$ ss -ltnp | grep ':8000'
```

ถ้าเป็น development server เก่า ให้หยุดตามปกติด้วย Ctrl+C ใน terminal เดิม หรือเปลี่ยนไปใช้ port อื่น:

```bash
$ python -m uvicorn app.main:app --reload --port 8001
```

อย่าหยุด process จนกว่าจะตรวจ PID และรู้ว่าเป็นโปรแกรมอะไร

## ตรวจผล

```bash
$ curl -i http://127.0.0.1:8001/docs
```

<!-- แหล่งข้อมูล: https://www.uvicorn.org/settings/#socket-binding -->
