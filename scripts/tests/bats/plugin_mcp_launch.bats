#!/usr/bin/env bats
#
# Contract tests for plugin/scripts/mcp-launch.sh.
#
# FAN-OUT SHARED: byte-identical in xberg, crawlberg, html-to-markdown, liter-llm and
# tree-sitter-language-pack. Unlike the scripts/lib suites, the five launchers are NOT one script
# plus a token -- they have diverged into different internal function names (`runnable` vs
# `runs_ok`, `try_download` vs `download_install`), different log wording and different download
# integrity postures. What has NOT diverged is what the script does when you run it.
#
# So this suite tests strictly through the process boundary: it runs the script and asserts the
# exit status, which launcher was exec'd, and that argv arrives intact. Internal names never
# appear, which is what lets one file cover five divergent implementations. Every parameter it
# needs is read out of the script in setup_file, so it names no product.
#
# The script exec()s, so the launcher it chooses BECOMES the process -- a stub that prints a
# marker is therefore direct evidence of the branch taken, not an inference from log text. ~keep

setup_file() {
	bats_load_library xberg-bats

	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPO_ROOT/plugin/scripts/mcp-launch.sh"
	[ -f "$SCRIPT" ] || {
		echo "no plugin/scripts/mcp-launch.sh under $REPO_ROOT" >&2
		return 1
	}

	# Read the four per-product parameters out of the script itself. Deriving them is what keeps
	# this file identical across repos; an empty derivation would silently turn most assertions
	# below into comparisons between two empty strings, so each one is checked. ~keep
	BINARY_NAME="$(sed -n 's/^BINARY_NAME="\(.*\)"$/\1/p' "$SCRIPT" | head -n 1)"
	NPM_PKG="$(sed -n 's/^NPM_PKG="\(.*\)"$/\1/p' "$SCRIPT" | head -n 1)"
	PYPI_PKG="$(sed -n 's/^PYPI_PKG="\(.*\)"$/\1/p' "$SCRIPT" | head -n 1)"
	LAUNCHER_VAR="$(sed -n 's/^LAUNCHER="\${\([A-Z_]*\):-auto}"$/\1/p' "$SCRIPT" | head -n 1)"

	local name
	for name in BINARY_NAME NPM_PKG PYPI_PKG LAUNCHER_VAR; do
		[ -n "${!name}" ] || {
			echo "could not read ${name} out of ${SCRIPT}; the suite would assert nothing" >&2
			return 1
		}
	done

	# The launchers must be genuinely absent, not merely shadowed by a stub: the script decides
	# what to do by probing `command -v npx`, so a prepended stub directory cannot express "this
	# machine has no npx" and the probe would find the developer's real copy. The product binary
	# is excluded for the same reason -- a developer with it installed would otherwise take the
	# PATH branch in every test. ~keep
	xberg_shadow_system_path_without npx uvx brew curl wget "$BINARY_NAME"

	export REPO_ROOT SCRIPT BINARY_NAME NPM_PKG PYPI_PKG LAUNCHER_VAR
}

setup() {
	bats_load_library xberg-bats
	xberg_setup_isolated

	PLUGIN_ROOT="$XBERG_WORK/plugin"
	mkdir -p "$PLUGIN_ROOT"
	LAUNCH_MODE="auto"
}

# Run the script in a clean environment with the isolated PATH.
#
# `env -i` rather than merely overriding PATH: the launcher reads CLAUDE_PLUGIN_ROOT and its own
# mode variable, and a developer who happens to export either would otherwise change the result
# of the suite on their machine only.
# XBERG_TRACE is forwarded explicitly. `env -i` strips it otherwise, and a trace stub whose
# output file is unset fails silently and writes nothing -- which makes xberg_assert_trace_empty
# pass whether or not the command was called. That is a test that cannot fail, so the variable is
# part of the contract of this helper rather than something a caller remembers. ~keep
launch() {
	run env -i \
		PATH="$(xberg_isolated_path)" \
		HOME="$XBERG_WORK/home" \
		XBERG_TRACE="$XBERG_TRACE" \
		CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
		"${LAUNCHER_VAR}=${LAUNCH_MODE}" \
		"$SCRIPT" "$@"
}

# An executable that reports a version, announces itself, and echoes argv one line per argument.
#
# The unconditional `<marker>:ran` line answers "which binary was exec'd" even when the script is
# launched with no arguments; the per-argument lines answer "what argv arrived". Keeping them
# separate matters because joining argv with "$*" cannot tell `a b` from `"a b"`, which is
# exactly the pass-through bug this suite needs to be able to see. ~keep
make_cli() {
	local target="$1" marker="${2:-CLI}"
	mkdir -p "$(dirname "$target")"
	printf '%s\n' \
		'#!/usr/bin/env bash' \
		"printf '${marker}:ran\\n'" \
		"for a in \"\$@\"; do printf '${marker}[%s]\\n' \"\$a\"; done" \
		'exit 0' >"$target"
	chmod +x "$target"
}

make_npx() {
	local binary="$XBERG_WORK/npm-package/$BINARY_NAME"
	make_cli "$binary" NPX
	xberg_stub npx \
		"[[ \"\$*\" == *'$NPM_PKG@latest'* ]] || exit 1" \
		"printf '%s\\n' '$binary'"
}

# An executable that refuses to report a version, which is how the script decides a binary it
# found is not usable.
make_broken_cli() {
	local target="$1"
	mkdir -p "$(dirname "$target")"
	printf '%s\n' \
		'#!/usr/bin/env bash' \
		'if [ "$1" = "--version" ]; then exit 1; fi' \
		'printf "BROKEN:ran\n"' \
		'exit 0' >"$target"
	chmod +x "$target"
}

cached_binary() { printf '%s/bin/%s' "$PLUGIN_ROOT" "$BINARY_NAME"; }

# --- launcher selection -------------------------------------------------------------------------

@test "mcp-launch should_reject_an_unrecognised_launcher_value" {
	LAUNCH_MODE="wget-please"
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "invalid ${LAUNCHER_VAR}='wget-please'"
	xberg_assert_output_contains "auto|npx|uvx|brew|download"
}

@test "mcp-launch should_accept_every_documented_launcher_value" {
	local mode
	for mode in auto npx uvx brew download; do
		LAUNCH_MODE="$mode"
		launch
		# Each of these still fails -- nothing is installed -- but it must fail at the end, not
		# at the argument check. Asserting the absence of the validation error is what separates
		# "the mode is accepted" from "the mode happens to fail the same way". ~keep
		[[ "$output" != *"invalid ${LAUNCHER_VAR}"* ]] || {
			echo "mode '$mode' was rejected as invalid" >&2
			return 1
		}
	done
}

@test "mcp-launch should_report_installation_instructions_when_no_strategy_can_provide_a_binary" {
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "could not locate or install"
	xberg_assert_output_contains "brew install xberg-io/tap/${BINARY_NAME}"
	xberg_assert_output_contains "cargo install"
}

# --- cached and PATH binaries -------------------------------------------------------------------

@test "mcp-launch should_exec_the_cached_binary_when_it_reports_a_version" {
	make_cli "$(cached_binary)" CACHED
	launch

	xberg_assert_status 0
	[[ "$output" == *"CACHED:ran"* ]]
}

@test "mcp-launch should_pass_arguments_through_to_the_cached_binary_verbatim" {
	make_cli "$(cached_binary)" CACHED
	launch --config "a b" --flag

	xberg_assert_status 0
	# One line per argument: "a b" must survive as a single argument, not become two.
	xberg_assert_output_contains "CACHED[--config]"
	xberg_assert_output_contains "CACHED[a b]"
	xberg_assert_output_contains "CACHED[--flag]"
}

@test "mcp-launch should_ignore_the_cached_binary_when_it_cannot_report_a_version" {
	make_broken_cli "$(cached_binary)"
	launch

	xberg_assert_status 1
	[[ "$output" != *"BROKEN:ran"* ]]
}

@test "mcp-launch should_exec_the_binary_on_path_when_there_is_no_cached_binary" {
	make_cli "$XBERG_STUB_BIN/$BINARY_NAME" ONPATH
	launch

	xberg_assert_status 0
	[[ "$output" == *"ONPATH:ran"* ]]
}

@test "mcp-launch should_ignore_a_path_binary_that_cannot_report_a_version" {
	make_broken_cli "$XBERG_STUB_BIN/$BINARY_NAME"
	launch

	xberg_assert_status 1
	[[ "$output" != *"BROKEN:ran"* ]]
}

@test "mcp-launch should_prefer_the_cached_binary_over_the_one_on_path" {
	make_cli "$(cached_binary)" CACHED
	make_cli "$XBERG_STUB_BIN/$BINARY_NAME" ONPATH
	launch

	xberg_assert_status 0
	[[ "$output" == *"CACHED:ran"* ]]
	[[ "$output" != *"ONPATH:ran"* ]]
}

# --- npx, uvx and brew --------------------------------------------------------------------------

@test "mcp-launch should_launch_through_npx_when_npx_can_run_the_package" {
	make_npx
	launch --stdio

	xberg_assert_status 0
	xberg_assert_output_contains "NPX:ran"
	# The user's own argv, not just the package the script names. Asserting only the package
	# passes against a launcher that drops "$@" entirely. ~keep
	xberg_assert_output_contains "NPX[--stdio]"
}

@test "mcp-launch should_fall_through_when_the_npx_probe_fails" {
	xberg_stub_exit npx 1
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "falling through"
}

@test "mcp-launch should_launch_through_uvx_when_uvx_can_run_the_package" {
	make_cli "$XBERG_STUB_BIN/uvx" UVX
	launch --stdio

	xberg_assert_status 0
	xberg_assert_output_contains "UVX[${PYPI_PKG}]"
	xberg_assert_output_contains "UVX[--stdio]"
}

@test "mcp-launch should_not_consult_npx_when_the_launcher_is_pinned_to_uvx" {
	LAUNCH_MODE="uvx"
	xberg_stub_trace npx
	make_cli "$XBERG_STUB_BIN/uvx" UVX
	launch

	xberg_assert_status 0
	[[ "$output" == *"UVX:ran"* ]]
	# The trace file is created only by a call. Its absence is the evidence. ~keep
	xberg_assert_trace_empty
}

@test "mcp-launch should_not_consult_uvx_when_the_launcher_is_pinned_to_npx" {
	LAUNCH_MODE="npx"
	# npx must be present but UNABLE to run the package. A working npx would exec and end the
	# script before uvx was ever reachable, so the assertion would hold whether or not the gate
	# existed -- measured: removing the uvx gate left this test green until npx was made to
	# fail here. ~keep
	xberg_stub_exit npx 1
	xberg_stub_trace uvx
	launch

	xberg_assert_status 1
	xberg_assert_trace_empty
}

@test "mcp-launch should_install_through_brew_and_exec_what_it_put_on_path" {
	LAUNCH_MODE="brew"
	# A brew that actually delivers the binary, so the post-install PATH lookup has something to
	# find. Asserting only that `brew install` ran would pass against a script that never used
	# the result. ~keep
	xberg_stub brew \
		"cat >\"\$0.called\" <<<\"\$*\"" \
		"printf '%s\\n' '#!/usr/bin/env bash' 'for a in \"\$@\"; do printf \"BREWED[%s]\\\\n\" \"\$a\"; done' 'exit 0' >\"$XBERG_STUB_BIN/$BINARY_NAME\"" \
		"chmod +x \"$XBERG_STUB_BIN/$BINARY_NAME\"" \
		'exit 0'
	launch serve

	xberg_assert_status 0
	[[ "$output" == *"BREWED[serve]"* ]]
	xberg_assert_file "$XBERG_STUB_BIN/brew.called" "install xberg-io/tap/${BINARY_NAME}"
}

@test "mcp-launch should_fall_through_when_brew_install_fails" {
	LAUNCH_MODE="brew"
	xberg_stub_exit brew 1
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "falling through"
}

# --- download -----------------------------------------------------------------------------------

@test "mcp-launch should_fall_through_when_the_platform_has_no_prebuilt_archive" {
	LAUNCH_MODE="download"
	xberg_stub uname \
		'case "$1" in' \
		'  -m) printf "sparc64\n" ;;' \
		'  *) printf "Plan9\n" ;;' \
		'esac'
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "falling through"
}

@test "mcp-launch should_fall_through_when_no_downloader_is_available" {
	LAUNCH_MODE="download"
	# curl and wget are both missing from the mirrored system path, which is the state this
	# branch exists for.
	launch

	xberg_assert_status 1
	xberg_assert_output_contains "falling through"
}

# The download path stops here, deliberately.
#
# Fall-through is shared -- every launcher declines rather than proceeds when the platform has no
# published archive or no downloader exists. What happens AFTER a successful download is not
# shared, and the five have diverged along four independent axes: how the release is located (a
# `releases/latest/download` URL versus a GitHub API query for tag_name), how the asset is named,
# which architectures are published, and -- the one that matters -- what integrity means.
#
# Three postures exist today across five products shipping the same launcher:
#   * no checksum at all; warns that HTTPS is the only protection, then installs
#   * fetches a SHA256SUMS asset and REFUSES to execute a binary with no entry
#   * fetches it, and on a missing entry warns and installs anyway
#
# A single byte-identical test covering all of that would have to model each implementation, and
# the version that "passed everywhere" passed in two repos only because an earlier step failed
# first -- it would have gone on passing had those launchers installed unverified binaries
# silently. A test that cannot fail for the reason it names is worse than no test, so the
# divergence is written down here for people to settle rather than frozen into an assertion. ~keep

# --- plugin root resolution ---------------------------------------------------------------------

@test "mcp-launch should_resolve_the_plugin_root_from_its_own_location_when_the_variable_is_unset" {
	# Without CLAUDE_PLUGIN_ROOT the script derives the plugin directory from BASH_SOURCE, so the
	# cached binary it looks for is the one in the repo's own plugin/bin. Asserting the derived
	# path directly would require writing into the working tree; asserting that it does NOT use
	# the temp root is the same claim without the side effect. ~keep
	make_cli "$(cached_binary)" CACHED
	run env -i \
		PATH="$(xberg_isolated_path)" \
		HOME="$XBERG_WORK/home" \
		"${LAUNCHER_VAR}=auto" \
		"$SCRIPT"

	[[ "$output" != *"CACHED:ran"* ]]
}
