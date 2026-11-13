#!/usr/bin/env python3
"""Check that packaged Ryurust libraries load and export what the C# wrappers use.

Usage: scripts/check-exports.py <lib-dir> [bindings-dir]

Every `bindings/csharp/Native*.cs` names its library (`LibraryName = "ryu_..."`)
and the exports it looks up (`GetExport<...>(handle, "ryu_...")`). For each
wrapper this loads the library from <lib-dir> (`libryu_*.so` on Linux,
`ryu_*.dll` on Windows) and checks every export. Loading also proves the
library's own dependencies resolve, e.g. that a Windows DLL doesn't need a
Visual C++ runtime that isn't installed. Exits non-zero on any failure.
"""

import ctypes
import re
import sys
from pathlib import Path


def library_file(lib_dir: Path, name: str) -> Path:
    if sys.platform == "win32":
        return lib_dir / f"{name}.dll"
    if sys.platform == "darwin":
        return lib_dir / f"lib{name}.dylib"
    return lib_dir / f"lib{name}.so"


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    lib_dir = Path(sys.argv[1]).resolve()
    bindings = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).parent.parent / "bindings" / "csharp"

    failures = 0
    checked = 0
    for wrapper in sorted(bindings.glob("Native*.cs")):
        text = wrapper.read_text(encoding="utf-8")
        lib = re.search(r'LibraryName\s*=\s*"([a-z0-9_]+)"', text)
        if not lib:
            print(f"{wrapper.name}: no LibraryName", file=sys.stderr)
            failures += 1
            continue
        exports = re.findall(r'GetExport<\w+>\(handle,\s*"([a-z0-9_]+)"\)', text)
        path = library_file(lib_dir, lib.group(1))
        try:
            handle = ctypes.CDLL(str(path))
        except OSError as e:
            print(f"FAIL {path.name}: does not load: {e}")
            failures += 1
            continue
        missing = [name for name in exports if not hasattr(handle, name)]
        checked += len(exports)
        if missing:
            print(f"FAIL {path.name}: missing {', '.join(missing)}")
            failures += 1
        else:
            print(f"ok   {path.name}: {len(exports)} exports")

    print(f"{checked} exports checked, {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
