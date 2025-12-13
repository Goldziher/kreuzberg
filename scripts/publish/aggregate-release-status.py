import os


def main() -> None:
    release_tag = os.environ.get("RELEASE_TAG", "unknown")

    registry_checks = {
        "Python (PyPI)": os.environ.get("CHECK_PYPI_EXISTS", ""),
        "Node (npm)": os.environ.get("CHECK_NPM_EXISTS", ""),
        "Ruby (RubyGems)": os.environ.get("CHECK_RUBYGEMS_EXISTS", ""),
        "Java (Maven)": os.environ.get("CHECK_MAVEN_EXISTS", ""),
        "C# (NuGet)": os.environ.get("CHECK_NUGET_EXISTS", ""),
    }

    publish_results = {
        "Python (PyPI)": os.environ.get("RESULT_PUBLISH_PYPI", ""),
        "Node (npm)": os.environ.get("RESULT_PUBLISH_NODE", ""),
        "WASM (npm)": os.environ.get("RESULT_PUBLISH_WASM", ""),
        "Ruby (RubyGems)": os.environ.get("RESULT_PUBLISH_RUBYGEMS", ""),
        "Java (Maven)": os.environ.get("RESULT_PUBLISH_MAVEN", ""),
        "C# (NuGet)": os.environ.get("RESULT_PUBLISH_NUGET", ""),
        "Homebrew": os.environ.get("RESULT_PUBLISH_HOMEBREW", ""),
        "Docker": os.environ.get("RESULT_PUBLISH_DOCKER", ""),
        "CLI Release": os.environ.get("RESULT_UPLOAD_CLI_RELEASE", ""),
        "C# Release": os.environ.get("RESULT_UPLOAD_CSHARP_RELEASE", ""),
        "Go FFI Libraries": os.environ.get("RESULT_UPLOAD_GO_RELEASE", ""),
    }

    summary_lines = [f"## Release Status for {release_tag}", ""]

    skipped = [k for k, v in registry_checks.items() if v == "true"]
    if skipped:
        summary_lines.append("### ⏭️  Already Published (Skipped)")
        summary_lines.extend([f"- {item}" for item in skipped])
        summary_lines.append("")

    success = [k for k, v in publish_results.items() if v == "success"]
    if success:
        summary_lines.append("### ✅ Published Successfully")
        summary_lines.extend([f"- {item}" for item in success])
        summary_lines.append("")

    failed = [k for k, v in publish_results.items() if v == "failure"]
    if failed:
        summary_lines.append("### ❌ Failed")
        summary_lines.extend([f"- {item}" for item in failed])
        summary_lines.append("")

    skipped_cond = [k for k, v in publish_results.items() if v == "skipped"]
    if skipped_cond:
        summary_lines.append("### ⊘ Skipped (Not Enabled)")
        summary_lines.extend([f"- {item}" for item in skipped_cond])
        summary_lines.append("")

    summary = "\n".join(summary_lines)

    step_summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if step_summary_path:
        with open(step_summary_path, "a", encoding="utf-8") as f:
            f.write(summary + "\n")

    github_output_path = os.environ.get("GITHUB_OUTPUT")
    if github_output_path:
        escaped = summary.replace("%", "%25").replace("\n", "%0A")
        with open(github_output_path, "a", encoding="utf-8") as f:
            f.write(f"release_summary={escaped}\n")


if __name__ == "__main__":
    main()
