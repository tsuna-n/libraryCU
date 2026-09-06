---
id: docker-daemon-not-running
language: docker
tool: docker
category: runtime
title: Docker - Cannot connect to the Docker daemon
tags:
  - daemon
  - runtime
keywords:
  - daemon
  - docker.sock
  - cannot-connect
---
# Docker - Cannot connect to the Docker daemon

Docker client ไม่สามารถเชื่อมต่อไปยัง Docker daemon ได้ มักเกิดจาก service ยังไม่ได้ start หรือ user ไม่มีสิทธิ์เข้าถึง `/var/run/docker.sock`

## ก่อนแก้ (Before - Error)
```bash
$ docker ps
Cannot connect to the Docker daemon at unix:///var/run/docker.sock. Is the docker daemon running?
```

## แก้แล้ว (After - Success)
```bash
# 1. สั่ง start service
$ sudo systemctl start docker

# 2. รันคำสั่งตรวจสอบสำเร็จ
$ docker ps
CONTAINER ID   IMAGE     COMMAND   CREATED   STATUS    PORTS     NAMES
```

## การทำงาน (How it works)
- เปิด service ด้วย `sudo systemctl start docker` (หรือเปิดอัตโนมัติตอนบูตด้วย `sudo systemctl enable docker`)
- หากติดสิทธิ์การเข้าถึง socket: เพิ่ม user เข้า docker group ด้วย `sudo usermod -aG docker $USER` แล้ว login ใหม่
- ตรวจสอบสถานะการทำงานด้วย `sudo systemctl status docker` หรือ `docker info`
