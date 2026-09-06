---
id: linux-command-not-found
kind: troubleshooting
language: linux
tool: linux
category: shell
title: Linux - Command not found
title_th: Linux - ไม่พบคำสั่ง
tags:
  - shell
  - path
keywords:
  - command
  - not-found
  - path
  - ไม่พบคำสั่ง
---
<!-- lbc:en -->
# Linux - Command not found

## Quick answer

`command not found` means the shell cannot find an executable with that name. Check the spelling and `PATH`, install the package if needed, or use a relative path such as `./script.sh`.

## Example

```bash
$ command -v htop
$ sudo apt install htop       # Debian/Ubuntu
$ sudo pacman -S htop         # Arch Linux
```

## Verify

Run `command -v htop`; it should print the executable path.

<!-- lbc:th -->
# Linux - ไม่พบคำสั่ง

## คำตอบสั้น ๆ

ข้อความ `command not found` หมายถึง shell หาไฟล์ executable ชื่อนั้นไม่พบ ให้ตรวจการสะกดและ `$PATH` ติดตั้งแพ็กเกจหากยังไม่มี หรือระบุ path โดยตรง เช่น `./script.sh`

## ตัวอย่าง

```bash
$ command -v htop
$ sudo apt install htop       # Debian/Ubuntu
$ sudo pacman -S htop         # Arch Linux
```

## ตรวจสอบผล

รัน `command -v htop` แล้วควรเห็น path ของโปรแกรม
