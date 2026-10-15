#!/usr/bin/env python3
"""Guard hand-declared FFI surfaces against the C header the FFI backend actually emitted.

Two of our bindings restate every native signature by hand and nothing checks the restatement:
C# `DllImport` in `NativeMethods.cs` and Java Panama `FunctionDescriptor` in `NativeLib.java`.
Both compile clean when wrong, and misbehave only when the call runs. Go escapes the whole
family because cgo type-checks against the real header at build time -- which is also why this
went unnoticed for so long.

GH#1595. `xberg_registry_sample_bytes` takes six parameters and returns `int32_t`. C# declared
three parameters and a pointer-width `IntPtr` return; the call site null-checked that "pointer",
so a success status of 1 passed the guard and `Marshal.PtrToStringUTF8` dereferenced address 1
before `FreeString` freed it. **Java declared the same function identically wrong.** The first
diagnosis -- "a bad template in the C# emitter" -- was wrong: it is every unverified backend.

## The property

For every native symbol a binding declares, the declaration must match the header in ARITY, in
per-parameter width class, and in return width class. This is deliberately compared against the
emitted header rather than a type set the check builds for itself: a fixture can only catch
disagreements about a rule both sides already share, and so is structurally blind to arity, to
return pointer-ness, and to a function rendered by the wrong template -- which is every part of
GH#1595.

## Width classes, not exact spellings

`uintptr_t` and `UIntPtr` are both pointer-width and agree; comparing spellings would
report them and train readers to ignore output. Panama layout constants classify through
the same path, so one comparison serves both surfaces.

A type this script CANNOT read fails the check rather than being skipped. "I could not
read this" and "these agree" must never produce the same result. The original version
skipped, and that single choice hid two real defects at once: `XBERGAlefHandle` was
unreadable, so almost no parameter was compared while the run reported a clean pass over
282 declarations; and `void` was unreadable, so five `int32_t` returns declared `void` in
C# (GH#1596) compared as agreeing. Skipping is how a check becomes decorative.

Pointer-width is treated as interchangeable with a fixed 64 bits, which is what makes a
`uintptr_t` return declared as C# `ulong` (seven of them today) legitimate rather than a
finding. That equivalence is TRUE ONLY BECAUSE every shipped runtime identifier is 64-bit,
so it is derived from `runtime.json.template` at run time instead of being assumed. Ship a
32-bit RID and the equivalence is withdrawn automatically and those declarations start
failing -- which is correct, because on that target `ulong` really would be the wrong
width. The alternative, a static waiver list, would have gone quietly stale at exactly the
moment it mattered.

## What is deliberately NOT guarded

Parameter NAMES and exact signedness. The emitters legitimately differ on both -- `this_`
vs `handle`, `int32_t` vs `int` for a bool-ish status -- and encoding that would make this
a change-detector rather than an invariant.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
HEADER = REPO_ROOT / "crates/xberg-ffi/include/xberg.h"
NATIVE_METHODS = REPO_ROOT / "packages/csharp/src/Xberg/NativeMethods.cs"
RUNTIME_TEMPLATE = REPO_ROOT / "packages/csharp/Xberg/runtime.json.template"
JAVA_NATIVE_LIB = REPO_ROOT / "packages/java/io/xberg/NativeLib.java"

# RID architecture suffixes that are 32-bit. A RID whose suffix is absent from BOTH this set
# and the 64-bit set is treated as 32-bit, so an unrecognised architecture tightens the check
# rather than silently relaxing it. ~keep
_ARCH_64 = frozenset({"x64", "arm64"})
_ARCH_32 = frozenset({"x86", "arm"})

# Known-broken pairs that are NOT this script's job to fix: `packages/csharp` is
# alef-generated, so the repair belongs in alef's C# backend and a hand-patch here would be
# reverted by the next regen. Each entry must name its issue. Remove an entry when the
# upstream fix lands -- the check then proves the fix rather than merely asserting it. ~keep
_SAMPLE_BYTES = "GH#1595 -- bytes-returning fn rendered with the string template"
_INT32_AS_LONG = "GH#1597 -- int32_t return declared JAVA_LONG"
# The plugin registry entry points alef 0.89.0 began declaring: 511 Java symbols became
# 535, and all 24 new ones return `int32_t` in the header while Panama declares the return
# JAVA_LONG -- the same GH#1597 generator defect the four above already carry. Waived rather
# than blocking because every one of the 24 generated call sites narrows the result with
# `(int)(long)` before using it (OcrBackendBridge.java:561 and its siblings), so the real
# low 32 bits are what reaches the caller; the declaration is wrong, not the behaviour.
# Verified by asserting all 24 narrow -- re-check that before extending this list again. ~keep
_PLUGIN_REGISTRY_INT32 = tuple(
    f"xberg_{verb}_{plugin}"
    for verb in ("register", "unregister", "clear")
    for plugin in (
        "document_extractor",
        "embedding_backend",
        "ocr_backend",
        "post_processor",
        "renderer",
        "reranker_backend",
        "tokenizer_backend",
        "validator",
    )
)
KNOWN_BROKEN = {
    "C#": {},
    "java": {
        "xberg_registry_sample_bytes": _SAMPLE_BYTES,
        "xberg_last_error_code": _INT32_AS_LONG,
        "xberg_ocr_backend_supports_language": _INT32_AS_LONG,
        # Same defect as its sibling above, and narrowed the same way: XbergRs.java:841 wraps the
        # call in `(int)(long)`, exactly as :798 does for `..._supports_language`. ~keep
        "xberg_ocr_backend_supports_language_for": _INT32_AS_LONG,
        "xberg_registry_is_empty": _INT32_AS_LONG,
        "xberg_verify_excerpt": _INT32_AS_LONG,
        **dict.fromkeys(_PLUGIN_REGISTRY_INT32, _INT32_AS_LONG),
    },
}

_HEADER_FN = re.compile(r"([A-Za-z_][\w]*(?:\s+[A-Za-z_][\w]*)*\s*\**)\s*\b(xberg_\w+)\s*\(([^;]*?)\)\s*;", re.DOTALL)
_CSHARP_FN = re.compile(
    r'EntryPoint\s*=\s*"(\w+)"[^;]*?extern\s+([\w\.\*<>\[\]\?]+)\s+\w+\s*\(([^;]*?)\)\s*;', re.DOTALL
)
_ENTRY_POINT = re.compile(r'EntryPoint\s*=\s*"(\w+)"')
_JAVA_HANDLE = re.compile(
    r'LIB\.find\("(xberg_\w+)"\).*?FunctionDescriptor\.(of|ofVoid)\(([^;]*?)\)\s*\)\s*;', re.DOTALL
)
_JAVA_SYMBOL = re.compile(r'LIB\.find\("(xberg_\w+)"\)')


POINTER = "ptr"
VOID = "void"

# Panama layout constants to ABI width. An unlisted layout resolves to None and therefore
# FAILS, same as an unreadable C type -- a new layout must be classified deliberately
# rather than silently skipping every declaration that uses it. ~keep
_JAVA_LAYOUTS = {
    "ADDRESS": POINTER,
    "JAVA_LONG": 64,
    "JAVA_DOUBLE": 64,
    "JAVA_INT": 32,
    "JAVA_FLOAT": 32,
    "JAVA_SHORT": 16,
    "JAVA_CHAR": 16,
    "JAVA_BYTE": 8,
    "JAVA_BOOLEAN": 8,
}
_WIDTHS: tuple[tuple[str, object], ...] = (
    ("uint64_t", 64),
    ("int64_t", 64),
    ("uintptr_t", POINTER),
    ("intptr_t", POINTER),
    ("size_t", POINTER),
    ("nuint", POINTER),
    ("nint", POINTER),
    ("ulong", 64),
    ("long", 64),
    ("double", 64),
    ("uint32_t", 32),
    ("int32_t", 32),
    ("uint", 32),
    ("float", 32),
    ("int", 32),
    ("uint8_t", 8),
    ("byte", 8),
    ("bool", 8),
)


def split_params(raw: str) -> list[str]:
    parts = [p.strip() for p in raw.split(",") if p.strip()]
    return [] if parts == ["void"] else parts


_SCALAR_TYPEDEF = re.compile(r"typedef\s+(\w[\w ]*?)\s+(\w+)\s*;")


def scalar_typedefs(header_source: str) -> dict[str, str]:
    """Map header typedef names to their underlying scalar spelling.

    `XBERGAlefHandle` is `uint64_t`, and it is the parameter type of very nearly every
    exported function. Leaving it unclassified made `width_class` return None for it, which
    this script treats as "skip" -- so every handle parameter went uncompared and a handle
    declared at the wrong width would have passed silently. That is the exact shape this
    check exists to catch, and a negative control is what exposed it. Resolve typedefs from
    the header rather than hardcoding the name, so a renamed or re-widened handle is picked
    up without editing this script. Opaque `typedef struct X X;` lines resolve to a
    non-scalar and are correctly left unclassified. ~keep
    """
    resolved = {}
    for underlying, name in _SCALAR_TYPEDEF.findall(header_source):
        underlying = underlying.strip()
        if underlying.startswith(("struct", "enum")):
            continue
        resolved[name] = underlying
    return resolved


def width_class(declaration: str, typedefs: dict[str, str] | None = None) -> object | None:
    """Classify a parameter or return type by ABI width, VOID, or None when unreadable.

    None means "this script could not read the type", and callers must treat that as a
    FAILURE rather than skipping the pair. "I could not read this" and "these agree" must
    never produce the same result -- that equivalence is what let a skipped
    `XBERGAlefHandle` hide every handle parameter from comparison. `void` is a real
    classification, not an unreadable one, so it is returned explicitly. ~keep
    """
    text = declaration.replace("const", "").strip()
    if text == "void":
        return VOID
    if text in _JAVA_LAYOUTS:
        return _JAVA_LAYOUTS[text]
    if "*" in text or "IntPtr" in text or "[]" in text or "string" in text:
        return POINTER
    if typedefs:
        for name, underlying in typedefs.items():
            if re.search(rf"\b{re.escape(name)}\b", text):
                text = f"{text} {underlying}"
                break
    for spelling, width in _WIDTHS:
        if re.search(rf"\b{spelling}\b", text):
            return width
    return None


def shipped_rids() -> list[str]:
    import json

    return sorted(json.loads(RUNTIME_TEMPLATE.read_text())["runtimes"])


def all_shipped_rids_are_64_bit(rids: list[str]) -> tuple[bool, list[str]]:
    """Report whether every shipped RID is 64-bit, naming the ones that are not."""
    not_64 = []
    for rid in rids:
        arch = rid.rsplit("-", 1)[-1]
        if arch in _ARCH_64:
            continue
        not_64.append(rid if arch in _ARCH_32 else f"{rid} (unrecognised arch `{arch}`)")
    return (not not_64), not_64


def widths_agree(left: object, right: object, pointer_is_64: bool) -> bool:
    return left == right or (pointer_is_64 and {left, right} == {POINTER, 64})


def parse_header(source: str) -> dict[str, tuple[str, list[str]]]:
    return {
        match.group(2): (match.group(1).strip(), split_params(match.group(3))) for match in _HEADER_FN.finditer(source)
    }


def parse_csharp(source: str) -> dict[str, tuple[str, list[str]]]:
    return {
        match.group(1): (match.group(2).strip(), split_params(match.group(3))) for match in _CSHARP_FN.finditer(source)
    }


def parse_java(source: str) -> dict[str, tuple[str, list[str]]]:
    """Map each Panama downcall handle to (return layout, [parameter layouts]).

    Two declaration shapes exist in the generated file: a long one with `.or(...)` symbol
    fallbacks ending in `.orElseThrow(...)`, and a short `.map(s -> ...).orElse(null)` one.
    A regex anchored on the trailing punctuation matched only the long shape and silently
    dropped the five functions using the short one -- so the descriptor args are located by
    scanning to the balanced closing paren instead, which is indifferent to what follows.

    `FunctionDescriptor.of(ret, args...)` puts the return first; `ofVoid(args...)` has none.
    Layout names are returned verbatim so `width_class` classifies them exactly as it does a
    C spelling, which keeps one comparison path for both surfaces. ~keep
    """
    parsed: dict[str, tuple[str, list[str]]] = {}
    for match in _JAVA_SYMBOL.finditer(source):
        name = match.group(1)
        descriptor = re.search(r"FunctionDescriptor\.(of|ofVoid)\(", source[match.end() :])
        if descriptor is None:
            continue
        cursor = match.end() + descriptor.end()
        depth, index = 1, cursor
        while index < len(source) and depth:
            if source[index] == "(":
                depth += 1
            elif source[index] == ")":
                depth -= 1
            index += 1
        layouts = re.findall(r"ValueLayout\.(\w+)", source[cursor : index - 1])
        if descriptor.group(1) == "ofVoid":
            parsed[name] = ("void", layouts)
        elif layouts:
            parsed[name] = (layouts[0], layouts[1:])
    return parsed


def compare(
    name: str,
    header: tuple[str, list[str]],
    managed: tuple[str, list[str]],
    *,
    pointer_is_64: bool,
    typedefs: dict[str, str],
    surface: str = "C#",
) -> list[str]:
    header_return, header_params = header
    managed_return, managed_params = managed
    if len(header_params) != len(managed_params):
        return [
            (
                f"{name}: arity -- header declares {len(header_params)} parameter(s), "
                f"{surface} declares {len(managed_params)}"
            )
        ]
    problems = []
    header_width = width_class(header_return, typedefs)
    managed_width = width_class(managed_return, typedefs)
    if header_width is None or managed_width is None:
        problems.append(f"{name}: return -- unreadable type, header `{header_return}` vs {surface} `{managed_return}`")
    elif not widths_agree(header_width, managed_width, pointer_is_64):
        problems.append(f"{name}: return -- header `{header_return}` vs {surface} `{managed_return}`")
    for index, (native, csharp) in enumerate(zip(header_params, managed_params, strict=True)):
        native_width = width_class(native, typedefs)
        csharp_width = width_class(csharp, typedefs)
        if native_width is None or csharp_width is None:
            problems.append(f"{name}: parameter {index} -- unreadable type, header `{native}` vs {surface} `{csharp}`")
        elif not widths_agree(native_width, csharp_width, pointer_is_64):
            problems.append(f"{name}: parameter {index} -- header `{native}` vs {surface} `{csharp}`")
    return problems


def _check_surface_coverage(
    surface: str,
    declared: set[str],
    managed_functions: dict[str, tuple[str, list[str]]],
    header_functions: dict[str, tuple[str, list[str]]],
) -> tuple[int, list[str]]:
    # Coverage is asserted, not assumed. An earlier hand-run of this comparison matched only
    # 210 of 282 C# entrypoints -- the header pattern required whitespace before the function
    # name and so skipped every `char *xberg_...` declaration -- and still reported the one
    # real defect. A partial scan and a complete one agreeing is luck. A scan that silently
    # examines a subset is the failure mode this check exists to prevent, so refuse to pass
    # rather than report a clean result over an unknown fraction of the surface. ~keep
    # A symbol this script found but could not parse a declaration for must FATAL, not be
    # dropped. Indexing straight into managed_functions raised KeyError the first time the
    # Java parser missed five short-form handles -- a crash at least announced itself, but
    # the same gap in a quieter place would have silently shrunk the compared set. ~keep
    unparsed = sorted(declared - set(managed_functions))
    if unparsed:
        print(f"[{surface}] FATAL: {len(unparsed)} symbol(s) found but not parsed into a declaration:")
        for name in unparsed[:20]:
            print(f"  {name}")
        print("This is a defect in this script's parser, not in the binding. Fix the parser.")
        return 1, []

    comparable = sorted(declared & set(header_functions))
    unmatched = sorted(declared - set(header_functions))
    print(f"[{surface}] declared: {len(declared)}   compared: {len(comparable)}")
    if unmatched:
        print(f"[{surface}] FATAL: {len(unmatched)} symbol(s) have no parseable header declaration:")
        for name in unmatched[:20]:
            print(f"  {name}")
        print("Either the symbol does not exist (a real defect) or this script's header")
        print("pattern missed it (a defect in this script). Both must be resolved, not skipped.")
        return 1, []
    if not comparable:
        print(f"[{surface}] FATAL: nothing was compared", file=sys.stderr)
        return 2, []

    return 0, comparable


def check_surface(
    surface: str,
    declared: set[str],
    managed_functions: dict[str, tuple[str, list[str]]],
    header_functions: dict[str, tuple[str, list[str]]],
    *,
    pointer_is_64: bool,
    typedefs: dict[str, str],
) -> tuple[int, list[str], list[str]]:
    """Compare one binding surface against the header. Returns (exit, failures, waived)."""
    waivers = KNOWN_BROKEN.get(surface, {})

    code, comparable = _check_surface_coverage(surface, declared, managed_functions, header_functions)
    if code:
        return code, [], []

    failures: list[str] = []
    waived: list[str] = []
    stale: list[str] = []
    for name in comparable:
        problems = compare(
            name,
            header_functions[name],
            managed_functions[name],
            pointer_is_64=pointer_is_64,
            typedefs=typedefs,
            surface=surface,
        )
        if not problems:
            if name in waivers:
                stale.append(name)
            continue
        if name in waivers:
            waived.extend(f"[{surface}] {problem}  [waived: {waivers[name]}]" for problem in problems)
        else:
            failures.extend(f"[{surface}] {problem}" for problem in problems)

    if stale:
        print(f"[{surface}] FATAL: waivers no longer reproduce and must be removed:")
        for name in stale:
            print(f"  {name} -- {waivers[name]}")
        return 1, failures, waived
    return 0, failures, waived


def main() -> int:
    for path in (HEADER, NATIVE_METHODS, RUNTIME_TEMPLATE, JAVA_NATIVE_LIB):
        if not path.is_file():
            print(f"FATAL: {path} not found", file=sys.stderr)
            return 2

    rids = shipped_rids()
    pointer_is_64, not_64 = all_shipped_rids_are_64_bit(rids)
    if pointer_is_64:
        print(f"all {len(rids)} shipped RID(s) are 64-bit; pointer-width == 64 for this check")
    else:
        print(f"32-bit RID(s) shipped ({', '.join(not_64)}); pointer-width is NOT 64")
        print("Declarations spelling a pointer-width native type as a fixed-64 managed type")
        print("are now reported -- on those targets the width genuinely differs.")

    header_source = HEADER.read_text()
    typedefs = scalar_typedefs(header_source)
    header_functions = parse_header(header_source)
    print(f"header declarations parsed: {len(header_functions)}")

    csharp_source = NATIVE_METHODS.read_text()
    java_source = JAVA_NATIVE_LIB.read_text()
    surfaces = (
        ("C#", set(_ENTRY_POINT.findall(csharp_source)), parse_csharp(csharp_source)),
        ("java", set(_JAVA_SYMBOL.findall(java_source)), parse_java(java_source)),
    )

    exit_code = 0
    all_failures: list[str] = []
    all_waived: list[str] = []
    for surface, declared, managed in surfaces:
        code, failures, waived = check_surface(
            surface,
            declared,
            managed,
            header_functions,
            pointer_is_64=pointer_is_64,
            typedefs=typedefs,
        )
        exit_code = max(exit_code, code)
        all_failures.extend(failures)
        all_waived.extend(waived)

    for line in all_waived:
        print(f"WAIVED  {line}")
    if all_failures:
        print(f"FATAL: {len(all_failures)} declaration/header disagreement(s):")
        for line in all_failures:
            print(f"  {line}")
        return 1
    if exit_code:
        return exit_code

    print(f"OK: every declaration agrees with the emitted header ({len(all_waived)} waived finding(s))")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
