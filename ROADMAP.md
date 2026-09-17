# libraryCube (lbc) — Enterprise Roadmap

> Roadmap เริ่มจาก Core CLI baseline `v0.4.0` และเวอร์ชันที่กำลังเตรียม release
> คือ `v0.5.0` ไปจนถึง `v1.0.0`
>
> เป้าหมายหลัก: เปลี่ยน libraryCube จาก Local-first Developer CLI ให้เป็น
> **Local-first Enterprise Developer Knowledge & Troubleshooting Platform**
> โดยยังคงหลักการ Privacy-first, Offline-first และให้ AI เป็น Optional Layer

---

## Version Summary

| Version | Focus | Target |
|---|---|---|
| `v0.4.0` | Core CLI Baseline | Developer Tool |
| `v0.5.0` | Security & Supply Chain | Secure Distribution |
| `v0.6.0` | Organization Knowledge | Team Knowledge Platform |
| `v0.7.0` | Policy & Audit | Enterprise Governance |
| `v0.8.0` | Identity / RBAC / Control Plane | Organization Deployment |
| `v0.9.0` | Enterprise Pilot & Hardening | Production Candidate |
| `v1.0.0` | Enterprise Production | Stable Enterprise Release |

---

# v0.4.0 — Core CLI Baseline

**Status:** Completed baseline

## Goal

ทำให้ `lbc` เป็น Local-first CLI ที่สามารถเก็บ knowledge, วิเคราะห์ error,
เสนอวิธีแก้, ใช้ AI แบบ optional และ rollback การแก้ไขได้อย่างปลอดภัย

## Existing Core Features

- [x] Rust CLI
- [x] Offline knowledge retrieval
- [x] `lbc add`
- [x] `lbc search`
- [x] `lbc inspect`
- [x] `lbc edit`
- [x] `lbc ask`
- [x] `lbc explain`
- [x] `lbc scan`
- [x] `lbc doctor`
- [x] `lbc fix`
- [x] AI-assisted patch preview
- [x] Explicit `--apply`
- [x] `--verify`
- [x] Verification timeout
- [x] Automatic rollback when verification fails
- [x] Recovery records
- [x] Manual `lbc rollback`
- [x] Secret redaction
- [x] Bounded input/output
- [x] Local chat memory
- [x] Optional persistent chat history
- [x] Knowledge packages
- [x] Project knowledge
- [x] User knowledge
- [x] Built-in knowledge
- [x] Thai / English output
- [x] Rust diagnostic parsing
- [x] Python diagnostic parsing
- [x] TypeScript diagnostic parsing
- [x] Node.js diagnostic parsing
- [x] Go diagnostic parsing
- [x] OpenAI support
- [x] OpenRouter support
- [x] Z.ai / GLM support
- [x] Ollama support
- [x] OpenAI-compatible endpoints
- [x] Linux CI pipeline
- [x] macOS CI pipeline
- [x] Windows CI pipeline
- [x] Release-binary test pipeline
- [x] SHA-256 release-checksum pipeline

## Known Gaps

- [x] Full `.gitignore` semantics
- [x] Strict project boundary
- [x] Transaction-safe package installation
- [x] Multi-process file locking
- [x] Cross-platform multi-user/shared-store ownership and ACL validation
- [x] Authenticated recovery records
- [x] Recovery retention policy
- [x] Complete override cycle validation
- [ ] Production native platform-signed binaries and notarization
- [x] SBOM generation pipeline
- [x] Dependency vulnerability-scanning pipeline
- [ ] Enterprise policy system (v0.7 scope)
- [ ] Organization audit logging (v0.7 scope)
- [ ] RBAC / SSO (v0.8 scope)
- [ ] Organization knowledge server (v0.6 scope)

The single-owner policy in `docs/shared-store-security.md` passed native
Linux/macOS/Windows validation on `e5e265a` (CircleCI 49/52/50).
Writable multi-user collaboration is not v0.4 scope: unsafe shared mutable
stores are rejected, not repaired or made collaborative.

---

# v0.5.0 — Security & Software Supply Chain

**Status:** Repository hardening, executable native signing/source gates and all
four hosted candidate gates passed starting revision `ac16da5`; local continuation
changes still require hosted CI on their eventual reviewed signed revision.
Administrator controls, production credentials, hosted attestation and production
release execution remain open. **NOT READY**, not released.
The existing public, empty `v0.5.0` GitHub Release contradicts its DRAFT labeling
and is not a verified production release. Owner reconciliation is required;
do not recreate or move the existing signed tag as an automatic repair.

## Goal

ทำให้ binary, dependency และ release pipeline มีความน่าเชื่อถือเพียงพอ
สำหรับการติดตั้งใน environment ของบริษัท

## Repository Security

- [ ] Protect `main`
- [ ] Require Pull Request before merge
- [ ] Require CI before merge
- [ ] Require at least 1 reviewer
- [ ] Block force push
- [ ] Block branch deletion
- [ ] Require resolved conversations
- [ ] Add repository ruleset
- [ ] Enable signed commits for release maintainers
- [ ] Require signed release tags

## Dependency Security

- [x] Add `cargo audit`
- [x] Add `cargo deny`
- [x] Create `deny.toml`
- [x] Detect known vulnerable crates
- [x] Detect banned crates
- [x] Detect incompatible licenses
- [x] Detect duplicated dependency versions when relevant
- [x] Run dependency checks in CI

## SBOM

- [x] Implement CycloneDX SBOM generation and validation
- [x] Require an SBOM in the release artifact manifest
- [x] Implement SBOM upload to a draft GitHub Release
- [x] Include dependency name
- [x] Include dependency version
- [x] Include license information
- [x] Include package hashes when supported
- [ ] Generate, independently validate, and attach the production `v0.5.0` SBOM

Example:

```text
lbc-0.5.0-x86_64-unknown-linux-gnu.tar.gz
lbc-0.5.0-x86_64-unknown-linux-gnu.tar.gz.sha256
lbc-0.5.0.cdx.json
lbc-0.5.0.provenance.json
```

## Release Signing

- [x] Implement fail-closed detached OpenPGP release signing
- [x] Include the Linux archive in the signed release manifest
- [x] Windows Authenticode signing plan
- [x] macOS Developer ID signing plan
- [x] macOS notarization plan
- [x] Implement pinned signed-commit and signed-annotated-tag CI source gate
- [x] Implement tag-only Windows Authenticode sign/verify/re-test/repackage job
- [x] Implement tag-only Developer ID/runtime/notarize/staple/verify job
- [x] Add stapled DMG delivery and digest-bound native verification records
- [x] Add disposable-key and mocked native-signing failure regressions
- [ ] Execute and independently verify production Authenticode signing
- [ ] Execute and independently verify Developer ID/notarization/stapling
- [x] Implement signature and public-key publication to a draft release
- [x] Document independent signature verification before installation
- [ ] Sign and independently verify the production `v0.5.0` release artifacts

Executable native jobs and exact protected-context prerequisites are specified
in `docs/native-code-signing.md` and `docs/circleci.md`. Mocks/disposable TEST
certificates do not prove production trust; real credential execution remains
EXTERNAL CREDENTIAL REQUIRED.

## Build Provenance

- [x] Implement tenant-generated SLSA v1-shaped provenance metadata
- [x] Record source commit SHA
- [x] Record CircleCI workflow and builder metadata
- [x] Bind exact origin, source, version/tag/ref and workflow/builder metadata
- [x] Record all 11 final archive/checksum/SBOM/native-record subject digests
- [x] Reject missing/duplicate/unexpected subjects and modified signed provenance
- [ ] Target SLSA Build Level 2 or equivalent workflow

## Security Process

- [x] Add `SECURITY.md`
- [x] Define supported versions
- [x] Define vulnerability reporting process
- [x] Define security response SLA
- [x] Define security release process
- [ ] Add private vulnerability reporting if available
- [x] Add security advisory workflow

## Core Hardening

- [x] Implement strict explicit project scope
- [x] Prevent project root escape
- [x] Complete `.gitignore` behavior or use a proven Git-compatible matcher
- [x] Improve symlink race protections
- [x] Add file locking for mutable stores
- [x] Transaction-safe package installation
- [x] Atomic config/history/package updates
- [x] Detect package corruption
- [x] Validate override cycles
- [x] Validate self-overrides
- [x] Validate same-priority conflicts
- [x] Add failure-injection tests
- [x] Add concurrent-write tests

## Release Gate

`v0.5.0` ห้าม release จนกว่า:

- [x] Local Linux validation ผ่านทั้งหมด
- [x] Hosted dependency-security CI ผ่านบน exact tested candidate revision
- [x] Hosted CI ผ่าน Linux บน exact tested candidate revision
- [x] Hosted CI ผ่าน macOS บน exact tested candidate revision
- [x] Hosted CI ผ่าน Windows บน exact tested candidate revision
- [x] Local `cargo audit` ผ่าน
- [x] Local `cargo deny check` ผ่าน
- [x] Local tests ผ่านทั้งหมด
- [x] Local CycloneDX SBOM generation/validation ผ่าน
- [x] Release scripts reject missing, mismatched, or tampered inputs
- [x] Require exact 25-asset production manifest and private verified upload snapshot
- [x] Re-check draft state before every mutation and verify remote digest set
- [x] Hold publication for owner approval after source/native gates
- [ ] Production artifact checksums ถูกสร้างและตรวจสอบ
- [ ] Production artifact signatures ถูกสร้างและตรวจสอบอย่างอิสระ
- [ ] Production provenance ถูกสร้างและตรวจสอบกับ published artifacts
- [ ] GitHub repository security controls ถูกเปิดใช้และทดสอบ
- [x] Security documentation พร้อม

Current starting-revision evidence: `ac16da5` passed dependency 151, Linux 152,
macOS 149 and Windows 150; source gate 153 failed for a missing maintainer PUBLIC
key. See `docs/release-0.5.0.md` for exact SHA/job IDs and continuation evidence.
Candidate checkmarks never establish production artifacts, administrator settings
or hosted SLSA Build Level 2. Approval does not replace these external gates.
All four jobs must pass again on the actual reviewed signed release revision.

---

# v0.6.0 — Organization Knowledge

## Goal

เปลี่ยน LBC จาก knowledge tool ของคนเดียว
ให้เป็น knowledge platform ที่หลายทีมในองค์กรใช้ร่วมกันได้

## New Knowledge Levels

เพิ่ม source ใหม่:

```text
project:
team:
organization:
user:
package:
builtin:
```

Recommended priority:

```text
Project
  ↓
Team
  ↓
Organization
  ↓
User
  ↓
Installed Package
  ↓
Builtin
```

## Organization Knowledge Repository

- [ ] Add organization knowledge repository
- [ ] Add team knowledge repository
- [ ] Support private Git repository as knowledge source
- [ ] Support internal HTTP registry
- [ ] Add `lbc knowledge sync`
- [ ] Add `lbc knowledge pull`
- [ ] Add `lbc knowledge status`
- [ ] Add versioned organization knowledge
- [ ] Add offline cache
- [ ] Add incremental sync
- [ ] Add conflict detection
- [ ] Add source provenance metadata

Example:

```bash
lbc knowledge login company
lbc knowledge sync

lbc search "deploy kubernetes"
```

Possible sources:

```text
organization:devops:k8s-deployment
team:backend:nestjs-database
project:payment-service:local-runbook
```

## Signed Knowledge Packages

- [ ] Package checksums
- [ ] Package signatures
- [ ] Publisher identity
- [ ] Package version verification
- [ ] Trusted publisher list
- [ ] Reject unsigned organization packages when policy requires
- [ ] Package integrity verification before install
- [ ] Package integrity verification after download

Example package:

```text
rust-company-standard/
├── package.toml
├── knowledge/
├── SHA256SUMS
└── signature.sig
```

## Knowledge Lifecycle

- [ ] Draft knowledge
- [ ] Review knowledge
- [ ] Publish knowledge
- [ ] Deprecate knowledge
- [ ] Archive knowledge
- [ ] Track author
- [ ] Track reviewer
- [ ] Track created date
- [ ] Track updated date
- [ ] Track expiration/review date

## Knowledge Quality

- [ ] Duplicate detection
- [ ] Stale knowledge detection
- [ ] Broken reference detection
- [ ] Knowledge validation command
- [ ] Knowledge health report
- [ ] Confidence / verification metadata
- [ ] Link knowledge to supported versions

Example:

```bash
lbc knowledge validate
lbc knowledge doctor
```

## Release Gate

- [ ] Organization source works offline after sync
- [ ] Team source isolation tested
- [ ] Package signatures verified
- [ ] Package tampering tests pass
- [ ] Knowledge conflict tests pass
- [ ] Organization repository recovery tested

---

# v0.7.0 — Enterprise Policy & Audit

## Goal

ให้องค์กรสามารถควบคุมว่า LBC อ่านอะไร ส่งอะไรออกไป
ใช้ AI provider ไหน และ execute อะไรได้บ้าง

---

## Enterprise Policy Engine

Policy priority:

```text
Organization Policy
        ↓
Team Policy
        ↓
Project Policy
        ↓
User Configuration
```

**Higher-level policy must not be bypassed by lower-level config.**

## AI Provider Policy

- [ ] Provider allowlist
- [ ] Provider denylist
- [ ] Model allowlist
- [ ] Endpoint allowlist
- [ ] Require TLS
- [ ] Block unknown endpoints
- [ ] Allow local-only AI mode
- [ ] Block remote AI completely
- [ ] Organization-controlled API gateway
- [ ] Maximum request size
- [ ] Maximum output size
- [ ] Provider timeout policy

Example:

```toml
[organization.ai]
allowed_providers = ["company-llm", "azure-openai"]
deny_unknown_providers = true
remote_source_code = false
```

## Data Policy

- [ ] Allow/deny source-code transmission
- [ ] Allow/deny diagnostic transmission
- [ ] Allow/deny knowledge transmission
- [ ] Allow/deny chat persistence
- [ ] File extension rules
- [ ] Directory exclusion rules
- [ ] Secret-file detection
- [ ] Sensitive project mode

Example:

```toml
[organization.privacy]
remote_source_code = false
remote_logs = true
persistent_chat = false

deny_paths = [
  ".env",
  ".ssh/",
  "secrets/",
  "production/"
]
```

## Fix Policy

- [ ] Allow fix preview
- [ ] Require approval before apply
- [ ] Disable apply for selected repositories
- [ ] Limit modified files
- [ ] Maximum patch size
- [ ] Protected directory list
- [ ] Protected branch detection
- [ ] Require clean Git worktree
- [ ] Require backup/recovery record

## Verification Policy

- [ ] Command allowlist
- [ ] Executable allowlist
- [ ] Environment variable allowlist
- [ ] Network policy
- [ ] CPU limit
- [ ] Memory limit
- [ ] Process timeout
- [ ] Child process control

Example:

```toml
[organization.verify]
allowed_commands = [
  "cargo check",
  "cargo test",
  "pnpm test",
  "go test"
]

network = false
```

## Verification Sandbox

- [ ] Linux sandbox implementation
- [ ] Restricted filesystem access
- [ ] Restricted HOME access
- [ ] No SSH key access
- [ ] No cloud credential access
- [ ] Network off by default
- [ ] Sanitized environment
- [ ] Process tree termination
- [ ] Resource limits
- [ ] Sandbox escape tests

## Audit Logging

Track at minimum:

- [ ] timestamp
- [ ] actor
- [ ] organization
- [ ] team
- [ ] project
- [ ] LBC version
- [ ] action
- [ ] target file
- [ ] provider
- [ ] model
- [ ] knowledge sources
- [ ] policy decision
- [ ] verification result
- [ ] recovery ID

Never log by default:

- [ ] raw API keys
- [ ] passwords
- [ ] full source files
- [ ] raw secrets
- [ ] unrestricted prompts

Example:

```json
{
  "timestamp": "2026-09-14T08:00:00Z",
  "actor": "developer01",
  "project": "payment-service",
  "action": "fix.apply",
  "file": "src/payment.rs",
  "provider": "company-llm",
  "verification": "passed",
  "recovery_id": "fix-ABC123"
}
```

## Audit Commands

- [ ] `lbc audit show`
- [ ] `lbc audit export`
- [ ] `lbc audit verify`
- [ ] JSON output
- [ ] SIEM-friendly output
- [ ] Configurable retention

## Release Gate

- [ ] Organization policy cannot be bypassed
- [ ] Policy override tests pass
- [ ] Audit logs redact secrets
- [ ] Sandbox tests pass
- [ ] AI provider policy tests pass
- [ ] Network-block tests pass

---

# v0.8.0 — Identity, RBAC & Enterprise Control Plane

## Goal

รองรับการใช้งานหลายทีม หลาย Developer และหลาย Project
ภายใต้การควบคุมขององค์กร

## Control Plane

Recommended architecture:

```text
              ┌──────────────────────────┐
              │ LBC Enterprise Server    │
              ├──────────────────────────┤
              │ Identity                 │
              │ RBAC                     │
              │ Policies                 │
              │ Knowledge Registry       │
              │ Package Registry         │
              │ Audit Metadata           │
              │ Device Management        │
              └─────────────┬────────────┘
                            │
                          HTTPS
                            │
          ┌─────────────────┼─────────────────┐
          ↓                 ↓                 ↓
      Developer A       Developer B       Developer C
          │                 │                 │
        LBC CLI           LBC CLI           LBC CLI
          │                 │                 │
      Local Code        Local Code        Local Code
      Local Index       Local Index       Local Index
```

**Source code should remain local by default.**

## Identity

- [ ] Organization login
- [ ] OIDC
- [ ] OAuth 2.0 where appropriate
- [ ] SSO
- [ ] Device identity
- [ ] Short-lived tokens
- [ ] Token refresh
- [ ] Token revocation
- [ ] Offline grace mode

Possible providers:

- [ ] Microsoft Entra ID
- [ ] Google Workspace
- [ ] Okta
- [ ] Keycloak
- [ ] Generic OIDC

## RBAC

Roles:

### Developer

- [ ] Read organization knowledge
- [ ] Search
- [ ] Ask
- [ ] Explain
- [ ] Fix preview

### Senior Developer

- [ ] Apply permitted fixes
- [ ] Publish team knowledge

### Team Lead

- [ ] Approve sensitive fixes
- [ ] Manage team packages
- [ ] Review team knowledge

### Security / Platform Admin

- [ ] Manage organization policy
- [ ] Manage AI providers
- [ ] Manage trusted publishers
- [ ] Review audit events

### Organization Admin

- [ ] Manage organization
- [ ] Manage teams
- [ ] Manage roles
- [ ] Manage global settings

## Device Management

- [ ] Device registration
- [ ] Device revoke
- [ ] CLI version inventory
- [ ] Minimum required version
- [ ] Policy sync status
- [ ] Knowledge sync status
- [ ] Last check-in

## Central Update Management

- [ ] Stable channel
- [ ] Beta channel
- [ ] Security-only updates
- [ ] Organization-pinned version
- [ ] Force minimum secure version
- [ ] Signed update metadata
- [ ] Safe rollback

Example:

```bash
lbc update check
lbc update
lbc update --channel stable
```

## Enterprise Server Security

- [ ] TLS
- [ ] Database encryption
- [ ] Secret manager integration
- [ ] Rate limiting
- [ ] API authentication
- [ ] Authorization middleware
- [ ] Audit logging
- [ ] Backup encryption
- [ ] Admin MFA through identity provider

## Release Gate

- [ ] OIDC integration tests pass
- [ ] Role isolation tests pass
- [ ] Revoked users cannot sync
- [ ] Revoked devices cannot sync
- [ ] Policy distribution tested
- [ ] Offline behavior documented
- [ ] Server backup/restore tested

---

# v0.9.0 — Enterprise Pilot & Production Hardening

## Goal

หยุดเพิ่ม feature ใหญ่ชั่วคราว และพิสูจน์ระบบกับ workload จริง

## Pilot Deployment

Deploy with:

- [ ] 1 organization
- [ ] 2–3 development teams
- [ ] 10–50 developers
- [ ] Multiple repositories
- [ ] Multiple languages
- [ ] Real internal knowledge

## Benchmark Dataset

สร้างชุด real-world errors อย่างน้อย:

```text
200–500 developer incidents
```

ครอบคลุม:

- [ ] Rust
- [ ] Python
- [ ] JavaScript
- [ ] TypeScript
- [ ] Node.js
- [ ] Go
- [ ] Docker
- [ ] Database
- [ ] CI/CD
- [ ] Linux

## Measure

- [ ] Retrieval Precision
- [ ] Retrieval Recall
- [ ] Top-K relevance
- [ ] Diagnosis success rate
- [ ] Fix proposal acceptance rate
- [ ] Fix verification success rate
- [ ] False recommendation rate
- [ ] Median diagnosis latency
- [ ] P95 latency
- [ ] Knowledge sync latency
- [ ] AI request latency
- [ ] Offline success rate

## Reliability

- [ ] Crash testing
- [ ] Corrupt configuration recovery
- [ ] Corrupt knowledge recovery
- [ ] Interrupted package install recovery
- [ ] Interrupted update recovery
- [ ] Disk-full tests
- [ ] Read-only filesystem tests
- [ ] Network-loss tests
- [ ] Server unavailable tests
- [ ] Concurrent CLI tests
- [ ] Power interruption simulation where practical

## Backup / Restore

Enterprise Server:

- [ ] Database backup
- [ ] Knowledge registry backup
- [ ] Policy backup
- [ ] Audit backup
- [ ] Encryption keys backup procedure

Test:

- [ ] Restore from backup
- [ ] Point-in-time recovery where supported
- [ ] Disaster recovery documentation

## Observability

Server metrics:

- [ ] request count
- [ ] error rate
- [ ] latency
- [ ] sync failures
- [ ] authentication failures
- [ ] policy failures
- [ ] knowledge package failures

CLI diagnostics:

```bash
lbc doctor --enterprise
```

Should report:

- [ ] CLI version
- [ ] server connectivity
- [ ] authentication
- [ ] policy version
- [ ] knowledge version
- [ ] package integrity
- [ ] update status

## Compatibility

Test supported OS matrix:

- [ ] Ubuntu LTS
- [ ] Debian
- [ ] Fedora
- [ ] Arch Linux
- [ ] macOS Intel
- [ ] macOS Apple Silicon
- [ ] Windows 11
- [ ] Windows PowerShell
- [ ] WSL where supported

## Security Testing

- [ ] Threat model
- [ ] Abuse cases
- [ ] Prompt injection tests
- [ ] Knowledge poisoning tests
- [ ] Malicious package tests
- [ ] Path traversal tests
- [ ] Symlink attacks
- [ ] Sandbox escape tests
- [ ] Token theft scenarios
- [ ] Audit tampering tests
- [ ] Dependency compromise scenarios

## Documentation

- [ ] Admin Guide
- [ ] Developer Guide
- [ ] Installation Guide
- [ ] Upgrade Guide
- [ ] Backup Guide
- [ ] Security Guide
- [ ] Policy Reference
- [ ] RBAC Reference
- [ ] Troubleshooting Guide
- [ ] Incident Response Guide

## Release Gate

`v0.9.0` ถือเป็น Release Candidate เมื่อ:

- [ ] Pilot ใช้งานจริงสำเร็จ
- [ ] ไม่มี Critical security issue ที่เปิดอยู่
- [ ] ไม่มี High severity regression ที่ยังไม่แก้
- [ ] Backup/restore ผ่าน
- [ ] Upgrade/rollback ผ่าน
- [ ] Benchmark พร้อมเผยแพร่ภายใน
- [ ] Documentation พร้อม

---

# v1.0.0 — Enterprise Production Release

## Goal

Stable Enterprise Release ที่สามารถติดตั้งและดูแลในองค์กรได้จริง

## Core Stability

- [ ] Stable CLI commands
- [ ] Stable configuration format
- [ ] Stable knowledge package format
- [ ] Stable policy format
- [ ] Stable audit format
- [ ] Backward compatibility policy
- [ ] Deprecation policy
- [ ] Semantic Versioning policy

## Enterprise Features

- [ ] Organization knowledge
- [ ] Team knowledge
- [ ] Signed packages
- [ ] Central policy
- [ ] AI governance
- [ ] Fix governance
- [ ] Verification sandbox
- [ ] Audit logging
- [ ] SSO/OIDC
- [ ] RBAC
- [ ] Device management
- [ ] Central update management
- [ ] Backup/restore
- [ ] Enterprise server

## Security

- [ ] Signed artifacts
- [ ] SBOM
- [ ] Build provenance
- [ ] Dependency scanning
- [ ] Security policy
- [ ] Vulnerability disclosure process
- [ ] Release integrity verification
- [ ] Secure update mechanism
- [ ] Security regression suite

## Production Operations

- [ ] Upgrade path tested
- [ ] Downgrade policy documented
- [ ] Rollback process tested
- [ ] Monitoring documented
- [ ] Disaster recovery tested
- [ ] Incident response documented
- [ ] Enterprise support process defined

## Recommended Support Policy

Example:

```text
v1.0.x  → Active support
v0.9.x  → Security fixes for limited period
v0.8.x  → End of support
```

## v1.0 Definition of Done

LibraryCube v1.0 is ready when an organization can:

1. Install a trusted and signed `lbc` binary.
2. Verify its integrity.
3. Authenticate with the organization.
4. Receive organization policy.
5. Sync trusted team/company knowledge.
6. Search and diagnose locally.
7. Use AI only through approved providers.
8. Prevent sensitive source transmission through policy.
9. Apply fixes under organization rules.
10. Verify fixes in a controlled environment.
11. Roll back failed changes.
12. Record auditable security-safe events.
13. Manage users and permissions.
14. Revoke users/devices.
15. Upgrade safely.
16. Restore enterprise services from backup.
17. Continue core offline workflows when the enterprise server is temporarily unavailable.

---

# Features Intentionally Deferred Beyond v1.0

สิ่งต่อไปนี้ไม่จำเป็นต่อ Enterprise v1.0 และควรทำหลัง core platform เสถียร

## v1.x / v2.x Candidates

- [ ] GUI Desktop application
- [ ] Full TUI redesign
- [ ] VS Code extension
- [ ] JetBrains extension
- [ ] Neovim integration
- [ ] Autonomous coding agent
- [ ] Multi-agent workflows
- [ ] Vector database cluster
- [ ] Semantic embedding service
- [ ] Knowledge recommendation engine
- [ ] Automatic knowledge generation
- [ ] Organization analytics dashboard
- [ ] Developer productivity analytics
- [ ] Marketplace
- [ ] Third-party executable plugin SDK
- [ ] Managed SaaS control plane

---

# Recommended Development Priority

Do not prioritize:

```text
More AI models
More UI
Autonomous Agent
Vector DB
Marketplace
```

before completing:

```text
Security
   ↓
Supply Chain
   ↓
Project Isolation
   ↓
Organization Knowledge
   ↓
Policy
   ↓
Audit
   ↓
RBAC / SSO
   ↓
Pilot
   ↓
v1.0
```

---

# Recommended Product Positioning

Avoid positioning LBC as:

> "Another AI coding assistant"

Recommended positioning:

> **Local-first Enterprise Developer Knowledge & Troubleshooting Platform**

Core concept:

```text
Developer Project
       +
Team Knowledge
       +
Organization Knowledge
       +
Diagnostics
       +
Controlled AI
       +
Safe Fix / Verify / Rollback
       +
Enterprise Policy & Audit
```

The differentiator should remain:

```text
Local-first
Privacy-first
Offline-capable
AI-optional
Organization-controlled
Auditable
```

---

# Final Roadmap

```text
v0.4.0
Core CLI
   ↓
v0.5.0
Security + Supply Chain
   ↓
v0.6.0
Organization Knowledge
   ↓
v0.7.0
Policy + Audit + Sandbox
   ↓
v0.8.0
SSO + RBAC + Enterprise Server
   ↓
v0.9.0
Pilot + Benchmark + Hardening
   ↓
v1.0.0
Enterprise Production
```
