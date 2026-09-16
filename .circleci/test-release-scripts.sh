#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_dir}/.." && pwd)"
cd "${repository_root}"
# shellcheck source=release-common.sh
source "${script_dir}/release-common.sh"

version="$(release_version)"
fixture_root="$(mktemp -d)"
dist_dir="${fixture_root}/dist"
key_home="${fixture_root}/key-home"
mkdir -p "${dist_dir}" "${key_home}"
chmod 700 "${key_home}"
server_pid=""
cleanup() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" >/dev/null 2>&1 || true
        wait "${server_pid}" >/dev/null 2>&1 || true
    fi
    rm -rf "${fixture_root}"
}
trap cleanup EXIT

set_release_inputs "${version}" false
for archive in "${RELEASE_ARCHIVES[@]}"; do
    printf 'release fixture for %s\n' "${archive}" > "${dist_dir}/${archive}"
    digest="$(sha256sum "${dist_dir}/${archive}" | awk '{print $1}')"
    printf '%s  %s\n' "${digest}" "${archive}" > "${dist_dir}/${archive}.sha256"
done
printf '%s\n' \
    '{"bomFormat":"CycloneDX","components":[{"name":"fixture","version":"1.0.0","licenses":[{"license":{"id":"MIT"}}],"hashes":[{"alg":"SHA-256","content":"00"}]}]}' \
    > "${dist_dir}/lbc-${version}.cdx.json"

export CIRCLE_SHA1
CIRCLE_SHA1="$(git rev-parse HEAD)"
export CIRCLE_PROJECT_USERNAME="fixture-owner"
export CIRCLE_PROJECT_REPONAME="fixture-repository"
export CIRCLE_WORKFLOW_ID="fixture-workflow"
export CIRCLE_WORKFLOW_NAME="ci_cd"
export CIRCLE_JOB="publish_github_release"
export CIRCLE_BUILD_URL="https://circleci.example.invalid/build/fixture"
export CIRCLE_TAG="v${version}"

bash .circleci/generate-provenance.sh "${dist_dir}"
if env -u LBC_RELEASE_SIGNING_KEY_BASE64 -u LBC_RELEASE_SIGNING_FINGERPRINT \
    bash .circleci/sign-release-assets.sh "${dist_dir}" >/dev/null 2>&1
then
    echo "Release signing accepted missing credentials" >&2
    exit 1
fi

GNUPGHOME="${key_home}" gpg --batch --passphrase '' --quick-generate-key \
    'libraryCube release fixture <release-fixture@example.invalid>' rsa2048 sign 1d
export LBC_RELEASE_SIGNING_FINGERPRINT
LBC_RELEASE_SIGNING_FINGERPRINT="$(
    GNUPGHOME="${key_home}" gpg --batch --with-colons --list-secret-keys \
        | awk -F: '$1 == "fpr" {print toupper($10); exit}'
)"
private_key="${fixture_root}/private-key.asc"
GNUPGHOME="${key_home}" gpg --batch --armor --export-secret-keys \
    "${LBC_RELEASE_SIGNING_FINGERPRINT}" > "${private_key}"
export LBC_RELEASE_SIGNING_KEY_BASE64
LBC_RELEASE_SIGNING_KEY_BASE64="$(base64 < "${private_key}" | tr -d '\n')"

valid_fingerprint="${LBC_RELEASE_SIGNING_FINGERPRINT}"
export LBC_RELEASE_SIGNING_FINGERPRINT="0123456789ABCDEF"
if bash .circleci/sign-release-assets.sh "${dist_dir}" >/dev/null 2>&1; then
    echo "Release signing accepted a short fingerprint" >&2
    exit 1
fi
export LBC_RELEASE_SIGNING_FINGERPRINT="0000000000000000000000000000000000000000"
if bash .circleci/sign-release-assets.sh "${dist_dir}" >/dev/null 2>&1; then
    echo "Release signing accepted a mismatched fingerprint" >&2
    exit 1
fi
export LBC_RELEASE_SIGNING_FINGERPRINT="${valid_fingerprint}"

bash .circleci/sign-release-assets.sh "${dist_dir}"
bash .circleci/verify-release-assets.sh "${dist_dir}"
set_release_assets "${version}"
expected_assets_file="${fixture_root}/expected-assets.txt"
printf '%s\n' "${RELEASE_ASSETS[@]}" > "${expected_assets_file}"
valid_dist="${fixture_root}/valid-dist"
cp -a "${dist_dir}" "${valid_dist}"

expect_verification_failure() {
    local candidate="$1"
    local description="$2"
    if bash .circleci/verify-release-assets.sh "${candidate}" >/dev/null 2>&1; then
        echo "Release verification accepted ${description}" >&2
        exit 1
    fi
}

case_dir="${fixture_root}/missing-signature"
cp -a "${valid_dist}" "${case_dir}"
rm "${case_dir}/${RELEASE_INPUTS[0]}.asc"
expect_verification_failure "${case_dir}" "a missing signature"

case_dir="${fixture_root}/missing-archive"
cp -a "${valid_dist}" "${case_dir}"
rm "${case_dir}/${RELEASE_ARCHIVES[0]}"
expect_verification_failure "${case_dir}" "a missing archive"

case_dir="${fixture_root}/missing-checksum"
cp -a "${valid_dist}" "${case_dir}"
rm "${case_dir}/${RELEASE_ARCHIVES[0]}.sha256"
expect_verification_failure "${case_dir}" "a missing checksum"

case_dir="${fixture_root}/bad-signature"
cp -a "${valid_dist}" "${case_dir}"
printf 'not a signature\n' > "${case_dir}/${RELEASE_INPUTS[0]}.asc"
expect_verification_failure "${case_dir}" "a malformed signature"

case_dir="${fixture_root}/missing-public-key"
cp -a "${valid_dist}" "${case_dir}"
rm "${case_dir}/lbc-release-signing-key.asc"
expect_verification_failure "${case_dir}" "a missing public key"

case_dir="${fixture_root}/malformed-public-key"
cp -a "${valid_dist}" "${case_dir}"
printf 'not a public key\n' > "${case_dir}/lbc-release-signing-key.asc"
expect_verification_failure "${case_dir}" "a malformed public key"

case_dir="${fixture_root}/unexpected-file"
cp -a "${valid_dist}" "${case_dir}"
printf 'stale\n' > "${case_dir}/unexpected.bin"
expect_verification_failure "${case_dir}" "an unexpected file"

case_dir="${fixture_root}/symlinked-input"
cp -a "${valid_dist}" "${case_dir}"
rm "${case_dir}/${RELEASE_INPUTS[0]}"
ln -s "${valid_dist}/${RELEASE_INPUTS[0]}" "${case_dir}/${RELEASE_INPUTS[0]}"
expect_verification_failure "${case_dir}" "a symlinked input"

case_dir="${fixture_root}/empty-input"
cp -a "${valid_dist}" "${case_dir}"
: > "${case_dir}/${RELEASE_INPUTS[0]}"
expect_verification_failure "${case_dir}" "an empty input"

case_dir="${fixture_root}/checksum-name-mismatch"
cp -a "${valid_dist}" "${case_dir}"
digest="$(sha256sum "${case_dir}/${RELEASE_ARCHIVES[0]}" | awk '{print $1}')"
printf '%s  wrong-name.tar.gz\n' "${digest}" > "${case_dir}/${RELEASE_ARCHIVES[0]}.sha256"
expect_verification_failure "${case_dir}" "a checksum-name mismatch"

case_dir="${fixture_root}/malformed-sbom"
cp -a "${valid_dist}" "${case_dir}"
printf '{}\n' > "${case_dir}/lbc-${version}.cdx.json"
expect_verification_failure "${case_dir}" "a malformed SBOM"

case_dir="${fixture_root}/wrong-provenance-commit"
cp -a "${valid_dist}" "${case_dir}"
provenance="${case_dir}/lbc-${version}.provenance.json"
jq '.predicate.buildDefinition.resolvedDependencies[0].digest.gitCommit = "0000000000000000000000000000000000000000"' \
    "${provenance}" > "${provenance}.tmp"
mv "${provenance}.tmp" "${provenance}"
expect_verification_failure "${case_dir}" "provenance for a different commit"

case_dir="${fixture_root}/changed-after-provenance"
cp -a "${valid_dist}" "${case_dir}"
printf 'changed after provenance\n' >> "${case_dir}/${RELEASE_ARCHIVES[0]}"
digest="$(sha256sum "${case_dir}/${RELEASE_ARCHIVES[0]}" | awk '{print $1}')"
printf '%s  %s\n' "${digest}" "${RELEASE_ARCHIVES[0]}" \
    > "${case_dir}/${RELEASE_ARCHIVES[0]}.sha256"
expect_verification_failure "${case_dir}" "an artifact changed after provenance generation"

run_publish_fixture() {
    local mode="$1"
    local expected_status="$2"
    local port_file="${fixture_root}/${mode}.port"
    local state_file="${fixture_root}/${mode}.json"
    local publish_log="${fixture_root}/${mode}.log"
    python3 .circleci/mock-github-release.py \
        --mode "${mode}" \
        --port-file "${port_file}" \
        --state-file "${state_file}" \
        --version "${version}" \
        --expected-assets-file "${expected_assets_file}" &
    server_pid="$!"
    for _ in {1..100}; do
        [[ -s "${port_file}" ]] && break
        sleep 0.05
    done
    if [[ ! -s "${port_file}" ]]; then
        echo "Mock GitHub API did not start" >&2
        exit 1
    fi
    local port
    port="$(<"${port_file}")"
    local status=0
    GITHUB_API_URL="http://127.0.0.1:${port}" \
    GITHUB_TOKEN="fixture-token" \
        bash .circleci/publish-github-release.sh "${dist_dir}" >"${publish_log}" 2>&1 \
        || status="$?"
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
    server_pid=""
    if [[ "${expected_status}" == "success" && "${status}" -ne 0 ]]; then
        echo "Complete draft publication failed" >&2
        sed -n '1,120p' "${publish_log}" >&2
        jq . "${state_file}" >&2
        exit 1
    fi
    if [[ "${expected_status}" == "failure" && "${status}" -eq 0 ]]; then
        echo "Unsafe release publication unexpectedly succeeded" >&2
        exit 1
    fi
    printf '%s\n' "${state_file}"
}

state_file="$(run_publish_fixture success success)"
jq -e --argjson expected "${#RELEASE_ASSETS[@]}" \
    '.created_draft and .published and (.draft == false) and (.assets | length == $expected) and (.error == null)' \
    "${state_file}" >/dev/null

state_file="$(run_publish_fixture resume success)"
jq -e --argjson expected "${#RELEASE_ASSETS[@]}" \
    '(.created_draft == false) and .published and (.draft == false) and (.assets | length == $expected) and (.error == null)' \
    "${state_file}" >/dev/null

state_file="$(run_publish_fixture fail-upload failure)"
jq -e '.created_draft and (.published == false) and .draft and (.assets | length == 3) and (.error == null)' \
    "${state_file}" >/dev/null

state_file="$(run_publish_fixture public failure)"
jq -e '(.created_draft == false) and .published and (.assets | length == 0) and (.error == null)' \
    "${state_file}" >/dev/null

for mode in api-error malformed unexpected duplicate tampered; do
    state_file="$(run_publish_fixture "${mode}" failure)"
    jq -e '(.published == false) and .draft' "${state_file}" >/dev/null
done

case_dir="${fixture_root}/tampered-input"
cp -a "${valid_dist}" "${case_dir}"
printf 'tampered\n' >> "${case_dir}/${RELEASE_INPUTS[0]}"
expect_verification_failure "${case_dir}" "a tampered archive"
