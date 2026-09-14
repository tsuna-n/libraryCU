#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_dir}/.." && pwd)"
cd "${repository_root}"

version="$(sed -n '/^\[package\]/,/^\[/{s/^version = "\([^"]*\)"/\1/p;}' Cargo.toml)"
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

archives=(
    "lbc-${version}-x86_64-unknown-linux-gnu.tar.gz"
    "lbc-${version}-universal-apple-darwin.tar.gz"
    "lbc-${version}-x86_64-pc-windows-msvc.zip"
)
for archive in "${archives[@]}"; do
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
export LBC_RELEASE_SIGNING_FINGERPRINT="0000000000000000000000000000000000000000"
if bash .circleci/sign-release-assets.sh "${dist_dir}" >/dev/null 2>&1; then
    echo "Release signing accepted a mismatched fingerprint" >&2
    exit 1
fi
export LBC_RELEASE_SIGNING_FINGERPRINT="${valid_fingerprint}"

bash .circleci/sign-release-assets.sh "${dist_dir}"
bash .circleci/verify-release-assets.sh "${dist_dir}"

run_publish_fixture() {
    local mode="$1"
    local expected_status="$2"
    local port_file="${fixture_root}/${mode}.port"
    local state_file="${fixture_root}/${mode}.json"
    python3 .circleci/mock-github-release.py \
        --mode "${mode}" \
        --port-file "${port_file}" \
        --state-file "${state_file}" \
        --version "${version}" &
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
        bash .circleci/publish-github-release.sh "${dist_dir}" >/dev/null 2>&1 \
        || status="$?"
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
    server_pid=""
    if [[ "${expected_status}" == "success" && "${status}" -ne 0 ]]; then
        echo "Complete draft publication failed" >&2
        exit 1
    fi
    if [[ "${expected_status}" == "failure" && "${status}" -eq 0 ]]; then
        echo "Unsafe release publication unexpectedly succeeded" >&2
        exit 1
    fi
    printf '%s\n' "${state_file}"
}

state_file="$(run_publish_fixture success success)"
jq -e '.created_draft and .published and (.draft == false) and (.assets | length == 17) and (.error == null)' \
    "${state_file}" >/dev/null

state_file="$(run_publish_fixture fail-upload failure)"
jq -e '.created_draft and (.published == false) and .draft and (.assets | length == 3) and (.error == null)' \
    "${state_file}" >/dev/null

state_file="$(run_publish_fixture public failure)"
jq -e '(.created_draft == false) and .published and (.assets | length == 0) and (.error == null)' \
    "${state_file}" >/dev/null

printf 'tampered\n' >> "${dist_dir}/${archives[0]}"
if bash .circleci/verify-release-assets.sh "${dist_dir}" >/dev/null 2>&1; then
    echo "Release verification accepted a tampered archive" >&2
    exit 1
fi
