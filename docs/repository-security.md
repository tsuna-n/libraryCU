# Repository security setup for v0.5.x

The repository owner must apply these controls to `main` in GitHub. They cannot
be enforced by files in a clone, so capture the ruleset ID and a screenshot or
API export in the release record after configuration.

Create a repository ruleset targeting the default branch with:

- Restrict deletions and force pushes.
- Require a pull request with at least one approving review.
- Dismiss stale approvals and require conversation resolution.
- Require the CircleCI workflow checks for build/test and dependency security.
- Require the branch to be current before merging.
- Allow bypass only for a documented emergency maintainer role.

Enable private vulnerability reporting and security advisories in repository
settings. Release maintainers must enable vigilant mode, sign release commits,
and create signed annotated tags. Verify a tag locally before release with:

```bash
git verify-tag v0.5.0
git tag --verify v0.5.0
```

Do not place a private signing key in GitHub, CircleCI configuration files, job
logs, artifacts, or the source tree. CircleCI receives the armored release key as
the masked `LBC_RELEASE_SIGNING_KEY_BASE64` variable and its full fingerprint as
`LBC_RELEASE_SIGNING_FINGERPRINT`. Restrict those variables to release contexts.

After setup, test the rules with a temporary pull request: an unreviewed PR, a
failing required check, a force push, and branch deletion must all be rejected.
Record the successful test against the exact ruleset revision.
