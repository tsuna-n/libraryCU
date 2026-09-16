#!/usr/bin/env bash
set -euo pipefail
set +x
umask 077

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
validate_signing_fingerprint "${LBC_RELEASE_SIGNING_FINGERPRINT}"

signing_home="$(mktemp -d)"
key_file="$(mktemp)"
trap 'rm -rf "${signing_home}"; rm -f "${key_file}"' EXIT
chmod 700 "${signing_home}"
export GNUPGHOME="${signing_home}"
printf '%s\n' "${LBC_RELEASE_SIGNING_PASSPHRASE:-}" > "${signing_home}/passphrase"

printf '%s' "${LBC_RELEASE_SIGNING_KEY_BASE64}" | base64 --decode > "${key_file}"
gpg --no-options --batch --import "${key_file}" >/dev/null 2>&1
validate_imported_identity "${LBC_RELEASE_SIGNING_FINGERPRINT}" true
actual_fingerprint="$(gpg --batch --with-colons --list-secret-keys "${LBC_RELEASE_SIGNING_FINGERPRINT}" | awk -F: '$1 == "fpr" {print $10; exit}')"
if [[ "${actual_fingerprint}" != "${LBC_RELEASE_SIGNING_FINGERPRINT}" ]]; then
    echo "Imported release key fingerprint does not match the configured fingerprint" >&2
    exit 1
fi

gpg --batch --armor --export "${actual_fingerprint}" > "${dist_dir}/lbc-release-signing-key.asc"
test -s "${dist_dir}/lbc-release-signing-key.asc"
for name in "${RELEASE_INPUTS[@]}"; do
    artifact="${dist_dir}/${name}"
    gpg --no-options --batch --yes --armor --digest-algo SHA256 --detach-sign \
        --pinentry-mode loopback --passphrase-fd 4 \
        --local-user "${actual_fingerprint}" \
        --output "${artifact}.asc" \
        "${artifact}" 4<"${signing_home}/passphrase"
    verify_openpgp_signature "${artifact}.asc" "${artifact}" "${actual_fingerprint}"
done
