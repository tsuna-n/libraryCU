#!/usr/bin/env bash
set -euo pipefail
set +x
umask 077

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/release-common.sh"
version="$(release_version)"
test -n "${version}"
: "${CIRCLE_TAG:?A production source gate requires a version tag}"
: "${CIRCLE_SHA1:?A production source gate requires the exact source SHA}"
: "${LBC_MAINTAINER_SIGNING_KEY_BASE64:?EXTERNAL CREDENTIAL REQUIRED: configure the maintainer PUBLIC verification key}"
: "${LBC_MAINTAINER_SIGNING_FINGERPRINT:?EXTERNAL CREDENTIAL REQUIRED: configure its full primary fingerprint}"
validate_release_tag "${version}"
release_repository >/dev/null
validate_signing_fingerprint "${LBC_MAINTAINER_SIGNING_FINGERPRINT}"
if [[ ! "${CIRCLE_SHA1}" =~ ^[0-9a-f]{40}$ && ! "${CIRCLE_SHA1}" =~ ^[0-9a-f]{64}$ ]]; then
    echo "Source SHA must be a full lowercase Git digest" >&2
    exit 1
fi
test "$(git rev-parse HEAD)" = "${CIRCLE_SHA1}"
test "$(git cat-file -t "refs/tags/${CIRCLE_TAG}")" = tag
test "$(git rev-parse "refs/tags/${CIRCLE_TAG}^{commit}")" = "${CIRCLE_SHA1}"
test -z "$(git status --porcelain --untracked-files=no)"

verification_home="$(mktemp -d)"
trap 'rm -rf "${verification_home}"' EXIT
export GNUPGHOME="${verification_home}"
printf '%s' "${LBC_MAINTAINER_SIGNING_KEY_BASE64}" | base64 --decode > "${verification_home}/maintainer.asc"
gpg --no-options --batch --import "${verification_home}/maintainer.asc" >/dev/null 2>&1
validate_imported_identity "${LBC_MAINTAINER_SIGNING_FINGERPRINT}"
# Explicitly select GPG/OpenPGP; clone configuration cannot select an unchecked
# custom verifier. Commit and annotated tag must both bind the pinned maintainer.
git -c gpg.program=gpg -c gpg.format=openpgp verify-commit --raw "${CIRCLE_SHA1}" \
    2>"${verification_home}/commit.status"
validate_gpg_status "${verification_home}/commit.status" "${LBC_MAINTAINER_SIGNING_FINGERPRINT}"
git -c gpg.program=gpg -c gpg.format=openpgp verify-tag --raw "refs/tags/${CIRCLE_TAG}" \
    2>"${verification_home}/tag.status"
validate_gpg_status "${verification_home}/tag.status" "${LBC_MAINTAINER_SIGNING_FINGERPRINT}"
printf 'Verified pinned maintainer commit and annotated tag: %s %s\n' "${CIRCLE_SHA1}" "${CIRCLE_TAG}"
