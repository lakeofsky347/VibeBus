#!/usr/bin/env python3
"""Create a public, deterministic CycloneDX JSON document from cargo-cyclonedx.

cargo-cyclonedx represents the local workspace package with ``path+file``
references.  Those references include the build directory, which must not be
published in an SBOM.  This tool replaces local package references with their
package URLs, preserves every dependency edge, removes volatile metadata, and
rejects an output that still contains a local or absolute file reference.

It intentionally uses only Python's standard library so the CI gate has no
additional dependency to resolve.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
from typing import Any
from urllib.parse import parse_qsl, unquote, urlencode


JsonObject = dict[str, Any]
FILE_URI_RE = re.compile(r"(?i)(?:path\+)?file:(?://)?(?:[a-z]:)?[/\\]")


def normalize_purl(value: str) -> str:
    """Drop only local-file qualifiers while retaining package identity."""
    if "?" not in value:
        return value

    base, query = value.split("?", 1)
    retained = [
        (key, item)
        for key, item in parse_qsl(query, keep_blank_values=True)
        if not unquote(item).lower().startswith("file:")
    ]
    return base if not retained else f"{base}?{urlencode(retained)}"


def local_ref_replacement(reference: str, component: JsonObject) -> str:
    """Return a deterministic, non-local replacement for a path+file bom-ref."""
    purl = component.get("purl")
    if isinstance(purl, str) and purl and not FILE_URI_RE.search(purl):
        base = purl
    else:
        name = str(component.get("name", "component"))
        version = str(component.get("version", "unknown"))
        base = f"urn:cdx:component:{name}@{version}"

    fragment = reference.partition("#")[2]
    purl_identity = base.removeprefix("pkg:cargo/").split("?", 1)[0]
    suffix = fragment.removeprefix(purl_identity).strip()
    if suffix:
        normalized_suffix = re.sub(r"\s+", "-", suffix)
        return f"{base}#{normalized_suffix}"
    return base


def collect_local_reference_replacements(node: Any, replacements: dict[str, str]) -> None:
    if isinstance(node, dict):
        purl = node.get("purl")
        if isinstance(purl, str):
            node["purl"] = normalize_purl(purl)
        reference = node.get("bom-ref")
        if isinstance(reference, str) and reference.lower().startswith("path+file:"):
            replacements[reference] = local_ref_replacement(reference, node)
        for value in node.values():
            collect_local_reference_replacements(value, replacements)
    elif isinstance(node, list):
        for item in node:
            collect_local_reference_replacements(item, replacements)


def replace_references(node: Any, replacements: dict[str, str]) -> Any:
    if isinstance(node, dict):
        normalized: JsonObject = {}
        for key, value in node.items():
            if key in {"bom-ref", "ref"} and isinstance(value, str):
                normalized[key] = replacements.get(value, value)
            elif key in {"dependsOn", "provides"} and isinstance(value, list):
                normalized[key] = [replacements.get(item, item) if isinstance(item, str) else item for item in value]
            else:
                normalized[key] = replace_references(value, replacements)
        return normalized
    if isinstance(node, list):
        return [replace_references(item, replacements) for item in node]
    return node


def stable_order(node: Any) -> Any:
    """Keep semantic collections stable without altering component contents."""
    if isinstance(node, dict):
        ordered = {key: stable_order(value) for key, value in node.items()}
        components = ordered.get("components")
        if isinstance(components, list):
            ordered["components"] = sorted(
                components,
                key=lambda item: (
                    str(item.get("bom-ref", "")),
                    str(item.get("purl", "")),
                    str(item.get("name", "")),
                    str(item.get("version", "")),
                ) if isinstance(item, dict) else (str(item), "", "", ""),
            )
        dependencies = ordered.get("dependencies")
        if isinstance(dependencies, list):
            for dependency in dependencies:
                if isinstance(dependency, dict) and isinstance(dependency.get("dependsOn"), list):
                    dependency["dependsOn"] = sorted(dependency["dependsOn"])
            ordered["dependencies"] = sorted(
                dependencies,
                key=lambda item: str(item.get("ref", "")) if isinstance(item, dict) else str(item),
            )
        return ordered
    if isinstance(node, list):
        return [stable_order(item) for item in node]
    return node


def forbidden_path_fragments() -> list[str]:
    candidates = [Path.cwd(), Path.home()]
    for variable in ("GITHUB_WORKSPACE", "RUNNER_TEMP"):
        value = os.environ.get(variable)
        if value:
            candidates.append(Path(value))

    fragments: list[str] = []
    for candidate in candidates:
        try:
            fragment = candidate.resolve().as_posix()
        except OSError:
            fragment = str(candidate).replace("\\", "/")
        if fragment and fragment not in fragments:
            fragments.append(fragment)
    return fragments


def iter_strings(node: Any) -> list[str]:
    if isinstance(node, dict):
        return [item for value in node.values() for item in iter_strings(value)]
    if isinstance(node, list):
        return [item for value in node for item in iter_strings(value)]
    return [node] if isinstance(node, str) else []


def contains_absolute_path(value: str) -> bool:
    normalized = value.replace("\\", "/")
    if FILE_URI_RE.search(normalized):
        return True
    without_urls = re.sub(r"(?i)[a-z][a-z0-9+.-]*://[^\s]+", "", normalized)
    return bool(
        re.search(r"(^|[\s(=])/(?:[^\s]*)", without_urls)
        or re.search(r"(?<![a-z0-9])[a-z]:/(?:[^\s]*)", without_urls, flags=re.IGNORECASE)
    )


def assert_public_document(document: JsonObject) -> None:
    serialized = json.dumps(document, ensure_ascii=False, sort_keys=True)
    normalized = serialized.replace("\\", "/")
    violations = [fragment for fragment in forbidden_path_fragments() if fragment in normalized]
    if any(contains_absolute_path(value) for value in iter_strings(document)):
        violations.append("absolute file path")
    if violations:
        raise ValueError("SBOM retains forbidden local-path content: " + ", ".join(sorted(set(violations))))


def normalize_document(document: JsonObject) -> JsonObject:
    normalized = copy.deepcopy(document)
    normalized.pop("serialNumber", None)
    metadata = normalized.get("metadata")
    if isinstance(metadata, dict):
        metadata.pop("timestamp", None)

    replacements: dict[str, str] = {}
    collect_local_reference_replacements(normalized, replacements)
    normalized = replace_references(normalized, replacements)
    normalized = stable_order(normalized)
    assert_public_document(normalized)
    return normalized


def encoded_document(document: JsonObject) -> bytes:
    return (json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def normalize_file(input_path: Path, output_path: Path) -> str:
    document = json.loads(input_path.read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise ValueError("CycloneDX input must be a JSON object")
    normalized = normalize_document(document)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    payload = encoded_document(normalized)
    output_path.write_bytes(payload)
    digest = hashlib.sha256(payload).hexdigest()
    print(f"normalized CycloneDX SBOM: {output_path} sha256={digest}")
    return digest


def self_test() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory).resolve()
        local_ref = f"path+file://{root.as_posix()}#vibebus@0.10.0"
        binary_ref = f"{local_ref} bin-target-0"
        fixture: JsonObject = {
            "bomFormat": "CycloneDX",
            "specVersion": "1.6",
            "serialNumber": "urn:uuid:volatile",
            "metadata": {
                "timestamp": "2026-01-01T00:00:00Z",
                "component": {
                    "bom-ref": local_ref,
                    "name": "vibebus",
                    "version": "0.10.0",
                    "purl": "pkg:cargo/vibebus@0.10.0?download_url=file://.",
                    "components": [{
                        "bom-ref": binary_ref,
                        "name": "vibebus",
                        "version": "0.10.0",
                        "purl": "pkg:cargo/vibebus@0.10.0?download_url=file://.#src/main.rs",
                    }],
                },
            },
            "components": [],
            "dependencies": [
                {"ref": binary_ref, "dependsOn": [local_ref]},
                {"ref": local_ref, "dependsOn": []},
            ],
        }
        first = normalize_document(fixture)
        second = normalize_document(fixture)
        if encoded_document(first) != encoded_document(second):
            raise AssertionError("normalization output is not deterministic")
        root_ref = "pkg:cargo/vibebus@0.10.0"
        if first.get("serialNumber") is not None or first.get("metadata", {}).get("timestamp") is not None:
            raise AssertionError("volatile metadata was retained")
        if first["metadata"]["component"]["bom-ref"] != root_ref:
            raise AssertionError("local root bom-ref was not canonicalized")
        if first["dependencies"][0]["ref"] not in {root_ref, f"{root_ref}#bin-target-0"}:
            raise AssertionError("dependency references were not preserved")
        for leaked_value in ("file:///tmp/private", f"{root.as_posix()}/private"):
            leaking = copy.deepcopy(fixture)
            leaking["metadata"]["properties"] = [{"name": "leak", "value": leaked_value}]
            try:
                normalize_document(leaking)
            except ValueError:
                pass
            else:
                raise AssertionError("absolute file path was not rejected")
    print("normalize-cyclonedx self-test passed")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, help="cargo-cyclonedx JSON input")
    parser.add_argument("--output", type=Path, help="sanitized CycloneDX JSON output")
    parser.add_argument("--self-test", action="store_true", help="run the built-in deterministic fixture test")
    arguments = parser.parse_args()
    if not arguments.self_test and (arguments.input is None or arguments.output is None):
        parser.error("--input and --output are required unless --self-test is used")
    return arguments


def main() -> None:
    arguments = parse_args()
    if arguments.self_test:
        self_test()
        return
    normalize_file(arguments.input, arguments.output)


if __name__ == "__main__":
    main()
