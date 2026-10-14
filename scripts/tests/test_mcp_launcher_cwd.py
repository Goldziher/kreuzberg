"""Verify launcher cwd and argv through actual npm package execution."""

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest


class LauncherDirectoryTests(unittest.TestCase):
    """Install a local fixture package without contacting a registry."""

    def test_should_preserve_project_directory_and_relative_configuration(self) -> None:
        """The launched process must see the caller's config and exact arguments."""
        assert shutil.which("npm") is not None and shutil.which("npx") is not None
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix="mcp-launch-cwd-") as temporary:
            directory = Path(temporary)
            package = directory / "package"
            package.mkdir()
            (package / "package.json").write_text(
                json.dumps(
                    {
                        "name": "mcp-launch-cwd-fixture",
                        "version": "1.0.0",
                        "bin": {"audit-mcp-cli": "cli.js"},
                    }
                )
            )
            executable = package / "cli.js"
            executable.write_text(
                '#!/usr/bin/env node\nconst fs = require("node:fs");\n'
                'if (process.argv.includes("--version")) process.exit(0);\n'
                'console.log(JSON.stringify({cwd: process.cwd(), config: fs.readFileSync("model.toml", "utf8"), '
                "args: process.argv.slice(2)}));\n"
            )
            executable.chmod(0o755)
            packed = subprocess.run(
                ["npm", "pack", "--json", "--pack-destination", str(directory)],
                cwd=package,
                capture_output=True,
                text=True,
                check=True,
            )
            archive = directory / json.loads(packed.stdout)[0]["filename"]
            project = directory / "caller project"
            project.mkdir()
            (project / "model.toml").write_text("provider-model-route")
            source = (root / "plugin/scripts/mcp-launch.sh").read_text()
            mode = re.search(r'LAUNCHER="\$\{([A-Z_]+):-auto\}"', source)
            assert mode is not None
            source = re.sub(
                r'^BINARY_NAME="[^"\n]+"', 'BINARY_NAME="audit-mcp-cli"', source, count=1, flags=re.MULTILINE
            )
            source = re.sub(
                r'npx(?: -y)? "\$NPM_PKG@latest"',
                'npx -y --package="' + str(archive) + '" audit-mcp-cli',
                source,
            )
            source = source.replace('"$NPM_PKG@latest"', '"' + str(archive) + '"')
            launcher = directory / "launcher.sh"
            launcher.write_text(source)
            launcher.chmod(0o755)
            environment = os.environ | {
                mode[1]: "npx",
                "CLAUDE_PLUGIN_ROOT": str(directory / "plugin"),
                "npm_config_cache": str(directory / "npm-cache"),
                "npm_config_offline": "true",
                "npm_config_yes": "true",
            }
            arguments = ["mcp", "relative file.txt", "", "--stdio"]
            result = subprocess.run(
                [str(launcher), *arguments],
                cwd=project,
                env=environment,
                capture_output=True,
                text=True,
                timeout=60,
                check=False,
            )
            assert result.returncode == 0, result.stderr
            payload = json.loads(result.stdout)
            assert Path(payload["cwd"]).resolve() == project.resolve()
            assert payload["config"] == "provider-model-route"
            assert payload["args"] == arguments


if __name__ == "__main__":
    unittest.main()
