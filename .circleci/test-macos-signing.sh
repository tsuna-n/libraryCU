#!/usr/bin/env bash
set -euo pipefail
# Native tools are deliberately mocked. Real tar/JSON/hash/ordering checks run;
# this does not establish Developer ID, Gatekeeper, or notarization evidence.
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${script_dir}/.."
source "${script_dir}/release-common.sh"
version="$(release_version)"
fixture_root="$(mktemp -d)"
trap 'rm -rf "${fixture_root}"' EXIT
export CIRCLE_SHA1="$(git rev-parse HEAD)" CIRCLE_TAG="v${version}" CIRCLE_JOB=sign_macos_release
export CIRCLE_PROJECT_USERNAME=tsuna-n CIRCLE_PROJECT_REPONAME=libraryCU CIRCLE_WORKFLOW_ID=mock-macos-workflow
export LBC_MACOS_IDENTITY_SHA1=BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB LBC_WINDOWS_CERT_THUMBPRINT=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
export LBC_MACOS_TEAM_ID=FIXTURE123 LBC_NOTARY_KEY_ID=FIXTURE123
export LBC_NOTARY_ISSUER_ID=11111111-1111-4111-8111-111111111111
export LBC_MACOS_CERTIFICATE_BASE64=bW9jayBjZXJ0aWZpY2F0ZQ== LBC_MACOS_CERTIFICATE_PASSWORD=mock-password
export LBC_NOTARY_KEY_BASE64=bW9jayBub3Rhcnkga2V5
export LBC_MOCK_PAYLOAD="${fixture_root}/mock-signed-payload"
python3 .circleci/create-release-fixture.py "${fixture_root}/candidate" "${version}" "${CIRCLE_SHA1}" "${CIRCLE_WORKFLOW_ID}"
uname() { printf 'Darwin\n'; }
base64() {
    if [[ "${1:-}" == -D ]]; then
        if [[ "$(command uname -s)" == Darwin ]]; then command base64 -D; else command base64 --decode; fi
    else command base64 "$@"; fi
}
security() {
    printf 'security-%s\n' "$1" >> "${LBC_MOCK_TRACE}"
    case "$1" in
        create-keychain) : > "${@: -1}" ;;
        import) [[ "${LBC_MOCK_CASE}" != malformed-certificate ]] ;;
        find-identity)
            if [[ "${LBC_MOCK_CASE}" == missing-identity || "${LBC_MOCK_CASE}" == expired-identity ]]; then
                printf '0 valid identities found\n'
            elif [[ "${LBC_MOCK_CASE}" == duplicate-identity ]]; then
                printf '1) %s "Developer ID Application: Fixture"\n2) %s "Developer ID Application: Fixture"\n' "${LBC_MACOS_IDENTITY_SHA1}" "${LBC_MACOS_IDENTITY_SHA1}"
            else printf '1) %s "Developer ID Application: Fixture"\n' "${LBC_MACOS_IDENTITY_SHA1}"; fi ;;
        *) return 0 ;;
    esac
}
openssl() {
    case "$1" in
        rand) command openssl "$@" ;;
        pkey) [[ "${LBC_MOCK_CASE}" != malformed-notary-key ]] ;;
        x509)
            if [[ "${LBC_MOCK_CASE}" == wrong-certificate ]]; then printf 'SHA1 Fingerprint=%040d\n' 0
            else printf 'SHA1 Fingerprint=%s\n' "${LBC_MACOS_IDENTITY_SHA1}"; fi ;;
        *) return 1 ;;
    esac
}
codesign() {
    printf 'codesign-%s\n' "$1" >> "${LBC_MOCK_TRACE}"
    case "$1" in
        --force)
            [[ "${LBC_MOCK_CASE}" != signing-failure ]] || return 1
            printf '\n# MOCK NATIVE SIGNATURE\n' >> "${@: -1}" ;;
        --verify)
            [[ "$*" == *'anchor apple generic'* && "$*" == *'1.2.840.113635.100.6.1.13'* ]] || return 1
            [[ "${LBC_MOCK_CASE}" != verification-failure ]] ;;
        --display)
            if [[ "$2" == --extract-certificates ]]; then printf 'mock public certificate\n' > "${3}0"; return 0; fi
            printf 'Authority=Developer ID Application: Fixture\n'
            if [[ "${LBC_MOCK_CASE}" == wrong-team ]]; then printf 'TeamIdentifier=WRONGTEAM1\n'
            else printf 'TeamIdentifier=%s\n' "${LBC_MACOS_TEAM_ID}"; fi
            [[ "${LBC_MOCK_CASE}" == missing-timestamp ]] || printf 'Timestamp=mock timestamp\n'
            [[ "${LBC_MOCK_CASE}" == missing-runtime ]] || printf 'flags=0x10000(runtime)\n' ;;
        *) return 1 ;;
    esac
}
cargo() { printf 'signed-cli\n' >> "${LBC_MOCK_TRACE}"; [[ "${LBC_MOCK_CASE}" != cli-failure ]]; }
hdiutil() {
    printf 'hdiutil-%s\n' "$1" >> "${LBC_MOCK_TRACE}"
    case "$1" in
        create)
            local previous="" argument source_folder=""
            for argument in "$@"; do [[ "${previous}" != -srcfolder ]] || source_folder="${argument}"; previous="${argument}"; done
            cp "${source_folder}/lbc" "${LBC_MOCK_PAYLOAD}"
            printf 'MOCK DISK IMAGE\n' > "${@: -1}" ;;
        attach)
            local previous="" argument mount_dir=""
            for argument in "$@"; do [[ "${previous}" != -mountpoint ]] || mount_dir="${argument}"; previous="${argument}"; done
            cp "${LBC_MOCK_PAYLOAD}" "${mount_dir}/lbc"
            [[ "${LBC_MOCK_CASE}" != mounted-binary-tampered ]] || printf '# tampered\n' >> "${mount_dir}/lbc" ;;
        detach) return 0 ;;
        *) return 1 ;;
    esac
}
xcrun() {
    printf 'xcrun-%s-%s\n' "$1" "$2" >> "${LBC_MOCK_TRACE}"
    if [[ "$1" == stapler ]]; then
        [[ "${LBC_MOCK_CASE}" != staple-failure || "$2" != staple ]] || return 1
        [[ "${LBC_MOCK_CASE}" != ticket-validation-failure || "$2" != validate ]] || return 1
        return 0
    fi
    local id=11111111-1111-4111-8111-111111111111
    case "$2" in
        submit)
            [[ "${LBC_MOCK_CASE}" != submission-failure ]] || return 1
            if [[ "${LBC_MOCK_CASE}" == rejected-notarization ]]; then printf '{"id":"%s","status":"Invalid"}\n' "${id}"
            elif [[ "${LBC_MOCK_CASE}" == missing-notary-id ]]; then printf '{"status":"Accepted"}\n'
            else printf '{"id":"%s","status":"Accepted"}\n' "${id}"; fi ;;
        info)
            [[ "${LBC_MOCK_CASE}" != inconsistent-notary-info ]] || id=22222222-2222-4222-8222-222222222222
            printf '{"id":"%s","status":"Accepted"}\n' "${id}" ;;
        log) printf '{"jobId":"%s","status":"Accepted","statusCode":0}\n' "${id}" > "${@: -1}" ;;
        *) return 1 ;;
    esac
}
spctl() { printf 'gatekeeper\n' >> "${LBC_MOCK_TRACE}"; [[ "${LBC_MOCK_CASE}" != gatekeeper-failure ]]; }
export -f uname base64 security openssl codesign cargo hdiutil xcrun spctl
passed=0
for test_case in success malformed-certificate missing-identity expired-identity duplicate-identity malformed-notary-key \
    signing-failure verification-failure wrong-team wrong-certificate missing-timestamp missing-runtime cli-failure \
    submission-failure rejected-notarization missing-notary-id inconsistent-notary-info staple-failure \
    ticket-validation-failure gatekeeper-failure mounted-binary-tampered missing-credential; do
    export LBC_MOCK_CASE="${test_case}" LBC_MOCK_TRACE="${fixture_root}/${test_case}.trace"
    output="${fixture_root}/${test_case}-output"
    : > "${LBC_MOCK_TRACE}"
    saved_certificate="${LBC_MACOS_CERTIFICATE_BASE64}"
    [[ "${test_case}" != missing-credential ]] || export LBC_MACOS_CERTIFICATE_BASE64=""
    status=0
    bash .circleci/sign-macos-release.sh "${fixture_root}/candidate" "${output}" > "${fixture_root}/${test_case}.log" 2>&1 || status=$?
    export LBC_MACOS_CERTIFICATE_BASE64="${saved_certificate}"
    if [[ "${test_case}" == success ]]; then
        if [[ "${status}" != 0 ]]; then sed -n '1,100p' "${fixture_root}/${test_case}.log" >&2; exit 1; fi
        for suffix in x86_64-unknown-linux-gnu.tar.gz x86_64-unknown-linux-gnu.tar.gz.sha256 \
            x86_64-pc-windows-msvc.zip x86_64-pc-windows-msvc.zip.sha256; do
            cp "${fixture_root}/candidate/lbc-${version}-${suffix}" "${output}/"
        done
        cp "${fixture_root}/candidate/lbc-${version}.cdx.json" "${fixture_root}/candidate/lbc-${version}.windows-signing.json" "${output}/"
        python3 .circleci/native_release.py "${output}" "${version}" "${CIRCLE_SHA1}" "${CIRCLE_WORKFLOW_ID}"
        awk '/codesign---force/ && !signed {signed=NR} /codesign---verify/ && !verified {verified=NR}
             /signed-cli/ {cli=NR} /xcrun-notarytool-submit/ {notary=NR}
             END {exit !(signed < verified && verified < cli && cli < notary)}' "${LBC_MOCK_TRACE}"
    elif [[ "${status}" == 0 || -e "${output}/lbc-${version}.macos-signing.json" ]]; then
        echo "Unsafe macOS signing fixture succeeded: ${test_case}" >&2; exit 1
    fi
    if grep -q 'security-create-keychain' "${LBC_MOCK_TRACE}"; then grep -q 'security-delete-keychain' "${LBC_MOCK_TRACE}"; fi
    passed=$((passed+1))
done
printf 'macOS signing fixtures: %s passed (MOCK native/Apple calls; no production signing/notarization evidence)\n' "${passed}"
