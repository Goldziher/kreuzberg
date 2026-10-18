import hashlib
import os
import re
import subprocess
import tarfile
import tempfile
from pathlib import Path

from test_publish_workflow_contracts import PHP_ABI_SCRIPT, PHP_STAGE_SCRIPT, WORKFLOW, job_block, run


def test_php_linux_artifacts_are_floor_built_and_verified() -> None:
    block = job_block(WORKFLOW.read_text(), "php-extension")
    setup_zig = block.index("Setup Zig for PHP glibc floor")
    install_zigbuild = block.index("Install verified cargo-zigbuild for GNU targets")
    setup_ort = block.index("Setup ONNX Runtime (Linux)")
    first_build = block.index("xberg-io/actions/build-php-extension@v1")
    rebuild = block.index("Rebuild PHP extension at the glibc floor")
    package = block.index("xberg-io/actions/package-php-pie@v1")
    stage_ort = block.index("scripts/ci/stage-php-linux-ort.sh")
    verify = block.index("scripts/ci/verify-php-linux-abi.sh")
    smoke = block.index("scripts/ci/smoke-test-php-debian12.sh")
    upload = block.index("actions/upload-artifact@v7", verify)

    assert (
        setup_zig < setup_ort < first_build < install_zigbuild < rebuild < package < stage_ort < verify < smoke < upload
    )
    ort_step = block[setup_ort : block.index("xberg-io/actions/build-php-extension@v1")]
    assert "if: contains(matrix.platform.target, '-linux-gnu')" in ort_step
    assert "dest-dir: crates/xberg-php" in ort_step
    assert 'version: "0.13.0"' in block
    assert 'TARGET_ARG="${TARGET}.${GLIBC_FLOOR}"' in block
    assert 'cargo zigbuild --locked -p xberg-php --release --target "${TARGET_ARG}"' in block
    assert "contains(matrix.platform.target, '-linux-gnu')" in block
    assert "EXPECTED_XBERG_SHA256: ${{ steps.php_floor_build.outputs.sha256 }}" in block
    assert "cargo-features:" not in block
    assert "ref: ${{ github.workflow_sha }}" in block
    assert "path: .release-tools" in block
    assert 'bash "$GITHUB_WORKSPACE/.release-tools/scripts/ci/verify-php-linux-abi.sh"' in block
    assert 'bash "$GITHUB_WORKSPACE/.release-tools/scripts/ci/smoke-test-php-debian12.sh"' in block


def test_php_release_requires_every_supported_php_version_on_every_platform() -> None:
    block = job_block(WORKFLOW.read_text(), "verify-assets")
    expected = {
        f"php_xberg-*{php_suffix}-{platform}"
        for version in ("8.3", "8.4", "8.5")
        for php_suffix, platform in (
            (f"_php{version}", "x86_64-linux-glibc-nts.tgz"),
            (f"_php{version}", "arm64-linux-glibc-nts.tgz"),
            (f"_php{version}", "arm64-darwin-bsdlibc-nts.tgz"),
            (f"-{version}", "nts-vs17-x86_64.zip"),
        )
    }
    actual = set(re.findall(r"(?m)^            (php_xberg-\S+)$", block))
    assert len(expected) == 12
    assert actual == expected, f"missing PHP release requirements: {sorted(expected - actual)}"


FAKE_PHP_OBJDUMP = (
    "#!/bin/sh\n"
    'if [ "$1" = "-p" ]; then\n'
    '  case "${PHP_ABI_FIXTURE:?}" in\n'
    "    static) printf '%s\\n' '  NEEDED       libc.so.6' ;;\n"
    "    nodelete) printf '%s\\n' '  NEEDED       libonnxruntime.so.1' '  RUNPATH      $ORIGIN' ;;\n"
    "    norpath) printf '%s\\n' '  NEEDED       libonnxruntime.so.1' ;;\n"
    "    numeric:*) printf '%s\\n' '  NEEDED       libonnxruntime.so.1' '  RUNPATH      $ORIGIN'"
    '      "  FLAGS_1              ${PHP_ABI_FIXTURE#numeric:}" ;;\n'
    "    *) printf '%s\\n' '  NEEDED       libonnxruntime.so.1' '  RUNPATH      $ORIGIN' '  FLAGS_1      Flags: NOW NODELETE' ;;\n"
    "  esac\n"
    "  exit 0\n"
    "fi\n"
    'case "${PHP_ABI_FIXTURE:?}" in\n'
    "  unsafe) printf '%s\\n' '000 GLIBC_2.39 symbol' '000 GLIBCXX_3.4.31 symbol' '000 CXXABI_1.3.15 symbol' ;;\n"
    "  unresolved) printf '%s\\n' '000 GLIBC_2.35 symbol'"
    " '0000000000000000      D  *UND*\t0000000000000000  Base        _ZNSt7__cxx1112basic_stringIcSt11char_traitsIcESaIcEE15_M_replace_coldEPcmPKcmm'"
    " '0000000000000000  w   D  *UND*\t0000000000000000  Base        _ZTVN10__cxxabiv117__class_type_infoE'"
    " '0000000000000000      DF *UND*\t0000000000000000 (GLIBC_2.2.5) strlen' ;;\n"
    "  *) printf '%s\\n' '000 GLIBC_2.35 symbol' '000 GLIBCXX_3.4.30 symbol' '000 CXXABI_1.3.13 symbol' ;;\n"
    "esac\n"
)


def php_fixture_archive(root: Path, ort_members: tuple[str, ...]) -> Path:
    payload = root / "payload"
    payload.mkdir()
    (payload / "xberg.so").write_bytes(b"ELF fixture")
    archive = root / "php_xberg.tgz"
    with tarfile.open(archive, "w:gz") as handle:
        handle.add(payload / "xberg.so", arcname="xberg.so")
        for member in ort_members:
            (payload / member).write_bytes(b"ELF onnxruntime fixture")
            handle.add(payload / member, arcname=member)
    return archive


def php_fixture_env(root: Path, profile: str) -> dict[str, str]:
    bin_dir = root / "bin"
    bin_dir.mkdir()
    objdump = bin_dir / "objdump"
    objdump.write_text(FAKE_PHP_OBJDUMP)
    objdump.chmod(0o755)
    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}{os.pathsep}{env['PATH']}"
    env["PHP_ABI_FIXTURE"] = profile
    return env


def run_php_abi_fixture(
    profile: str,
    expected_digest: str | None = None,
    ort_members: tuple[str, ...] = ("libonnxruntime.so.1",),
) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        archive = php_fixture_archive(root, ort_members)
        env = php_fixture_env(root, profile)
        env["EXPECTED_XBERG_SHA256"] = expected_digest or hashlib.sha256(b"ELF fixture").hexdigest()
        return run(["bash", str(PHP_ABI_SCRIPT), str(archive), "2.36", "3.4.30", "1.3.13"], root, env)


def test_php_linux_abi_gate_accepts_debian_12_compatible_symbols() -> None:
    result = run_php_abi_fixture("safe")
    assert result.returncode == 0, result.stderr
    assert "max GLIBC 2.35 <= 2.36" in result.stderr
    assert "max GLIBCXX 3.4.30 <= 3.4.30" in result.stderr
    assert "max CXXABI 1.3.13 <= 1.3.13" in result.stderr
    assert "libonnxruntime.so.1 max GLIBC 2.35 <= 2.36" in result.stderr
    assert "packaged xberg.so matches the floor-built artifact" in result.stderr


def test_php_linux_abi_gate_rejects_both_symbol_floor_regressions() -> None:
    result = run_php_abi_fixture("unsafe")
    assert result.returncode != 0
    assert "requires GLIBC 2.39 > 2.36" in result.stderr
    assert "requires GLIBCXX 3.4.31 > 3.4.30" in result.stderr
    assert "requires CXXABI 1.3.15 > 1.3.13" in result.stderr
    assert "libonnxruntime.so.1 requires GLIBC 2.39 > 2.36" in result.stderr


def test_php_linux_abi_gate_rejects_unversioned_undefined_cxx_symbols() -> None:
    result = run_php_abi_fixture("unresolved")
    assert result.returncode != 0
    assert "leaves 1 C++ symbols undefined and unversioned" in result.stderr
    assert "_M_replace_coldEPcmPKcmm" in result.stderr
    assert "_ZTVN10__cxxabiv117__class_type_infoE" not in result.stderr


def test_php_linux_abi_gate_reports_no_unresolved_cxx_symbols_when_clean() -> None:
    result = run_php_abi_fixture("safe")
    assert result.returncode == 0, result.stderr
    assert "no unresolved C++ symbols" in result.stderr


def test_php_linux_abi_gate_rejects_a_different_packaged_binary() -> None:
    result = run_php_abi_fixture("safe", "0" * 64)
    assert result.returncode != 0
    assert "packaged xberg.so does not match the floor-built artifact" in result.stderr


def test_php_linux_abi_gate_requires_dynamic_onnxruntime_shipped_beside_the_extension() -> None:
    statically_linked = run_php_abi_fixture("static")
    assert statically_linked.returncode != 0
    assert "does not link libonnxruntime dynamically" in statically_linked.stderr

    not_shipped = run_php_abi_fixture("safe", ort_members=())
    assert not_shipped.returncode != 0
    assert "libonnxruntime.so.1 is not shipped beside xberg.so" in not_shipped.stderr

    no_rpath = run_php_abi_fixture("norpath")
    assert no_rpath.returncode != 0
    assert "has no $ORIGIN rpath" in no_rpath.stderr


def test_php_linux_abi_gate_rejects_extensions_unmapped_before_atexit() -> None:
    result = run_php_abi_fixture("nodelete")
    assert result.returncode != 0
    assert "xberg.so is not NODELETE" in result.stderr
    assert "SIGSEGV" in result.stderr


def test_php_linux_abi_gate_reads_gnu_objdump_numeric_nodelete_flags() -> None:
    for flags in ("0x0000000000000008", "0x9", "0x8000009"):
        result = run_php_abi_fixture(f"numeric:{flags}")
        assert result.returncode == 0, (flags, result.stderr)
        assert "xberg.so is NODELETE" in result.stderr
    for flags in ("0x1", "0x0", "", "0x9$(touch injected)", "0x10000000000000008"):
        result = run_php_abi_fixture(f"numeric:{flags}")
        assert result.returncode != 0, (flags, result.stderr)
        assert "xberg.so is not NODELETE" in result.stderr


def test_php_smoke_gate_preserves_load_and_teardown_failure_status() -> None:
    source = (WORKFLOW.parents[2] / "scripts/ci/smoke-test-php-debian12.sh").read_text()
    start = re.search(r"(?m)^    (?:if ! )?php ", source)
    assert start is not None
    fragment = source[start.start() : source.rindex("\n  '")]
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        for name, command in (("apt-get", ":"), ("gdb", "echo diagnostic >&2")):
            stub = root / name
            stub.write_text(f"#!/bin/sh\n{command}\n")
            stub.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = f"{root}{os.pathsep}{env['PATH']}"
        for status in (0, 1, 139):
            functions = f"php() {{ return {status}; }}\nextension=fixture.so\n"
            result = run(["sh", "-euc", functions + fragment], root, env)
            assert result.returncode == status, (status, result.returncode, result.stderr)
            assert ("diagnostic" in result.stderr) == (status != 0)


def run_php_stage_fixture(
    profile: str, ort_files: tuple[str, ...]
) -> tuple[subprocess.CompletedProcess[str], list[str], str]:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        archive = php_fixture_archive(root, ())
        env = php_fixture_env(root, profile)
        ort_dir = root / "ort"
        ort_dir.mkdir()
        for name in ort_files:
            (ort_dir / name).write_bytes(b"ELF onnxruntime fixture")
        result = run(["bash", str(PHP_STAGE_SCRIPT), str(archive), str(ort_dir)], root, env)
        with tarfile.open(archive) as handle:
            members = sorted(handle.getnames())
        sidecar = Path(f"{archive}.sha256")
        sidecar_text = sidecar.read_text() if sidecar.exists() else ""
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        if result.returncode == 0:
            assert sidecar_text == f"{digest}  php_xberg.tgz\n"
        return result, members, sidecar_text


def test_php_stage_script_adds_the_needed_onnxruntime_beside_the_extension() -> None:
    result, members, _ = run_php_stage_fixture("safe", ("libonnxruntime.so.1.24.2", "libonnxruntime.so"))
    assert result.returncode == 0, result.stderr
    assert members == ["libonnxruntime.so.1", "xberg.so"]


def test_php_stage_script_refuses_a_statically_linked_extension() -> None:
    result, members, _ = run_php_stage_fixture("static", ("libonnxruntime.so.1",))
    assert result.returncode != 0
    assert "does not link libonnxruntime dynamically" in result.stderr
    assert members == ["xberg.so"]


def test_php_stage_script_fails_when_the_runtime_is_missing() -> None:
    result, members, _ = run_php_stage_fixture("safe", ())
    assert result.returncode != 0
    assert "no libonnxruntime.so.1 under" in result.stderr
    assert members == ["xberg.so"]


if __name__ == "__main__":
    test_php_linux_artifacts_are_floor_built_and_verified()
    test_php_release_requires_every_supported_php_version_on_every_platform()
    test_php_linux_abi_gate_accepts_debian_12_compatible_symbols()
    test_php_linux_abi_gate_rejects_both_symbol_floor_regressions()
    test_php_linux_abi_gate_rejects_unversioned_undefined_cxx_symbols()
    test_php_linux_abi_gate_reports_no_unresolved_cxx_symbols_when_clean()
    test_php_linux_abi_gate_rejects_a_different_packaged_binary()
    test_php_linux_abi_gate_requires_dynamic_onnxruntime_shipped_beside_the_extension()
    test_php_linux_abi_gate_rejects_extensions_unmapped_before_atexit()
    test_php_linux_abi_gate_reads_gnu_objdump_numeric_nodelete_flags()
    test_php_smoke_gate_preserves_load_and_teardown_failure_status()
    test_php_stage_script_adds_the_needed_onnxruntime_beside_the_extension()
    test_php_stage_script_refuses_a_statically_linked_extension()
    test_php_stage_script_fails_when_the_runtime_is_missing()
