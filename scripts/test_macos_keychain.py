#!/usr/bin/env python3
import json
import os
import pty
import subprocess
import sys
import tempfile
from pathlib import Path


AGENT = "macos-keychain-fixture"


def invoke(binary: Path, root: Path, data_home: Path, *arguments: str, expect_ok=True):
    completed = subprocess.run(
        [str(binary), "--root", str(root), "--data-home", str(data_home), *arguments],
        check=False,
        capture_output=True,
        text=True,
    )
    if expect_ok and completed.returncode != 0:
        raise RuntimeError(f"VibeBus command failed with exit code {completed.returncode}")
    if not expect_ok:
        return completed
    try:
        value = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError("VibeBus command did not return JSON") from error
    if value.get("ok") is not True:
        raise RuntimeError("VibeBus command returned an unsuccessful response")
    return value


def invoke_interactive(
    binary: Path, root: Path, data_home: Path, confirmation: str, *arguments: str
):
    master_fd, slave_fd = pty.openpty()
    try:
        process = subprocess.Popen(
            [str(binary), "--root", str(root), "--data-home", str(data_home), *arguments],
            stdin=slave_fd,
            stdout=subprocess.PIPE,
            stderr=slave_fd,
            text=True,
        )
        os.close(slave_fd)
        slave_fd = -1
        os.write(master_fd, f"{confirmation}\n".encode("utf-8"))
        try:
            stdout, _ = process.communicate(timeout=15)
        except subprocess.TimeoutExpired as error:
            process.kill()
            process.communicate()
            raise RuntimeError("interactive VibeBus command timed out") from error
        if process.returncode != 0:
            raise RuntimeError(
                f"interactive VibeBus command failed with exit code {process.returncode}"
            )
        try:
            value = json.loads(stdout)
        except json.JSONDecodeError as error:
            raise RuntimeError("interactive VibeBus command did not return JSON") from error
        if value.get("ok") is not True:
            raise RuntimeError("interactive VibeBus command returned an unsuccessful response")
        return value
    finally:
        if slave_fd >= 0:
            os.close(slave_fd)
        os.close(master_fd)


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: test_macos_keychain.py VIBEBUS_BINARY")
    binary = Path(sys.argv[1]).resolve()
    if not binary.is_file():
        raise SystemExit(f"VibeBus binary does not exist: {binary}")

    checks = 0
    with tempfile.TemporaryDirectory(prefix="vibebus-keychain-project-") as project_dir, tempfile.TemporaryDirectory(
        prefix="vibebus-keychain-data-"
    ) as data_dir:
        root = Path(project_dir)
        data_home = Path(data_dir)
        invoke(binary, root, data_home, "init", "--name", "macOS Keychain fixture")
        project_id = json.loads(
            (root / ".vibebus" / "project.json").read_text(encoding="utf-8")
        )["projectId"]
        try:
            registration = invoke(
                binary,
                root,
                data_home,
                "register",
                "--name",
                AGENT,
                "--role",
                "test",
                "--store-credentials",
            )["result"]
            assert registration["secretsRedacted"] is True
            assert "token" not in registration and "recoveryKey" not in registration
            assert registration["credentials"]["backend"] == "macos-keychain"
            assert registration["credentials"]["supported"] is True
            assert registration["credentials"]["stored"] is True
            checks += 1

            status = invoke(
                binary, root, data_home, "credential", "status", "--agent", AGENT
            )["result"]
            assert status["stored"] is True and status["tokenGeneration"] == 1
            checks += 1

            invoke(binary, root, data_home, "inbox", "--agent", AGENT)
            checks += 1

            recovery = invoke(
                binary,
                root,
                data_home,
                "agent",
                "recover",
                "--name",
                AGENT,
                "--store-credentials",
            )["result"]
            assert recovery["secretsRedacted"] is True
            assert recovery["tokenGeneration"] == 2
            assert "token" not in recovery and "recoveryKey" not in recovery
            checks += 1

            status = invoke(
                binary, root, data_home, "credential", "status", "--agent", AGENT
            )["result"]
            assert status["stored"] is True and status["tokenGeneration"] == 2
            checks += 1

            deleted = invoke(
                binary, root, data_home, "credential", "delete", "--agent", AGENT
            )["result"]
            assert deleted["deleted"] is True
            assert deleted["credentials"]["stored"] is False
            checks += 1

            rejected = invoke(
                binary, root, data_home, "inbox", "--agent", AGENT, expect_ok=False
            )
            assert rejected.returncode != 0
            checks += 1

            operator = invoke_interactive(
                binary, root, data_home, project_id, "operator", "init"
            )["result"]
            assert operator["secretRedacted"] is True
            assert "operatorSecret" not in operator
            assert operator["credential"]["backend"] == "macos-keychain"
            assert operator["credential"]["stored"] is True
            assert operator["generation"] == 1
            checks += 1

            operator_status = invoke(
                binary, root, data_home, "operator", "status"
            )["result"]
            assert operator_status["ready"] is True
            assert operator_status["operator"]["generation"] == 1
            assert operator_status["credential"]["generation"] == 1
            checks += 1

            rotated = invoke_interactive(
                binary,
                root,
                data_home,
                f"rotate:{project_id}",
                "operator",
                "rotate",
            )["result"]
            assert rotated["secretRedacted"] is True
            assert "operatorSecret" not in rotated
            assert rotated["generation"] == 2
            checks += 1

            operator_deleted = invoke_interactive(
                binary,
                root,
                data_home,
                f"delete:{project_id}",
                "operator",
                "delete-credential",
            )["result"]
            assert operator_deleted["deleted"] is True
            assert operator_deleted["ready"] is False
            assert operator_deleted["credential"]["stored"] is False
            assert operator_deleted["operator"]["generation"] == 2
            checks += 1
        finally:
            invoke(
                binary,
                root,
                data_home,
                "credential",
                "delete",
                "--agent",
                AGENT,
                expect_ok=False,
            )
            try:
                invoke_interactive(
                    binary,
                    root,
                    data_home,
                    f"delete:{project_id}",
                    "operator",
                    "delete-credential",
                )
            except Exception:
                pass

    print(
        json.dumps(
            {
                "ok": True,
                "checks": checks,
                "failures": 0,
                "credentialBackend": "macos-keychain",
                "operatorBackend": "macos-keychain",
                "cleanup": "verified",
            },
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
