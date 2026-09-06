---
id: python-pip-ssl-certificate-error
kind: troubleshooting
language: python
tool: pip
category: networking
title: Python - pip SSL certificate verify failed
title_th: Python - แก้ pip SSL certificate verify failed
tags:
  - python
  - pip
  - ssl
  - certificate
keywords:
  - CERTIFICATE_VERIFY_FAILED
  - SSL certificate verify failed
  - unable to get local issuer certificate
  - self signed certificate in certificate chain
  - trusted-host
  - pip SSL
  - ตรวจ certificate ไม่ผ่าน
  - ใบรับรองไม่ถูกต้อง
verification_status: unverified
---
<!-- lbc:en -->
# Python - pip SSL certificate verify failed

## Fix

`CERTIFICATE_VERIFY_FAILED` means Python cannot build a trusted chain for the server certificate. Check system date/time, target hostname, and whether a company proxy replaces TLS certificates.

Update the operating-system CA bundle. If your organization uses a private CA, obtain its CA file from the administrator and configure pip explicitly:

```bash
$ python -m pip install --cert /path/to/company-ca.pem PACKAGE_NAME
```

Set the documented proxy only when required. Do not use `--trusted-host` or disable TLS verification as a permanent fix because that removes package-download authentication.

## Verify

Retry without insecure flags and confirm the certificate hostname and issuer match the intended repository.

<!-- Source: https://pip.pypa.io/en/stable/topics/https-certificates/ -->

<!-- lbc:th -->
# Python - แก้ pip SSL certificate verify failed

## วิธีแก้

`CERTIFICATE_VERIFY_FAILED` หมายถึง Python สร้าง trust chain ของ server certificate ไม่ได้ ให้ตรวจวันเวลาเครื่อง hostname และว่า proxy ขององค์กรเปลี่ยน TLS certificate หรือไม่

อัปเดต CA bundle ของระบบ ถ้าองค์กรใช้ private CA ให้ขอไฟล์ CA จากผู้ดูแลแล้วกำหนดให้ pip:

```bash
$ python -m pip install --cert /path/to/company-ca.pem PACKAGE_NAME
```

ตั้ง proxy ตามเอกสารเฉพาะเมื่อจำเป็น ห้ามใช้ `--trusted-host` หรือปิด TLS verification เป็นวิธีถาวร เพราะจะเสียการยืนยันแหล่ง package

## ตรวจผล

ลองใหม่โดยไม่มี insecure flag แล้วตรวจว่า hostname และ issuer ของ certificate ตรงกับ repository ที่ต้องการ

<!-- แหล่งข้อมูล: https://pip.pypa.io/en/stable/topics/https-certificates/ -->
