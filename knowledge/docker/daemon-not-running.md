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

The Docker client cannot reach the daemon, usually because the service is stopped or the user lacks access to `/var/run/docker.sock`.

Start the service with `sudo systemctl start docker` and enable it at boot with `sudo systemctl enable docker`. Check status with `sudo systemctl status docker` and `docker info`. For permission errors, add the user to the `docker` group and log in again.
