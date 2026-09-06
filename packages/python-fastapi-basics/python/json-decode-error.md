---
id: python-json-decode-error
kind: troubleshooting
language: python
tool: python
category: data
error_code: JSONDecodeError
title: Python - JSONDecodeError
title_th: Python - แก้ JSONDecodeError
tags:
  - python
  - json
  - decoding
keywords:
  - JSONDecodeError
  - Expecting value
  - invalid JSON
  - json decode
  - JSON ไม่ถูกต้อง
  - แปลง JSON ไม่ได้
verification_status: unverified
---
<!-- lbc:en -->
# Python - JSONDecodeError

## Fix

Inspect the raw text before decoding. An empty response, an HTML error page, trailing comma, or single-quoted keys are not valid JSON.

```python
import json
raw = response.text
if not raw.strip():
    raise ValueError("empty response body")
try:
    data = json.loads(raw)
except json.JSONDecodeError as exc:
    raise ValueError(
        f"invalid JSON at line {exc.lineno}, column {exc.colno}: {raw[:200]!r}"
    ) from exc
```

For HTTP calls, check the status and content type before parsing:

```python
response.raise_for_status()
if "application/json" not in response.headers.get("content-type", ""):
    raise ValueError("response is not JSON")
```

## Verify

Test one valid JSON body, an empty body, and the original invalid body.

<!-- Source: https://docs.python.org/3/library/json.html#json.JSONDecodeError -->

<!-- lbc:th -->
# Python - แก้ JSONDecodeError

## วิธีแก้

ตรวจข้อความดิบก่อน decode เพราะ response ว่าง หน้า error แบบ HTML, comma ตัวท้าย หรือ key ที่ใช้ single quote ไม่ใช่ JSON ที่ถูกต้อง

```python
import json
raw = response.text
if not raw.strip():
    raise ValueError("empty response body")
try:
    data = json.loads(raw)
except json.JSONDecodeError as exc:
    raise ValueError(
        f"invalid JSON at line {exc.lineno}, column {exc.colno}: {raw[:200]!r}"
    ) from exc
```

ถ้าเป็น HTTP ให้ตรวจ status และ content type ก่อน parse:

```python
response.raise_for_status()
if "application/json" not in response.headers.get("content-type", ""):
    raise ValueError("response is not JSON")
```

## ตรวจผล

ทดสอบ JSON ที่ถูกต้อง, body ว่าง และ body เดิมที่เคยพัง

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/json.html#json.JSONDecodeError -->
