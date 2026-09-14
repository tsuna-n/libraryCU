#!/usr/bin/env bash
set -euo pipefail

dist_dir="${1:-dist}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=release-common.sh
source "${script_dir}/release-common.sh"

version="$(release_version)"
if [[ -z "${version}" ]]; then
    echo "Could not read the package version from Cargo.toml" >&2
    exit 1
fi
validate_release_tag "${version}"
require_release_inputs "${dist_dir}" "${version}" true

: "${LBC_RELEASE_SIGNING_KEY_BASE64:?Set the masked release signing key in the CircleCI release context}"
: "${LBC_RELEASE_SIGNING_FINGERPRINT:?Set the full release signing fingerprint in the CircleCI release context}"
if [[ ! "${LBC_RELEASE_SIGNING_FINGERPRINT}" =~ ^[0-9A-F]{40}$ && ! "${LBC_RELEASE_SIGNING_FINGERPRINT}" =~ ^[0-9A-F]{64}$ ]]; then
    echo "Release signing fingerprint must be a full uppercase fingerprint" >&2
    exit 1
fi

signing_home="$(mktemp -d)"
key_file="$(mktemp)"
trap 'rm -rf "${signing_home}"; rm -f "${key_file}"' EXIT
chmod 700 "${signing_home}"
export GNUPGHOME="${signing_home}"

printf '%s' "${LBC_RELEASE_SIGNING_KEY_BASE64}" | base64 --decode > "${key_file}"
gpg --batch --import "${key_file}" >/dev/null 2>&1
actual_fingerprint="$(gpg --batch --with-colons --list-secret-keys "${LBC_RELEASE_SIGNING_FINGERPRINT}" | awk -F: '$1 == "fpr" {print $10; exit}')"
if [[ "${actual_fingerprint}" != "${LBC_RELEASE_SIGNING_FINGERPRINT}" ]]; then
    echo "Imported release key fingerprint does not match the configured fingerprint" >&2
    exit 1
fi

gpg --batch --armor --export "${actual_fingerprint}" > "${dist_dir}/lbc-release-signing-key.asc"
test -s "${dist_dir}/lbc-release-signing-key.asc"
for name in "${RELEASE_INPUTS[@]}"; do
    artifact="${dist_dir}/${name}"
    gpg --batch --yes --armor --detach-sign \
        --local-user "${actual_fingerprint}" \
        --output "${artifact}.asc" \
        "${artifact}"
    gpg --batch --verify "${artifact}.asc" "${artifact}" >/dev/null 2>&1
done
