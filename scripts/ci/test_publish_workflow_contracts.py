import ast
import hashlib
import os
import re
import shutil
import subprocess
import tarfile
import tempfile
import textwrap
from pathlib import Path

WORKFLOW = Path(__file__).parents[2] / ".github" / "workflows" / "publish.yaml"
GO_SETUP = Path(__file__).parents[2] / "packages" / "go" / "cmd" / "setup" / "main.go"
GO_RELEASE_SCRIPT = Path(__file__).parent / "package-go-release.sh"
GO_MUSL_RELEASE_SCRIPT = Path(__file__).parent / "package-go-musl-release.sh"
DOCKER_WORKFLOW = Path(__file__).parents[2] / ".github" / "workflows" / "publish-docker.yaml"
PUBDEV_WORKFLOW = Path(__file__).parents[2] / ".github" / "workflows" / "publish-pubdev.yaml"
CI_WORKFLOW = Path(__file__).parents[2] / ".github" / "workflows" / "ci-lint.yaml"
RUST_UNIT_SCRIPT = Path(__file__).parent / "rust" / "run-unit-tests.sh"
PHP_ABI_SCRIPT = Path(__file__).parent / "verify-php-linux-abi.sh"
PUBLISHER_BOT = "xberg-dev-publisher[bot]"
PUBLISH_PUB_SHA = "a25ae95253ee755ac5f691f7e1053dcb104cdee7"


def job_block(workflow: str, job: str) -> str:
    match = re.search(rf"(?ms)^  {re.escape(job)}:\n(.*?)(?=^  [a-zA-Z0-9_-]+:\n|\Z)", workflow)
    if match is None:
        raise AssertionError(f"publish workflow has no {job!r} job")
    return match.group(1)


def job_if_expression(block: str) -> str:
    lines = block.splitlines()
    for index, line in enumerate(lines):
        if line == "    if: >-":
            expression_lines = []
            for continuation in lines[index + 1 :]:
                if not continuation.startswith("      "):
                    break
                expression_lines.append(continuation.strip())
            expression = " ".join(expression_lines)
            assert expression.startswith("${{") and expression.endswith("}}")
            return expression.removeprefix("${{").removesuffix("}}").strip()
    raise AssertionError("job has no folded if expression")


def attribute_value(node: ast.Attribute, context: dict[str, object]) -> object:
    parts: list[str] = []
    current: ast.expr = node
    while isinstance(current, ast.Attribute):
        parts.append(current.attr)
        current = current.value
    if not isinstance(current, ast.Name):
        raise AssertionError(f"unsupported condition attribute: {ast.dump(node)}")
    parts.append(current.id)

    value: object = context
    for part in reversed(parts):
        assert isinstance(value, dict) and part in value, part
        value = value[part]
    return value


def evaluate_condition(expression: str, context: dict[str, object]) -> bool:
    translated = expression.replace("||", "or").replace("&&", "and").replace("startsWith", "starts_with")

    def evaluate(node: ast.expr) -> object:
        if isinstance(node, ast.BoolOp):
            values = [bool(evaluate(value)) for value in node.values]
            if isinstance(node.op, ast.Or):
                return any(values)
            if isinstance(node.op, ast.And):
                return all(values)
        elif isinstance(node, ast.Compare) and len(node.ops) == len(node.comparators) == 1:
            left = evaluate(node.left)
            right = evaluate(node.comparators[0])
            if isinstance(node.ops[0], ast.NotEq):
                return left != right
        elif isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "starts_with":
            assert len(node.args) == 2 and not node.keywords
            return str(evaluate(node.args[0])).startswith(str(evaluate(node.args[1])))
        elif isinstance(node, ast.Attribute):
            return attribute_value(node, context)
        elif isinstance(node, ast.Constant) and isinstance(node.value, str):
            return node.value
        raise AssertionError(f"unsupported condition syntax: {ast.dump(node)}")

    parsed = ast.parse(translated, mode="eval")
    return bool(evaluate(parsed.body))


def github_context(event_name: str, tag: str, actor: str, triggering_actor: str) -> dict[str, object]:
    return {
        "github": {
            "event_name": event_name,
            "actor": actor,
            "triggering_actor": triggering_actor,
            "event": {"release": {"tag_name": tag}},
        }
    }


def step_script(block: str, step: str) -> str:
    marker = f"      - name: {step}\n"
    step_start = block.find(marker)
    if step_start < 0:
        raise AssertionError(f"job has no shell step named {step!r}")
    run_marker = "        run: |\n"
    script_start = block.find(run_marker, step_start)
    if script_start < 0:
        raise AssertionError(f"step {step!r} has no run block")
    script_start += len(run_marker)
    script_end = block.find("        shell: bash", script_start)
    if script_end < 0:
        raise AssertionError(f"step {step!r} has no bash shell declaration")
    script = textwrap.dedent(block[script_start:script_end])
    return re.sub(r"(?m)^(\s*)sleep \$\(\(attempt \* 3\)\)$", r"\1:", script)


def run(command: list[str], cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, env=env, text=True, capture_output=True, check=False)


def git(cwd: Path, *args: str) -> subprocess.CompletedProcess[str]:
    result = run(["git", *args], cwd)
    assert result.returncode == 0, result.stderr
    return result


def create_tap_repositories(root: Path) -> tuple[Path, Path, Path]:
    origin = root / "tap.git"
    seed = root / "seed"
    runner = root / "runner"
    competitor = root / "competitor"

    git(root, "init", "--bare", "--initial-branch=main", str(origin))
    git(root, "init", "--initial-branch=main", str(seed))
    (seed / "Formula").mkdir()
    (seed / "Formula" / "xberg.rb").write_text("version 1.0.0\n")
    git(seed, "add", "Formula/xberg.rb")
    git(
        seed,
        "-c",
        "user.name=seed",
        "-c",
        "user.email=seed@example.com",
        "commit",
        "-m",
        "seed",
    )
    git(seed, "remote", "add", "origin", str(origin))
    git(seed, "push", "-u", "origin", "main")
    git(root, "clone", str(origin), str(runner))
    git(root, "clone", str(origin), str(competitor))
    return origin, runner, competitor


def run_homebrew_push_script(
    job: str,
    runner: Path,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    step = "Commit and push to tap" if job == "publish-homebrew-formula" else "Commit and push bottle DSL"
    script = step_script(job_block(WORKFLOW.read_text(), job), step)
    script = script.replace("${{ needs.prepare.outputs.version }}", "1.1.0")
    return run(["bash", "-euo", "pipefail", "-c", script], runner, env)


def test_release_event_guards_cover_initial_actor_and_tag_policy() -> None:
    publish_prepare = job_block(WORKFLOW.read_text(), "prepare")
    publish_expression = job_if_expression(publish_prepare)
    assert publish_expression == (
        "github.event_name != 'release' || (startsWith(github.event.release.tag_name, 'v') "
        "&& github.actor != 'xberg-dev-publisher[bot]')"
    )

    cases = (
        ("workflow_dispatch", "", PUBLISHER_BOT, PUBLISHER_BOT, True),
        ("repository_dispatch", "", PUBLISHER_BOT, PUBLISHER_BOT, True),
        ("release", "v1.1.0", "Goldziher", "Goldziher", True),
        ("release", "benchmark-run-123", "Goldziher", "Goldziher", False),
        ("release", "v1.1.0", PUBLISHER_BOT, PUBLISHER_BOT, False),
        # A human rerun changes triggering_actor, not the actor that originated the event. ~keep
        ("release", "v1.1.0", PUBLISHER_BOT, "Goldziher", False),
    )
    for event_name, tag, actor, triggering_actor, expected in cases:
        context = github_context(event_name, tag, actor, triggering_actor)
        assert evaluate_condition(publish_expression, context) is expected

    docker_prepare = job_block(DOCKER_WORKFLOW.read_text(), "prepare")
    docker_expression = job_if_expression(docker_prepare)
    assert docker_expression == ("github.event_name != 'release' || startsWith(github.event.release.tag_name, 'v')")
    docker_cases = (
        ("workflow_dispatch", "", True),
        ("repository_dispatch", "", True),
        ("release", "v1.1.0", True),
        ("release", "benchmark-run-123", False),
    )
    for event_name, tag, expected in docker_cases:
        context = github_context(event_name, tag, PUBLISHER_BOT, PUBLISHER_BOT)
        assert evaluate_condition(docker_expression, context) is expected

    dispatch = github_context("workflow_dispatch", "", PUBLISHER_BOT, PUBLISHER_BOT)
    assert not evaluate_condition(publish_expression.replace("||", "&&", 1), dispatch)
    assert not evaluate_condition(docker_expression.replace("||", "&&", 1), dispatch)


def test_pubdev_publish_action_is_pinned_to_problem_matcher_fix() -> None:
    workflow = PUBDEV_WORKFLOW.read_text()
    assert f"xberg-io/actions/publish-pub@{PUBLISH_PUB_SHA} # v1.8.146" in workflow
    assert "xberg-io/actions/publish-pub@v1" not in workflow


def test_homebrew_push_rebases_after_concurrent_non_conflicting_push() -> None:
    for job in ("publish-homebrew-formula", "publish-homebrew-bottles"):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            origin, runner, competitor = create_tap_repositories(root)
            (runner / "Formula" / "xberg.rb").write_text("version 1.1.0\n")
            (competitor / "Formula" / "other.rb").write_text("other 2.0.0\n")
            git(competitor, "add", "Formula/other.rb")
            git(
                competitor,
                "-c",
                "user.name=competitor",
                "-c",
                "user.email=competitor@example.com",
                "commit",
                "-m",
                "other release",
            )
            git(competitor, "push", "origin", "main")

            result = run_homebrew_push_script(job, runner)
            assert result.returncode == 0, result.stderr
            verification = root / "verification"
            git(root, "clone", str(origin), str(verification))
            assert (verification / "Formula" / "xberg.rb").read_text() == "version 1.1.0\n"
            assert (verification / "Formula" / "other.rb").read_text() == "other 2.0.0\n"


def test_homebrew_push_stops_after_five_rejections() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        _, runner, _ = create_tap_repositories(root)
        (runner / "Formula" / "xberg.rb").write_text("version 1.1.0\n")

        real_git = shutil.which("git")
        assert real_git is not None
        wrapper_dir = root / "bin"
        wrapper_dir.mkdir()
        counter = root / "push-count"
        wrapper = wrapper_dir / "git"
        wrapper.write_text(
            "#!/bin/sh\n"
            'if [ "$1" = "push" ]; then\n'
            f"  count=$(cat '{counter}' 2>/dev/null || echo 0)\n"
            f"  echo $((count + 1)) > '{counter}'\n"
            "  exit 1\n"
            "fi\n"
            f'exec "{real_git}" "$@"\n'
        )
        wrapper.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = f"{wrapper_dir}{os.pathsep}{env['PATH']}"

        result = run_homebrew_push_script("publish-homebrew-formula", runner, env)
        assert result.returncode != 0
        assert counter.read_text().strip() == "5"
        assert "could not push to the tap after 5 attempts" in result.stdout


def test_homebrew_push_aborts_rebase_conflict_without_force() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        origin, runner, competitor = create_tap_repositories(root)
        (runner / "Formula" / "xberg.rb").write_text("runner version\n")
        (competitor / "Formula" / "xberg.rb").write_text("competitor version\n")
        git(competitor, "add", "Formula/xberg.rb")
        git(
            competitor,
            "-c",
            "user.name=competitor",
            "-c",
            "user.email=competitor@example.com",
            "commit",
            "-m",
            "conflicting release",
        )
        git(competitor, "push", "origin", "main")

        result = run_homebrew_push_script("publish-homebrew-formula", runner)
        assert result.returncode != 0
        assert "refusing to force the push" in result.stdout
        assert not (runner / ".git" / "rebase-merge").exists()
        assert not (runner / ".git" / "rebase-apply").exists()
        verification = root / "verification"
        git(root, "clone", str(origin), str(verification))
        assert (verification / "Formula" / "xberg.rb").read_text() == "competitor version\n"


def test_swift_dry_run_checks_run_artifact_without_release_mutation() -> None:
    block = job_block(WORKFLOW.read_text(), "update-swift-package-manifest")
    assert "name: swift-artifactbundle" in block
    assert "needs.prepare.outputs.dry_run == 'true'" in block
    assert 'if [ "$DRY_RUN" != "true" ]; then' in block
    assert "gh release download" in block

    for step in (
        "Update Package.swift with version and checksum",
        "Commit and push Package.swift",
    ):
        mutation = block.index(f"- name: {step}")
        next_step = block.find("\n      - ", mutation + 1)
        step_block = block[mutation : next_step if next_step >= 0 else None]
        assert "needs.prepare.outputs.dry_run != 'true'" in step_block


def test_glibc_ffi_jobs_build_lzma_statically() -> None:
    jobs_and_actions = {
        "go-ffi-libraries": "task go:publish:package",
        "c-ffi-libraries": "xberg-io/actions/build-rust-ffi@v1",
        "java-natives": "xberg-io/actions/build-java-natives@v1",
        "csharp-natives": "xberg-io/actions/build-csharp-natives@v1",
        # elixir-natives is deliberately absent: its linux-gnu legs moved to
        # elixir-natives-manylinux, which builds inside a manylinux_2_28 container where
        # liblzma comes from the base image and vendor-native-closure.sh bundles it. The
        # remaining macos/windows legs are not glibc targets, so there is no
        # LZMA_API_STATIC step for this contract to find.
        "dart-server-natives": "- name: Build Dart server native",
    }
    workflow = WORKFLOW.read_text()
    for job, action in jobs_and_actions.items():
        block = job_block(workflow, job)
        setup = block.index('echo "LZMA_API_STATIC=1" >> "$GITHUB_ENV"')
        build = block.index(action)
        assert setup < build, f"{job} configures static liblzma after its build"
        setup_block = block[block.rfind("\n      - ", 0, setup) : setup]
        assert "contains(matrix.target, '-linux-gnu')" in setup_block, job


def test_go_release_matrix_covers_every_generated_setup_platform() -> None:
    block = job_block(WORKFLOW.read_text(), "go-ffi-libraries")
    expected = {
        ("linux-x86_64", "x86_64-unknown-linux-gnu", "false"),
        ("linux-aarch64", "aarch64-unknown-linux-gnu", "false"),
        ("linux-x86_64-musl", "x86_64-unknown-linux-musl", "true"),
        ("linux-aarch64-musl", "aarch64-unknown-linux-musl", "true"),
        ("macos-arm64", "aarch64-apple-darwin", "false"),
        ("macos-x86_64", "x86_64-apple-darwin", "false"),
        ("windows-x86_64", "x86_64-pc-windows-msvc", "false"),
    }
    actual = set(re.findall(r"platform: ([^, }]+), target: ([^, }]+), musl: (true|false)", block))

    assert actual == expected
    setup = GO_SETUP.read_text()
    assert 'fmt.Sprintf("%s-go-v%s-%s.tar.gz", assetPrefix, version, platform)' in setup
    assert 'return base + "-musl", nil' in setup


def test_release_uses_supported_macos_deployment_floor() -> None:
    workflow = WORKFLOW.read_text()
    env = workflow[workflow.index("env:\n") : workflow.index("\npermissions:")]
    assert 'MACOSX_DEPLOYMENT_TARGET: "11.0"' in env


def test_go_release_packaging_matches_static_and_musl_setup_contracts() -> None:
    block = job_block(WORKFLOW.read_text(), "go-ffi-libraries")
    assert "task go:publish:package TARGET=${{ matrix.target }}" in block
    assert "task go:publish:package:musl TARGET=${{ matrix.target }}" in block
    assert "archive-name: xberg-go-${{ matrix.platform }}.tar.gz" not in block

    release_script = GO_RELEASE_SCRIPT.read_text()
    assert "xberg-go-v$VERSION-$PLATFORM.tar.gz" in release_script
    assert 'cargo zigbuild --locked -p xberg-ffi --release --target "$TARGET.$GLIBC_FLOOR"' in release_script
    assert "alef publish build --lang go --target" in release_script
    assert "native-static-libs.txt" in release_script
    assert 'grep -q "/lib/$STATIC_NAME$"' in release_script

    musl_script = GO_MUSL_RELEASE_SCRIPT.read_text()
    assert "xberg-go-v$VERSION-$PLATFORM.tar.gz" in musl_script
    assert 'grep -q "/lib/libxberg_ffi.so$"' in musl_script


def test_glibc_native_closures_are_strictly_verified() -> None:
    workflow = WORKFLOW.read_text()
    for job in ("csharp-natives", "dart-server-natives"):
        block = job_block(workflow, job)
        vendor = block.index("scripts/ci/vendor-native-closure.sh")
        verify = block.index("scripts/ci/verify-glibc-floor.sh")
        assert vendor < verify, f"{job} verifies before vendoring its closure"


def test_php_linux_artifacts_are_floor_built_and_verified() -> None:
    block = job_block(WORKFLOW.read_text(), "php-extension")
    setup_zig = block.index("Setup Zig for PHP glibc floor")
    install_zigbuild = block.index("Install cargo-zigbuild for PHP glibc floor")
    rebuild = block.index("Rebuild PHP extension at the glibc floor")
    package = block.index("xberg-io/actions/package-php-pie@v1")
    verify = block.index("scripts/ci/verify-php-linux-abi.sh")
    smoke = block.index("scripts/ci/smoke-test-php-debian12.sh")
    upload = block.index("actions/upload-artifact@v7", verify)

    assert setup_zig < install_zigbuild < rebuild < package < verify < smoke < upload
    assert 'TARGET_ARG="${TARGET}.${GLIBC_FLOOR}"' in block
    assert 'cargo zigbuild --locked -p xberg-php --release --target "${TARGET_ARG}"' in block
    assert "contains(matrix.platform.target, '-linux-gnu')" in block
    assert "EXPECTED_XBERG_SHA256: ${{ steps.php_floor_build.outputs.sha256 }}" in block
    assert "cargo-features:" not in block


def run_php_abi_fixture(profile: str, expected_digest: str | None = None) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        payload = root / "payload"
        payload.mkdir()
        (payload / "xberg.so").write_bytes(b"ELF fixture")
        archive = root / "php_xberg.tgz"
        with tarfile.open(archive, "w:gz") as handle:
            handle.add(payload / "xberg.so", arcname="xberg.so")

        bin_dir = root / "bin"
        bin_dir.mkdir()
        objdump = bin_dir / "objdump"
        objdump.write_text(
            "#!/bin/sh\n"
            'case "${PHP_ABI_FIXTURE:?}" in\n'
            "  safe) printf '%s\\n' '000 GLIBC_2.35 symbol' '000 GLIBCXX_3.4.30 symbol' '000 CXXABI_1.3.13 symbol' ;;\n"
            "  unsafe) printf '%s\\n' '000 GLIBC_2.39 symbol' '000 GLIBCXX_3.4.31 symbol' '000 CXXABI_1.3.15 symbol' ;;\n"
            "esac\n"
        )
        objdump.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = f"{bin_dir}{os.pathsep}{env['PATH']}"
        env["PHP_ABI_FIXTURE"] = profile
        env["EXPECTED_XBERG_SHA256"] = expected_digest or hashlib.sha256(b"ELF fixture").hexdigest()
        return run(["bash", str(PHP_ABI_SCRIPT), str(archive), "2.36", "3.4.30", "1.3.13"], root, env)


def test_php_linux_abi_gate_accepts_debian_12_compatible_symbols() -> None:
    result = run_php_abi_fixture("safe")
    assert result.returncode == 0, result.stderr
    assert "max GLIBC 2.35 <= 2.36" in result.stderr
    assert "max GLIBCXX 3.4.30 <= 3.4.30" in result.stderr
    assert "max CXXABI 1.3.13 <= 1.3.13" in result.stderr
    assert "packaged xberg.so matches the floor-built artifact" in result.stderr


def test_php_linux_abi_gate_rejects_both_symbol_floor_regressions() -> None:
    result = run_php_abi_fixture("unsafe")
    assert result.returncode != 0
    assert "requires GLIBC 2.39 > 2.36" in result.stderr
    assert "requires GLIBCXX 3.4.31 > 3.4.30" in result.stderr
    assert "requires CXXABI 1.3.15 > 1.3.13" in result.stderr


def test_php_linux_abi_gate_rejects_a_different_packaged_binary() -> None:
    result = run_php_abi_fixture("safe", "0" * 64)
    assert result.returncode != 0
    assert "packaged xberg.so does not match the floor-built artifact" in result.stderr


def test_cli_release_enables_metal_only_for_macos_arm64() -> None:
    block = job_block(WORKFLOW.read_text(), "cli-binaries")
    macos_arm64 = re.search(
        r"os: macos-15,\s+target: aarch64-apple-darwin,\s+"
        r'extra_cargo_args: "([^"]+)"',
        block,
    )
    assert macos_arm64 is not None
    assert "candle-metal" in macos_arm64.group(1).split(",")
    assert block.count("candle-metal") == 1


def test_docker_preflight_checks_out_the_resolved_candidate() -> None:
    block = job_block(DOCKER_WORKFLOW.read_text(), "check-docker")
    assert "ref: ${{ needs.prepare.outputs.checkout_ref }}" in block
    assert "ref: ${{ needs.prepare.outputs.tag }}" not in block


def test_macos_unit_tests_reclaim_artifacts_before_libheif() -> None:
    script = RUST_UNIT_SCRIPT.read_text()
    gliner = script.index("cargo test --locked --no-fail-fast -p xberg-gliner")
    cleanup = script.index('if [ "$(uname -s)" = "Darwin" ]; then', gliner)
    libheif = script.index("cargo test --locked --no-fail-fast -p xberg-libheif", cleanup)
    assert gliner < cleanup < libheif
    cleanup_block = script[cleanup:libheif]
    assert "task rust:clean" in cleanup_block


def test_xberg_unit_artifacts_are_reclaimed_before_workspace_tests() -> None:
    script = RUST_UNIT_SCRIPT.read_text()
    xberg = script.index('cargo test --locked --no-fail-fast -p xberg --features "$xberg_test_features"')
    workspace = script.index('echo "=== cargo test --workspace (all features, excluding xberg) ==="', xberg)
    between = script[xberg:workspace]
    assert re.search(r"(?m)^  cargo clean -p xberg \|\| exit$", between) is not None


def test_publish_contracts_run_in_ci() -> None:
    assert "python3 scripts/ci/test_publish_workflow_contracts.py" in CI_WORKFLOW.read_text()


if __name__ == "__main__":
    test_release_event_guards_cover_initial_actor_and_tag_policy()
    test_pubdev_publish_action_is_pinned_to_problem_matcher_fix()
    test_homebrew_push_rebases_after_concurrent_non_conflicting_push()
    test_homebrew_push_stops_after_five_rejections()
    test_homebrew_push_aborts_rebase_conflict_without_force()
    test_swift_dry_run_checks_run_artifact_without_release_mutation()
    test_glibc_ffi_jobs_build_lzma_statically()
    test_go_release_matrix_covers_every_generated_setup_platform()
    test_release_uses_supported_macos_deployment_floor()
    test_go_release_packaging_matches_static_and_musl_setup_contracts()
    test_glibc_native_closures_are_strictly_verified()
    test_php_linux_artifacts_are_floor_built_and_verified()
    test_php_linux_abi_gate_accepts_debian_12_compatible_symbols()
    test_php_linux_abi_gate_rejects_both_symbol_floor_regressions()
    test_php_linux_abi_gate_rejects_a_different_packaged_binary()
    test_cli_release_enables_metal_only_for_macos_arm64()
    test_docker_preflight_checks_out_the_resolved_candidate()
    test_macos_unit_tests_reclaim_artifacts_before_libheif()
    test_xberg_unit_artifacts_are_reclaimed_before_workspace_tests()
    test_publish_contracts_run_in_ci()
