---
id: python-unicode-errors
kind: troubleshooting
language: python
tool: python
category: text
error_code: UnicodeDecodeError
title: Python - UnicodeDecodeError and UnicodeEncodeError
title_th: Python - แก้ UnicodeDecodeError และ UnicodeEncodeError
tags:
  - python
  - unicode
  - encoding
keywords:
  - UnicodeDecodeError
  - codec cannot decode byte
  - invalid start byte
  - UnicodeEncodeError
  - codec cannot encode character
  - mojibake
  - อ่าน encoding ไม่ได้
  - เขียนตัวอักษรไม่ได้
verification_status: unverified
---
<!-- lbc:en -->
# Python - UnicodeDecodeError and UnicodeEncodeError

## Decode error

`UnicodeDecodeError: invalid start byte` means bytes were decoded with the wrong encoding or the input is damaged. Obtain the encoding from the file format, HTTP `Content-Type`, or producer, then pass it explicitly:

```python
from pathlib import Path
text = Path("data.txt").read_text(encoding="utf-8")
```

Do not switch blindly to `latin-1`; it accepts every byte and can hide corrupted text.

## Encode error

`UnicodeEncodeError` means the output encoding cannot represent a character. Write UTF-8 where supported:

```python
Path("result.txt").write_text(text, encoding="utf-8")
```

Use `errors="replace"` or `errors="ignore"` only when losing characters is an explicit product decision.

## Verify

Round-trip representative Thai, English, emoji, and the original failing input.

<!-- Source: https://docs.python.org/3/howto/unicode.html -->

<!-- lbc:th -->
# Python - แก้ UnicodeDecodeError และ UnicodeEncodeError

## Decode error

`UnicodeDecodeError: invalid start byte` หมายถึง decode bytes ด้วย encoding ผิดหรือ input เสีย ให้หา encoding จากรูปแบบไฟล์, HTTP `Content-Type` หรือระบบที่สร้างข้อมูล แล้วระบุให้ชัด:

```python
from pathlib import Path
text = Path("data.txt").read_text(encoding="utf-8")
```

อย่าเปลี่ยนเป็น `latin-1` แบบเดา เพราะรับได้ทุก byte และอาจซ่อนข้อความที่เสีย

## Encode error

`UnicodeEncodeError` หมายถึง encoding ปลายทางแทนตัวอักษรนั้นไม่ได้ ให้เขียนเป็น UTF-8 เมื่อระบบรองรับ:

```python
Path("result.txt").write_text(text, encoding="utf-8")
```

ใช้ `errors="replace"` หรือ `errors="ignore"` เฉพาะเมื่อยอมเสียตัวอักษรตามข้อกำหนดจริง

## ตรวจผล

ทดสอบ round-trip ด้วยภาษาไทย อังกฤษ emoji และ input เดิมที่เคยพัง

<!-- แหล่งข้อมูล: https://docs.python.org/3/howto/unicode.html -->
