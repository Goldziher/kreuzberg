#!/usr/bin/env bash

# Validate release tag and compute release metadata
#
# This script handles release metadata computation from different GitHub event types:
# - workflow_dispatch: Uses inputs.tag, inputs.dry_run, inputs.ref, inputs.targets
# - release: Uses github.event.release.tag_name
# - repository_dispatch: Uses github.event.client_payload
# - Other: Uses GITHUB_REF_NAME as fallback
#
# Environment Variables:
#   - GITHUB_EVENT_NAME: Type of GitHub event
#   - GITHUB_REF_NAME: Current ref name (for fallback)
#
# Inputs (via environment):
#   - INPUT_TAG: Release tag (e.g., v4.0.0-rc.1)
#   - INPUT_DRY_RUN: Prepare artifacts without publishing (optional, default: false)
#   - INPUT_REF: Git ref to build (optional, defaults to tag)
#   - INPUT_TARGETS: Comma-separated list of release targets (optional)
#   - EVENT_RELEASE_TAG: Tag from release event
#   - EVENT_RELEASE_DRY_RUN: Dry run from release event
#   - EVENT_DISPATCH_TAG: Tag from repository_dispatch
#   - EVENT_DISPATCH_DRY_RUN: Dry run from repository_dispatch
#   - EVENT_DISPATCH_REF: Ref from repository_dispatch
#   - EVENT_DISPATCH_TARGETS: Targets from repository_dispatch

set -euo pipefail

event="${GITHUB_EVENT_NAME:-${1:-}}"
tag=""
dry_run_input="false"
ref_input=""
targets_input=""

# Determine source of inputs based on event type
if [[ "$event" == "workflow_dispatch" ]]; then
	tag="${INPUT_TAG:-}"
	dry_run_input="${INPUT_DRY_RUN:-false}"
	ref_input="${INPUT_REF:-}"
	targets_input="${INPUT_TARGETS:-}"
elif [[ "$event" == "release" ]]; then
	tag="${EVENT_RELEASE_TAG:-}"
	dry_run_input="false"
	ref_input="refs/tags/${tag}"
	targets_input=""
elif [[ "$event" == "repository_dispatch" ]]; then
	tag="${EVENT_DISPATCH_TAG:-}"
	dry_run_input="${EVENT_DISPATCH_DRY_RUN:-false}"
	ref_input="${EVENT_DISPATCH_REF:-}"
	targets_input="${EVENT_DISPATCH_TARGETS:-}"
else
	tag="${GITHUB_REF_NAME:-}"
	dry_run_input="false"
	ref_input=""
	targets_input=""
	if [[ "$tag" == *-pre* || "$tag" == *-rc* ]]; then
		dry_run_input="true"
	fi
fi

# Validate tag
if [[ -z "$tag" ]]; then
	echo "Release tag could not be determined" >&2
	exit 1
fi

if [[ "$tag" != v* ]]; then
	echo "Tag must start with 'v' (e.g., v4.0.0-rc.1)" >&2
	exit 1
fi

# Extract version from tag
version="${tag#v}"

# Determine ref to checkout
if [[ -n "$ref_input" ]]; then
	ref="$ref_input"
else
	ref="refs/tags/${tag}"
fi

# Determine checkout_ref and target_sha for git operations
if [[ "$ref" =~ ^[0-9a-f]{40}$ ]]; then
	checkout_ref="refs/heads/main"
	target_sha="$ref"
elif [[ "$ref" =~ ^refs/ ]]; then
	checkout_ref="$ref"
	target_sha=""
else
	checkout_ref="refs/heads/${ref}"
	target_sha=""
fi

# Determine matrix_ref (for matrix builds)
if [[ "$ref" =~ ^[0-9a-f]{40}$ ]]; then
	matrix_ref="main"
elif [[ "$ref" =~ ^refs/heads/(.+)$ ]]; then
	matrix_ref="${BASH_REMATCH[1]}"
elif [[ "$ref" =~ ^refs/tags/(.+)$ ]]; then
	matrix_ref="${BASH_REMATCH[1]}"
else
	matrix_ref="$ref"
fi

# Determine if this is a tag
if [[ "$ref" =~ ^refs/tags/ ]]; then
	is_tag="true"
else
	is_tag="false"
fi

# Normalize target list
normalize_target_list() {
	local raw="$1"
	raw="${raw:-all}"
	if [[ -z "$raw" ]]; then
		echo "all"
	else
		echo "$raw"
	fi
}

targets_value=$(normalize_target_list "$targets_input")

# Initialize all release flags
release_python=false
release_node=false
release_ruby=false
release_cli=false
release_crates=false
release_docker=false
release_homebrew=false
release_java=false
release_csharp=false

# Helper function to set all targets
set_all_targets() {
	release_python=true
	release_node=true
	release_ruby=true
	release_cli=true
	release_crates=true
	release_docker=true
	release_homebrew=true
	release_java=true
	release_csharp=true
}

# Parse requested targets
mapfile -t requested_targets < <(echo "$targets_value" | tr ',' '\n')

processed_any=false
for raw_target in "${requested_targets[@]}"; do
	trimmed=$(echo "$raw_target" | tr '[:upper:]' '[:lower:]' | xargs)
	if [[ -z "$trimmed" ]]; then
		continue
	fi
	processed_any=true
	case "$trimmed" in
	all | '*' | 'default')
		set_all_targets
		break
		;;
	python)
		release_python=true
		;;
	node)
		release_node=true
		;;
	ruby)
		release_ruby=true
		;;
	cli)
		release_cli=true
		;;
	crates)
		release_crates=true
		;;
	docker)
		release_docker=true
		;;
	homebrew)
		release_homebrew=true
		;;
	java)
		release_java=true
		;;
	csharp | dotnet | cs | nuget)
		release_csharp=true
		;;
	none)
		release_python=false
		release_node=false
		release_ruby=false
		release_cli=false
		release_crates=false
		release_docker=false
		release_homebrew=false
		release_java=false
		release_csharp=false
		;;
	*)
		echo "Unknown release target '$trimmed'. Allowed: all, python, node, ruby, cli, crates, docker, homebrew, java, csharp." >&2
		exit 1
		;;
	esac
done

# Homebrew requires CLI
if [[ "$release_homebrew" == "true" ]]; then
	release_cli=true
fi

# If no targets were processed, default to all
if [[ "$processed_any" == "false" ]]; then
	set_all_targets
fi

# Build enabled targets list
enabled_targets=()
if [[ "$release_python" == "true" ]]; then enabled_targets+=("python"); fi
if [[ "$release_node" == "true" ]]; then enabled_targets+=("node"); fi
if [[ "$release_ruby" == "true" ]]; then enabled_targets+=("ruby"); fi
if [[ "$release_cli" == "true" ]]; then enabled_targets+=("cli"); fi
if [[ "$release_crates" == "true" ]]; then enabled_targets+=("crates"); fi
if [[ "$release_docker" == "true" ]]; then enabled_targets+=("docker"); fi
if [[ "$release_homebrew" == "true" ]]; then enabled_targets+=("homebrew"); fi
if [[ "$release_java" == "true" ]]; then enabled_targets+=("java"); fi
if [[ "$release_csharp" == "true" ]]; then enabled_targets+=("csharp"); fi

# Summarize targets
if [[ ${#enabled_targets[@]} -eq 9 ]]; then
	release_targets_summary="all"
elif [[ ${#enabled_targets[@]} -eq 0 ]]; then
	release_targets_summary="none"
else
	release_targets_summary=$(
		IFS=','
		echo "${enabled_targets[*]}"
	)
fi

# Determine if any release is enabled
release_any="false"
if [[ ${#enabled_targets[@]} -gt 0 ]]; then
	release_any="true"
fi

# Output results
cat <<JSON
{
  "tag": "$tag",
  "version": "$version",
  "ref": "$ref",
  "checkout_ref": "$checkout_ref",
  "target_sha": "$target_sha",
  "matrix_ref": "$matrix_ref",
  "dry_run": ${dry_run_input:-false},
  "is_tag": $is_tag,
  "release_targets": "$release_targets_summary",
  "release_any": $release_any,
  "release_python": $release_python,
  "release_node": $release_node,
  "release_ruby": $release_ruby,
  "release_cli": $release_cli,
  "release_crates": $release_crates,
  "release_docker": $release_docker,
  "release_homebrew": $release_homebrew,
  "release_java": $release_java,
  "release_csharp": $release_csharp
}
JSON
