---
id: git-merge-conflict
language: git
tool: git
category: version-control
title: Git - Merge conflict
tags:
  - merge
  - conflict
keywords:
  - conflict
  - merge
  - rebase
---
# Git - Merge conflict

เกิดขึ้นเมื่อทั้งสอง branch มีการแก้ไขโค้ดที่บรรทัดเดียวกัน Git รวมไฟล์อัตโนมัติไม่ได้ จึงแทรก conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) ลงในไฟล์

## ก่อนแก้ (Before - Conflict)
```text
<<<<<<< HEAD
const PORT = 3000;
=======
const PORT = 8080;
>>>>>>> feature-branch
```

## แก้แล้ว (After - Success)
```javascript
// เลือกโค้ดที่ถูกต้อง และลบ conflict markers ออกให้หมด
const PORT = 8080;
```
```bash
# บันทึกไฟล์และ commit เพื่อเสร็จสิ้นการ merge
$ git add app.js
$ git commit -m "fix: resolve merge conflict on PORT"
# SUCCESS: [main a1b2c3d] fix: resolve merge conflict on PORT
```

## การทำงาน (How it works)
- ใช้ `git status` ดูรายชื่อไฟล์ที่ติดสถานะ conflict
- เปิดไฟล์แก้ไข เลือกบรรทัดที่ต้องการ และลบเครื่องหมาย `<<<<<<<`, `=======`, `>>>>>>>` ออกทั้งหมด
- ตรวจสอบว่าไม่มี markers หลงเหลือด้วย `git diff --check` แล้ว `git add <file>` และ `git commit`
