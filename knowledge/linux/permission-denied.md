---
id: linux-permission-denied
language: linux
tool: linux
category: permissions
title: Linux - Permission denied (EACCES)
tags:
  - permissions
  - filesystem
keywords:
  - permission
  - denied
  - eacces
  - chmod
---
# Linux - Permission denied (EACCES)

Process ไม่มีสิทธิ์ (read, write หรือ execute) บนไฟล์หรือ directory ทำให้ kernel ส่ง error `Permission denied` (EACCES/EPERM)

## ก่อนแก้ (Before - Error)
```bash
$ ./deploy.sh
bash: ./deploy.sh: Permission denied
```

## แก้แล้ว (After - Success)
```bash
# เพิ่มสิทธิ์ execute (+x) ให้กับ script
$ chmod +x deploy.sh

# รันใหม่สำเร็จ
$ ./deploy.sh
# SUCCESS: Deploy script starts execution
```

## การทำงาน (How it works)
- ตรวจสอบ permission และ owner ปัจจุบันด้วย `ls -l <file>`
- กรณีขาดสิทธิ์ execute: ปรับสิทธิ์ด้วย `chmod +x <file>`
- กรณีขาดสิทธิ์ read/write: ปรับโหมดด้วย `chmod 644 <file>` หรือเปลี่ยน owner ด้วย `sudo chown $USER:$USER <file>`
- กรณีเป็นไฟล์ระบบที่ต้องใช้ root: รันคำสั่งด้วย `sudo <command>`
