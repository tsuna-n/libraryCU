---
id: docker-port-in-use
language: docker
tool: docker
category: networking
title: Docker - Port already in use
tags:
  - ports
  - networking
keywords:
  - address-already-in-use
  - bind
  - port
---
# Docker - Port already in use

Port บนโฮสต์ที่ระบุใน container port mapping ถูกโปรเซสอื่นหรือ container อื่นใช้งานอยู่แล้ว ทำให้เกิด error `bind: address already in use`

## ก่อนแก้ (Before - Error)
```bash
$ docker run -p 8080:80 nginx
docker: Error response from daemon: driver failed programming external connectivity: bind: address already in use.
```

## แก้แล้ว (After - Success)
```bash
# ทางเลือกที่ 1: เปลี่ยน port บนเครื่อง host ที่ไม่ชน
$ docker run -p 8081:80 nginx
# SUCCESS: container รันผ่าน port 8081 ได้ปกติ

# ทางเลือกที่ 2: ค้นหาและหยุด process ที่ใช้งาน port 8080 อยู่
$ lsof -i :8080
$ docker stop <old_container_id>
```

## การทำงาน (How it works)
- รูปแบบของ port mapping คือ `-p <host_port>:<container_port>` หาก host_port ถูกใช้งานแล้วจะไม่สามารถ bind ซ้ำได้
- ค้นหา process ที่จอง port ด้วย `ss -ltnp | grep <port>` หรือ `lsof -i :<port>`
- ตรวจสอบความถูกต้องด้วย `docker ps` หรือ `docker compose up`
