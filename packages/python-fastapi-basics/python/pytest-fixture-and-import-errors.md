---
id: python-pytest-fixture-and-import-errors
kind: troubleshooting
language: python
tool: pytest
category: testing
title: Python - pytest fixture not found and import errors
title_th: Python - แก้ pytest fixture not found และ import error
tags:
  - python
  - pytest
  - fixture
  - import
keywords:
  - fixture not found
  - pytest fixture
  - ImportError while importing test module
  - test collection error
  - collected 0 items
  - no tests ran
  - หา fixture ไม่เจอ
  - pytest import ไม่ได้
verification_status: unverified
---
<!-- lbc:en -->
# Python - pytest fixture not found and import errors

## Fixture not found

`fixture ... not found` means the fixture is not visible to that test or its name is wrong. Put shared fixtures in `conftest.py` at the test directory or a parent, decorate them, and inspect visibility:

```python
import pytest
@pytest.fixture
def client():
    return build_client()
```

```bash
$ python -m pytest --fixtures -q
```

## Import or collection error

`ImportError while importing test module` or a pytest collection error means test imports failed before assertions ran. Run pytest through the active interpreter from the project root:

```bash
$ python -m pytest tests/path/test_file.py -vv
```

Install the project into the venv when it uses a `src/` layout, avoid duplicate test module basenames without packages, and fix the first import traceback before debugging assertions.

## Verify

Run the single test node, then its file, then the complete suite.

<!-- Source: https://docs.pytest.org/en/stable/example/simple.html#package-directory-level-fixtures-setups -->

<!-- lbc:th -->
# Python - แก้ pytest fixture not found และ import error

## Fixture not found

`fixture ... not found` หมายถึง test มองไม่เห็น fixture หรือชื่อไม่ตรง ให้วาง shared fixture ใน `conftest.py` ที่ test directory หรือ parent ใส่ decorator และตรวจ fixture ที่มองเห็น:

```python
import pytest
@pytest.fixture
def client():
    return build_client()
```

```bash
$ python -m pytest --fixtures -q
```

## Import หรือ collection error

`ImportError while importing test module` หรือ pytest collection error หมายถึง import พังก่อนเริ่ม assertion ให้รัน pytest ผ่าน interpreter ที่เปิดใช้อยู่จาก project root:

```bash
$ python -m pytest tests/path/test_file.py -vv
```

ถ้าใช้ `src/` layout ให้ติดตั้ง project เข้า venv หลีกเลี่ยงชื่อ test module ซ้ำกันโดยไม่มี package และแก้ import traceback ตัวแรกก่อนดู assertion

## ตรวจผล

รัน test node เดียวก่อน จากนั้นทั้งไฟล์และทั้ง suite

<!-- แหล่งข้อมูล: https://docs.pytest.org/en/stable/example/simple.html#package-directory-level-fixtures-setups -->
