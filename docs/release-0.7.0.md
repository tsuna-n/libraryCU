# v0.7.0 — Knowledge maintenance

Source version prepared on 2026-11-12. The executable remains `lbc` and the
Rust package remains `librarycube`.

## Changes

- `lbc knowledge validate` reports Markdown/YAML, ID, reference, override and
  date warnings without changing files. `knowledge review --max-age-days 365`
  finds old or undated notes; the age threshold is configurable.
- `knowledge list --bundled` lists packages included in the executable.
  `knowledge install --bundled python-fastapi-basics` installs all 40 bilingual
  Python/FastAPI notes offline, even outside a source checkout.
- Local package installs verify supplied SHA-256 manifests before publication,
  preserve atomic installation, and reject corruption, duplicates and aliases.
  `knowledge list --json` reports installed packages and integrity status.
- New notes receive creation/update dates. Edits retain stable IDs, creation
  dates, references and custom YAML values; legacy notes remain supported.
- Edits and confirmed import/restore replacements first save exact private
  revisions. Failed revision writes preserve the original note; history is
  never automatically deleted.

## Compatibility and limits

Package removal now requires explicit confirmation:

```bash
lbc knowledge remove PACKAGE_NAME --yes
```

Update scripts that previously called `knowledge remove` without `--yes`.
Existing local-directory package installation and Markdown formats still work.
No package command downloads content; URL package sources are rejected.

Age warnings are review hints, not semantic correctness checks. External URLs
and Markdown hyperlinks are not fetched or validated. Checksums detect corruption,
not publisher authenticity; legacy packages without checksums remain unverified.
Editing may reformat YAML and comments, while the exact previous Markdown remains
in revision history. Revisions consume disk space and are excluded from portable
current-note backups; copy revision directories separately when needed.

## Local validation

Linux checks on this source version passed:

| Check | Result |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --locked --quiet` | PASS: 360 tests, no failures or ignored tests |
| `cargo build --locked --release` | PASS |
| `target/release/lbc --version` | `lbc 0.7.0` |

Native Windows/macOS execution and hosted CI results are not covered by these
local checks.

## Publication

This change updates source and version metadata. Production publication still
requires hosted CI, signed source identity, native Windows/macOS signing and
owner approval under the existing release policy. Historical release evidence
does not establish that these gates passed for v0.7.0.
