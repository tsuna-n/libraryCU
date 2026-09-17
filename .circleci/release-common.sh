#!/usr/bin/env bash

# Shared release-asset contract. Callers must enable `set -euo pipefail`.

release_version() {
    sed -n '/^\[package\]/,/^\[/{s/^version = "\([^"]*\)"/\1/p;}' Cargo.toml
}

validate_release_tag() {
    local version="$1"
    if [[ -n "${CIRCLE_TAG:-}" && "${CIRCLE_TAG}" != "v${version}" ]]; then
        echo "Release tag ${CIRCLE_TAG} does not match Cargo.toml version ${version}" >&2
        return 1
    fi
}

set_release_inputs() {
    local version="$1"
    local include_provenance="${2:-false}"
    local linux="lbc-${version}-x86_64-unknown-linux-gnu.tar.gz"
    local macos="lbc-${version}-universal-apple-darwin.tar.gz"
    local windows="lbc-${version}-x86_64-pc-windows-msvc.zip"
    local macos_dmg="lbc-${version}-universal-apple-darwin.dmg"

    RELEASE_ARCHIVES=("${linux}" "${macos}" "${windows}" "${macos_dmg}")
    RELEASE_INPUTS=(
        "${RELEASE_ARCHIVES[0]}"
        "${RELEASE_ARCHIVES[0]}.sha256"
        "${RELEASE_ARCHIVES[1]}"
        "${RELEASE_ARCHIVES[1]}.sha256"
        "${RELEASE_ARCHIVES[2]}"
        "${RELEASE_ARCHIVES[2]}.sha256"
        "${macos_dmg}"
        "${macos_dmg}.sha256"
        "lbc-${version}.cdx.json"
        "lbc-${version}.windows-signing.json"
        "lbc-${version}.macos-signing.json"
    )
    if [[ "${include_provenance}" == "true" ]]; then
        RELEASE_INPUTS+=("lbc-${version}.provenance.json")
    fi
}

# The one canonical production release manifest. Tests and publication callers
# must consume this array rather than maintaining their own asset-name lists.
set_release_assets() {
    local version="$1"
    set_release_inputs "${version}" true
    RELEASE_ASSETS=(
        "${RELEASE_INPUTS[@]}"
        "${RELEASE_INPUTS[@]/%/.asc}"
        "lbc-release-signing-key.asc"
    )
}

release_repository() {
    local repository
    local remote
    remote="$(git remote get-url origin 2>/dev/null)" || remote=""
    case "${remote}" in
        git@github.com:tsuna-n/libraryCU|git@github.com:tsuna-n/libraryCU.git|\
        ssh://git@github.com/tsuna-n/libraryCU|ssh://git@github.com/tsuna-n/libraryCU.git|\
        https://github.com/tsuna-n/libraryCU|https://github.com/tsuna-n/libraryCU.git)
            remote="tsuna-n/libraryCU" ;;
        *)
            echo "Release origin must be the canonical GitHub HTTPS or Git SSH URL for tsuna-n/libraryCU" >&2
            return 1 ;;
    esac
    if [[ -n "${CIRCLE_PROJECT_USERNAME:-}" && -n "${CIRCLE_PROJECT_REPONAME:-}" ]]; then
        repository="${CIRCLE_PROJECT_USERNAME}/${CIRCLE_PROJECT_REPONAME}"
    else
        repository="${remote}"
    fi
    if [[ "${repository}" != "tsuna-n/libraryCU" || "${remote}" != "tsuna-n/libraryCU" ]]; then
        echo "Release repository must be exactly tsuna-n/libraryCU" >&2
        return 1
    fi
    printf '%s\n' "${repository}"
}

require_release_inputs() {
    local dist_dir="$1"
    local version="$2"
    local include_provenance="${3:-false}"
    set_release_inputs "${version}" "${include_provenance}"
    if [[ ! -d "${dist_dir}" || -L "${dist_dir}" ]]; then
        echo "Release directory is missing or unsafe" >&2
        return 1
    fi

    local name
    for name in "${RELEASE_INPUTS[@]}"; do
        if [[ ! -f "${dist_dir}/${name}" || -L "${dist_dir}/${name}" ]]; then
            echo "Required release artifact is missing or unsafe: ${dist_dir}/${name}" >&2
            return 1
        fi
        if [[ ! -s "${dist_dir}/${name}" ]]; then
            echo "Required release artifact is empty: ${dist_dir}/${name}" >&2
            return 1
        fi
    done

    local archive checksum expected actual
    for archive in "${RELEASE_ARCHIVES[@]}"; do
        checksum="${dist_dir}/${archive}.sha256"
        expected="$(sha256sum "${dist_dir}/${archive}" | awk -v name="${archive}" '{print $1 "  " name}')"
        actual="$(tr -d '\r' < "${checksum}")"
        actual="${actual%$'\n'}"
        if [[ "${actual}" != "${expected}" ]]; then
            echo "Checksum file does not match ${archive}" >&2
            return 1
        fi
    done

    local sbom="${dist_dir}/lbc-${version}.cdx.json"
    python3 "${script_dir}/release_json.py" "${sbom}"
    if ! jq -e '
        .bomFormat == "CycloneDX" and
        (.specVersion == "1.3" or .specVersion == "1.4" or .specVersion == "1.5" or .specVersion == "1.6") and
        (.version | type == "number" and . > 0 and . == floor) and
        (.components | type == "array" and length > 0) and
        all(.components[];
            (.name | type == "string" and length > 0) and
            (.version | type == "string" and length > 0) and
            (.licenses | type == "array" and length > 0) and
            all(.licenses[];
                (.expression | type == "string" and length > 0) or
                (.license.id | type == "string" and length > 0) or
                (.license.name | type == "string" and length > 0)
            ) and
            (.hashes == null or (.hashes | type == "array")) and
            all(.hashes[]?;
                .alg == "SHA-256" and (.content | type == "string" and test("^[0-9a-fA-F]{64}$"))
            )
        ) and
        any(.components[]; ((.hashes // []) | length > 0))
    ' "${sbom}" >/dev/null; then
        echo "CycloneDX SBOM is invalid or lacks required component metadata" >&2
        return 1
    fi

    if [[ "${include_provenance}" == "true" ]]; then
        validate_provenance "${dist_dir}" "${version}"
    fi
    python3 "${script_dir}/native_release.py" "${dist_dir}" "${version}" \
        "${CIRCLE_SHA1:-$(git rev-parse HEAD)}" "${CIRCLE_WORKFLOW_ID:-local}"
}

validate_provenance() {
    local dist_dir="$1"
    local version="$2"
    local source_sha="${CIRCLE_SHA1:-$(git rev-parse HEAD)}"
    local repository
    repository="$(release_repository)"
    local records
    records="$(mktemp)"
    local name digest
    set_release_inputs "${version}" false
    for name in "${RELEASE_INPUTS[@]}"; do
        digest="$(sha256sum "${dist_dir}/${name}" | awk '{print $1}')"
        jq -cn --arg name "${name}" --arg sha256 "${digest}" \
            '{name: $name, digest: {sha256: $sha256}}' >> "${records}"
    done
    local expected
    expected="$(jq -s '.' "${records}")"
    rm -f "${records}"

    python3 "${script_dir}/release_json.py" "${dist_dir}/lbc-${version}.provenance.json"
    if ! jq -e \
        --argjson expected "${expected}" \
        --arg source_sha "${source_sha}" \
        --arg repository "${repository}" \
        --arg version "${version}" \
        --arg workflow_id "${CIRCLE_WORKFLOW_ID:-local}" \
        --arg workflow_name "${CIRCLE_WORKFLOW_NAME:-ci_cd}" \
        --arg job_name "${CIRCLE_JOB:-local}" \
        --arg build_url "${CIRCLE_BUILD_URL:-local}" '
        ._type == "https://in-toto.io/Statement/v1" and
        .predicateType == "https://slsa.dev/provenance/v1" and
        .subject == $expected and
        .predicate.buildDefinition.buildType == "https://circleci.com/librarycube/release/v1" and
        .predicate.buildDefinition.externalParameters == {
            repository: $repository, version: $version,
            tag: ("v" + $version), ref: ("refs/tags/v" + $version)
        } and
        (.predicate.buildDefinition.resolvedDependencies | length == 1) and
        .predicate.buildDefinition.resolvedDependencies[0].uri == ("git+https://github.com/" + $repository) and
        .predicate.buildDefinition.resolvedDependencies[0].digest.gitCommit == $source_sha and
        .predicate.runDetails.builder.id == ("https://circleci.com/gh/" + $repository) and
        .predicate.runDetails.metadata.invocationId == $workflow_id and ($workflow_id | length > 0) and
        .predicate.runDetails.metadata.jobName == $job_name and ($job_name | length > 0) and
        .predicate.runDetails.metadata.buildUrl == $build_url and ($build_url | length > 0) and
        .predicate.runDetails.metadata.workflowName == $workflow_name and ($workflow_name | length > 0) and
        .predicate.runDetails.metadata.tag == ("v" + $version)
    ' "${dist_dir}/lbc-${version}.provenance.json" >/dev/null; then
        echo "Provenance does not describe the exact release inputs and source revision" >&2
        return 1
    fi
    set_release_inputs "${version}" true
}

validate_signing_fingerprint() {
    if [[ ! "$1" =~ ^[0-9A-F]{40}$ && ! "$1" =~ ^[0-9A-F]{64}$ ]]; then
        echo "Signing fingerprint must be a full uppercase primary-key fingerprint" >&2
        return 1
    fi
}

# GNUPGHOME must be a fresh private directory. Reject extra primary keys and
# unusable/expired/revoked primary identities; valid signing subkeys are allowed.
validate_imported_identity() {
    local fingerprint="$1" secret_allowed="${2:-false}"
    validate_signing_fingerprint "${fingerprint}"
    local listing
    listing="$(gpg --no-options --batch --with-colons --list-keys)"
    if ! awk -F: -v expected="${fingerprint}" '
        $1 == "pub" {count++; unusable += ($2 ~ /[redi]/); primary=1}
        $1 == "fpr" && primary {matched += ($10 == expected); primary=0}
        END {exit !(count == 1 && matched == 1 && unusable == 0)}
    ' <<< "${listing}"; then
        echo "Signing key must contain exactly one matching usable primary identity" >&2
        return 1
    fi
    if ! awk -F: '
        $1 == "pub" {exit !(($4 ~ /^(1|2|3)$/ && $3 >= 2048) ||
                            ($4 == 19 && $3 >= 256) || ($4 == 22 && $3 >= 255))}
    ' <<< "${listing}"; then
        echo "Primary signing identity uses a weak or unsupported public-key algorithm" >&2
        return 1
    fi
    local secrets
    secrets="$(gpg --no-options --batch --with-colons --list-secret-keys | awk -F: '$1 == "sec" {count++} END {print count+0}')"
    if [[ "${secret_allowed}" == false && "${secrets}" != 0 ]]; then
        echo "A public verification key must not contain private key material" >&2
        return 1
    fi
}

# A zero GPG exit code alone can accept cryptographically valid signatures from
# expired/revoked identities. Pin the machine-readable signature verdict instead.
validate_gpg_status() {
    local status_file="$1" fingerprint="$2"
    if ! awk -v expected="${fingerprint}" '
        $1 == "[GNUPG:]" {
            if ($2 == "NEWSIG") signatures++
            if ($2 == "GOODSIG") good++
            if ($2 == "VALIDSIG") {
                valid++
                if (($3 != expected && $12 != expected) || $10 !~ /^(8|9|10)$/) bad++
            }
            if ($2 ~ /^(BADSIG|ERRSIG|EXPSIG|EXPKEYSIG|REVKEYSIG|KEYREVOKED|NO_PUBKEY|FAILURE|ERROR|NODATA)$/) bad++
        }
        END {exit !(signatures == 1 && good == 1 && valid == 1 && bad == 0)}
    ' "${status_file}"; then
        echo "Signature identity/status is invalid, expired, revoked, weak, or ambiguous" >&2
        return 1
    fi
    local signer listing
    signer="$(awk '$1 == "[GNUPG:]" && $2 == "VALIDSIG" {print $3}' "${status_file}")"
    listing="$(gpg --no-options --batch --with-colons --list-keys)"
    if ! awk -F: -v signer="${signer}" '
        $1 == "pub" || $1 == "sub" {bits=$3; algorithm=$4; key=1}
        $1 == "fpr" && key {
            if ($10 == signer) accepted = ((algorithm ~ /^(1|2|3)$/ && bits >= 2048) ||
                                         (algorithm == 19 && bits >= 256) ||
                                         (algorithm == 22 && bits >= 255))
            key=0
        }
        END {exit !accepted}
    ' <<< "${listing}"; then
        echo "Signature uses a weak or unsupported signing key" >&2
        return 1
    fi
}

verify_openpgp_signature() {
    local signature="$1" artifact="$2" fingerprint="$3" status_file
    status_file="$(mktemp)"
    if ! gpg --no-options --batch --no-auto-key-retrieve --no-auto-key-import \
        --status-fd 3 --verify "${signature}" "${artifact}" 3>"${status_file}" >/dev/null 2>&1; then
        rm -f "${status_file}"
        echo "OpenPGP signature verification failed" >&2
        return 1
    fi
    local result=0
    validate_gpg_status "${status_file}" "${fingerprint}" || result=$?
    rm -f "${status_file}"
    return "${result}"
}
