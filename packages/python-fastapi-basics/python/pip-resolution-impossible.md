---
id: python-pip-resolution-impossible
kind: troubleshooting
language: python
tool: pip
category: packaging
title: Python - pip ResolutionImpossible dependency conflict
title_th: Python - แก้ pip ResolutionImpossible dependency conflict
tags:
  - python
  - pip
  - dependencies
keywords:
  - ResolutionImpossible
  - dependency conflict
  - conflicting dependencies
  - pip resolver
  - pip is looking at multiple versions
  - resolver does not currently take into account
  - dependency ชนกัน
  - package คนละ version
verification_status: unverified
---
<!-- lbc:en -->
# Python - pip ResolutionImpossible dependency conflict

## Fix

`ResolutionImpossible` means no version set satisfies all declared constraints. Reproduce in a clean venv and read the final conflict lines to identify the two requirements that demand incompatible versions:

```bash
$ python3 -m venv .venv-check
$ .venv-check/bin/python -m pip install -r requirements.txt
```

Change only direct requirements: remove an unnecessary exact pin, choose compatible top-level package versions, or add a reviewed constraint. Do not install with `--no-deps` because it can create a broken environment.

After installation, check the resolved environment:

```bash
$ python -m pip check
$ python -m pip freeze
```

## Verify

Recreate the environment from scratch using the updated requirements and run tests.

<!-- Source: https://pip.pypa.io/en/stable/topics/dependency-resolution/ -->

<!-- lbc:th -->
# Python - แก้ pip ResolutionImpossible dependency conflict

## วิธีแก้

`ResolutionImpossible` หมายถึงไม่มีชุด version ที่ตรงกับ constraint ทั้งหมด ให้ทำซ้ำใน venv ใหม่และอ่านบรรทัด conflict ช่วงท้ายเพื่อหา requirement สองตัวที่ต้องการ version ไม่เข้ากัน:

```bash
$ python3 -m venv .venv-check
$ .venv-check/bin/python -m pip install -r requirements.txt
```

แก้เฉพาะ direct requirement: เอา exact pin ที่ไม่จำเป็นออก เลือก version ของ top-level package ที่เข้ากัน หรือเพิ่ม constraint ที่ตรวจแล้ว ห้ามใช้ `--no-deps` เพื่อฝืนติดตั้ง เพราะอาจได้ environment ที่พัง

หลังติดตั้งให้ตรวจชุด dependency:

```bash
$ python -m pip check
$ python -m pip freeze
```

## ตรวจผล

สร้าง environment ใหม่จากศูนย์ด้วย requirements ที่แก้แล้วและรัน tests

<!-- แหล่งข้อมูล: https://pip.pypa.io/en/stable/topics/dependency-resolution/ -->
