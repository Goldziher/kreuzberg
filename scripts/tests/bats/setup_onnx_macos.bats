#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	SCRIPT="$REPO_ROOT/scripts/ci/actions/setup-onnx-runtime/macos.sh"
	export RUNNER_TEMP="$XBERG_WORK/runner" GITHUB_WORKSPACE="$XBERG_WORK/workspace"
	export RUSTFLAGS='' DYLD_LIBRARY_PATH='/ambient/dyld'
	export DYLD_FALLBACK_LIBRARY_PATH='/ambient/fallback' LIBRARY_PATH='/ambient/lib'
	ORT_ROOT="$RUNNER_TEMP/onnxruntime/onnxruntime-osx-arm64-1.24.2"
	DEST="$GITHUB_WORKSPACE/native"
	mkdir -p "$ORT_ROOT/lib" "$GITHUB_WORKSPACE"
	printf 'cached-ort-fixture\n' >"$ORT_ROOT/lib/libonnxruntime.1.24.2.dylib"
	xberg_stub curl 'printf "curl\n" >>"$XBERG_TRACE"; exit 99'
	xberg_stub tar 'printf "tar\n" >>"$XBERG_TRACE"; exit 99'
}

assert_system_environment() {
	local flags="$1"
	cat >"$XBERG_WORK/expected-env" <<EXPECTED
ORT_LIB_LOCATION=$ORT_ROOT/lib
ORT_PREFER_DYNAMIC_LINK=1
ORT_SKIP_DOWNLOAD=1
ORT_STRATEGY=system
ORT_DYLIB_PATH=$ORT_ROOT/lib/libonnxruntime.1.24.2.dylib
DYLD_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/dyld
DYLD_FALLBACK_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/fallback
LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/lib
RUSTFLAGS=$flags
EXPECTED
	[ "$(wc -l <"$GITHUB_ENV" | tr -d ' ')" -eq 9 ]
	cmp "$XBERG_WORK/expected-env" "$GITHUB_ENV"
	cmp "$ORT_ROOT/lib/libonnxruntime.1.24.2.dylib" "$DEST/libonnxruntime.1.24.2.dylib"
	[ ! -s "$XBERG_TRACE" ]
}

assert_bundled_environment() {
	cat >"$XBERG_WORK/expected-env" <<EXPECTED
DYLD_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/dyld
DYLD_FALLBACK_LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/fallback
LIBRARY_PATH=$ORT_ROOT/lib:$DEST:/ambient/lib
EXPECTED
	[ "$(wc -l <"$GITHUB_ENV" | tr -d ' ')" -eq 3 ]
	cmp "$XBERG_WORK/expected-env" "$GITHUB_ENV"
	cmp "$ORT_ROOT/lib/libonnxruntime.1.24.2.dylib" "$DEST/libonnxruntime.1.24.2.dylib"
	[ ! -s "$XBERG_TRACE" ]
}

@test "setup_onnx_macos should_keep_default_rpath_and_all_system_environment_rows" {
	run bash "$SCRIPT" 1.24.2 native arm64 system
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_accept_explicit_true_rpath" {
	run bash "$SCRIPT" 1.24.2 native arm64 system true
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_preserve_ambient_flags_with_default_rpath" {
	export RUSTFLAGS='-D warnings -C debuginfo=1'
	run bash "$SCRIPT" 1.24.2 native arm64 system
	xberg_assert_status 0
	assert_system_environment "$RUSTFLAGS -L $ORT_ROOT/lib -C link-arg=-Wl,-rpath,@loader_path"
}

@test "setup_onnx_macos should_omit_setup_rpath_when_the_package_owns_it" {
	run bash "$SCRIPT" 1.24.2 native arm64 system false
	xberg_assert_status 0
	assert_system_environment "-L $ORT_ROOT/lib"
}

@test "setup_onnx_macos should_preserve_ambient_flags_and_ort_search_when_rpath_is_disabled" {
	export RUSTFLAGS='-D warnings -C debuginfo=1'
	run bash "$SCRIPT" 1.24.2 native arm64 system false
	xberg_assert_status 0
	assert_system_environment "$RUSTFLAGS -L $ORT_ROOT/lib"
}

@test "setup_onnx_macos should_preserve_default_bundled_environment" {
	run bash "$SCRIPT" 1.24.2 native arm64 bundled
	xberg_assert_status 0
	assert_bundled_environment
}

@test "setup_onnx_macos should_preserve_bundled_environment_when_rpath_is_disabled" {
	export RUSTFLAGS='-D warnings'
	run bash "$SCRIPT" 1.24.2 native arm64 bundled false
	xberg_assert_status 0
	assert_bundled_environment
}

assert_invalid_rpath() {
	run bash "$SCRIPT" 1.24.2 native arm64 system "$1"
	xberg_assert_status 2
	xberg_assert_output_contains 'add-macos-rpath must be true or false'
	[ ! -e "$DEST" ]
	[ ! -s "$GITHUB_ENV" ]
	[ ! -s "$XBERG_TRACE" ]
}

@test "setup_onnx_macos should_reject_invalid_rpath_before_staging" {
	assert_invalid_rpath unexpected
}

@test "setup_onnx_macos should_reject_explicit_empty_rpath_before_staging" {
	assert_invalid_rpath ''
}

@test "setup_onnx_macos should_reject_case_variant_rpath_before_staging" {
	assert_invalid_rpath TRUE
}
