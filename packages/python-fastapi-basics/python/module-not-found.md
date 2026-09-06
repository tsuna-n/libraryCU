---
id: python-module-not-found
kind: troubleshooting
language: python
tool: python
category: imports
error_code: ModuleNotFoundError
title: Python - ModuleNotFoundError or ImportError
title_th: Python - แก้ ModuleNotFoundError หรือ ImportError
tags:
  - python
  - import
  - module
keywords:
  - ModuleNotFoundError
  - ImportError
  - No module named
  - cannot import name
  - หา module ไม่เจอ
  - import ไม่ได้
verification_status: unverified
---
<!-- lbc:en -->
# Python - ModuleNotFoundError or ImportError

## Fix

First confirm that the active Python can see the package:

```bash
$ python -c "import sys; print(sys.executable)"
$ python -m pip show PACKAGE_NAME
```

If it is missing, install it with the same interpreter:

```bash
$ python -m pip install PACKAGE_NAME
```

For your own project module, run from the project root and use the full package path:

```bash
$ python -m app.main
```

Avoid naming local files after standard or installed modules, such as `fastapi.py`, `json.py`, or `typing.py`.

## Verify

```bash
$ python -c "import PACKAGE_NAME; print(PACKAGE_NAME.__file__)"
```

<!-- Source: https://docs.python.org/3/tutorial/modules.html#the-module-search-path -->

<!-- lbc:th -->
# Python - แก้ ModuleNotFoundError หรือ ImportError

## วิธีแก้

ตรวจว่า Python ตัวที่กำลังใช้อยู่มองเห็น package หรือไม่:

```bash
$ python -c "import sys; print(sys.executable)"
$ python -m pip show PACKAGE_NAME
```

ถ้ายังไม่มี ให้ติดตั้งผ่าน interpreter ตัวเดียวกัน:

```bash
$ python -m pip install PACKAGE_NAME
```

ถ้าเป็น module ในโปรเจกต์ ให้รันจาก project root และใช้ package path แบบเต็ม:

```bash
$ python -m app.main
```

อย่าตั้งชื่อไฟล์ในโปรเจกต์ชนกับ module เช่น `fastapi.py`, `json.py` หรือ `typing.py`

## ตรวจผล

```bash
$ python -c "import PACKAGE_NAME; print(PACKAGE_NAME.__file__)"
```

<!-- แหล่งข้อมูล: https://docs.python.org/3/tutorial/modules.html#the-module-search-path -->
