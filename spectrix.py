#!/usr/bin/env python3
"""Cross-platform development launcher for Spectrix."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


PROJECT_ROOT = Path(__file__).resolve().parent


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Sync Spectrix's Python environment and run the Rust application."
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Use Cargo's faster debug build instead of the default release build.",
    )
    parser.add_argument(
        "--no-sync",
        action="store_true",
        help="Skip uv sync when the locked environment is already up to date.",
    )
    return parser.parse_args()


def require_tool(name: str, install_url: str) -> str:
    executable = shutil.which(name)
    if executable is None:
        raise RuntimeError(f"{name} is required. Install it from {install_url}")
    return executable


def display_command(command: list[str]) -> None:
    if os.name == "nt":
        rendered = subprocess.list2cmdline(command)
    else:
        rendered = " ".join(command)
    print(f"> {rendered}", flush=True)


def sync_python_environment(uv: str) -> None:
    command = [uv, "sync", "--locked"]
    display_command(command)
    subprocess.run(command, cwd=PROJECT_ROOT, check=True)


def query_uv_python(uv: str) -> tuple[str, list[str]]:
    query = (
        "import json, site, sys; "
        "print(json.dumps({'executable': sys.executable, "
        "'site_packages': site.getsitepackages()}))"
    )
    command = [uv, "run", "--locked", "--no-sync", "python", "-c", query]
    result = subprocess.run(
        command,
        cwd=PROJECT_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    details = json.loads(result.stdout)
    return details["executable"], details["site_packages"]


def build_environment(python: str, site_packages: list[str]) -> dict[str, str]:
    environment = os.environ.copy()
    environment["PYO3_PYTHON"] = python

    python_paths = [*site_packages]
    current_python_path = environment.get("PYTHONPATH")
    if current_python_path:
        python_paths.extend(current_python_path.split(os.pathsep))
    environment["PYTHONPATH"] = os.pathsep.join(dict.fromkeys(python_paths))

    return environment


def cargo_command(cargo: str, debug: bool) -> list[str]:
    command = [cargo, "run", "--locked"]
    if not debug:
        command.append("--release")
    return command


def main() -> int:
    args = parse_args()

    try:
        uv = require_tool("uv", "https://docs.astral.sh/uv/getting-started/installation/")
        cargo = require_tool("cargo", "https://rustup.rs/")

        if not args.no_sync:
            sync_python_environment(uv)

        python, site_packages = query_uv_python(uv)
        environment = build_environment(python, site_packages)
        command = cargo_command(cargo, args.debug)

        print(f"Using Python: {python}")
        display_command(command)
        return subprocess.run(
            command,
            cwd=PROJECT_ROOT,
            env=environment,
            check=False,
        ).returncode
    except (json.JSONDecodeError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"Unable to launch Spectrix: {error}", file=sys.stderr)
        return 1
    except RuntimeError as error:
        print(f"Unable to launch Spectrix: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
