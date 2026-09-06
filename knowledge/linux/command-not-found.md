---
id: linux-command-not-found
language: linux
tool: linux
category: shell
title: Linux - Command not found
tags:
  - shell
  - path
keywords:
  - command
  - not-found
  - path
---
# Linux - Command not found

Shell ไม่พบโปรแกรม executable ตามชื่อที่เรียกในทุก directory ที่ระบุใน `$PATH` หรือยังไม่ได้ติดตั้งโปรแกรมนั้น

## ก่อนแก้ (Before - Error)
```bash
$ htop
bash: htop: command not found
```

## แก้แล้ว (After - Success)
```bash
# 1. ติดตั้งโปรแกรมผ่าน package manager
$ sudo pacman -S htop   # หรือ sudo apt install htop

# 2. เรียกใช้งานได้สำเร็จ
$ htop
# SUCCESS: เปิดโปรแกรม htop ทำงานได้ปกติ
```

## การทำงาน (How it works)
- ตรวจสอบคำสั่งด้วย `which <command>` และตรวจสอบ directories ใน `$PATH` ด้วย `echo $PATH`
- หากเป็น script หรือ binary ที่ไม่ได้อยู่ใน `$PATH` ให้รันโดยระบุ path เช่น `./script.sh` หรือเพิ่ม directory เข้า `$PATH`
- ตรวจสอบความถูกต้องเมื่อ `which <command>` แสดง path ของตัวโปรแกรม
