"""Exercise the real build script's iOS compiler-runtime linkage contract."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FAKE_XCRUN = r"""#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

args = sys.argv[1:]
with Path(os.environ["IOS_TEST_CALLS"]).open("a") as stream:
    stream.write(json.dumps(args) + "\n")
if args[:1] == ["--sdk"]:
    if os.environ.get("IOS_TEST_QUERY_FAILURE"):
        sys.exit("compiler resource unavailable")
    if os.environ.get("IOS_TEST_NON_UTF8"):
        sys.stdout.buffer.write(b"\xff")
        sys.exit(0)
    print(os.environ.get("IOS_TEST_RESOURCE", ""))
elif args[:1] == ["lipo"]:
    if os.environ.get("IOS_TEST_SLICE_FAILURE"):
        sys.exit("requested architecture unavailable")
    Path(args[-1]).write_bytes(b"!<arch>\n" + args[args.index("-thin") + 1].encode())
else:
    sys.exit("unexpected xcrun operation")
"""


class IosCompilerRuntimeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.build_directory = tempfile.TemporaryDirectory(prefix="xberg-build-script-")
        cls.build_script = Path(cls.build_directory.name) / "build-script"
        rustc = shutil.which("rustc")
        if rustc is None:
            raise RuntimeError("rustc is required to verify the actual build script")
        subprocess.run(
            [rustc, "--edition=2024", str(ROOT / "crates/xberg/build.rs"), "-o", str(cls.build_script)],
            check=True,
            capture_output=True,
            text=True,
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls.build_directory.cleanup()

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory(prefix="xberg ios runtime ")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        xcrun = self.bin / "xcrun"
        xcrun.write_text(FAKE_XCRUN)
        xcrun.chmod(0o755)
        self.resource = self.root / "clang resources"
        self.runtime = self.resource / "lib/darwin"
        self.runtime.mkdir(parents=True)
        for name in ["ios", "iossim"]:
            (self.runtime / f"libclang_rt.{name}.a").write_bytes(b"\xca\xfe\xba\xbe\x00\x00\x00\x02")
        self.out = self.root / "cargo output"
        self.out.mkdir()
        self.calls = self.root / "xcrun-calls.jsonl"

    def run_build(self, **overrides: str | None) -> subprocess.CompletedProcess[str]:
        env = {key: value for key, value in os.environ.items() if not key.startswith("CARGO_")}
        env.update(
            PATH=f"{self.bin}{os.pathsep}{env['PATH']}",
            CARGO_CFG_TARGET_OS="ios",
            CARGO_CFG_TARGET_ABI="",
            CARGO_CFG_TARGET_ARCH="aarch64",
            CARGO_FEATURE_ORT_BUNDLED="1",
            OUT_DIR=str(self.out),
            XBERG_BUILD_ID="ios-runtime-test",
            IOS_TEST_CALLS=str(self.calls),
            IOS_TEST_RESOURCE=str(self.resource),
        )
        for key, value in overrides.items():
            if value is None:
                env.pop(key, None)
            else:
                env[key] = value
        return subprocess.run([str(self.build_script)], env=env, capture_output=True, text=True, check=False)

    def recorded_calls(self) -> list[list[str]]:
        return [json.loads(line) for line in self.calls.read_text().splitlines()] if self.calls.exists() else []

    def assert_runtime(self, abi: str, arch: str, sdk: str, library: str, apple_arch: str) -> None:
        result = self.run_build(CARGO_CFG_TARGET_ABI=abi, CARGO_CFG_TARGET_ARCH=arch)
        assert result.returncode == 0, result.stderr
        assert f"cargo::rustc-link-lib=static={library}\n" in result.stdout
        assert f"cargo::rustc-link-search=native={self.out}\n" in result.stdout
        assert (self.out / f"lib{library}.a").read_bytes() == b"!<arch>\n" + apple_arch.encode()
        calls = self.recorded_calls()
        assert calls[0] == ["--sdk", sdk, "clang", "--print-resource-dir"]
        assert calls[1] == [
            "lipo",
            str(self.runtime / f"lib{library}.a"),
            "-thin",
            apple_arch,
            "-output",
            str(self.out / f"lib{library}.a"),
        ]
        assert len(calls) == 2
        assert f"cargo::rerun-if-changed={self.runtime / f'lib{library}.a'}\n" in result.stdout
        for variable in ["DEVELOPER_DIR", "TOOLCHAINS", "SDKROOT"]:
            assert f"cargo::rerun-if-env-changed={variable}\n" in result.stdout

    def test_device_runtime_is_bundled_for_arm64(self) -> None:
        self.assert_runtime("", "aarch64", "iphoneos", "clang_rt.ios", "arm64")

    def test_arm64_simulator_uses_simulator_runtime(self) -> None:
        self.assert_runtime("sim", "aarch64", "iphonesimulator", "clang_rt.iossim", "arm64")

    def test_x86_64_simulator_uses_matching_architecture(self) -> None:
        self.assert_runtime("sim", "x86_64", "iphonesimulator", "clang_rt.iossim", "x86_64")

    def test_already_thin_archive_is_copied_without_lipo(self) -> None:
        archive = self.runtime / "libclang_rt.ios.a"
        archive.write_bytes(b"!<arch>\nexisting object archive")
        result = self.run_build()
        assert result.returncode == 0, result.stderr
        assert (self.out / archive.name).is_file(), "the native runtime must be staged for bundling"
        assert (self.out / archive.name).read_bytes() == archive.read_bytes()
        assert len(self.recorded_calls()) == 1

    def test_other_targets_and_nonbundled_ort_do_not_query_xcode(self) -> None:
        cases = [
            {"CARGO_CFG_TARGET_OS": "macos"},
            {"CARGO_CFG_TARGET_OS": "linux"},
            {"CARGO_CFG_TARGET_OS": "android"},
            {"CARGO_FEATURE_ORT_BUNDLED": None},
            {"CARGO_FEATURE_ORT_BUNDLED": None, "CARGO_FEATURE_ORT_DYNAMIC": "1"},
        ]
        for overrides in cases:
            with self.subTest(overrides=overrides):
                result = self.run_build(**overrides)
                assert result.returncode == 0, result.stderr
                assert "cargo::rustc-link-" not in result.stdout
        assert self.recorded_calls() == []

    def test_compiler_query_failure_names_the_sdk_and_operation(self) -> None:
        result = self.run_build(IOS_TEST_QUERY_FAILURE="1")
        assert result.returncode != 0
        assert "iphoneos compiler runtime directory" in result.stderr
        assert "compiler resource unavailable" in result.stderr

    def test_empty_compiler_resource_directory_is_rejected(self) -> None:
        result = self.run_build(IOS_TEST_RESOURCE="")
        assert result.returncode != 0
        assert "empty iphoneos compiler runtime directory" in result.stderr

    def test_invalid_compiler_resource_encoding_names_the_sdk(self) -> None:
        result = self.run_build(IOS_TEST_NON_UTF8="1")
        assert result.returncode != 0
        assert "decode iphoneos compiler runtime directory" in result.stderr

    def test_missing_runtime_archive_is_rejected(self) -> None:
        (self.runtime / "libclang_rt.ios.a").unlink()
        result = self.run_build()
        assert result.returncode != 0
        assert "read iphoneos compiler runtime archive" in result.stderr

    def test_architecture_extraction_failure_is_reported(self) -> None:
        result = self.run_build(IOS_TEST_SLICE_FAILURE="1")
        assert result.returncode != 0
        assert "extract arm64 from iphoneos compiler runtime" in result.stderr
        assert "requested architecture unavailable" in result.stderr


if __name__ == "__main__":
    unittest.main()
