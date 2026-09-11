#!/usr/bin/env bash
set -euo pipefail

dist_dir="${1:-dist}"
api_url="${GITHUB_API_URL:-https://api.github.com}"

: "${GITHUB_TOKEN:?Set GITHUB_TOKEN in the CircleCI project settings}"
: "${CIRCLE_TAG:?This job must run for a Git tag}"
: "${CIRCLE_PROJECT_USERNAME:?CircleCI did not provide CIRCLE_PROJECT_USERNAME}"
: "${CIRCLE_PROJECT_REPONAME:?CircleCI did not provide CIRCLE_PROJECT_REPONAME}"

repository="${CIRCLE_PROJECT_USERNAME}/${CIRCLE_PROJECT_REPONAME}"
release_endpoint="${api_url}/repos/${repository}/releases"
response_file="$(mktemp)"
payload_file="$(mktemp)"
trap 'rm -f "${response_file}" "${payload_file}"' EXIT

github_request() {
    local method="$1"
    local url="$2"
    shift 2

    curl --silent --show-error \
        --request "${method}" \
        --header "Authorization: Bearer ${GITHUB_TOKEN}" \
        --header "Accept: application/vnd.github+json" \
        --header "X-GitHub-Api-Version: 2022-11-28" \
        --output "${response_file}" \
        --write-out '%{http_code}' \
        "$@" \
        "${url}"
}

show_api_error() {
    local action="$1"
    local status="$2"
    echo "GitHub API failed while ${action} (HTTP ${status})" >&2
    jq -r '.message // .' "${response_file}" >&2 || sed -n '1,40p' "${response_file}" >&2
}

status="$(github_request GET "${release_endpoint}/tags/${CIRCLE_TAG}")"
case "${status}" in
    200)
        ;;
    404)
        jq -n \
            --arg tag "${CIRCLE_TAG}" \
            '{tag_name: $tag, name: $tag, generate_release_notes: true}' \
            > "${payload_file}"
        status="$(github_request POST "${release_endpoint}" \
            --header 'Content-Type: application/json' \
            --data-binary "@${payload_file}")"
        if [[ "${status}" != "201" ]]; then
            show_api_error "creating release ${CIRCLE_TAG}" "${status}"
            exit 1
        fi
        ;;
    *)
        show_api_error "finding release ${CIRCLE_TAG}" "${status}"
        exit 1
        ;;
esac

release_id="$(jq -r '.id // empty' "${response_file}")"
upload_url="$(jq -r '.upload_url // empty | sub("\\{.*$"; "")' "${response_file}")"
if [[ -z "${release_id}" || -z "${upload_url}" ]]; then
    echo "GitHub response did not contain a release ID and upload URL" >&2
    exit 1
fi

status="$(github_request GET "${release_endpoint}/${release_id}/assets?per_page=100")"
if [[ "${status}" != "200" ]]; then
    show_api_error "listing assets for release ${CIRCLE_TAG}" "${status}"
    exit 1
fi
assets_json="$(<"${response_file}")"

shopt -s nullglob
artifacts=("${dist_dir}"/*)
if [[ "${#artifacts[@]}" -eq 0 ]]; then
    echo "No release artifacts found in ${dist_dir}" >&2
    exit 1
fi

uploaded_count=0
for artifact in "${artifacts[@]}"; do
    [[ -f "${artifact}" ]] || continue
    asset_name="$(basename "${artifact}")"
    existing_id="$(jq -r --arg name "${asset_name}" \
        '.[] | select(.name == $name) | .id' <<<"${assets_json}" | head -n 1)"

    if [[ -n "${existing_id}" ]]; then
        status="$(github_request DELETE "${release_endpoint}/assets/${existing_id}")"
        if [[ "${status}" != "204" ]]; then
            show_api_error "replacing ${asset_name}" "${status}"
            exit 1
        fi
    fi

    encoded_name="$(jq -rn --arg name "${asset_name}" '$name | @uri')"
    case "${asset_name}" in
        *.tar.gz) content_type="application/gzip" ;;
        *.sha256) content_type="text/plain" ;;
        *) content_type="application/octet-stream" ;;
    esac

    status="$(github_request POST "${upload_url}?name=${encoded_name}" \
        --header "Content-Type: ${content_type}" \
        --data-binary "@${artifact}")"
    if [[ "${status}" != "201" ]]; then
        show_api_error "uploading ${asset_name}" "${status}"
        exit 1
    fi
    echo "Uploaded ${asset_name} to ${repository} release ${CIRCLE_TAG}"
    uploaded_count=$((uploaded_count + 1))
done

if [[ "${uploaded_count}" -eq 0 ]]; then
    echo "No regular release artifacts found in ${dist_dir}" >&2
    exit 1
fi
