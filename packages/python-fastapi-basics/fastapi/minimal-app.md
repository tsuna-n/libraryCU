---
id: fastapi-minimal-app
kind: concept
language: python
tool: fastapi
category: setup
title: FastAPI - Create and run a minimal application
title_th: FastAPI - สร้างและรันแอปขั้นต่ำ
tags:
  - python
  - fastapi
  - uvicorn
  - setup
keywords:
  - start FastAPI
  - minimal app
  - run server
  - เริ่ม FastAPI
  - รัน FastAPI
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - Create and run a minimal application

## Steps

Create and activate a virtual environment, then install FastAPI with its standard dependencies:

```bash
$ python3 -m venv .venv
$ source .venv/bin/activate
$ python -m pip install "fastapi[standard]"
```

Create `main.py`:

```python
from fastapi import FastAPI
app = FastAPI()
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}
```

Run it:

```bash
$ python -m uvicorn main:app --reload
```

Use `--reload` for development only.

## Verify

Open `http://127.0.0.1:8000/docs` or run:

```bash
$ curl http://127.0.0.1:8000/health
```

<!-- Source: https://fastapi.tiangolo.com/deployment/manually/ -->

<!-- lbc:th -->
# FastAPI - สร้างและรันแอปขั้นต่ำ

## ขั้นตอน

สร้างและเปิด virtual environment แล้วติดตั้ง FastAPI พร้อม dependency มาตรฐาน:

```bash
$ python3 -m venv .venv
$ source .venv/bin/activate
$ python -m pip install "fastapi[standard]"
```

สร้างไฟล์ `main.py`:

```python
from fastapi import FastAPI
app = FastAPI()
@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}
```

รันด้วย:

```bash
$ python -m uvicorn main:app --reload
```

ใช้ `--reload` เฉพาะตอนพัฒนา

## ตรวจผล

เปิด `http://127.0.0.1:8000/docs` หรือรัน:

```bash
$ curl http://127.0.0.1:8000/health
```

<!-- แหล่งข้อมูล: https://fastapi.tiangolo.com/deployment/manually/ -->
