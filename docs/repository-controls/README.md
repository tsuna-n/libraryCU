# Prepared GitHub controls — not applied

These JSON files are reviewable REST request bodies, checked against the
[GitHub ruleset API](https://docs.github.com/en/rest/repos/rules#create-a-repository-ruleset)
on 2026-10-01. JSON parsing and local field/context checks passed; server acceptance
and enforcement are **NOT RUN**. They contain no credentials.

[Authenticated observations](../evidence/external-state.json) report repository
account `admin: true`, no inherited/repository rulesets, no active rules on main,
traditional protection 404, and private reporting disabled. The account permission
does not establish the token's Administration write scope. The blocker is pending
hosted configuration/authorization and enforcement evidence, not an assertion
that the account lacks admin permission.

[AGENT.md](../../AGENT.md#scope-external-dependencies-and-release-evidence) says:
“A roadmap checkbox does not itself authorize changing hosted repository rules”.
The current request requires checking/preparing these controls and permits external
gates to remain blocked. No hosted settings or disposable test refs were changed.
After explicit owner authorization, use an account/token with Administration write
permission, review the live rules first, and apply only missing controls.

- `main-ruleset.json`: default branch, no bypass, required signed commits,
  pull request, one independent current approval, stale-review dismissal,
  conversation resolution, strict four actual CircleCI contexts, no force push
  or deletion. Confirm a separate eligible reviewer exists before enabling it.
- `tag-creation-ruleset.json`: `v*` creation restricted to the repository owner
  (GitHub user ID 187157083). Review/replace that actor only from an independently
  approved release-maintainer list. Bypass applies solely to creation.
- `tag-immutable-ruleset.json`: a separate no-bypass rule blocks `v*` updates and
  deletion even for the creation actor. Combining these with a single bypass
  ruleset would unintentionally grant that actor update/delete privileges.

The tag rules do not cryptographically verify annotated-tag signatures. Keep the
protected `verify-release-source.sh` gate and independent source identity pins.
Required check names do not independently identify their submitting integration;
review the actual CircleCI installation and pin `integration_id` if applicable,
without guessing an app ID.

The following commands are continuation instructions, **not executed**. Use API
version `2026-03-10` matching the reviewed schema. Record created rule IDs; export
and compare each rule, conditions and bypass list rather than checking existence.
Existing rules must be reviewed/updated by their actual IDs; do not create duplicates.

```bash
gh api repos/tsuna-n/libraryCU/rulesets --paginate
gh api repos/tsuna-n/libraryCU/rules/branches/main

gh api --method POST -H 'X-GitHub-Api-Version: 2026-03-10' \
  repos/tsuna-n/libraryCU/rulesets \
  --input docs/repository-controls/main-ruleset.json
gh api --method POST -H 'X-GitHub-Api-Version: 2026-03-10' \
  repos/tsuna-n/libraryCU/rulesets \
  --input docs/repository-controls/tag-creation-ruleset.json
gh api --method POST -H 'X-GitHub-Api-Version: 2026-03-10' \
  repos/tsuna-n/libraryCU/rulesets \
  --input docs/repository-controls/tag-immutable-ruleset.json
gh api --method PUT repos/tsuna-n/libraryCU/private-vulnerability-reporting

gh api repos/tsuna-n/libraryCU/rulesets/ACTUAL_RULESET_ID
gh api repos/tsuna-n/libraryCU/rules/branches/main
gh api repos/tsuna-n/libraryCU/private-vulnerability-reporting
```

Require `enabled: true` for private reporting and retain the existing SECURITY.md
disclosure workflow. Traditional branch protection can still return 404 when
rulesets enforce equivalent controls; evaluate active rules, not that endpoint
alone. Run the authorized disposable-ref/PR rejection tests in
[repository-security.md](../repository-security.md#read-only-verification-after-owner-configuration)
under equivalent conditions. Never negative-test live main or production tags.
Only authenticated exports and applicable enforcement tests can close E01/E02;
configuration files alone cannot. Reconcile the existing tag/public-release/source
conflict under separate explicit release/tag authorization before production work.
