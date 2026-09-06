---
id: python-pip-wheel-build-error
kind: troubleshooting
language: python
tool: pip
category: packaging
title: Python - pip failed building wheel
title_th: Python - แก้ pip failed building wheel
tags:
  - python
  - pip
  - wheel
  - build
keywords:
  - Failed building wheel
  - Could not build wheels
  - subprocess-exited-with-error
  - error Microsoft Visual C++ required
  - gcc failed
  - Python.h no such file
  - cargo not found
  - สร้าง wheel ไม่ได้
  - compiler หาย
verification_status: unverified
---
<!-- lbc:en -->
# Python - pip failed building wheel

## Fix

`Failed building wheel` means pip could not use a compatible prebuilt wheel and the source build failed. Upgrade build tooling inside the venv, then retry with verbose output:

```bash
$ python -m pip install --upgrade pip setuptools wheel
$ python -m pip install -v PACKAGE_NAME
```

Read the first compiler or missing-header error above the final `subprocess-exited-with-error`. Use a package release that supports your Python/OS/CPU, or install the documented system compiler and development headers for that package.

Do not repeatedly use `sudo pip`. If the package has no wheel for a very new Python version, use a supported Python version in a fresh venv or wait for an upstream release.

## Verify

```bash
$ python -m pip show PACKAGE_NAME
$ python -c "import PACKAGE_NAME"
```

<!-- Source: https://pip.pypa.io/en/stable/cli/pip_install/ -->

<!-- lbc:th -->
# Python - แก้ pip failed building wheel

## วิธีแก้

`Failed building wheel` หมายถึง pip ใช้ prebuilt wheel ที่เข้ากันไม่ได้และ build จาก source ไม่ผ่าน ให้อัปเดต build tooling ภายใน venv แล้วรันแบบ verbose:

```bash
$ python -m pip install --upgrade pip setuptools wheel
$ python -m pip install -v PACKAGE_NAME
```

อ่าน compiler error หรือ missing header ตัวแรกที่อยู่ก่อน `subprocess-exited-with-error` ตอนท้าย ใช้ package release ที่รองรับ Python/OS/CPU ปัจจุบัน หรือติดตั้ง compiler และ development headers ตามเอกสารของ package นั้น

อย่าลอง `sudo pip` ซ้ำ ๆ ถ้ายังไม่มี wheel สำหรับ Python รุ่นใหม่มาก ให้ใช้ Python รุ่นที่ package รองรับใน venv ใหม่หรือรอ upstream

## ตรวจผล

```bash
$ python -m pip show PACKAGE_NAME
$ python -c "import PACKAGE_NAME"
```

<!-- แหล่งข้อมูล: https://pip.pypa.io/en/stable/cli/pip_install/ -->
