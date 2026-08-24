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
from collections.abc import Sequence


PROJECT_ROOT = Path(__file__).resolve().parent


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Sync Spectrix's Python environment and run the Rust application."
    )
    logging = parser.add_mutually_exclusive_group()
    logging.add_argument(
        "--info",
        dest="log_level",
        action="store_const",
        const="info",
        help="Show info-level Rust logs in the terminal.",
    )
    logging.add_argument(
        "--debug",
        dest="log_level",
        action="store_const",
        const="debug",
        help="Show debug-level Rust logs and use Cargo's debug build.",
    )
    parser.add_argument(
        "--debug-build",
        action="store_true",
        help="Use Cargo's debug build without changing the Rust log filter.",
    )
    parser.add_argument(
        "--no-sync",
        action="store_true",
        help="Skip uv sync when the locked environment is already up to date.",
    )
    parser.add_argument(
        "--reset-state",
        action="store_true",
        help="Back up persisted Spectrix state and start with a clean session.",
    )
    return parser.parse_args(argv)


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


def build_environment(
    python: str,
    site_packages: list[str],
    log_level: str | None = None,
    reset_state: bool = False,
) -> dict[str, str]:
    environment = os.environ.copy()
    environment["PYO3_PYTHON"] = python

    python_paths = [*site_packages]
    current_python_path = environment.get("PYTHONPATH")
    if current_python_path:
        python_paths.extend(current_python_path.split(os.pathsep))
    environment["PYTHONPATH"] = os.pathsep.join(dict.fromkeys(python_paths))
    if log_level is not None:
        environment["RUST_LOG"] = log_level
    if reset_state:
        environment["SPECTRIX_RESET_STATE"] = "1"

    return environment


def cargo_command(cargo: str, debug_build: bool, console: bool = False) -> list[str]:
    command = [cargo, "run", "--locked"]
    if console:
        command.extend(["--features", "console"])
    if not debug_build:
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
        debug_build = args.debug_build or args.log_level == "debug"
        environment = build_environment(
            python,
            site_packages,
            args.log_level,
            reset_state=args.reset_state,
        )
        command = cargo_command(cargo, debug_build, console=args.log_level is not None)

        print(f"Using Python: {python}")
        if args.log_level is not None:
            print(f"Rust log level: {args.log_level}")
        if args.reset_state:
            print("Persisted app state: back up and reset")
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
