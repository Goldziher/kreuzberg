import ast
import itertools
import os
import re
import shutil
import subprocess
import tempfile
import textwrap
from pathlib import Path

from zigbuild_release_contracts import assert_zigbuild_negative_controls, validate_zigbuild_workflow

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


def test_every_release_zigbuild_consumer_uses_the_verified_tool() -> None:
    validate_zigbuild_workflow(WORKFLOW.read_text())


def test_zigbuild_wiring_gate_rejects_missing_wrong_and_early_installers() -> None:
    assert_zigbuild_negative_controls(WORKFLOW.read_text())


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


def test_swift_ios_builds_pin_the_deployment_target_to_the_manifest_floor() -> None:
    manifest = (Path(__file__).parents[2] / "Package.swift").read_text()
    floor = re.search(r"\.iOS\(\.v(\d+)\)", manifest)
    assert floor is not None, "Package.swift declares no iOS floor"
    swift = job_block(WORKFLOW.read_text(), "swift-target")
    build = re.search(
        r"(?ms)^      - uses: xberg-io/actions/build-swift-artifactbundle@[^\n]+\n(.*?)(?=^      - |\Z)", swift
    )
    assert build is not None
    pinned = re.search(r'(?m)^          IPHONEOS_DEPLOYMENT_TARGET: "([0-9.]+)"$', build[1])
    assert pinned is not None, "swift-target build step does not set IPHONEOS_DEPLOYMENT_TARGET"
    assert pinned[1] == f"{floor[1]}.0"


def test_macos_onnx_action_forwards_rpath_option_only_to_macos() -> None:
    action = (WORKFLOW.parents[1] / "actions/setup-onnx-runtime/action.yml").read_text()
    option = re.findall(r"(?m)^  add-macos-rpath:\n((?:^    [^\n]*\n)+)", action)
    assert len(option) == 1, "one actual action input must exist"
    assert re.findall(r'(?m)^    default: "true"$', option[0]) == ['    default: "true"']
    macos = re.findall(
        r"(?m)^    - name: Prepare ONNX Runtime \(macOS\)\n"
        r"      if: runner.os == \'macOS\'\n"
        r"      shell: bash\n"
        r"      run: ([^\n]+)$",
        action,
    )
    assert len(macos) == 1, "one actual macOS action run step must be examined"
    arguments = re.findall(r'"(\$\{\{ inputs\.[a-z-]+ \}\})"', macos[0])
    assert macos[0].startswith("scripts/ci/actions/setup-onnx-runtime/macos.sh ")
    expected_arguments = [
        "${{ inputs.ort-version }}",
        "${{ inputs.dest-dir }}",
        "${{ inputs.arch-id }}",
        "${{ inputs.strategy }}",
        "${{ inputs.add-macos-rpath }}",
    ]
    assert arguments == expected_arguments
    assert macos[0] == " ".join(
        [
            "scripts/ci/actions/setup-onnx-runtime/macos.sh",
            *(f'"{argument}"' for argument in expected_arguments),
        ]
    )
    assert len(re.findall(r"(?m)^      run: .*inputs\.add-macos-rpath", action)) == 1


def test_csharp_native_build_leaves_rpath_to_generated_ffi_package() -> None:
    csharp = job_block(WORKFLOW.read_text(), "csharp-natives")
    setup = re.findall(
        r"(?m)^      - name: Setup ONNX Runtime\n"
        r"        uses: \./\.github/actions/setup-onnx-runtime\n"
        r"        with:\n((?:^          [^\n]*\n)+)",
        csharp,
    )
    assert len(setup) == 1, "one actual C# setup action step must be examined"
    assert re.findall(r'(?m)^          add-macos-rpath: "false"$', setup[0]) == [
        '          add-macos-rpath: "false"',
    ]
    assert re.findall(r'(?m)^          ort-version: "1\.24\.2"$', setup[0]) == [
        '          ort-version: "1.24.2"',
    ]


if __name__ == "__main__":
    test_macos_onnx_action_forwards_rpath_option_only_to_macos()
    test_csharp_native_build_leaves_rpath_to_generated_ffi_package()
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
    test_cli_release_enables_metal_only_for_macos_arm64()
    test_docker_preflight_checks_out_the_resolved_candidate()
    test_macos_unit_tests_reclaim_artifacts_before_libheif()
    test_xberg_unit_artifacts_are_reclaimed_before_workspace_tests()
    test_rust_ci_enables_bundled_oem_regressions_on_every_platform()
    test_initial_release_checkout_pins_omitted_ref_dry_runs()
    test_release_retag_skips_boolean_and_string_dry_runs_in_both_channels()
    test_every_release_zigbuild_consumer_uses_the_verified_tool()
    test_zigbuild_wiring_gate_rejects_missing_wrong_and_early_installers()
    test_helm_publish_does_not_depend_on_the_cli_image_manifest()
    test_swift_static_library_builds_disable_host_extension_symbol_lookup()
    test_swift_ios_builds_pin_the_deployment_target_to_the_manifest_floor()
    test_publish_contracts_run_in_ci()
    test_system_dependency_changes_trigger_rust_and_e2e_ci()
