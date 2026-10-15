import contextlib
import importlib.util
import io
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-ffi-declaration-parity.py")
SPEC = importlib.util.spec_from_file_location("ffi_declaration_parity", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)
Signature = tuple[str, list[str]]


class DeclarationParityTests(unittest.TestCase):
    def check_surface(
        self, declared: set[str], managed: dict[str, Signature], header: dict[str, Signature], surface: str = "C#"
    ) -> tuple[tuple[int, list[str], list[str]], str, str]:
        stdout, stderr = (io.StringIO(), io.StringIO())
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            result = CHECKER.check_surface(surface, declared, managed, header, pointer_is_64=True, typedefs={})
        return (result, stdout.getvalue(), stderr.getvalue())

    def test_should_reject_declared_symbols_missing_from_managed_parser(self) -> None:
        result, stdout, stderr = self.check_surface({"xberg_missing"}, {}, {})
        assert result == (1, [], [])
        assert "1 symbol(s) found but not parsed into a declaration" in stdout
        assert "  xberg_missing\n" in stdout
        assert stderr == ""

    def test_should_reject_managed_symbols_missing_from_header(self) -> None:
        signature = ("int32_t", [])
        result, stdout, stderr = self.check_surface({"xberg_missing"}, {"xberg_missing": signature}, {})
        assert result == (1, [], [])
        assert "declared: 1   compared: 0" in stdout
        assert "1 symbol(s) have no parseable header declaration" in stdout
        assert stderr == ""

    def test_should_reject_empty_surface_instead_of_reporting_vacuous_success(self) -> None:
        result, stdout, stderr = self.check_surface(set(), {}, {})
        assert result == (2, [], [])
        assert stdout == "[C#] declared: 0   compared: 0\n"
        assert stderr == "[C#] FATAL: nothing was compared\n"

    def test_should_report_sorted_comparisons_and_unwaived_mismatches(self) -> None:
        declared = {"xberg_z", "xberg_a"}
        header = dict.fromkeys(declared, ("int32_t", []))
        managed = dict.fromkeys(declared, ("long", []))
        result, stdout, stderr = self.check_surface(declared, managed, header)
        assert result == (
            0,
            [
                "[C#] xberg_a: return -- header `int32_t` vs C# `long`",
                "[C#] xberg_z: return -- header `int32_t` vs C# `long`",
            ],
            [],
        )
        assert stdout == "[C#] declared: 2   compared: 2\n"
        assert stderr == ""

    def test_should_keep_only_reproducing_waivers(self) -> None:
        name = "xberg_last_error_code"
        result, stdout, stderr = self.check_surface({name}, {name: ("JAVA_LONG", [])}, {name: ("int32_t", [])}, "java")
        assert result == (
            0,
            [],
            [f"[java] {name}: return -- header `int32_t` vs java `JAVA_LONG`  [waived: {CHECKER._INT32_AS_LONG}]"],
        )
        assert stdout == "[java] declared: 1   compared: 1\n"
        assert stderr == ""

    def test_should_reject_stale_waivers(self) -> None:
        name = "xberg_last_error_code"
        signature = ("int32_t", [])
        result, stdout, stderr = self.check_surface({name}, {name: signature}, {name: signature}, "java")
        assert result == (1, [], [])
        assert "waivers no longer reproduce and must be removed" in stdout
        assert f"  {name} -- {CHECKER._INT32_AS_LONG}" in stdout
        assert stderr == ""

    def test_should_detect_arity_width_and_unreadable_types(self) -> None:
        for header, managed, expected in [
            (("int32_t", ["int32_t"]), ("int", []), "arity"),
            (("int32_t", []), ("void", []), "return"),
            (("opaque_type", []), ("int", []), "return -- unreadable type"),
            (("void", ["int32_t"]), ("void", ["long"]), "parameter 0"),
            (("void", ["opaque_type"]), ("void", ["int"]), "parameter 0 -- unreadable type"),
        ]:
            with self.subTest(header=header, managed=managed):
                result, _, _ = self.check_surface({"xberg_test"}, {"xberg_test": managed}, {"xberg_test": header})
                assert result[0] == 0
                assert len(result[1]) == 1
                assert f"xberg_test: {expected}" in result[1][0]
                assert result[2] == []
