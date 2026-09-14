#!/usr/bin/env bash
set -euo pipefail

dist_dir="${1:-dist}"
version="$(sed -n '/^\[package\]/,/^\[/{s/^version = "\([^"]*\)"/\1/p;}' Cargo.toml)"
if [[ -z "${version}" ]]; then
    echo "Could not read the package version from Cargo.toml" >&2
    exit 1
fi
source_sha="${CIRCLE_SHA1:-$(git rev-parse HEAD)}"
repository="${CIRCLE_PROJECT_USERNAME:-unknown}/${CIRCLE_PROJECT_REPONAME:-unknown}"
workflow_id="${CIRCLE_WORKFLOW_ID:-local}"
generated_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
records="$(mktemp)"
trap 'rm -f "${records}"' EXIT

find "${dist_dir}" -maxdepth 1 -type f \( -name '*.tar.gz' -o -name '*.zip' -o -name '*.cdx.json' \) -print0 \
    | sort -z \
    | while IFS= read -r -d '' artifact; do
        digest="$(sha256sum "${artifact}" | awk '{print $1}')"
        jq -cn \
            --arg name "$(basename "${artifact}")" \
            --arg sha256 "${digest}" \
            '{name: $name, digest: {sha256: $sha256}}' >> "${records}"
    done

if [[ ! -s "${records}" ]]; then
    echo "No release archives or SBOM found in ${dist_dir}" >&2
    exit 1
fi

jq -s \
    --arg version "${version}" \
    --arg source_sha "${source_sha}" \
    --arg repository "${repository}" \
    --arg workflow_id "${workflow_id}" \
    --arg generated_at "${generated_at}" \
    '{
        _type: "https://in-toto.io/Statement/v1",
        predicateType: "https://slsa.dev/provenance/v1",
        subject: .,
        predicate: {
            buildDefinition: {
                buildType: "https://circleci.com/librarycube/release/v1",
                externalParameters: {version: $version},
                resolvedDependencies: [{uri: ("git+https://github.com/" + $repository), digest: {gitCommit: $source_sha}}]
            },
            runDetails: {
                builder: {id: "https://circleci.com/"},
                metadata: {invocationId: $workflow_id, startedOn: $generated_at}
            }
        }
    }' "${records}" > "${dist_dir}/lbc-${version}.provenance.json"

jq -e '.subject | length > 0' "${dist_dir}/lbc-${version}.provenance.json" >/dev/null
