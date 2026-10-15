setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	INSTALL_SCRIPT="$REPO_ROOT/scripts/ci/install-system-deps/install-linux.sh"
	export LIBHEIF_PREFIX="$XBERG_WORK/libheif" LIBHEIF_VERSION=1.19.8
	export PKG_CONFIG_LIBDIR="$LIBHEIF_PREFIX/lib/pkgconfig" PKG_CONFIG_PATH=
	mkdir -p "$PKG_CONFIG_LIBDIR"
	cat >"$PKG_CONFIG_LIBDIR/libheif.pc" <<'PC'
Name: libheif
Description: cached libheif requiring an external development package
Version: 1.19.8
Requires: libsharpyuv
Libs: -lheif
Cflags: -I/example/heif
PC
	# Execute the actual preflight without installing packages or rebuilding libheif. ~keep
	awk '/^resolved_libheif_version=/{capture=1} /^echo "libheif symbol verification:/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/validate-libheif.sh"
	[ "$(grep -c '^resolved_libheif_version=' "$XBERG_WORK/validate-libheif.sh")" -eq 1 ]
}

@test "install_linux should_reject_cached_libheif_when_required_pkg_config_dependency_is_missing" {
	run pkg-config --modversion libheif
	xberg_assert_status 0
	xberg_assert_output "1.19.8"
	run pkg-config --cflags --libs libheif
	xberg_assert_status 1
	xberg_assert_output_contains "libsharpyuv"

	run bash -euo pipefail "$XBERG_WORK/validate-libheif.sh"
	xberg_assert_status 1
	xberg_assert_output_contains "libheif"
}

@test "install_linux should_accept_cached_libheif_when_required_pkg_config_dependency_is_available" {
	cat >"$PKG_CONFIG_LIBDIR/libsharpyuv.pc" <<'PC'
Name: libsharpyuv
Description: complete dependency fixture
Version: 0.2.0
Libs: -lsharpyuv
PC
	run pkg-config --cflags --libs libheif
	xberg_assert_status 0
	local flags
	read -r -a flags <<<"$output"
	[ "${flags[*]}" = "-I/example/heif -lheif -lsharpyuv" ]

	run bash -euo pipefail "$XBERG_WORK/validate-libheif.sh"
	xberg_assert_status 0
	xberg_assert_output_contains "pkg-config resolves libheif 1.19.8"
}

@test "install_linux should_install_sharpyuv_development_metadata_for_cached_libheif" {
	awk '/^packages=\(/{capture=1} /^echo "Installing dependencies/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/packages.sh"
	[ "$(grep -c '^packages=(' "$XBERG_WORK/packages.sh")" -eq 1 ]
	xberg_stub_exit apt-cache 0
	run bash -euo pipefail -c 'source "$1"; printf "%s\n" "${packages[@]}" | grep -cx libsharpyuv-dev' \
		_ "$XBERG_WORK/packages.sh"
	xberg_assert_status 0
	xberg_assert_output "1"
}

@test "install_linux should_preserve_older_distribution_packages_when_sharpyuv_development_package_is_unavailable" {
	awk '/^packages=\(/{capture=1} /^echo "Installing dependencies/{capture=0} capture' \
		"$INSTALL_SCRIPT" >"$XBERG_WORK/packages.sh"
	[ "$(grep -c '^packages=(' "$XBERG_WORK/packages.sh")" -eq 1 ]
	xberg_stub_exit apt-cache 1
	run bash -euo pipefail -c 'source "$1"; printf "%s\n" "${packages[@]}" | grep -cx libsharpyuv-dev' \
		_ "$XBERG_WORK/packages.sh"
	xberg_assert_status 1
	xberg_assert_output "0"
}
