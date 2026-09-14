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

    RELEASE_INPUTS=(
        "${linux}"
        "${linux}.sha256"
        "${macos}"
        "${macos}.sha256"
        "${windows}"
        "${windows}.sha256"
        "lbc-${version}.cdx.json"
    )
    if [[ "${include_provenance}" == "true" ]]; then
        RELEASE_INPUTS+=("lbc-${version}.provenance.json")
    fi
}

require_release_inputs() {
    local dist_dir="$1"
    local version="$2"
    local include_provenance="${3:-false}"
    set_release_inputs "${version}" "${include_provenance}"

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
    for archive in \
        "lbc-${version}-x86_64-unknown-linux-gnu.tar.gz" \
        "lbc-${version}-universal-apple-darwin.tar.gz" \
        "lbc-${version}-x86_64-pc-windows-msvc.zip"
    do
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
    if ! jq -e '
        .bomFormat == "CycloneDX" and
        (.components | length > 0) and
        all(.components[];
            (.name | length > 0) and
            (.version | length > 0) and
            ((.licenses // []) | length > 0)
        ) and
        any(.components[]; ((.hashes // []) | length > 0))
    ' "${sbom}" >/dev/null; then
        echo "CycloneDX SBOM is invalid or lacks required component metadata" >&2
        return 1
    fi

    if [[ "${include_provenance}" == "true" ]]; then
        validate_provenance "${dist_dir}" "${version}"
    fi
}

validate_provenance() {
    local dist_dir="$1"
    local version="$2"
    local source_sha="${CIRCLE_SHA1:-$(git rev-parse HEAD)}"
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

    if ! jq -e --argjson expected "${expected}" --arg source_sha "${source_sha}" '
        ._type == "https://in-toto.io/Statement/v1" and
        .predicateType == "https://slsa.dev/provenance/v1" and
        .subject == $expected and
        (.predicate.buildDefinition.resolvedDependencies | length == 1) and
        (.predicate.buildDefinition.resolvedDependencies[0].uri | startswith("git+https://github.com/")) and
        .predicate.buildDefinition.resolvedDependencies[0].digest.gitCommit == $source_sha and
        (.predicate.runDetails.builder.id | length > 0) and
        (.predicate.runDetails.metadata.invocationId | length > 0)
    ' "${dist_dir}/lbc-${version}.provenance.json" >/dev/null; then
        echo "Provenance does not describe the exact release inputs and source revision" >&2
        return 1
    fi
    set_release_inputs "${version}" true
}
