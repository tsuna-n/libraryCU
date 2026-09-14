# Repository security setup for v0.5.x

The repository owner must apply these controls to `main` in GitHub. They cannot
be enforced by files in a clone, so capture the ruleset ID and a screenshot or
API export in the release record after configuration.

Create an **active** branch ruleset targeting the default branch (`main`, or the
ruleset target `~DEFAULT_BRANCH`) with:

- Restrict deletions and block force pushes.
- Require a pull request before merging, with at least one approving review.
- Dismiss stale approvals when new commits are pushed, and require approval of
  the most recent reviewable push if the repository's workflow needs it.
- Require conversation resolution before merging.
- Require these exact status contexts: `ci/circleci: build_and_test` and
  `ci/circleci: dependency_security`.
- Require branches to be current with the target before merging (strict checks).
- Require signed commits on the protected branch.
- Allow bypass only for a documented emergency maintainer role.

Create a second active tag ruleset for `v*` that blocks deletion and updates and
requires signed changes where GitHub supports that rule. This complements, but
does not replace, locally verifying the signed annotated tag object.

Enable private vulnerability reporting and security advisories in **Settings →
Code security and analysis**. Release maintainers must enable vigilant mode,
sign release commits, and create signed annotated tags. Verify the commit's
GitHub `verified` state and verify the tag object locally before release with:

```bash
git verify-tag v0.5.0
git tag --verify v0.5.0
```

Do not place a private signing key in GitHub, CircleCI configuration files, job
logs, artifacts, or the source tree. CircleCI receives the armored release key as
the masked `LBC_RELEASE_SIGNING_KEY_BASE64` variable and its full fingerprint as
`LBC_RELEASE_SIGNING_FINGERPRINT`. Restrict those variables to release contexts.

After setup, test the rules with a temporary pull request: direct push, an
unreviewed PR, a stale approval after a new push, an unresolved conversation, a
failing required check, a force push, branch deletion, tag update, and tag
deletion must all be rejected. Record the ruleset IDs/API exports, private
reporting API state, signed commit/tag verification, and successful negative
tests against the exact release revision. Only that evidence permits the related
`ROADMAP.md` checkboxes to be completed.
