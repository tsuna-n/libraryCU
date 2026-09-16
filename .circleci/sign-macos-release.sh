#!/usr/bin/env bash
set -euo pipefail
set +x
umask 077

# Production only. Candidate jobs never invoke this or receive its context.
test "$(uname -s)" = Darwin
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${script_dir}/release-common.sh"
candidate_dir="${1:-dist}"
output_dir="${2:-production-dist}"
version="$(release_version)"
test -n "${version}"
: "${CIRCLE_TAG:?Production native signing requires the gated release tag}"
: "${CIRCLE_SHA1:?Production native signing requires the gated source SHA}"
: "${CIRCLE_WORKFLOW_ID:?Missing workflow identity}"
test "${CIRCLE_JOB:-}" = sign_macos_release
validate_release_tag "${version}"
release_repository >/dev/null
test "$(git rev-parse HEAD)" = "${CIRCLE_SHA1}"
for variable in LBC_MACOS_CERTIFICATE_BASE64 LBC_MACOS_CERTIFICATE_PASSWORD \
    LBC_MACOS_IDENTITY_SHA1 LBC_MACOS_TEAM_ID LBC_NOTARY_KEY_BASE64 \
    LBC_NOTARY_KEY_ID LBC_NOTARY_ISSUER_ID; do
    if [[ -z "${!variable:-}" ]]; then
        echo "EXTERNAL CREDENTIAL REQUIRED: ${variable}" >&2
        exit 1
    fi
done
[[ "${LBC_MACOS_IDENTITY_SHA1}" =~ ^[0-9A-F]{40}$ ]]
[[ "${LBC_MACOS_TEAM_ID}" =~ ^[A-Z0-9]{10}$ ]]
[[ "${LBC_NOTARY_KEY_ID}" =~ ^[A-Z0-9]{10}$ ]]
[[ "${LBC_NOTARY_ISSUER_ID}" =~ ^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$ ]]

package="lbc-${version}-universal-apple-darwin"
archive="${package}.tar.gz"
test -d "${candidate_dir}" && test ! -L "${candidate_dir}"
test -f "${candidate_dir}/${archive}" && test ! -L "${candidate_dir}/${archive}"
test -f "${candidate_dir}/${archive}.sha256" && test ! -L "${candidate_dir}/${archive}.sha256"
candidate_hash="$(shasum -a 256 "${candidate_dir}/${archive}" | awk '{print $1}')"
test "$(tr -d '\r' < "${candidate_dir}/${archive}.sha256")" = "${candidate_hash}  ${archive}"
tested_binary_hash="$(python3 "${script_dir}/native_release.py" package-hash "${candidate_dir}/${archive}" "${package}" macos)"
mkdir -p "${output_dir}"
test ! -L "${output_dir}"
for name in "${archive}" "${archive}.sha256" "${package}.dmg" "${package}.dmg.sha256" "lbc-${version}.macos-signing.json"; do
    test ! -e "${output_dir}/${name}" && test ! -L "${output_dir}/${name}"
done

native_home="$(mktemp -d)"
native_home="$(cd "${native_home}" && pwd -P)"
keychain="${native_home}/release.keychain-db"
mountpoint="${native_home}/mounted"
mounted=false
cleanup() {
    if [[ "${mounted}" == true ]]; then hdiutil detach "${mountpoint}" >/dev/null 2>&1 || true; fi
    security delete-keychain "${keychain}" >/dev/null 2>&1 || true
    rm -rf "${native_home}"
}
trap cleanup EXIT
mkdir "${native_home}/package"
tar -xzf "${candidate_dir}/${archive}" -C "${native_home}/package"
package_root="${native_home}/package/${package}"
binary="${package_root}/lbc"
test "$(shasum -a 256 "${binary}" | awk '{print $1}')" = "${tested_binary_hash}"

printf '%s' "${LBC_MACOS_CERTIFICATE_BASE64}" | base64 -D > "${native_home}/identity.p12"
printf '%s' "${LBC_NOTARY_KEY_BASE64}" | base64 -D > "${native_home}/notary.p8"
openssl pkey -in "${native_home}/notary.p8" -noout >/dev/null 2>&1
keychain_password="$(openssl rand -hex 32)"
security create-keychain -p "${keychain_password}" "${keychain}" >/dev/null 2>&1
security set-keychain-settings -lut 3600 "${keychain}" >/dev/null 2>&1
security unlock-keychain -p "${keychain_password}" "${keychain}" >/dev/null 2>&1
security import "${native_home}/identity.p12" -k "${keychain}" \
    -P "${LBC_MACOS_CERTIFICATE_PASSWORD}" -T /usr/bin/codesign >/dev/null 2>&1
security set-key-partition-list -S apple-tool:,apple:,codesign: -s \
    -k "${keychain_password}" "${keychain}" >/dev/null 2>&1
identities="$(security find-identity -v -p codesigning "${keychain}")"
test "$(printf '%s\n' "${identities}" | awk -v pin="${LBC_MACOS_IDENTITY_SHA1}" '$2 == pin {count++} END {print count+0}')" = 1

developer_requirement="anchor apple generic and certificate leaf[subject.OU] = \"${LBC_MACOS_TEAM_ID}\" and certificate leaf[field.1.2.840.113635.100.6.1.13] exists"
verify_code() {
    local target="$1" runtime="${2:-false}" display fingerprint
    codesign --verify --strict --verbose=2 -R="${developer_requirement}" "${target}"
    display="$(codesign --display --verbose=4 "${target}" 2>&1)"
    grep -Fqx "TeamIdentifier=${LBC_MACOS_TEAM_ID}" <<< "${display}"
    grep -Eq '^Authority=Developer ID Application:' <<< "${display}"
    grep -Eq '^Timestamp=.+' <<< "${display}"
    if [[ "${runtime}" == true ]]; then grep -Eq 'flags=.*\(.*runtime.*\)' <<< "${display}"; fi
    codesign --display --extract-certificates "${native_home}/certificate" "${target}" >/dev/null 2>&1
    fingerprint="$(openssl x509 -inform DER -in "${native_home}/certificate0" -noout -fingerprint -sha1 | sed 's/.*=//; s/://g')"
    test "${fingerprint}" = "${LBC_MACOS_IDENTITY_SHA1}"
}
codesign --force --options runtime --timestamp --keychain "${keychain}" \
    --sign "${LBC_MACOS_IDENTITY_SHA1}" "${binary}"
verify_code "${binary}" true
test "$("${binary}" --version)" = "lbc ${version}"
mkdir -p "${native_home}/fixtures/config" "${native_home}/fixtures/data" \
    "${native_home}/fixtures/cache" "${native_home}/fixtures/tmp"
export LBC_CONFIG="${native_home}/fixtures/config.toml"
export XDG_CONFIG_HOME="${native_home}/fixtures/config"
export XDG_DATA_HOME="${native_home}/fixtures/data"
export XDG_CACHE_HOME="${native_home}/fixtures/cache"
export TMPDIR="${native_home}/fixtures/tmp"
LBC_TEST_BINARY="${binary}" cargo test --locked --test cli
signed_binary_hash="$(shasum -a 256 "${binary}" | awk '{print $1}')"

# A raw tar cannot be stapled. Ship an additional signed/stapled DMG containing
# exactly the same universal CLI and installer; do not invent a .app bundle.
dmg="${native_home}/${package}.dmg"
hdiutil create -volname "${package}" -srcfolder "${package_root}" -format UDZO "${dmg}"
codesign --force --timestamp --keychain "${keychain}" --sign "${LBC_MACOS_IDENTITY_SHA1}" "${dmg}"
verify_code "${dmg}"
notary_credentials=(--key "${native_home}/notary.p8" --key-id "${LBC_NOTARY_KEY_ID}" --issuer "${LBC_NOTARY_ISSUER_ID}")
xcrun notarytool submit "${dmg}" "${notary_credentials[@]}" --wait --output-format json > "${native_home}/submission.json"
python3 "${script_dir}/release_json.py" "${native_home}/submission.json"
notary_id="$(jq -er '.id | select(type == "string")' "${native_home}/submission.json")"
[[ "${notary_id}" =~ ^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$ ]]
jq -e '.status == "Accepted"' "${native_home}/submission.json" >/dev/null
xcrun notarytool info "${notary_id}" "${notary_credentials[@]}" --output-format json > "${native_home}/info.json"
python3 "${script_dir}/release_json.py" "${native_home}/info.json"
jq -e --arg id "${notary_id}" '.id == $id and .status == "Accepted"' "${native_home}/info.json" >/dev/null
xcrun notarytool log "${notary_id}" "${notary_credentials[@]}" "${native_home}/notary-log.json"
python3 "${script_dir}/release_json.py" "${native_home}/notary-log.json"
jq -e --arg id "${notary_id}" '.jobId == $id and .status == "Accepted" and .statusCode == 0' "${native_home}/notary-log.json" >/dev/null
xcrun stapler staple "${dmg}"
xcrun stapler validate "${dmg}"
verify_code "${dmg}"
verify_code "${binary}" true
spctl --assess --type execute --verbose=2 "${binary}"
spctl --assess --type open --context context:primary-signature --verbose=2 "${dmg}"
test "$(shasum -a 256 "${binary}" | awk '{print $1}')" = "${signed_binary_hash}"
mkdir "${mountpoint}"
mounted=true
hdiutil attach -readonly -nobrowse -mountpoint "${mountpoint}" "${dmg}" >/dev/null
verify_code "${mountpoint}/lbc" true
test "$(shasum -a 256 "${mountpoint}/lbc" | awk '{print $1}')" = "${signed_binary_hash}"
hdiutil detach "${mountpoint}"
mounted=false

tar -czf "${native_home}/${archive}" -C "${native_home}/package" "${package}"
test "$(python3 "${script_dir}/native_release.py" package-hash "${native_home}/${archive}" "${package}" macos)" = "${signed_binary_hash}"
archive_hash="$(shasum -a 256 "${native_home}/${archive}" | awk '{print $1}')"
dmg_hash="$(shasum -a 256 "${dmg}" | awk '{print $1}')"
jq -n --arg version "${version}" --arg source "${CIRCLE_SHA1}" --arg workflow "${CIRCLE_WORKFLOW_ID}" \
    --arg pin "${LBC_MACOS_IDENTITY_SHA1}" --arg team "${LBC_MACOS_TEAM_ID}" \
    --arg binary "${package}/lbc" --arg binary_hash "${signed_binary_hash}" \
    --arg archive "${archive}" --arg archive_hash "${archive_hash}" --arg dmg "${package}.dmg" \
    --arg dmg_hash "${dmg_hash}" --arg notary_id "${notary_id}" '{
        schemaVersion:1,kind:"librarycube/native-verification/v1",platform:"macos",mode:"production",
        repository:"tsuna-n/libraryCU",sourceSha:$source,version:$version,tag:("v"+$version),
        workflowId:$workflow,jobName:"sign_macos_release",signer:{certificateSha1:$pin,teamId:$team},
        binary:{path:$binary,sha256:$binary_hash},archives:[{name:$archive,sha256:$archive_hash},{name:$dmg,sha256:$dmg_hash}],
        verification:{signature:true,timestamp:true,hardenedRuntime:true,releaseCli:true,packagedBinary:true,gatekeeper:true,
            notarization:{id:$notary_id,status:"Accepted",stapled:true,validated:true}}
    }' > "${native_home}/receipt.json"
mv "${native_home}/${archive}" "${dmg}" "${output_dir}/"
printf '%s  %s\n' "${archive_hash}" "${archive}" > "${output_dir}/${archive}.sha256"
printf '%s  %s\n' "${dmg_hash}" "${package}.dmg" > "${output_dir}/${package}.dmg.sha256"
mv "${native_home}/receipt.json" "${output_dir}/lbc-${version}.macos-signing.json"
printf 'Native macOS verification completed for %s; notarization %s Accepted/stapled/validated\n' "${CIRCLE_SHA1}" "${notary_id}"
