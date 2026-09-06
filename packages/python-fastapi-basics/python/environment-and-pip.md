---
id: python-environment-and-pip
kind: troubleshooting
language: python
tool: python
category: environment
title: Python - Create a virtual environment and use the correct pip
title_th: Python - สร้าง virtual environment และใช้ pip ให้ถูกตัว
tags:
  - python
  - venv
  - pip
  - environment
keywords:
  - virtual environment
  - wrong pip
  - package installed but not found
  - ติดตั้งแล้ว import ไม่ได้
  - pip ผิดตัว
verification_status: unverified
---
<!-- lbc:en -->
# Python - Create a virtual environment and use the correct pip

## Fix

Create one environment per project, activate it, and install packages through the same Python interpreter that runs the application.

```bash
$ python3 -m venv .venv
$ source .venv/bin/activate
$ python -m pip install --upgrade pip
$ python -m pip install -r requirements.txt
```

On Windows PowerShell, activate it with:

```powershell
PS> .venv\Scripts\Activate.ps1
```

## Verify

```bash
$ python -c "import sys; print(sys.executable)"
$ python -m pip --version
```

Both paths should point inside `.venv`. Prefer `python -m pip` over a bare `pip` command.

<!-- Source: https://docs.python.org/3/tutorial/venv.html -->

<!-- lbc:th -->
# Python - สร้าง virtual environment และใช้ pip ให้ถูกตัว

## วิธีแก้

สร้าง environment แยกต่อโปรเจกต์ เปิดใช้งาน แล้วติดตั้ง package ผ่าน Python ตัวเดียวกับที่ใช้รันแอป

```bash
$ python3 -m venv .venv
$ source .venv/bin/activate
$ python -m pip install --upgrade pip
$ python -m pip install -r requirements.txt
```

บน Windows PowerShell ให้เปิดใช้งานด้วย:

```powershell
PS> .venv\Scripts\Activate.ps1
```

## ตรวจผล

```bash
$ python -c "import sys; print(sys.executable)"
$ python -m pip --version
```

ทั้งสอง path ควรชี้เข้า `.venv` ใช้ `python -m pip` แทน `pip` เปล่า ๆ เพื่อลดปัญหาติดตั้งผิด Python

<!-- แหล่งข้อมูล: https://docs.python.org/3/tutorial/venv.html -->
