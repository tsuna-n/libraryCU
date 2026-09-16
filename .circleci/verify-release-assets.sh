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
: "${LBC_RELEASE_SIGNING_FINGERPRINT:?Set the full release signing fingerprint in the CircleCI release context}"
validate_signing_fingerprint "${LBC_RELEASE_SIGNING_FINGERPRINT}"

public_key="${dist_dir}/lbc-release-signing-key.asc"
if [[ ! -f "${public_key}" || -L "${public_key}" || ! -s "${public_key}" ]]; then
    echo "Release public signing key is missing or unsafe" >&2
    exit 1
fi
for name in "${RELEASE_INPUTS[@]}"; do
    if [[ ! -f "${dist_dir}/${name}.asc" || -L "${dist_dir}/${name}.asc" || ! -s "${dist_dir}/${name}.asc" ]]; then
        echo "Detached signature is missing or unsafe: ${name}.asc" >&2
        exit 1
    fi
done

verification_home="$(mktemp -d)"
trap 'rm -rf "${verification_home}"' EXIT
chmod 700 "${verification_home}"
export GNUPGHOME="${verification_home}"
gpg --no-options --batch --import "${public_key}" >/dev/null 2>&1
validate_imported_identity "${LBC_RELEASE_SIGNING_FINGERPRINT}"
primary_key_count="$(gpg --batch --with-colons --list-keys | awk -F: '$1 == "pub" {count++} END {print count + 0}')"
if [[ "${primary_key_count}" -ne 1 ]]; then
    echo "Release public key file must contain exactly one primary key" >&2
    exit 1
fi
actual_fingerprint="$(gpg --batch --with-colons --list-keys "${LBC_RELEASE_SIGNING_FINGERPRINT}" | awk -F: '$1 == "fpr" {print $10; exit}')"
if [[ "${actual_fingerprint}" != "${LBC_RELEASE_SIGNING_FINGERPRINT}" ]]; then
    echo "Release public key fingerprint does not match the configured fingerprint" >&2
    exit 1
fi
for name in "${RELEASE_INPUTS[@]}"; do
    verify_openpgp_signature "${dist_dir}/${name}.asc" "${dist_dir}/${name}" "${actual_fingerprint}"
done

# Refuse to publish accidental or stale files that are outside the manifest.
set_release_assets "${version}"
allowed="$(printf '%s\n' "${RELEASE_ASSETS[@]}" | sort)"
actual="$(find "${dist_dir}" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort)"
if [[ "${actual}" != "${allowed}" ]]; then
    echo "Release directory contains files outside the verified manifest" >&2
    if ! diff -u <(printf '%s\n' "${allowed}") <(printf '%s\n' "${actual}") >&2; then
        echo "Release manifest mismatch shown above" >&2
    fi
    exit 1
fi
