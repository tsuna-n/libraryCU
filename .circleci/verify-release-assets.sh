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
: "${LBC_RELEASE_SIGNING_FINGERPRINT:?Set the full release signing fingerprint in the CircleCI release context}"
if [[ ! "${LBC_RELEASE_SIGNING_FINGERPRINT}" =~ ^[0-9A-F]{40}$ && ! "${LBC_RELEASE_SIGNING_FINGERPRINT}" =~ ^[0-9A-F]{64}$ ]]; then
    echo "Release signing fingerprint must be a full uppercase fingerprint" >&2
    exit 1
fi

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
gpg --batch --import "${public_key}" >/dev/null 2>&1
actual_fingerprint="$(gpg --batch --with-colons --list-keys "${LBC_RELEASE_SIGNING_FINGERPRINT}" | awk -F: '$1 == "fpr" {print $10; exit}')"
if [[ "${actual_fingerprint}" != "${LBC_RELEASE_SIGNING_FINGERPRINT}" ]]; then
    echo "Release public key fingerprint does not match the configured fingerprint" >&2
    exit 1
fi
for name in "${RELEASE_INPUTS[@]}"; do
    gpg --batch --verify "${dist_dir}/${name}.asc" "${dist_dir}/${name}" >/dev/null 2>&1
done

# Refuse to publish accidental or stale files that are outside the manifest.
allowed="$(printf '%s\n' "${RELEASE_INPUTS[@]}" "${RELEASE_INPUTS[@]/%/.asc}" "lbc-release-signing-key.asc" | sort)"
actual="$(find "${dist_dir}" -maxdepth 1 \( -type f -o -type l \) -printf '%f\n' | sort)"
if [[ "${actual}" != "${allowed}" ]]; then
    echo "Release directory contains files outside the verified manifest" >&2
    diff -u <(printf '%s\n' "${allowed}") <(printf '%s\n' "${actual}") >&2 || true
    exit 1
fi
