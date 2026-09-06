---
id: python-pip-externally-managed
kind: troubleshooting
language: python
tool: pip
category: packaging
title: Python - pip externally-managed-environment
title_th: Python - แก้ pip externally-managed-environment
tags:
  - python
  - pip
  - venv
  - packaging
keywords:
  - externally-managed-environment
  - externally managed environment
  - PEP 668
  - break-system-packages
  - pip install denied
  - ระบบจัดการ Python อยู่
  - pip ติดตั้งไม่ได้
verification_status: unverified
---
<!-- lbc:en -->
# Python - pip externally-managed-environment

## Fix

`externally-managed-environment` means the operating system owns that Python installation. Create a project virtual environment instead of modifying system packages:

```bash
$ python3 -m venv .venv
$ source .venv/bin/activate
$ python -m pip install --upgrade pip
$ python -m pip install PACKAGE_NAME
```

On Windows PowerShell use `.venv\Scripts\Activate.ps1`. For a standalone Python command-line application, `pipx install PACKAGE_NAME` may be more suitable.

Avoid `--break-system-packages` unless you deliberately accept the risk of conflicting with the OS package manager.

## Verify

```bash
$ python -c "import sys; print(sys.prefix); print(sys.base_prefix)"
$ python -m pip show PACKAGE_NAME
```

Inside a venv, `sys.prefix` should differ from `sys.base_prefix`.

<!-- Source: https://packaging.python.org/en/latest/specifications/externally-managed-environments/ -->

<!-- lbc:th -->
# Python - แก้ pip externally-managed-environment

## วิธีแก้

`externally-managed-environment` หมายถึง Python ตัวนี้ถูกดูแลโดยระบบปฏิบัติการ ให้สร้าง virtual environment ของโปรเจกต์แทนการแก้ system package:

```bash
$ python3 -m venv .venv
$ source .venv/bin/activate
$ python -m pip install --upgrade pip
$ python -m pip install PACKAGE_NAME
```

บน Windows PowerShell ใช้ `.venv\Scripts\Activate.ps1` ถ้าเป็น Python command-line application แยกตัว การใช้ `pipx install PACKAGE_NAME` อาจเหมาะกว่า

หลีกเลี่ยง `--break-system-packages` เว้นแต่ตั้งใจรับความเสี่ยงที่จะชนกับ OS package manager

## ตรวจผล

```bash
$ python -c "import sys; print(sys.prefix); print(sys.base_prefix)"
$ python -m pip show PACKAGE_NAME
```

เมื่ออยู่ใน venv ค่า `sys.prefix` ควรต่างจาก `sys.base_prefix`

<!-- แหล่งข้อมูล: https://packaging.python.org/en/latest/specifications/externally-managed-environments/ -->
