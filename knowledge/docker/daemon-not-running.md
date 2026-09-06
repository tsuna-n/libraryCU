---
id: docker-daemon-not-running
kind: troubleshooting
language: docker
tool: docker
category: runtime
title: Docker - Cannot connect to the Docker daemon
title_th: Docker - เชื่อมต่อ Docker daemon ไม่ได้
tags:
  - daemon
  - runtime
keywords:
  - daemon
  - docker.sock
  - cannot-connect
  - daemon not running
  - เชื่อมต่อ docker ไม่ได้
  - daemon ไม่ทำงาน
---
<!-- lbc:en -->
# Docker - Cannot connect to the Docker daemon

## Quick answer

The Docker client cannot reach the daemon when the service is stopped or the current user cannot access its socket. Check the service first, then inspect socket permissions if the daemon is already running.

## Check and start the service

```bash
$ sudo systemctl status docker
$ sudo systemctl start docker
$ docker info
```

If the service is running but access is denied, inspect `/var/run/docker.sock`. Adding a user to the `docker` group grants root-equivalent Docker access, so do it only when that trust level is appropriate.

<!-- lbc:th -->
# Docker - เชื่อมต่อ Docker daemon ไม่ได้

## คำตอบสั้น ๆ

Docker client เชื่อมต่อ daemon ไม่ได้เมื่อ service หยุดทำงาน หรือ user ปัจจุบันไม่มีสิทธิ์เข้าถึง socket ให้ตรวจ service ก่อน แล้วค่อยตรวจ permission ของ socket หาก daemon ทำงานอยู่แล้ว

## ตรวจและเริ่ม service

```bash
$ sudo systemctl status docker
$ sudo systemctl start docker
$ docker info
```

ถ้า service ทำงานแต่ยังเข้าไม่ได้ ให้ตรวจ `/var/run/docker.sock` การเพิ่ม user เข้า group `docker` เทียบเท่ากับการให้สิทธิ์ระดับ root ผ่าน Docker จึงควรทำเฉพาะเมื่อยอมรับระดับความไว้วางใจนี้ได้
