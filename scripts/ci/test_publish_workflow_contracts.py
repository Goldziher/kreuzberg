import ast
import hashlib
import itertools
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
PHP_STAGE_SCRIPT = Path(__file__).parent / "stage-php-linux-ort.sh"
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


def evaluate_expression(expression: str, context: dict[str, object]) -> object:
    translated = expression.replace("||", "or").replace("&&", "and").replace("startsWith", "starts_with")
    translated = re.sub(r"!(?!=)", "not ", translated).strip()
    translated = re.sub(r"\.[a-zA-Z_][a-zA-Z0-9_-]*", lambda match: match[0].replace("-", "_"), translated)

    def evaluate(node: ast.expr) -> object:
        if isinstance(node, ast.BoolOp):
            result = evaluate(node.values[0])
            for value in node.values[1:]:
                if (isinstance(node.op, ast.Or) and not result) or (isinstance(node.op, ast.And) and result):
                    result = evaluate(value)
            return result
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.Not):
            return not evaluate(node.operand)
        if isinstance(node, ast.Compare) and len(node.ops) == len(node.comparators) == 1:
            left = evaluate(node.left)
            right = evaluate(node.comparators[0])
            if isinstance(node.ops[0], ast.NotEq):
                return left != right
            if isinstance(node.ops[0], ast.Eq):
                return left == right
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "starts_with":
            assert len(node.args) == 2 and not node.keywords
            return str(evaluate(node.args[0])).startswith(str(evaluate(node.args[1])))
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "cancelled":
            assert not node.args and not node.keywords
            return context["cancelled"]
        if isinstance(node, ast.Attribute):
            return attribute_value(node, context)
        if isinstance(node, ast.Constant) and isinstance(node.value, (str, bool)):
            return node.value
        if isinstance(node, ast.Name) and node.id in ("true", "false"):
            return node.id == "true"
        raise AssertionError(f"unsupported condition syntax: {ast.dump(node)}")

    parsed = ast.parse(translated, mode="eval")
    return evaluate(parsed.body)


def evaluate_condition(expression: str, context: dict[str, object]) -> bool:
    return bool(evaluate_expression(expression, context))


def github_context(event_name: str, tag: str, actor: str, triggering_actor: str) -> dict[str, object]:
    return {
        "inputs": {"plugins_only": False},
        "github": {
            "event_name": event_name,
            "actor": actor,
            "triggering_actor": triggering_actor,
            "event": {"release": {"tag_name": tag}},
        },
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
        "!inputs.plugins-only && (github.event_name != 'release' || (startsWith(github.event.release.tag_name, 'v') "
        "&& github.actor != 'xberg-dev-publisher[bot]'))"
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


def test_plugin_only_dispatch_skips_every_core_release_job() -> None:
    workflow = WORKFLOW.read_text().split("\njobs:\n", 1)[1]
    jobs = re.findall(r"(?m)^  ([a-zA-Z0-9_-]+):$", workflow)
    core_jobs = [job for job in jobs if job not in ("dispatch-plugin-npm", "dispatch-plugin-hermes")]
    assert len(core_jobs) == 84
    for job in core_jobs:
        expression = job_if_expression(job_block(workflow, job))
        assert expression.startswith("!inputs.plugins-only && ("), job
    prepare_expression = job_if_expression(job_block(workflow, "prepare"))
    assert evaluate_condition(prepare_expression, {"inputs": {"plugins_only": True}}) is False


def test_normal_php_retry_preserves_release_and_cancellation_guards() -> None:
    expression = job_if_expression(job_block(WORKFLOW.read_text(), "php-extension"))
    for plugins_only, cancelled, dry_run, crates_result, expected in (
        (False, False, "false", "skipped", True),
        (True, False, "false", "skipped", False),
        (False, True, "false", "skipped", False),
        (False, False, "true", "skipped", False),
        (False, False, "false", "failure", False),
    ):
        context = {
            "inputs": {"plugins_only": plugins_only},
            "cancelled": cancelled,
            "needs": {
                "prepare": {"outputs": {"dry_run": dry_run, "release_php": "true", "force_republish": "true"}},
                "publish_crates": {"result": crates_result},
                "check_packagist": {"outputs": {"exists": "true"}},
            },
        }
        assert evaluate_condition(expression, context) is expected


def test_partial_release_retries_skip_plugin_distribution_and_announcements() -> None:
    workflow = WORKFLOW.read_text()
    full_release_guard = "needs.prepare.outputs.release_targets == 'all'"
    for job in (
        "announce-discord",
        "publish-plugin-opencode",
        "publish-plugin-hermes",
        "publish-plugin-artifacts",
        "plugin-release-report",
    ):
        block = job_block(workflow, job)
        condition = block[: block.index("    runs-on:")]
        assert full_release_guard in condition, f"{job} runs during partial release retries"

    php = job_block(workflow, "php-extension")
    assert "needs.prepare.outputs.release_php == 'true'" in php
    assert full_release_guard not in php
    finalize = job_block(workflow, "release-finalize")
    assert "go-module-path: ${{ needs.prepare.outputs.release_go == 'true' && 'packages/go' || '' }}" in finalize


def test_php_partial_retry_publishes_packagist_only_after_successful_upload() -> None:
    block = job_block(WORKFLOW.read_text(), "publish-packagist")
    condition = block[: block.index("    runs-on:")]
    assert "!cancelled()" in condition
    assert "needs.upload-php-pie-release.result == 'success'" in condition
    assert "needs.prepare.outputs.release_php == 'true'" in condition


def test_release_finalization_requires_successful_asset_verification() -> None:
    block = job_block(WORKFLOW.read_text(), "finalize-github-release-after-uploads")
    condition = job_if_expression(block)
    assert "needs.verify-assets.result == 'success'" in condition
    assert "needs.verify-assets.result != 'failure'" not in condition
    assert "!cancelled()" in condition
    second = job_block(WORKFLOW.read_text(), "release-finalize")
    second_condition = second[: second.index("    runs-on:")]
    assert "needs.finalize-github-release-after-uploads.result == 'success'" in second_condition
    assert "needs.finalize-github-release-after-uploads.result != 'failure'" not in second_condition


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
    setup_ort = block.index("Setup ONNX Runtime (Linux)")
    first_build = block.index("xberg-io/actions/build-php-extension@v1")
    rebuild = block.index("Rebuild PHP extension at the glibc floor")
    package = block.index("xberg-io/actions/package-php-pie@v1")
    stage_ort = block.index("scripts/ci/stage-php-linux-ort.sh")
    verify = block.index("scripts/ci/verify-php-linux-abi.sh")
    smoke = block.index("scripts/ci/smoke-test-php-debian12.sh")
    upload = block.index("actions/upload-artifact@v7", verify)

    assert (
        setup_zig < install_zigbuild < setup_ort < first_build < rebuild < package < stage_ort < verify < smoke < upload
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


def test_rust_ci_enables_bundled_oem_regressions_on_every_platform() -> None:
    root = WORKFLOW.parents[2]
    regression = (root / "crates/xberg/tests/tesseract_engine_mode.rs").read_text()
    assert '#![cfg(all(feature = "ocr", feature = "bundle-tessdata-eng"))]' in regression
    assert len(re.findall(r"#\[tokio::test\]", regression)) == 4
    sources = (
        (RUST_UNIT_SCRIPT.read_text(), "xberg_test_features"),
        ((WORKFLOW.parent / "ci-rust.yaml").read_text(), "features"),
    )
    checked = 0
    for system, machine in (("Linux", "x86_64"), ("Linux", "aarch64"), ("Darwin", "arm64")):
        selected = []
        for source, variable in sources:
            selection = re.search(
                rf"(?m)^ +{variable}=full,[^\n]+\n +if .*?\n.*?^ +fi$",
                source,
                re.DOTALL,
            )
            assert selection is not None, variable
            shell = (
                ('uname() { if [ "$1" = "-s" ]; then printf "%s\\n" "$2"; else printf "%s\\n" "$3"; fi; }\n')
                .replace('"$2"', f'"{system}"')
                .replace('"$3"', f'"{machine}"')
            )
            result = run(
                ["bash", "-euo", "pipefail", "-c", shell + selection[0] + f'\nprintf "%s\\n" "${{{variable}}}"'],
                root,
            )
            assert result.returncode == 0, result.stderr
            features = set(result.stdout.splitlines()[-1].split(","))
            base = {"full-no-heic", "heic"} if (system, machine) == ("Linux", "aarch64") else {"full"}
            assert features == base | {"formula-recognition", "bundle-tessdata-eng"}, (
                system,
                machine,
                variable,
                features,
            )
            selected.append(features)
            checked += 1
        assert selected[0] == selected[1], (system, machine, selected)
    assert checked == 6


def prepare_step(name: str) -> str:
    prepare = job_block(WORKFLOW.read_text(), "prepare")
    step = re.search(rf"(?ms)^      - name: {re.escape(name)}\n(.*?)(?=^      - |\Z)", prepare)
    assert step is not None, name
    return step[1]


def test_initial_release_checkout_pins_omitted_ref_dry_runs() -> None:
    checkout = prepare_step("Checkout")
    ref = re.search(r"(?m)^          ref: \$\{\{ (.+) \}\}$", checkout)
    assert ref is not None
    expression = ref[1]
    mismatches = []
    checked = 0
    for dry, payload_dry in itertools.product((False, True, "false", "true", None), repeat=2):
        for republish, explicit_ref, tag in (
            (False, "", ""),
            (False, "chosen", ""),
            (False, "", "v1"),
            (True, "", ""),
            (True, "chosen", "v1"),
        ):
            context = {
                "inputs": {"republish": republish, "ref": explicit_ref, "tag": tag, "dry_run": dry},
                "github": {
                    "event": {"repository": {"default_branch": "main"}, "client_payload": {"dry_run": payload_dry}},
                    "sha": "frozen-dispatch-sha",
                    "ref": "refs/heads/main",
                },
            }
            expected = (
                (explicit_ref or "main")
                if republish
                else explicit_ref
                or tag
                or (
                    "frozen-dispatch-sha"
                    if any(value is True or value == "true" for value in (dry, payload_dry))
                    else "refs/heads/main"
                )
            )
            actual = evaluate_expression(expression, context)
            if actual != expected:
                mismatches.append((dry, payload_dry, republish, explicit_ref, tag, actual, expected))
            checked += 1
    assert checked == 125
    assert not mismatches, mismatches


def test_release_retag_skips_boolean_and_string_dry_runs_in_both_channels() -> None:
    step = prepare_step("Retag for republish")
    raw = step.partition("        if:")[2].partition("        uses:")[0].strip().removeprefix(">-").strip()
    expression = " ".join(raw.removeprefix("${{").removesuffix("}}").split())
    mismatches = []
    checked = 0
    for republish in ((False, False), (True, False), (False, True), (True, True), ("true", False), (False, "true")):
        for dry_input in (False, True, "false", "true", None):
            for dry_payload in (False, True, "false", "true", None):
                context = {
                    "inputs": {"republish": republish[0], "dry_run": dry_input},
                    "github": {"event": {"client_payload": {"republish": republish[1], "dry_run": dry_payload}}},
                }
                expected = any(value is True for value in republish) and not any(
                    value is True or value == "true" for value in (dry_input, dry_payload)
                )
                actual = evaluate_condition(expression, context)
                if actual != expected:
                    mismatches.append((republish, dry_input, dry_payload, actual, expected))
                checked += 1
    assert checked == 150
    assert not mismatches, mismatches


def test_helm_publish_does_not_depend_on_the_cli_image_manifest() -> None:
    # The chart only references xberg:<version> and xberg:<version>-core. merge-docker is one
    # matrix job, so a cli leg that fails (cli/amd64 timing out leaves a single-arch digest,
    # which "Verify digests present" rejects) turns its aggregate result into `failure` and
    # skipped the chart for v1.3.6-v1.3.8. ~keep
    workflow = DOCKER_WORKFLOW.read_text()
    helm = job_block(workflow, "publish-helm")
    assert "merge-docker.result" not in helm
    assert re.search(r"(?m)^      - chart-images$", helm) is not None
    assert "needs.chart-images.result == 'success'" in helm
    images = job_block(workflow, "chart-images")
    assert "xberg-cli" not in images
    assert "ghcr.io/xberg-io/xberg:${{ needs.prepare.outputs.version }}" in images
    assert "ghcr.io/xberg-io/xberg:${{ needs.prepare.outputs.version }}-core" in images


def test_publish_contracts_run_in_ci() -> None:
    assert "task release:test:contracts" in CI_WORKFLOW.read_text()


def test_system_dependency_changes_trigger_rust_and_e2e_ci() -> None:
    workflows = ("ci-rust.yaml", "ci-e2e.yaml")
    required_paths = {"scripts/ci/install-system-deps/**", ".github/actions/install-system-deps/**"}
    checked = 0
    for name in workflows:
        workflow = (WORKFLOW.parent / name).read_text()
        push = re.search(r"(?ms)^on:\n  push:\n(.*?)(?=^  \S|^\S|\Z)", workflow)
        assert push is not None, f"{name} has no on.push block"
        paths = re.search(r"(?ms)^    paths:\n(.*?)(?=^    \S|\Z)", push.group(1))
        assert paths is not None, f"{name} has no on.push.paths block"
        patterns = set(re.findall(r'^      - "([^"\n]+)"$', paths.group(1), re.MULTILINE))
        assert required_paths <= patterns, f"{name} does not trigger for {sorted(required_paths - patterns)}"
        checked += 1
    assert checked == 2


def test_swift_static_library_builds_disable_host_extension_symbol_lookup() -> None:
    workflow = WORKFLOW.read_text()
    swift = job_block(workflow, "swift-target")
    setup = re.search(r"(?ms)^      - uses: xberg-io/actions/setup-rust@[^\n]+\n(.*?)(?=^      - |\Z)", swift)
    assert setup is not None
    assert 'macos-dynamic-lookup: "false"' in setup[1]
    assert "aarch64-apple-ios-sim" in swift
    assert "targets: ${{ matrix.target }}" in swift


if __name__ == "__main__":
    test_release_event_guards_cover_initial_actor_and_tag_policy()
    test_plugin_only_dispatch_skips_every_core_release_job()
    test_normal_php_retry_preserves_release_and_cancellation_guards()
    test_partial_release_retries_skip_plugin_distribution_and_announcements()
    test_php_partial_retry_publishes_packagist_only_after_successful_upload()
    test_release_finalization_requires_successful_asset_verification()
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
    test_cli_release_enables_metal_only_for_macos_arm64()
    test_docker_preflight_checks_out_the_resolved_candidate()
    test_macos_unit_tests_reclaim_artifacts_before_libheif()
    test_xberg_unit_artifacts_are_reclaimed_before_workspace_tests()
    test_rust_ci_enables_bundled_oem_regressions_on_every_platform()
    test_initial_release_checkout_pins_omitted_ref_dry_runs()
    test_release_retag_skips_boolean_and_string_dry_runs_in_both_channels()
    test_helm_publish_does_not_depend_on_the_cli_image_manifest()
    test_swift_static_library_builds_disable_host_extension_symbol_lookup()
    test_publish_contracts_run_in_ci()
    test_system_dependency_changes_trigger_rust_and_e2e_ci()
