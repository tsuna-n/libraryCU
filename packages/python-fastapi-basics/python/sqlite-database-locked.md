---
id: python-sqlite-database-locked
kind: troubleshooting
language: python
tool: sqlite3
category: database
title: Python - sqlite3 database is locked
title_th: Python - แก้ sqlite3 database is locked
tags:
  - python
  - sqlite
  - database
  - locking
keywords:
  - sqlite3.OperationalError
  - database is locked
  - database table is locked
  - SQLite busy
  - transaction open
  - database ถูกล็อก
  - sqlite เขียนไม่ได้
verification_status: unverified
---
<!-- lbc:en -->
# Python - sqlite3 database is locked

## Fix

`sqlite3.OperationalError: database is locked` means another connection holds a conflicting transaction. Find code paths that leave transactions open, commit or roll back promptly, and always close connections:

```python
import sqlite3
with sqlite3.connect("app.db", timeout=5) as connection:
    connection.execute("UPDATE jobs SET status = ? WHERE id = ?", ("done", job_id))
```

Keep write transactions short and do network/file work outside them. A larger timeout only waits longer; it does not fix a connection that never commits. For suitable workloads, evaluate WAL mode, but SQLite still has one writer at a time.

## Verify

Run two representative concurrent operations and confirm both finish, then check that exception paths roll back and close the connection.

<!-- Source: https://docs.python.org/3/library/sqlite3.html#transaction-control -->

<!-- lbc:th -->
# Python - แก้ sqlite3 database is locked

## วิธีแก้

`sqlite3.OperationalError: database is locked` หมายถึง connection อื่นค้าง transaction ที่ชนกัน ให้หา code path ที่เปิด transaction ทิ้งไว้ commit หรือ rollback ให้เร็ว และปิด connection เสมอ:

```python
import sqlite3
with sqlite3.connect("app.db", timeout=5) as connection:
    connection.execute("UPDATE jobs SET status = ? WHERE id = ?", ("done", job_id))
```

ทำ write transaction ให้สั้นและย้ายงาน network/file ออกไปข้างนอก การเพิ่ม timeout แค่รอนานขึ้น ไม่ได้แก้ connection ที่ไม่เคย commit ถ้า workload เหมาะให้พิจารณา WAL mode แต่ SQLite ยังเขียนพร้อมกันได้ครั้งละหนึ่ง writer

## ตรวจผล

รัน operation ที่ใช้งานจริงสองตัวพร้อมกันและตรวจว่าจบทั้งคู่ แล้วตรวจว่า exception path rollback และปิด connection

<!-- แหล่งข้อมูล: https://docs.python.org/3/library/sqlite3.html#transaction-control -->
