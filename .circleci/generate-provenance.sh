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
require_release_inputs "${dist_dir}" "${version}" false

source_sha="${CIRCLE_SHA1:-$(git rev-parse HEAD)}"
repository="$(release_repository)"
workflow_id="${CIRCLE_WORKFLOW_ID:-local}"
workflow_name="${CIRCLE_WORKFLOW_NAME:-ci_cd}"
job_name="${CIRCLE_JOB:-local}"
build_url="${CIRCLE_BUILD_URL:-local}"
branch="${CIRCLE_BRANCH:-}"
tag="${CIRCLE_TAG:-v${version}}"
records="$(mktemp)"
provenance_tmp="$(mktemp "${dist_dir}/.lbc-provenance.XXXXXX")"
trap 'rm -f "${records}" "${provenance_tmp}"' EXIT

if [[ ! "${source_sha}" =~ ^[0-9a-fA-F]{40}$ && ! "${source_sha}" =~ ^[0-9a-fA-F]{64}$ ]]; then
    echo "Source revision is not a full Git commit digest" >&2
    exit 1
fi

for name in "${RELEASE_INPUTS[@]}"; do
        artifact="${dist_dir}/${name}"
        digest="$(sha256sum "${artifact}" | awk '{print $1}')"
        jq -cn \
            --arg name "${name}" \
            --arg sha256 "${digest}" \
            '{name: $name, digest: {sha256: $sha256}}' >> "${records}"
done

jq -s \
    --arg version "${version}" \
    --arg source_sha "${source_sha}" \
    --arg repository "${repository}" \
    --arg workflow_id "${workflow_id}" \
    --arg workflow_name "${workflow_name}" \
    --arg job_name "${job_name}" \
    --arg build_url "${build_url}" \
    --arg branch "${branch}" \
    --arg tag "${tag}" \
    '{
        _type: "https://in-toto.io/Statement/v1",
        predicateType: "https://slsa.dev/provenance/v1",
        subject: .,
        predicate: {
            buildDefinition: {
                buildType: "https://circleci.com/librarycube/release/v1",
                externalParameters: {
                    repository: $repository, version: $version,
                    tag: $tag, ref: ("refs/tags/" + $tag)
                },
                resolvedDependencies: [{uri: ("git+https://github.com/" + $repository), digest: {gitCommit: $source_sha}}]
            },
            runDetails: {
                builder: {id: ("https://circleci.com/gh/" + $repository)},
                metadata: {
                    invocationId: $workflow_id,
                    workflowName: $workflow_name,
                    jobName: $job_name,
                    buildUrl: $build_url,
                    branch: $branch,
                    tag: $tag
                }
            }
        }
    }' "${records}" > "${provenance_tmp}"

jq -e --argjson expected "${#RELEASE_INPUTS[@]}" \
    '.subject | length == $expected' "${provenance_tmp}" >/dev/null
mv "${provenance_tmp}" "${dist_dir}/lbc-${version}.provenance.json"
