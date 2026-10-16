"""Check release Zig consumers and their pinned tool boundaries."""

import ast
import re


def _job_block(workflow: str, job: str) -> str:
    match = re.search(rf"(?ms)^  {re.escape(job)}:\n(.*?)(?=^  [a-zA-Z0-9_-]+:\n|\Z)", workflow)
    assert match is not None, job
    return match[1]


ZIGBUILD_ACTION_SHA = "0b670903ab1fe160a17492d5de61ec16f28a7264"
ZIGBUILD_SOURCE_SHA = "89c352ae6ff0bbafe17788da327764d9ef92240b"
ZIGBUILD_CALLERS = {
    "php-extension": ("Restore workspace manifest after the out-of-workspace PHP build", "Rebuild PHP extension", 6),
    "go-ffi-libraries": ("Install alef CLI", "Package versioned Go archive", 2),
    "c-ffi-libraries": ("Setup Zig for glibc", "xberg-io/actions/build-rust-ffi@", 2),
    "java-natives": ("Setup Zig for glibc", "xberg-io/actions/build-java-natives@", 2),
    "csharp-natives": ("Setup Zig for glibc", "xberg-io/actions/build-csharp-natives@", 2),
    "dart-server-natives": ("Setup Zig for glibc", "      - name: Build Dart server native", 2),
    "swift-target": ("Install system dependencies", "xberg-io/actions/build-swift-artifactbundle@", 2),
}


def validate_zigbuild_caller(name: str, block: str) -> str:
    steps = re.findall(r"(?ms)^      - .*?(?=^      - |\Z)", block)
    installers = [step for step in steps if "cargo-zigbuild-revision:" in step]
    assert len(installers) == 1, (name, "installer count", len(installers))
    installer = installers[0]
    assert f"uses: xberg-io/actions/setup-rust@{ZIGBUILD_ACTION_SHA}" in installer, (name, "action pin")
    assert f'cargo-zigbuild-revision: "{ZIGBUILD_SOURCE_SHA}"' in installer, (name, "source pin")
    target = "matrix.platform.target" if name == "php-extension" else "matrix.target"
    assert f"if: contains({target}, '-linux-gnu')" in installer, (name, "GNU mask")
    assert f"target: ${{{{ {target} }}}}" in installer, (name, "target")
    barrier, builder, expected_count = ZIGBUILD_CALLERS[name]
    assert block.index(barrier) < block.index(installer) < block.index(builder), (name, "installer order")
    triples = re.findall(r"(?:target: |^          - )((?:aarch64|x86_64)-unknown-linux-gnu)\b", block, re.MULTILINE)
    assert sorted(triples) == ["aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"], (name, triples)
    actual_count = len(triples)
    if name == "php-extension":
        php = re.search(r"(?m)^        php: (\[[^\n]+\])$", block)
        assert php is not None
        actual_count *= len(ast.literal_eval(php[1]))
    if name == "swift-target":
        assert 'macos-dynamic-lookup: "false"' in installer
    assert actual_count == expected_count, (name, actual_count, expected_count)
    return installer


def validate_zigbuild_workflow(workflow: str) -> None:
    consumers = {}
    for name in re.findall(r"(?m)^  ([a-zA-Z0-9_-]+):$", workflow):
        block = _job_block(workflow, name)
        if (
            "cargo zigbuild" in block
            or "task go:publish:package TARGET=" in block
            or "glibc-version:" in block
            or "targets: ${{ matrix.target }}" in block
        ):
            consumers[name] = block
    assert set(consumers) == set(ZIGBUILD_CALLERS), ("consumer census", sorted(consumers))
    for name, block in consumers.items():
        validate_zigbuild_caller(name, block)
    assert len(consumers) == 7
    assert sum(value[2] for value in ZIGBUILD_CALLERS.values()) == 18
    assert "command -v cargo-zigbuild" not in workflow


def assert_zigbuild_negative_controls(workflow: str) -> None:
    validate_zigbuild_workflow(workflow)
    block = _job_block(workflow, "php-extension")
    installer = validate_zigbuild_caller("php-extension", block)
    controls = (
        (block.replace(ZIGBUILD_ACTION_SHA, "0" * 40), "action pin"),
        (block.replace(f'          cargo-zigbuild-revision: "{ZIGBUILD_SOURCE_SHA}"\n', ""), "installer count"),
        (block.replace(installer, "").replace("    steps:\n", "    steps:\n" + installer), "installer order"),
    )
    rejected = 0
    for broken, diagnostic in controls:
        caught = None
        try:
            validate_zigbuild_workflow(workflow.replace(block, broken))
        except AssertionError as error:
            caught = str(error)
        assert caught is not None and diagnostic in caught, (diagnostic, caught)
        rejected += 1
    assert rejected == 3
