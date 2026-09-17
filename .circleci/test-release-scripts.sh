#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd "${script_dir}/.." && pwd)"
cd "${repository_root}"
python3 .circleci/test-native-archives.py
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

export CIRCLE_SHA1
CIRCLE_SHA1="$(git rev-parse HEAD)"
export CIRCLE_PROJECT_USERNAME="tsuna-n"
export CIRCLE_PROJECT_REPONAME="libraryCU"
export CIRCLE_WORKFLOW_ID="fixture-workflow"
export CIRCLE_WORKFLOW_NAME="ci_cd"
export CIRCLE_JOB="publish_github_release"
export CIRCLE_BUILD_URL="https://circleci.example.invalid/build/fixture"
export CIRCLE_TAG="v${version}"
export LBC_WINDOWS_CERT_THUMBPRINT="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
export LBC_MACOS_IDENTITY_SHA1="BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
export LBC_MACOS_TEAM_ID="FIXTURE123"
printf 'SIMULATED native archive/receipt fixtures: NOT certificate or notarization evidence\n'
python3 .circleci/create-release-fixture.py "${dist_dir}" "${version}" "${CIRCLE_SHA1}" "${CIRCLE_WORKFLOW_ID}"

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

expect_signing_failure() {
    if "$@" bash .circleci/sign-release-assets.sh "${dist_dir}" >/dev/null 2>&1; then
        echo "Release signing accepted invalid/missing credentials" >&2
        exit 1
    fi
}
expect_signing_failure env -u LBC_RELEASE_SIGNING_KEY_BASE64
expect_signing_failure env -u LBC_RELEASE_SIGNING_FINGERPRINT
expect_signing_failure env LBC_RELEASE_SIGNING_KEY_BASE64=not-base64
expect_signing_failure env LBC_RELEASE_SIGNING_KEY_BASE64=bm90IGEga2V5

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

# Real signed source fixture. It never tags or configures the actual repository.
source_repo="${fixture_root}/source-repository"
mkdir "${source_repo}"
cp Cargo.toml "${source_repo}/Cargo.toml"
git -C "${source_repo}" init -q
git -C "${source_repo}" remote add origin git@github.com:tsuna-n/libraryCU.git
git -C "${source_repo}" config user.name 'Release source fixture'
git -C "${source_repo}" config user.email 'source@example.invalid'
git -C "${source_repo}" config user.signingkey "${valid_fingerprint}"
git -C "${source_repo}" config gpg.program gpg
git -C "${source_repo}" add Cargo.toml
GNUPGHOME="${key_home}" git -C "${source_repo}" commit -S -qm 'Signed fixture'
GNUPGHOME="${key_home}" git -C "${source_repo}" tag -s "v${version}" -m 'Signed release fixture'
maintainer_public="$(GNUPGHOME="${key_home}" gpg --batch --armor --export "${valid_fingerprint}" | base64 | tr -d '\n')"
source_gate() (
    cd "${source_repo}"
    env CIRCLE_SHA1="$(git rev-parse HEAD)" \
        LBC_MAINTAINER_SIGNING_KEY_BASE64="${maintainer_public}" \
        LBC_MAINTAINER_SIGNING_FINGERPRINT="${valid_fingerprint}" "$@" \
        bash "${script_dir}/verify-release-source.sh"
)
expect_source_failure() {
    local description="$1" diagnostic="$2"
    shift 2
    if source_gate "$@" >"${fixture_root}/source.log" 2>&1; then
        echo "Source gate accepted ${description}" >&2; exit 1
    fi
    if ! grep -Fq "${diagnostic}" "${fixture_root}/source.log"; then
        echo "Source gate failed for the wrong reason: ${description}" >&2
        sed -n '1,80p' "${fixture_root}/source.log" >&2; exit 1
    fi
}
source_gate
git -C "${source_repo}" config gpg.openpgp.program /bin/false
source_gate  # Local Git config cannot choose an alternate OpenPGP verifier.
git -C "${source_repo}" config --unset gpg.openpgp.program
valid_tag_object="$(git -C "${source_repo}" rev-parse "refs/tags/v${version}")"
GNUPGHOME="${key_home}" git -C "${source_repo}" tag -s v9.9.9 -m 'Wrong internally signed version'
git -C "${source_repo}" update-ref "refs/tags/v${version}" "refs/tags/v9.9.9"
expect_source_failure 'signed tag with wrong internal version' 'signed tag metadata'
git -C "${source_repo}" update-ref "refs/tags/v${version}" "${valid_tag_object}"
expect_source_failure 'missing fingerprint' LBC_MAINTAINER_SIGNING_FINGERPRINT LBC_MAINTAINER_SIGNING_FINGERPRINT=
for pin in not-a-fingerprint 0123456789ABCDEF "${valid_fingerprint,,}"; do
    expect_source_failure 'malformed fingerprint' 'full uppercase primary-key fingerprint' LBC_MAINTAINER_SIGNING_FINGERPRINT="${pin}"
done
expect_source_failure 'missing public key' LBC_MAINTAINER_SIGNING_KEY_BASE64 LBC_MAINTAINER_SIGNING_KEY_BASE64=
expect_source_failure 'malformed public key encoding' 'must be base64' LBC_MAINTAINER_SIGNING_KEY_BASE64=not-base64
expect_source_failure 'non-key public export' 'cannot be imported' LBC_MAINTAINER_SIGNING_KEY_BASE64=bm90IGEga2V5
expect_source_failure 'wrong key pin' 'matching usable primary identity' LBC_MAINTAINER_SIGNING_FINGERPRINT="0000000000000000000000000000000000000000"
expect_source_failure 'private key material' 'must not contain private key material' LBC_MAINTAINER_SIGNING_KEY_BASE64="${LBC_RELEASE_SIGNING_KEY_BASE64}"
expect_source_failure 'wrong exact revision' 'HEAD does not match' CIRCLE_SHA1="0000000000000000000000000000000000000000"
expect_source_failure 'wrong Cargo/tag version' 'does not match' CIRCLE_TAG=v9.9.9
for origin in git@github.com:foreign/repository.git tsuna-n/libraryCU https://evil.example/tsuna-n/libraryCU.git; do
    git -C "${source_repo}" remote set-url origin "${origin}"
    expect_source_failure 'wrong origin despite canonical CI metadata' 'canonical GitHub'
done
for origin in https://github.com/tsuna-n/libraryCU.git ssh://git@github.com/tsuna-n/libraryCU.git; do
    git -C "${source_repo}" remote set-url origin "${origin}"
    source_gate
done
git -C "${source_repo}" remote set-url origin git@github.com:tsuna-n/libraryCU.git
git -C "${source_repo}" tag -d "v${version}" >/dev/null
git -C "${source_repo}" tag "v${version}"
expect_source_failure 'lightweight tag' 'must exist and be annotated'
git -C "${source_repo}" tag -d "v${version}" >/dev/null
git -C "${source_repo}" -c tag.gpgsign=false tag -a "v${version}" -m 'Unsigned annotated fixture'
expect_source_failure 'unsigned annotated tag on a signed commit' 'release tag is unsigned'
git -C "${source_repo}" tag -d "v${version}" >/dev/null
GNUPGHOME="${key_home}" git -C "${source_repo}" tag -s "v${version}" -m 'Signed release fixture'
GNUPGHOME="${key_home}" git -C "${source_repo}" commit -S --allow-empty -qm 'Different signed fixture'
expect_source_failure 'signed tag targeting a different signed commit' 'tag target does not match'
git -C "${source_repo}" -c commit.gpgsign=false commit --allow-empty -qm 'Unsigned fixture'
git -C "${source_repo}" tag -d "v${version}" >/dev/null
GNUPGHOME="${key_home}" git -C "${source_repo}" tag -s "v${version}" -m 'Signed tag on unsigned commit'
expect_source_failure 'unsigned commit with correctly targeted signed tag' 'release commit is unsigned'
git -C "${source_repo}" replace HEAD HEAD~1
expect_source_failure 'replacement object concealing unsigned commit' 'release commit is unsigned'
git -C "${source_repo}" replace -d HEAD >/dev/null

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

case_dir="${fixture_root}/unexpected-directory"
cp -a "${valid_dist}" "${case_dir}"
mkdir "${case_dir}/unverified-subdirectory"
expect_verification_failure "${case_dir}" "an unexpected directory"

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

for mutation in \
    '.predicate.buildDefinition.externalParameters.repository = "foreign/repository"' \
    '.predicate.buildDefinition.resolvedDependencies[0].uri = "git+https://github.com/foreign/repository"' \
    '.predicate.buildDefinition.externalParameters.version = "9.9.9"' \
    '.predicate.buildDefinition.externalParameters.tag = "v9.9.9"' \
    '.predicate.buildDefinition.externalParameters.ref = "refs/heads/main"' \
    '.predicate.buildDefinition.buildType = "foreign-builder"' \
    '.predicate.runDetails.builder.id = "foreign-builder"' \
    '.predicate.runDetails.metadata.tag = "v9.9.9"' \
    '.predicate.runDetails.metadata.invocationId = "foreign-workflow"' \
    '.subject |= .[1:]' \
    '.subject += [.subject[0]]' \
    '.subject += [{name:"unexpected.bin",digest:{sha256:("0" * 64)}}]' \
    '.subject[0].digest.sha256 = ("0" * 64)' \
    '.predicate.runDetails.metadata.note = "modified after signing"'; do
    case_dir="${fixture_root}/mutated-provenance"
    rm -rf "${case_dir}"
    cp -a "${valid_dist}" "${case_dir}"
    provenance="${case_dir}/lbc-${version}.provenance.json"
    jq "${mutation}" "${provenance}" > "${provenance}.tmp"
    mv "${provenance}.tmp" "${provenance}"
    expect_verification_failure "${case_dir}" "mutated provenance: ${mutation}"
done
if CIRCLE_PROJECT_USERNAME=foreign bash .circleci/generate-provenance.sh "${dist_dir}" >/dev/null 2>&1; then
    echo "Provenance accepted a foreign release repository" >&2
    exit 1
fi

case_dir="${fixture_root}/duplicate-json-member"
cp -a "${valid_dist}" "${case_dir}"
sbom="${case_dir}/lbc-${version}.cdx.json"
sed -E 's/"bomFormat"[[:space:]]*:[[:space:]]*"CycloneDX"/"bomFormat":"foreign","bomFormat":"CycloneDX"/' "${sbom}" > "${sbom}.tmp"
mv "${sbom}.tmp" "${sbom}"
expect_verification_failure "${case_dir}" "duplicate JSON members"

case_dir="${fixture_root}/altered-sbom-signature"
cp -a "${valid_dist}" "${case_dir}"
GNUPGHOME="${key_home}" gpg --batch --yes --armor --digest-algo SHA1 \
    --local-user "${valid_fingerprint}" --detach-sign \
    --output "${case_dir}/lbc-${version}.cdx.json.asc" "${case_dir}/lbc-${version}.cdx.json"
expect_verification_failure "${case_dir}" "a weak SHA-1 signature"

# Real expired key/signature fixture: sign while the one-day key was valid,
# then verify now. This is not a mocked GPG status or a production identity.
expired_home="${fixture_root}/expired-home"
mkdir -m 700 "${expired_home}"
GNUPGHOME="${expired_home}" gpg --batch --faked-system-time 1577836800 --passphrase '' \
    --quick-generate-key 'Expired fixture <expired@example.invalid>' rsa2048 sign 1d >/dev/null 2>&1
expired_fingerprint="$(GNUPGHOME="${expired_home}" gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" {print $10; exit}')"
expired_private="$(GNUPGHOME="${expired_home}" gpg --batch --armor --export-secret-keys "${expired_fingerprint}" | base64 | tr -d '\n')"
expect_signing_failure env LBC_RELEASE_SIGNING_KEY_BASE64="${expired_private}" LBC_RELEASE_SIGNING_FINGERPRINT="${expired_fingerprint}"
case_dir="${fixture_root}/expired-signature"
cp -a "${valid_dist}" "${case_dir}"
GNUPGHOME="${expired_home}" gpg --batch --armor --export "${expired_fingerprint}" > "${case_dir}/lbc-release-signing-key.asc"
GNUPGHOME="${expired_home}" gpg --batch --yes --faked-system-time 1577836860 --armor \
    --digest-algo SHA256 --local-user "${expired_fingerprint}" --detach-sign \
    --output "${case_dir}/lbc-${version}.cdx.json.asc" "${case_dir}/lbc-${version}.cdx.json" >/dev/null 2>&1
if LBC_RELEASE_SIGNING_FINGERPRINT="${expired_fingerprint}" bash .circleci/verify-release-assets.sh "${case_dir}" >/dev/null 2>&1; then
    echo "Release verification accepted an expired identity" >&2
    exit 1
fi

revoked_home="${fixture_root}/revoked-home"
mkdir -m 700 "${revoked_home}"
GNUPGHOME="${revoked_home}" gpg --batch --import "${private_key}" >/dev/null 2>&1
sed 's/^://' "${key_home}/openpgp-revocs.d/${valid_fingerprint}.rev" > "${fixture_root}/revocation.asc"
GNUPGHOME="${revoked_home}" gpg --batch --import "${fixture_root}/revocation.asc" >/dev/null 2>&1
revoked_private="$(GNUPGHOME="${revoked_home}" gpg --batch --armor --export-secret-keys "${valid_fingerprint}" | base64 | tr -d '\n')"
expect_signing_failure env LBC_RELEASE_SIGNING_KEY_BASE64="${revoked_private}"
case_dir="${fixture_root}/revoked-signature"
cp -a "${valid_dist}" "${case_dir}"
GNUPGHOME="${revoked_home}" gpg --batch --armor --export "${valid_fingerprint}" > "${case_dir}/lbc-release-signing-key.asc"
expect_verification_failure "${case_dir}" "a revoked identity"

# Test native-record policy directly, so bad-record tests do not pass merely
# because the old OpenPGP signature also fails after a JSON edit.
for platform in windows macos; do
    for mutation in \
        '.mode = "candidate"' '.sourceSha = ("0" * 40)' '.repository = "foreign/repository"' \
        '.version = "9.9.9"' '.tag = "v9.9.9"' '.workflowId = "foreign-workflow"' \
        '.signer.certificateSha1 = ("0" * 40)' '.binary.sha256 = ("0" * 64)' \
        '.archives[0].sha256 = ("0" * 64)' '.verification.signature = false' \
        '.verification.timestamp = false' '.verification.releaseCli = false' \
        '.verification.packagedBinary = false'; do
        case_dir="${fixture_root}/mutated-native-record-${platform}"
        rm -rf "${case_dir}"; cp -a "${valid_dist}" "${case_dir}"
        receipt="${case_dir}/lbc-${version}.${platform}-signing.json"
        jq "${mutation}" "${receipt}" > "${receipt}.tmp"; mv "${receipt}.tmp" "${receipt}"
        if python3 .circleci/native_release.py "${case_dir}" "${version}" "${CIRCLE_SHA1}" "${CIRCLE_WORKFLOW_ID}" >/dev/null 2>&1; then
            echo "Native validator accepted ${platform}: ${mutation}" >&2; exit 1
        fi
    done
done
for mutation in '.signer.teamId = "WRONGTEAM1"' '.verification.hardenedRuntime = false' \
    '.verification.gatekeeper = false' '.verification.notarization.status = "Invalid"' \
    '.verification.notarization.stapled = false' '.verification.notarization.validated = false' \
    '.verification.notarization.id = "------------------------------------"'; do
    case_dir="${fixture_root}/mutated-notarization"
    rm -rf "${case_dir}"; cp -a "${valid_dist}" "${case_dir}"
    receipt="${case_dir}/lbc-${version}.macos-signing.json"
    jq "${mutation}" "${receipt}" > "${receipt}.tmp"; mv "${receipt}.tmp" "${receipt}"
    if python3 .circleci/native_release.py "${case_dir}" "${version}" "${CIRCLE_SHA1}" "${CIRCLE_WORKFLOW_ID}" >/dev/null 2>&1; then
        echo "Native validator accepted invalid notarization: ${mutation}" >&2; exit 1
    fi
done

case_dir="${fixture_root}/private-public-key"
cp -a "${valid_dist}" "${case_dir}"
cp "${private_key}" "${case_dir}/lbc-release-signing-key.asc"
expect_verification_failure "${case_dir}" "private key material in the public-key asset"

# A real signing subkey must verify through the configured PRIMARY fingerprint.
subkey_home="${fixture_root}/subkey-home"
mkdir -m 700 "${subkey_home}"
GNUPGHOME="${subkey_home}" gpg --batch --passphrase '' --quick-generate-key \
    'Subkey fixture <subkey@example.invalid>' rsa2048 cert 1d >/dev/null 2>&1
subkey_primary="$(GNUPGHOME="${subkey_home}" gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" {print $10; exit}')"
GNUPGHOME="${subkey_home}" gpg --batch --passphrase '' --quick-add-key "${subkey_primary}" ed25519 sign 1d >/dev/null 2>&1
subkey_private="$(GNUPGHOME="${subkey_home}" gpg --batch --armor --export-secret-keys "${subkey_primary}" | base64 | tr -d '\n')"
subkey_public="$(GNUPGHOME="${subkey_home}" gpg --batch --armor --export "${subkey_primary}" | base64 | tr -d '\n')"
expect_source_failure 'wrong public key with trusted original pin' 'matching usable primary identity' LBC_MAINTAINER_SIGNING_KEY_BASE64="${subkey_public}"
GNUPGHOME="${subkey_home}" git -C "${source_repo}" -c user.signingkey="${subkey_primary}" commit -S --allow-empty -qm 'Signing subkey source'
git -C "${source_repo}" tag -d "v${version}" >/dev/null
GNUPGHOME="${subkey_home}" git -C "${source_repo}" -c user.signingkey="${subkey_primary}" tag -s "v${version}" -m 'Signing subkey tag'
source_gate LBC_MAINTAINER_SIGNING_KEY_BASE64="${subkey_public}" LBC_MAINTAINER_SIGNING_FINGERPRINT="${subkey_primary}"
subkey_pin="$(GNUPGHOME="${subkey_home}" gpg --batch --with-colons --list-keys | awk -F: '$1 == "sub" {subkey=1} $1 == "fpr" && subkey {print $10; exit}')"
expect_source_failure 'subkey pin instead of primary' 'matching usable primary identity' LBC_MAINTAINER_SIGNING_KEY_BASE64="${subkey_public}" LBC_MAINTAINER_SIGNING_FINGERPRINT="${subkey_pin}"
case_dir="${fixture_root}/valid-signing-subkey"
cp -a "${valid_dist}" "${case_dir}"
LBC_RELEASE_SIGNING_KEY_BASE64="${subkey_private}" LBC_RELEASE_SIGNING_FINGERPRINT="${subkey_primary}" \
    bash .circleci/sign-release-assets.sh "${case_dir}"
LBC_RELEASE_SIGNING_FINGERPRINT="${subkey_primary}" bash .circleci/verify-release-assets.sh "${case_dir}"

weak_home="${fixture_root}/weak-home"
mkdir -m 700 "${weak_home}"
GNUPGHOME="${weak_home}" gpg --batch --passphrase '' --quick-generate-key \
    'Weak TEST fixture <weak@example.invalid>' rsa1024 sign 1d >/dev/null 2>&1
weak_pin="$(GNUPGHOME="${weak_home}" gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" {print $10; exit}')"
weak_private="$(GNUPGHOME="${weak_home}" gpg --batch --armor --export-secret-keys "${weak_pin}" | base64 | tr -d '\n')"
expect_signing_failure env LBC_RELEASE_SIGNING_KEY_BASE64="${weak_private}" LBC_RELEASE_SIGNING_FINGERPRINT="${weak_pin}"
# Exercise weak signing SUBKEY rejection independently of primary-key policy.
GNUPGHOME="${subkey_home}" gpg --batch --passphrase '' --quick-add-key "${subkey_primary}" rsa1024 sign 1d >/dev/null 2>&1
weak_subkey="$(GNUPGHOME="${subkey_home}" gpg --batch --with-colons --list-keys | awk -F: '$1 == "sub" {weak=($3 == 1024)} $1 == "fpr" && weak {print $10; exit}')"
GNUPGHOME="${subkey_home}" gpg --batch --armor --export "${subkey_primary}" > "${case_dir}/lbc-release-signing-key.asc"
GNUPGHOME="${subkey_home}" gpg --batch --yes --armor --digest-algo SHA256 --local-user "${weak_subkey}!" \
    --detach-sign --output "${case_dir}/lbc-${version}.cdx.json.asc" "${case_dir}/lbc-${version}.cdx.json"
if LBC_RELEASE_SIGNING_FINGERPRINT="${subkey_primary}" bash .circleci/verify-release-assets.sh "${case_dir}" >"${fixture_root}/weak.log" 2>&1; then
    echo "Verification accepted a weak RSA signing subkey" >&2; exit 1
fi
grep -Fq 'weak or unsupported signing key' "${fixture_root}/weak.log"

# Protected disposable key: the production script reads the masked password
# through a private file descriptor, never a command-line argument.
protected_home="${fixture_root}/protected-home"
mkdir -m 700 "${protected_home}"
GNUPGHOME="${protected_home}" gpg --batch --pinentry-mode loopback --passphrase 'fixture-only-password' \
    --quick-generate-key 'Protected fixture <protected@example.invalid>' ed25519 sign 1d >/dev/null 2>&1
protected_fingerprint="$(GNUPGHOME="${protected_home}" gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" {print $10; exit}')"
protected_private="$(GNUPGHOME="${protected_home}" gpg --batch --pinentry-mode loopback --passphrase 'fixture-only-password' \
    --armor --export-secret-keys "${protected_fingerprint}" | base64 | tr -d '\n')"
case_dir="${fixture_root}/protected-key"
cp -a "${valid_dist}" "${case_dir}"
for password in '' wrong-password; do
    if LBC_RELEASE_SIGNING_KEY_BASE64="${protected_private}" LBC_RELEASE_SIGNING_FINGERPRINT="${protected_fingerprint}" \
        LBC_RELEASE_SIGNING_PASSPHRASE="${password}" bash .circleci/sign-release-assets.sh "${case_dir}" >/dev/null 2>&1; then
        echo "Protected release key accepted a missing/wrong passphrase" >&2; exit 1
    fi
done
LBC_RELEASE_SIGNING_KEY_BASE64="${protected_private}" LBC_RELEASE_SIGNING_FINGERPRINT="${protected_fingerprint}" \
    LBC_RELEASE_SIGNING_PASSPHRASE='fixture-only-password' bash .circleci/sign-release-assets.sh "${case_dir}"
LBC_RELEASE_SIGNING_FINGERPRINT="${protected_fingerprint}" bash .circleci/verify-release-assets.sh "${case_dir}"

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
        --expected-assets-file "${expected_assets_file}" \
        --local-dist "${dist_dir}" &
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
jq -e --arg tag "v${version}" --arg source "${CIRCLE_SHA1}" \
    '.publication_payload.name == $tag and .publication_payload.prerelease == false and
     (.publication_payload.body | contains($source)) and
     (.publication_payload.body | contains("SLSA Build Level 2"))' "${state_file}" >/dev/null

state_file="$(run_publish_fixture mutate-local success)"
jq -e '.published_by_client and (.error == null)' "${state_file}" >/dev/null
grep -q 'injected original-directory mutation' "${dist_dir}/${RELEASE_INPUTS[0]}"
rm -rf "${dist_dir}"
cp -a "${valid_dist}" "${dist_dir}"
state_file="$(run_publish_fixture public-race failure)"
jq -e '(.published_by_client == false) and (.uploads | length == 1) and (.deletes | length == 0)' "${state_file}" >/dev/null

state_file="$(run_publish_fixture fail-upload failure)"
jq -e '.created_draft and (.published == false) and .draft and (.assets | length == 3) and (.error == null)' \
    "${state_file}" >/dev/null

state_file="$(run_publish_fixture public failure)"
jq -e '(.created_draft == false) and .published and (.assets | length == 0) and (.error == null)' \
    "${state_file}" >/dev/null

for mode in api-error malformed unexpected duplicate tampered invalid-id; do
    state_file="$(run_publish_fixture "${mode}" failure)"
    jq -e '(.published == false) and .draft' "${state_file}" >/dev/null
    if [[ "${mode}" == invalid-id ]]; then
        jq -e '(.uploads | length == 0) and (.deletes | length == 0)' "${state_file}" >/dev/null
    fi
done

case_dir="${fixture_root}/tampered-input"
cp -a "${valid_dist}" "${case_dir}"
printf 'tampered\n' >> "${case_dir}/${RELEASE_INPUTS[0]}"
expect_verification_failure "${case_dir}" "a tampered archive"
printf 'Release fixtures passed: real disposable OpenPGP/source signatures, strict provenance/native records, 12 draft API scenarios\n'
