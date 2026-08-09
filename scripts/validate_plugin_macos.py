#!/usr/bin/env python3
"""Compatibility entry point for the shared macOS plugin validator."""

import json
import sys
from pathlib import Path

from plugin_package import normalize_arch, validate_plugin


def main() -> None:
    if len(sys.argv) not in (2, 3):
        raise SystemExit("usage: validate_plugin_macos.py PLUGIN_ROOT [MARKETPLACE_PATH]")
    repo_root = Path(__file__).resolve().parents[1]
    result = validate_plugin(
        repo_root,
        Path(sys.argv[1]),
        "macos",
        normalize_arch(),
        "cargo",
        Path(sys.argv[2]) if len(sys.argv) == 3 else None,
        "stage",
    )
    print(json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
    main()
