#!/usr/bin/env bash
set -euo pipefail

dist_dir="${1:-dist}"
: "${LBC_RELEASE_SIGNING_KEY_BASE64:?Set the masked release signing key in the CircleCI release context}"
: "${LBC_RELEASE_SIGNING_FINGERPRINT:?Set the full release signing fingerprint in the CircleCI release context}"

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
shopt -s nullglob
artifacts=("${dist_dir}"/*)
for artifact in "${artifacts[@]}"; do
    [[ -f "${artifact}" ]] || continue
    case "${artifact}" in
        *.asc) continue ;;
    esac
    gpg --batch --yes --armor --detach-sign \
        --local-user "${actual_fingerprint}" \
        --output "${artifact}.asc" \
        "${artifact}"
    gpg --batch --verify "${artifact}.asc" "${artifact}" >/dev/null 2>&1
done
