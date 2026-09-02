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

The host port requested in the container mapping is already bound by another process or container, so the engine fails with `bind: address already in use`.

Find the process holding the port with `ss -ltnp | grep <port>` or `lsof -i :<port>`, stop the conflicting container (`docker ps`, `docker stop`), or publish a different host port (`-p 8081:80`). Verify by rerunning `docker run` or `docker compose up`.
