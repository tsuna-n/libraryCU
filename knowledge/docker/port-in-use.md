---
id: docker-port-in-use
kind: troubleshooting
language: docker
tool: docker
category: networking
title: Docker - Port already in use
title_th: Docker - พอร์ตถูกใช้งานอยู่แล้ว
tags:
  - ports
  - networking
keywords:
  - address-already-in-use
  - bind
  - port
  - port conflict
  - พอร์ตชน
  - พอร์ตถูกใช้งาน
---
<!-- lbc:en -->
# Docker - Port already in use

## Quick answer

`bind: address already in use` means another process or container already owns the requested host port. Find and stop that listener, or map the container to a different host port.

## Find the current listener

```bash
$ ss -ltnp | grep ':8080'
$ docker ps --filter publish=8080
```

## Use a different host port

```bash
$ docker run -p 8081:80 nginx
```

Verify with `docker ps` and connect to port `8081`.

<!-- lbc:th -->
# Docker - พอร์ตถูกใช้งานอยู่แล้ว

## คำตอบสั้น ๆ

`bind: address already in use` หมายถึงมี process หรือ container อื่นใช้พอร์ตบน host อยู่แล้ว ให้หาและหยุดตัวที่จองพอร์ต หรือเปลี่ยนไปใช้ host port อื่น

## หาตัวที่ใช้พอร์ตอยู่

```bash
$ ss -ltnp | grep ':8080'
$ docker ps --filter publish=8080
```

## เปลี่ยน host port

```bash
$ docker run -p 8081:80 nginx
```

ตรวจสอบด้วย `docker ps` แล้วลองเชื่อมต่อผ่านพอร์ต `8081`
