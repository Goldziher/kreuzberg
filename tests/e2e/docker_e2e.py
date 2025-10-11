import argparse
import asyncio
import json
import random
import string
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

DOCKER_IMAGES = {
    "base": "kreuzberg:base",
    "core": "kreuzberg:core",
    "full": "kreuzberg:full",
}

OPTIONAL_IMAGES: set[str] = set()

SECURITY_CONFIG = {
    "max_container_runtime": 300,
    "max_memory": "2g",
    "max_cpu": "1.0",
    "readonly_volumes": True,
    "security_opt": ["no-new-privileges"],
}

TEST_DIR = Path(__file__).parent.parent
TEST_FILES_DIR = TEST_DIR.parent / "test_documents"

test_results: dict[str, dict[str, Any]] = {}


def run_command(cmd: list[str], timeout: int = 30) -> tuple[int, str, str]:
    try:
        result = subprocess.run(cmd, check=False, capture_output=True, text=True, timeout=timeout, encoding="utf-8")
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "Command timed out"
    except Exception as e:
        return -1, "", str(e)


async def run_command_async(cmd: list[str], timeout: int = 30) -> tuple[int, str, str]:
    try:
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )

        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
            return proc.returncode or 0, stdout.decode(), stderr.decode()
        except asyncio.TimeoutError:
            proc.kill()
            await proc.communicate()
            return -1, "", f"Command timed out after {timeout} seconds"

    except Exception as e:
        return -1, "", str(e)


def generate_random_container_name() -> str:
    suffix = "".join(random.choices(string.ascii_lowercase + string.digits, k=8))
    return f"kreuzberg-test-{suffix}"


def test_image_exists(image_name: str) -> bool:
    cmd = ["docker", "images", "-q", image_name]
    exit_code, stdout, _ = run_command(cmd)
    return exit_code == 0 and stdout.strip() != ""


def test_cli_help(image_name: str) -> bool:
    cmd = [
        "docker",
        "run",
        "--rm",
        "--security-opt",
        "no-new-privileges",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "--help",
    ]
    exit_code, stdout, _stderr = run_command(cmd)
    success = exit_code == 0 and "Text extraction from documents" in stdout
    if not success:
        pass
    return success


def test_cli_version(image_name: str) -> bool:
    cmd = [
        "docker",
        "run",
        "--rm",
        "--security-opt",
        "no-new-privileges",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "--version",
    ]
    exit_code, stdout, _stderr = run_command(cmd)
    success = exit_code == 0 and "kreuzberg" in stdout.lower()
    if not success:
        pass
    return success


def test_api_health(image_name: str) -> bool:
    container_name = generate_random_container_name()
    port = random.randint(9000, 9999)

    cmd = [
        "docker",
        "run",
        "-d",
        "--name",
        container_name,
        "--memory",
        str(SECURITY_CONFIG["max_memory"]),
        "--cpus",
        str(SECURITY_CONFIG["max_cpu"]),
        "--security-opt",
        "no-new-privileges",
        "-p",
        f"{port}:8000",
        image_name,
    ]
    exit_code, _container_id, _stderr = run_command(cmd)
    if exit_code != 0:
        return False

    try:
        time.sleep(5)

        import urllib.request

        try:
            response = urllib.request.urlopen(f"http://localhost:{port}/health", timeout=5)
            data = json.loads(response.read().decode())
            status_ok = data.get("status") == "ok" if isinstance(data, dict) else False
            return response.status == 200 and status_ok
        except Exception:
            return False
    finally:
        run_command(["docker", "stop", container_name], timeout=10)
        run_command(["docker", "rm", container_name], timeout=10)


def test_file_extraction(image_name: str, test_file: str) -> bool:
    with tempfile.TemporaryDirectory():
        test_file_path = TEST_FILES_DIR / test_file
        if not test_file_path.exists():
            return False

        cmd = [
            "docker",
            "run",
            "--rm",
            "--memory",
            "512m",
            "--cpus",
            "0.5",
            "--security-opt",
            "no-new-privileges",
            "-v",
            f"{test_file_path.parent}:/data:ro",
            image_name,
            "python",
            "-m",
            "kreuzberg",
            "extract",
            f"/data/{test_file_path.name}",
        ]
        exit_code, stdout, stderr = run_command(cmd, timeout=60)

        success = exit_code == 0 and len(stdout) > 10

        if success:
            sensitive_patterns = ["/root", "/home", "/etc/passwd", "/proc"]
            for pattern in sensitive_patterns:
                if pattern in stdout:
                    success = False
                    break

        if not success and stderr:
            pass
        return success


def test_ocr_extraction(image_name: str, image_variant: str) -> bool:
    test_file = "images/ocr_image.jpg"
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    cmd = [
        "docker",
        "run",
        "--rm",
        "--memory",
        "1g",
        "--cpus",
        "1.0",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{test_file_path.parent}:/data:ro",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "extract",
        f"/data/{test_file_path.name}",
        "--force-ocr",
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=120)

    success = exit_code == 0 and len(stdout) > 20
    if not success and stderr:
        pass
    return success


def test_api_extraction(image_name: str) -> bool:
    container_name = generate_random_container_name()
    port = random.randint(9000, 9999)

    cmd = [
        "docker",
        "run",
        "-d",
        "--name",
        container_name,
        "--memory",
        str(SECURITY_CONFIG["max_memory"]),
        "--cpus",
        str(SECURITY_CONFIG["max_cpu"]),
        "--security-opt",
        "no-new-privileges",
        "-p",
        f"{port}:8000",
        image_name,
    ]
    exit_code, _container_id, stderr = run_command(cmd)
    if exit_code != 0:
        return False

    try:
        time.sleep(5)

        test_content = "Test content for API extraction"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
            f.write(test_content)
            temp_file = f.name

        try:
            cmd = ["curl", "-s", "-X", "POST", f"http://localhost:{port}/extract", "-F", f"files=@{temp_file}"]
            exit_code, stdout, stderr = run_command(cmd, timeout=30)

            if exit_code == 0:
                try:
                    response = json.loads(stdout)
                    if isinstance(response, list) and len(response) > 0:
                        content = response[0].get("content", "")
                        success = test_content in content
                    else:
                        success = False
                except json.JSONDecodeError:
                    success = False
            else:
                success = False

            if not success and stderr:
                pass
            return success

        finally:
            Path(temp_file).unlink()

    finally:
        run_command(["docker", "stop", container_name], timeout=10)
        run_command(["docker", "rm", container_name], timeout=10)


def test_table_extraction(image_name: str) -> bool:
    test_file = "pdfs_with_tables/tiny.pdf"
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    cmd = [
        "docker",
        "run",
        "--rm",
        "--memory",
        "1g",
        "--cpus",
        "1.0",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{test_file_path.parent.parent}:/data:ro",
        image_name,
        "python",
        "-c",
        """
import asyncio
from kreuzberg import extract_file, ExtractionConfig

async def main():
    from kreuzberg._types import TableExtractionConfig
    result = await extract_file(
        '/data/pdfs_with_tables/tiny.pdf',
        config=ExtractionConfig(tables=TableExtractionConfig())
    )
    print(f"Tables found: {len(result.tables)}")
    return len(result.tables) > 0

success = asyncio.run(main())
exit(0 if success else 1)
""",
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=60)
    success = exit_code == 0 and "Tables found:" in stdout
    if not success and stderr:
        pass
    return success


def test_volume_security(image_name: str) -> bool:
    with tempfile.TemporaryDirectory() as tmpdir:
        test_file = Path(tmpdir) / "test.txt"
        test_content = "Test content for volume security"
        test_file.write_text(test_content)

        cmd = [
            "docker",
            "run",
            "--rm",
            "-v",
            f"{tmpdir}:/data:ro",
            image_name,
            "python",
            "-c",
            "import os; print(os.access('/data/test.txt', os.W_OK))",
        ]

        exit_code, stdout, _ = run_command(cmd)
        return exit_code == 0 and stdout.strip() == "False"


def test_resource_limits(image_name: str) -> bool:
    cmd = [
        "docker",
        "run",
        "--rm",
        "--memory",
        "256m",
        "--memory-swap",
        "256m",
        image_name,
        "python",
        "-c",
        "import sys; print(sys.maxsize > 0)",
    ]

    exit_code, stdout, _ = run_command(cmd)
    return exit_code == 0 and stdout.strip() == "True"


def test_malicious_input_handling(image_name: str) -> bool:
    with tempfile.TemporaryDirectory() as tmpdir:
        cmd = [
            "docker",
            "run",
            "--rm",
            "-v",
            f"{tmpdir}:/data:ro",
            image_name,
            "python",
            "-m",
            "kreuzberg",
            "extract",
            "/data/../etc/passwd",
        ]
        exit_code, stdout, _ = run_command(cmd)

        return exit_code != 0 or "passwd" not in stdout


def test_libreoffice_extraction(image_name: str) -> bool:
    test_file = "legacy_office/unit_test_lists.doc"
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    cmd = [
        "docker",
        "run",
        "--rm",
        "--memory",
        "1g",
        "--cpus",
        "1.0",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{TEST_FILES_DIR}:/data:ro",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "extract",
        f"/data/{test_file}",
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=120)
    success = exit_code == 0 and len(stdout) > 20

    if not success and stderr:
        pass
    return success


def test_chunking_extraction(image_name: str) -> bool:
    test_file = "text/contract.txt"
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    cmd = [
        "docker",
        "run",
        "--rm",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{TEST_FILES_DIR}:/data:ro",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "extract",
        f"/data/{test_file}",
        "--chunk-content",
        "--max-chars",
        "500",
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=60)
    success = exit_code == 0 and len(stdout) > 10

    if not success and stderr:
        pass
    return success


def test_langdetect_extraction(image_name: str) -> bool:
    test_file = "text/contract.txt"
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    cmd = [
        "docker",
        "run",
        "--rm",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{TEST_FILES_DIR}:/data:ro",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "extract",
        f"/data/{test_file}",
        "--auto-detect-language",
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=60)
    success = exit_code == 0 and len(stdout) > 10

    if not success and stderr:
        pass
    return success


def test_email_extraction(image_name: str) -> bool:
    test_file = "email/html_only.eml"
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    cmd = [
        "docker",
        "run",
        "--rm",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{TEST_FILES_DIR}:/data:ro",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "extract",
        f"/data/{test_file}",
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=60)
    success = exit_code == 0 and len(stdout) > 10

    if not success and stderr:
        pass
    return success


def test_entity_extraction(image_name: str) -> bool:
    test_file = "text/contract.txt"
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    cmd = [
        "docker",
        "run",
        "--rm",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{TEST_FILES_DIR}:/data:ro",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "extract",
        f"/data/{test_file}",
        "--extract-entities",
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=120)
    success = exit_code == 0 and len(stdout) > 10

    if not success and stderr:
        pass
    return success


def run_tests_for_image(image_variant: str, image_name: str) -> dict[str, bool]:
    results = {}

    results["exists"] = test_image_exists(image_name)
    if not results["exists"]:
        return results

    results["cli_help"] = test_cli_help(image_name)
    results["cli_version"] = test_cli_version(image_name)
    results["api_health"] = test_api_health(image_name)

    results["extract_txt"] = test_file_extraction(image_name, "text/contract.txt")
    results["extract_pdf"] = test_file_extraction(image_name, "pdfs/searchable.pdf")
    results["extract_docx"] = test_file_extraction(image_name, "office/document.docx")

    results["ocr"] = test_ocr_extraction(image_name, image_variant)

    results["api_extract"] = test_api_extraction(image_name)

    if image_variant in ("core", "full"):
        results["chunking"] = test_chunking_extraction(image_name)
        results["langdetect"] = test_langdetect_extraction(image_name)
        results["email"] = test_email_extraction(image_name)

    if image_variant == "full":
        results["libreoffice"] = test_libreoffice_extraction(image_name)
        results["entities"] = test_entity_extraction(image_name)

    results["volume_security"] = test_volume_security(image_name)
    results["resource_limits"] = test_resource_limits(image_name)
    results["malicious_input"] = test_malicious_input_handling(image_name)

    return results


def print_summary(all_results: dict[str, dict[str, bool]]) -> bool:
    total_tests = 0
    total_passed = 0

    for results in all_results.values():
        if not results.get("exists", False):
            continue

        passed = sum(1 for v in results.values() if v)
        total = len(results)
        total_tests += total
        total_passed += passed

        failed_tests = [test for test, result in results.items() if not result]
        if failed_tests:
            pass

    success_rate = (total_passed / total_tests * 100) if total_tests > 0 else 0

    return success_rate == 100


def main() -> None:
    parser = argparse.ArgumentParser(description="Kreuzberg Docker E2E Tests")
    parser.add_argument("--image", help="Test a specific image variant or full image name", default=None)
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    args = parser.parse_args()

    exit_code, _, _ = run_command(["docker", "--version"])
    if exit_code != 0:
        sys.exit(1)

    all_results = {}

    if args.image:
        if args.image in DOCKER_IMAGES:
            variant = args.image
            image_name = DOCKER_IMAGES[variant]
            all_results[variant] = run_tests_for_image(variant, image_name)
        else:
            image_name = args.image
            variant = image_name.split(":")[-1] if ":" in image_name else "custom"
            all_results[variant] = run_tests_for_image(variant, image_name)
    else:
        for image_variant, image_name in DOCKER_IMAGES.items():
            all_results[image_variant] = run_tests_for_image(image_variant, image_name)

    report_file = TEST_DIR / "e2e" / "test_report.json"
    report_file.parent.mkdir(parents=True, exist_ok=True)
    with report_file.open("w") as f:
        json.dump(all_results, f, indent=2, default=str)

    success = print_summary(all_results)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
