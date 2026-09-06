---
id: fastapi-uvicorn-import-error
kind: troubleshooting
language: python
tool: uvicorn
category: startup
title: FastAPI - Uvicorn could not import module or app
title_th: FastAPI - Uvicorn import module หรือ app ไม่ได้
tags:
  - python
  - fastapi
  - uvicorn
  - import
keywords:
  - Error loading ASGI app
  - Could not import module
  - Attribute app not found
  - uvicorn import error
  - โหลด ASGI app ไม่ได้
  - หา app ไม่เจอ
verification_status: unverified
---
<!-- lbc:en -->
# FastAPI - Uvicorn could not import module or app

## Fix

The target format is `module.path:variable_name`. If the file is `app/main.py` and it contains `app = FastAPI()`, run from the project root:

```bash
$ python -m uvicorn app.main:app --reload
```

If source code is below `src/`, either use its full import path after installing the project or tell Uvicorn where to look:

```bash
$ python -m uvicorn app.main:app --app-dir src --reload
```

Check the import without Uvicorn:

```bash
$ python -c "from app.main import app; print(app)"
```

Fix the first traceback error. Also rename local files such as `fastapi.py` or `uvicorn.py` that shadow installed packages.

## Verify

The import command should succeed, then Uvicorn should print its listening address.

<!-- Source: https://www.uvicorn.org/settings/ -->

<!-- lbc:th -->
# FastAPI - Uvicorn import module หรือ app ไม่ได้

## วิธีแก้

target ต้องอยู่ในรูป `module.path:variable_name` ถ้าไฟล์คือ `app/main.py` และข้างในมี `app = FastAPI()` ให้รันจาก project root:

```bash
$ python -m uvicorn app.main:app --reload
```

ถ้า source อยู่ใต้ `src/` ให้ติดตั้ง project แล้วใช้ import path แบบเต็ม หรือบอก Uvicorn ว่าให้ค้นที่ไหน:

```bash
$ python -m uvicorn app.main:app --app-dir src --reload
```

แยกตรวจ import โดยไม่ผ่าน Uvicorn:

```bash
$ python -c "from app.main import app; print(app)"
```

แก้ error แรกใน traceback และเปลี่ยนชื่อไฟล์ เช่น `fastapi.py` หรือ `uvicorn.py` ที่บัง package จริง

## ตรวจผล

คำสั่ง import ต้องผ่าน จากนั้น Uvicorn ควรแสดง address ที่กำลัง listen

<!-- แหล่งข้อมูล: https://www.uvicorn.org/settings/ -->
