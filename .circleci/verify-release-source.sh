#!/usr/bin/env bash
set -euo pipefail
set +x
umask 077
export GIT_NO_REPLACE_OBJECTS=1

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/release-common.sh"
version="$(release_version)"
test -n "${version}"
: "${CIRCLE_TAG:?A production source gate requires a version tag}"
: "${CIRCLE_SHA1:?A production source gate requires the exact source SHA}"
for variable in LBC_MAINTAINER_SIGNING_FINGERPRINT LBC_MAINTAINER_SIGNING_KEY_BASE64; do
    if [[ -z "${!variable:-}" ]]; then
        echo "EXTERNAL CREDENTIAL REQUIRED: set ${variable} in protected CircleCI context lbc-release-identity; see docs/circleci.md#source-identity-preflight" >&2
        exit 1
    fi
done
validate_release_tag "${version}"
release_repository >/dev/null
validate_signing_fingerprint "${LBC_MAINTAINER_SIGNING_FINGERPRINT}"
if [[ ! "${CIRCLE_SHA1}" =~ ^[0-9a-f]{40}$ && ! "${CIRCLE_SHA1}" =~ ^[0-9a-f]{64}$ ]]; then
    echo "Source SHA must be a full lowercase Git digest" >&2
    exit 1
fi
if [[ "$(git rev-parse HEAD)" != "${CIRCLE_SHA1}" ]]; then
    echo "Source gate: checkout HEAD does not match CIRCLE_SHA1" >&2; exit 1
fi
if [[ "$(git cat-file -t "refs/tags/${CIRCLE_TAG}" 2>/dev/null || true)" != tag ]]; then
    echo "Source gate: version tag must exist and be annotated, not lightweight" >&2; exit 1
fi
if [[ "$(git rev-parse "refs/tags/${CIRCLE_TAG}^{commit}")" != "${CIRCLE_SHA1}" ]]; then
    echo "Source gate: annotated tag target does not match CIRCLE_SHA1" >&2; exit 1
fi
tag_header="$(git cat-file -p "refs/tags/${CIRCLE_TAG}" | sed -n '1,3p')"
expected_header="$(printf 'object %s\ntype commit\ntag %s' "${CIRCLE_SHA1}" "${CIRCLE_TAG}")"
if [[ "${tag_header}" != "${expected_header}" ]]; then
    echo "Source gate: signed tag metadata must name CIRCLE_TAG and directly target CIRCLE_SHA1" >&2; exit 1
fi
if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
    echo "Source gate: tracked release source has uncommitted changes" >&2; exit 1
fi

verification_home="$(mktemp -d)"
trap 'rm -rf "${verification_home}"' EXIT
export GNUPGHOME="${verification_home}"
if ! printf '%s' "${LBC_MAINTAINER_SIGNING_KEY_BASE64}" | base64 --decode > "${verification_home}/maintainer.asc"; then
    echo "Source gate: LBC_MAINTAINER_SIGNING_KEY_BASE64 must be base64 of the armored PUBLIC export" >&2; exit 1
fi
if ! gpg --no-options --batch --import "${verification_home}/maintainer.asc" >/dev/null 2>&1; then
    echo "Source gate: maintainer public signing key cannot be imported" >&2; exit 1
fi
validate_imported_identity "${LBC_MAINTAINER_SIGNING_FINGERPRINT}"
# Explicitly select GPG/OpenPGP; clone configuration cannot select an unchecked
# custom verifier. Commit and annotated tag must both bind the pinned maintainer.
if ! git -c gpg.program=gpg -c gpg.openpgp.program=gpg -c gpg.format=openpgp verify-commit --raw "${CIRCLE_SHA1}" \
    2>"${verification_home}/commit.status"; then
    echo "Source gate: release commit is unsigned or its OpenPGP signature failed" >&2; exit 1
fi
validate_gpg_status "${verification_home}/commit.status" "${LBC_MAINTAINER_SIGNING_FINGERPRINT}"
if ! git -c gpg.program=gpg -c gpg.openpgp.program=gpg -c gpg.format=openpgp verify-tag --raw "refs/tags/${CIRCLE_TAG}" \
    2>"${verification_home}/tag.status"; then
    echo "Source gate: annotated release tag is unsigned or its OpenPGP signature failed" >&2; exit 1
fi
validate_gpg_status "${verification_home}/tag.status" "${LBC_MAINTAINER_SIGNING_FINGERPRINT}"
printf 'Verified pinned maintainer commit and annotated tag: %s %s\n' "${CIRCLE_SHA1}" "${CIRCLE_TAG}"
