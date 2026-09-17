# Repository security setup for v0.5.x

The repository owner must apply these controls to `main` in GitHub. They cannot
be enforced by files in a clone, so capture the ruleset ID and a screenshot or
API export in the release record after configuration.

Status: EXTERNAL ADMIN ACTION REQUIRED. Read-only public API checks on
2026-09-17 returned zero repository rulesets and private vulnerability reporting
`enabled: false`. No authenticated GitHub API/token was available; traditional
branch-protection and organization/inherited settings are unverified, not
asserted absent. The main protection endpoint returned `404 Branch not protected`;
authenticated/inherited control evidence is still required. A signed annotated
`v0.5.0` tag and an empty public release do exist; the release's DRAFT labeling
is inconsistent with `draft: false`. Commit `ac16da5` has GitHub verification
`verified: false`, `reason: unknown_key`. Upload the matching PUBLIC source key
to the maintainer's GitHub signing-key settings and re-query verification; do
not infer cryptographic identity merely from the signature text. Do not infer
enabled protection from documentation or candidate CI passes.

Create an **active** branch ruleset targeting the default branch (`main`, or the
ruleset target `~DEFAULT_BRANCH`) with:

- Restrict deletions and block force pushes.
- Require a pull request before merging, with at least one approving review.
- Dismiss stale approvals when new commits are pushed, and require approval of
  the most recent reviewable push if the repository's workflow needs it.
- Require conversation resolution before merging.
- Require these exact status contexts: `ci/circleci: build_and_test`,
  `ci/circleci: dependency_security`, `ci/circleci: build_macos`, and
  `ci/circleci: build_windows`. Confirm the actual context names on a candidate
  PR before saving the ruleset; all four must be required on the release revision.
- Require branches to be current with the target before merging (strict checks).
- Require signed commits on the protected branch.
- Allow bypass only for a documented emergency maintainer role.

Create a second active tag ruleset for `v*` that restricts creation to release
maintainers and blocks deletion and updates, with no undocumented bypass.
GitHub's signed-commit rule does not cryptographically verify an annotated tag
signature. The implemented tag-only `.circleci/verify-release-source.sh` checks
both the exact signed commit and annotated tag with the protected PUBLIC
maintainer export and full primary fingerprint. Retain local tag verification
too; do not replace it with a signed commit badge.

Enable private vulnerability reporting and security advisories in **Settings →
Code security and analysis**. Release maintainers must enable vigilant mode,
sign release commits, and create signed annotated tags. Verify the commit's
GitHub `verified` state and verify the tag object locally before release with:

```bash
git verify-commit EXPECTED_RELEASE_SHA
git verify-tag v0.5.0
git tag --verify v0.5.0
```

## Read-only verification after owner configuration

On the owner's authenticated GitHub CLI, export these responses to the private
release record (never export tokens). Confirm each ruleset's enforcement,
target/ref conditions, bypass actors and all rules, not merely its existence:

```bash
gh api repos/tsuna-n/libraryCU/rulesets --paginate
gh api repos/tsuna-n/libraryCU/rulesets/RULESET_ID
gh api repos/tsuna-n/libraryCU/branches/main/protection
gh api repos/tsuna-n/libraryCU/private-vulnerability-reporting
gh api repos/tsuna-n/libraryCU/commits/EXPECTED_RELEASE_SHA
gh api repos/tsuna-n/libraryCU/commits/EXPECTED_RELEASE_SHA/status
gh api repos/tsuna-n/libraryCU/commits/EXPECTED_RELEASE_SHA/check-runs
```

Require `enabled: true` for private reporting and a genuinely verified release
commit with the expected signer (not merely an unverified/unsigned SHA). Inspect
organization rules/inherited protection if applicable. Treat 401/403/404 from
private protection endpoints as unavailable evidence, not proof of no rule.
The expected PUBLIC source key and fingerprint belong to `lbc-release-identity`;
production artifact signing is a separate identity in `lbc-release`.

Do not place a private signing key in GitHub, CircleCI configuration files, job
logs, artifacts, or the source tree. CircleCI receives the armored release key as
the masked `LBC_RELEASE_SIGNING_KEY_BASE64` variable and its full fingerprint as
`LBC_RELEASE_SIGNING_FINGERPRINT`. Restrict those variables to release contexts.

After setup, have the owner explicitly authorize disposable test refs/PRs
under equivalent rules. Never attempt a destructive push/delete against live
`main` or a production tag as a negative test. A temporary pull request alone
does not safely exercise branch/tag deletion; create disposable protected refs
with identical rule conditions and retain the authenticated rejection evidence.
Direct push, an
unreviewed PR, a stale approval after a new push, an unresolved conversation, a
failing required check, a force push, branch deletion, tag update, and tag
deletion must all be rejected. Record the ruleset IDs/API exports, private
reporting API state, signed commit/tag verification, and successful negative
tests against the exact release revision. Only that evidence permits the related
`ROADMAP.md` checkboxes to be completed.

Reference: [GitHub available rules](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets).
