---
id: python-network-errors
kind: troubleshooting
language: python
tool: python
category: networking
title: Python - ConnectionError TimeoutError and DNS errors
title_th: Python - แก้ ConnectionError TimeoutError และ DNS error
tags:
  - python
  - network
  - timeout
  - dns
keywords:
  - ConnectionError
  - ConnectionRefusedError
  - connection refused
  - ConnectionResetError
  - connection reset by peer
  - ConnectionAbortedError
  - BrokenPipeError
  - TimeoutError
  - timed out
  - socket.gaierror
  - name or service not known
  - temporary failure in name resolution
  - เชื่อมต่อไม่ได้
  - หมดเวลา
  - หา DNS ไม่เจอ
verification_status: unverified
---
<!-- lbc:en -->
# Python - ConnectionError TimeoutError and DNS errors

## Connection refused

`ConnectionRefusedError` means the host answered but nothing accepted the target port. Confirm scheme, host, port, and that the service is listening:

```bash
$ ss -ltnp
$ curl -v http://127.0.0.1:8000/health
```

## DNS error

`socket.gaierror` or `name or service not known` means hostname resolution failed. Check the hostname for a typo, then test resolution from the same container or machine:

```bash
$ getent hosts api.example.com
```

## Timeout or reset

For `TimeoutError`, distinguish connect, read, write, and pool timeouts. Set explicit finite timeouts and retry only idempotent operations with bounded backoff. `ConnectionResetError` or `BrokenPipeError` means the peer closed the connection; check server logs, proxy limits, payload size, and idle timeout.

## Verify

Test the exact URL from the same runtime environment, then retry the application call with network debug logging and secrets redacted.

<!-- Source: https://docs.python.org/3/library/exceptions.html#ConnectionError -->

<!-- lbc:th -->
# Python - แก้ ConnectionError TimeoutError และ DNS error

## Connection refused

`ConnectionRefusedError` หมายถึง host ตอบได้แต่ไม่มี service รับ port เป้าหมาย ให้ตรวจ scheme, host, port และว่า service กำลัง listen:

```bash
$ ss -ltnp
$ curl -v http://127.0.0.1:8000/health
```

## DNS error

`socket.gaierror` หรือ `name or service not known` หมายถึง resolve hostname ไม่ได้ ให้ตรวจตัวสะกดแล้วทดสอบจาก container หรือเครื่องเดียวกับที่รันแอป:

```bash
$ getent hosts api.example.com
```

## Timeout หรือ reset

ถ้าเป็น `TimeoutError` ให้แยกว่า timeout ตอน connect, read, write หรือรอ pool ตั้ง timeout ที่มีขอบเขต และ retry เฉพาะ operation ที่ทำซ้ำได้อย่างปลอดภัยด้วย backoff จำกัด ถ้าเป็น `ConnectionResetError` หรือ `BrokenPipeError` แปลว่าปลายทางปิด connection ให้ตรวจ server log, proxy limit, ขนาด payload และ idle timeout

## ตรวจผล

ทดสอบ URL เดิมจาก runtime environment เดียวกัน แล้วลอง application call พร้อม network debug log ที่ปิดบัง secret

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/exceptions.html#ConnectionError -->
