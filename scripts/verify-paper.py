#!/usr/bin/env python3
"""Run the built app's synthetic-only check without launching a window."""
import json
import os
from pathlib import Path
import plistlib
import selectors
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass

PROTOCOL = b"metten-paper-check-v1"
OUTPUT_LIMIT = 65536


@dataclass
class Result:
    code: int
    output: str
    workspace: Path | None = None
    process: subprocess.Popen | None = None


def executable(app):
    contents = Path(app).resolve() / "Contents"
    with (contents / "Info.plist").open("rb") as stream:
        name = plistlib.load(stream).get("CFBundleExecutable")
    if not isinstance(name, str) or not name or Path(name).name != name:
        raise ValueError("Invalid bundle executable")
    binary = (contents / "MacOS" / name).resolve()
    if binary.parent != (contents / "MacOS").resolve() or not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError("Bundle executable unavailable")
    found = False
    with binary.open("rb") as stream:
        overlap = b""
        while chunk := stream.read(1024 * 1024):
            if PROTOCOL in overlap + chunk:
                found = True
                break
            overlap = chunk[-len(PROTOCOL):]
    if not found:
        raise ValueError("This bundle has no headless paper-check protocol marker")
    return binary


def check_database(workspace):
    with sqlite3.connect(f"file:{workspace / 'session.sqlite'}?mode=ro", uri=True) as db:
        if db.execute("PRAGMA user_version").fetchone() != (5,):
            raise ValueError("Unexpected paper schema")
        if db.execute("SELECT cash FROM paper_runs ORDER BY run_id").fetchall() != [(10736870000,), (17368750000,)]:
            raise ValueError("Paper cash did not reconcile")
        if db.execute("SELECT payout,net FROM paper_settlements ORDER BY settlement_id").fetchall() != [(1000000000, 736870000), (10000000000, 7368750000)]:
            raise ValueError("Paper settlement did not reconcile")
        if db.execute("SELECT state,filled_quantity,cost,fee FROM paper_attempts ORDER BY attempt_id").fetchall() != [("full", 10, 250000000, 13130000), ("partial", 100, 2500000000, 131250000)]:
            raise ValueError("Paper outcomes did not reconcile")


def run(app, timeout=30, grace=5):
    binary = executable(app)
    workspace = Path(tempfile.mkdtemp(prefix="metten-paper-check-")).resolve()
    identity = workspace.stat()
    process = None
    output = bytearray()
    reason = ""
    selector = selectors.DefaultSelector()
    try:
        environment = {key: value for key, value in os.environ.items() if key not in {"METTEN_TEST_WORKSPACE", "METTEN_VERIFY_FD", "METTEN_SOURCES_CHECK"}}
        environment["RUST_BACKTRACE"] = "0"
        process = subprocess.Popen([str(binary), "--paper-check", str(workspace)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=environment)
        os.set_blocking(process.stdout.fileno(), False)
        selector.register(process.stdout, selectors.EVENT_READ)
        deadline = time.monotonic() + timeout
        stopped = False
        while process.poll() is None:
            for key, _ in selector.select(0.05):
                chunk = os.read(key.fd, 4096)
                if chunk:
                    remaining = OUTPUT_LIMIT - len(output)
                    output.extend(chunk[:remaining])
                    if len(chunk) > remaining:
                        reason = "Paper check output exceeded its bound"
            if not stopped and (reason or time.monotonic() >= deadline):
                reason = reason or "Paper check timed out"
                try:
                    process.stdin.write(b"STOP\n")
                    process.stdin.flush()
                except (BrokenPipeError, OSError):
                    pass
                stopped = True
                deadline = time.monotonic() + grace
            if stopped and time.monotonic() >= deadline and process.poll() is None:
                return Result(1, f"{reason}; exit unconfirmed. Preserved {workspace}; owned PID {process.pid}.", workspace, process)
        process.wait()
        while True:
            try:
                chunk = os.read(process.stdout.fileno(), 4096)
            except BlockingIOError:
                break
            if not chunk:
                break
            remaining = OUTPUT_LIMIT - len(output)
            output.extend(chunk[:remaining])
            if len(chunk) > remaining:
                reason = "Paper check output exceeded its bound"
        text = output.decode("utf-8", errors="replace").strip()
        if process.returncode != 0 or reason:
            return Result(1, f"{reason or 'Paper check failed'} (exit {process.returncode})\n{text}")
        if not text:
            raise ValueError("Missing paper result")
        current = workspace.lstat()
        if workspace.is_symlink() or (current.st_dev, current.st_ino) != (identity.st_dev, identity.st_ino):
            raise ValueError("Owned workspace identity changed")
        result = json.loads(text.splitlines()[-1])
        if result.get("protocol") != PROTOCOL.decode() or result.get("kind") != "paper_check" or result.get("origin") != "synthetic_controlled":
            raise ValueError("Missing finite synthetic paper result")
        check_database(workspace)
        return Result(0, json.dumps(result, sort_keys=True))
    except (OSError, ValueError, sqlite3.Error) as error:
        if process is not None and process.poll() is None:
            try:
                process.stdin.write(b"STOP\n")
                process.stdin.flush()
                process.wait(timeout=grace)
            except (OSError, subprocess.TimeoutExpired):
                return Result(1, f"Paper runner failed; exit unconfirmed. Preserved {workspace}; owned PID {process.pid}.", workspace, process)
        return Result(1, f"Paper runner failed: {error}")
    finally:
        selector.close()
        if process is None or process.poll() is not None:
            if process is not None:
                process.stdin.close()
                process.stdout.close()
            try:
                current = workspace.lstat()
            except FileNotFoundError:
                current = None
            if current is not None and not workspace.is_symlink() and (current.st_dev, current.st_ino) == (identity.st_dev, identity.st_ino):
                shutil.rmtree(workspace)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("Usage: python3 scripts/verify-paper.py <built-Metten.app>")
    try:
        outcome = run(sys.argv[1])
    except (OSError, ValueError) as error:
        sys.exit(f"Paper runner could not start: {error}")
    print(outcome.output, flush=True)
    sys.exit(outcome.code)
