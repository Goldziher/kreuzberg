#!/usr/bin/env python3
"""
Rust build system with automatic toolchain detection and fallback.

This script handles Rust toolchain setup and building with multiple fallback options.
It's designed to work in various environments including CI, development, and production.
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import List, Optional, Tuple


class RustBuildError(Exception):
    """Custom exception for Rust build errors."""

    pass


class RustToolchainManager:
    """Manages Rust toolchain detection and setup."""

    def __init__(self):
        self.rustc_path: Optional[str] = None
        self.cargo_path: Optional[str] = None
        self.rust_version: Optional[str] = None
        self.cargo_version: Optional[str] = None

    def find_rust_installations(self) -> List[Tuple[str, str]]:
        """Find all possible Rust installations on the system."""
        installations = []

        # Common installation paths
        paths_to_check = [
            # System installations
            ("/usr/bin/rustc", "/usr/bin/cargo"),
            ("/usr/local/bin/rustc", "/usr/local/bin/cargo"),
            # User installations
            (os.path.expanduser("~/.cargo/bin/rustc"), os.path.expanduser("~/.cargo/bin/cargo")),
            (os.path.expanduser("~/.local/bin/rustc"), os.path.expanduser("~/.local/bin/cargo")),
            # Conda installations
            (os.path.expanduser("~/miniconda3/bin/rustc"), os.path.expanduser("~/miniconda3/bin/cargo")),
            (os.path.expanduser("~/anaconda3/bin/rustc"), os.path.expanduser("~/anaconda3/bin/cargo")),
        ]

        # Check PATH for additional installations
        for path_dir in os.environ.get("PATH", "").split(os.pathsep):
            rustc_path = os.path.join(path_dir, "rustc")
            cargo_path = os.path.join(path_dir, "cargo")
            if os.path.isfile(rustc_path) and os.path.isfile(cargo_path):
                paths_to_check.append((rustc_path, cargo_path))

        # Filter to only existing installations
        for rustc_path, cargo_path in paths_to_check:
            if os.path.isfile(rustc_path) and os.path.isfile(cargo_path):
                installations.append((rustc_path, cargo_path))

        return installations

    def test_rust_installation(self, rustc_path: str, cargo_path: str) -> bool:
        """Test if a Rust installation works properly."""
        try:
            # Test rustc
            result = subprocess.run([rustc_path, "--version"], capture_output=True, text=True, timeout=10)
            if result.returncode != 0:
                return False

            # Test cargo
            result = subprocess.run([cargo_path, "--version"], capture_output=True, text=True, timeout=10)
            if result.returncode != 0:
                return False

            return True
        except (subprocess.TimeoutExpired, subprocess.CalledProcessError, FileNotFoundError):
            return False

    def setup_toolchain(self) -> bool:
        """Setup the best available Rust toolchain."""
        installations = self.find_rust_installations()

        if not installations:
            raise RustBuildError("No Rust installations found on the system")

        print(f"Found {len(installations)} potential Rust installations")

        for i, (rustc_path, cargo_path) in enumerate(installations):
            print(f"Testing installation {i + 1}: {rustc_path}")

            if self.test_rust_installation(rustc_path, cargo_path):
                self.rustc_path = rustc_path
                self.cargo_path = cargo_path

                # Get versions
                try:
                    result = subprocess.run([rustc_path, "--version"], capture_output=True, text=True, timeout=10)
                    self.rust_version = result.stdout.strip()

                    result = subprocess.run([cargo_path, "--version"], capture_output=True, text=True, timeout=10)
                    self.cargo_version = result.stdout.strip()
                except subprocess.TimeoutExpired:
                    pass

                print(f"✓ Using Rust installation: {rustc_path}")
                print(f"  {self.rust_version}")
                print(f"  {self.cargo_version}")
                return True
            else:
                print(f"✗ Installation {i + 1} failed tests")

        raise RustBuildError("No working Rust installation found")

    def get_environment(self) -> dict:
        """Get environment variables for the Rust toolchain."""
        if not self.rustc_path or not self.cargo_path:
            raise RustBuildError("Rust toolchain not set up")

        env = os.environ.copy()
        env["RUSTC"] = self.rustc_path
        env["CARGO"] = self.cargo_path

        # Add cargo bin to PATH
        cargo_bin_dir = os.path.dirname(self.cargo_path)
        if cargo_bin_dir not in env.get("PATH", ""):
            env["PATH"] = f"{cargo_bin_dir}:{env.get('PATH', '')}"

        return env


class RustBuilder:
    """Handles Rust building with Maturin."""

    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.toolchain_manager = RustToolchainManager()

    def setup_rust(self) -> None:
        """Setup Rust toolchain."""
        print("Setting up Rust toolchain...")
        self.toolchain_manager.setup_toolchain()

    def check_rust_installation(self) -> bool:
        """Check if Rust is properly installed."""
        try:
            self.toolchain_manager.setup_toolchain()
            return True
        except RustBuildError:
            return False

    def run_cargo_command(self, args: List[str], cwd: Optional[Path] = None) -> subprocess.CompletedProcess:
        """Run a cargo command with the configured toolchain."""
        if not self.toolchain_manager.rustc_path:
            raise RustBuildError("Rust toolchain not set up")

        cmd = [self.toolchain_manager.cargo_path] + args
        env = self.toolchain_manager.get_environment()

        print(f"Running: {' '.join(cmd)}")

        return subprocess.run(
            cmd,
            cwd=cwd or self.project_root,
            env=env,
            capture_output=False,  # Let output go to terminal
            text=True,
        )

    def build_with_maturin(self, release: bool = True, features: Optional[List[str]] = None) -> bool:
        """Build the Rust module using Maturin."""
        print("Building Rust module with Maturin...")

        # Setup Rust first
        self.setup_rust()

        # Prepare maturin command
        cmd = ["maturin", "develop"]

        if release:
            cmd.append("--release")

        if features:
            cmd.extend(["--features", ",".join(features)])

        # Run maturin
        try:
            result = subprocess.run(
                cmd,
                cwd=self.project_root,
                env=self.toolchain_manager.get_environment(),
                capture_output=False,
                text=True,
            )

            if result.returncode == 0:
                print("✓ Maturin build successful")
                return True
            else:
                print(f"✗ Maturin build failed with return code {result.returncode}")
                return False

        except FileNotFoundError:
            print("✗ Maturin not found. Please install it with: pip install maturin")
            return False
        except Exception as e:
            print(f"✗ Maturin build failed: {e}")
            return False

    def test_rust_build(self) -> bool:
        """Test if the Rust code compiles."""
        print("Testing Rust compilation...")

        try:
            result = self.run_cargo_command(["check"])
            if result.returncode == 0:
                print("✓ Rust compilation test passed")
                return True
            else:
                print("✗ Rust compilation test failed")
                return False
        except Exception as e:
            print(f"✗ Rust compilation test failed: {e}")
            return False

    def run_rust_tests(self) -> bool:
        """Run Rust tests."""
        print("Running Rust tests...")

        try:
            result = self.run_cargo_command(["test", "--release"])
            if result.returncode == 0:
                print("✓ Rust tests passed")
                return True
            else:
                print("✗ Rust tests failed")
                return False
        except Exception as e:
            print(f"✗ Rust tests failed: {e}")
            return False

    def run_rust_linting(self) -> bool:
        """Run Rust linting (fmt + clippy)."""
        print("Running Rust linting...")

        success = True

        # Run rustfmt
        try:
            result = self.run_cargo_command(["fmt", "--", "--check"])
            if result.returncode != 0:
                print("✗ Rust formatting check failed")
                success = False
            else:
                print("✓ Rust formatting check passed")
        except Exception as e:
            print(f"✗ Rust formatting check failed: {e}")
            success = False

        # Run clippy
        try:
            result = self.run_cargo_command(["clippy", "--", "-D", "warnings"])
            if result.returncode != 0:
                print("✗ Clippy check failed")
                success = False
            else:
                print("✓ Clippy check passed")
        except Exception as e:
            print(f"✗ Clippy check failed: {e}")
            success = False

        return success


def main():
    """Main entry point."""
    project_root = Path(__file__).parent.parent
    builder = RustBuilder(project_root)

    if len(sys.argv) < 2:
        print("Usage: python build_rust.py <command>")
        print("Commands:")
        print("  setup     - Setup Rust toolchain")
        print("  check     - Check Rust compilation")
        print("  test      - Run Rust tests")
        print("  lint      - Run Rust linting")
        print("  build     - Build with Maturin")
        print("  all       - Run all checks and build")
        sys.exit(1)

    command = sys.argv[1].lower()

    try:
        if command == "setup":
            builder.setup_rust()
        elif command == "check":
            builder.setup_rust()
            success = builder.test_rust_build()
            sys.exit(0 if success else 1)
        elif command == "test":
            builder.setup_rust()
            success = builder.run_rust_tests()
            sys.exit(0 if success else 1)
        elif command == "lint":
            builder.setup_rust()
            success = builder.run_rust_linting()
            sys.exit(0 if success else 1)
        elif command == "build":
            success = builder.build_with_maturin()
            sys.exit(0 if success else 1)
        elif command == "all":
            builder.setup_rust()

            # Run all checks
            checks_passed = True
            checks_passed &= builder.test_rust_build()
            checks_passed &= builder.run_rust_linting()
            checks_passed &= builder.run_rust_tests()

            if checks_passed:
                success = builder.build_with_maturin()
                sys.exit(0 if success else 1)
            else:
                print("✗ Some checks failed, skipping build")
                sys.exit(1)
        else:
            print(f"Unknown command: {command}")
            sys.exit(1)

    except RustBuildError as e:
        print(f"Rust build error: {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\nBuild interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
