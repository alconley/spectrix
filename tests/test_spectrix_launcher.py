from __future__ import annotations

from contextlib import redirect_stderr
import importlib.util
from io import StringIO
import os
from pathlib import Path
import unittest


PROJECT_ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "spectrix_launcher", PROJECT_ROOT / "spectrix.py"
)
assert SPEC is not None and SPEC.loader is not None
launcher = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(launcher)


class LauncherArgumentsTests(unittest.TestCase):
    def test_info_logging_uses_release_build(self) -> None:
        args = launcher.parse_args(["--info"])

        self.assertEqual(args.log_level, "info")
        self.assertFalse(args.debug_build)

    def test_debug_logging_uses_debug_build(self) -> None:
        args = launcher.parse_args(["--debug"])
        debug_build = args.debug_build or args.log_level == "debug"

        self.assertEqual(args.log_level, "debug")
        self.assertTrue(debug_build)

    def test_log_levels_are_mutually_exclusive(self) -> None:
        with redirect_stderr(StringIO()), self.assertRaises(SystemExit):
            launcher.parse_args(["--info", "--debug"])

    def test_reset_state_is_opt_in(self) -> None:
        self.assertFalse(launcher.parse_args([]).reset_state)
        self.assertTrue(launcher.parse_args(["--reset-state"]).reset_state)


class LauncherEnvironmentTests(unittest.TestCase):
    def test_explicit_log_level_sets_rust_log(self) -> None:
        environment = launcher.build_environment(
            "python", [os.path.join("venv", "site-packages")], "info"
        )

        self.assertEqual(environment["RUST_LOG"], "info")

    def test_no_log_level_preserves_existing_rust_log(self) -> None:
        previous = os.environ.get("RUST_LOG")
        os.environ["RUST_LOG"] = "spectrix=trace"
        try:
            environment = launcher.build_environment("python", [])
        finally:
            if previous is None:
                os.environ.pop("RUST_LOG", None)
            else:
                os.environ["RUST_LOG"] = previous

        self.assertEqual(environment["RUST_LOG"], "spectrix=trace")

    def test_reset_state_sets_recovery_environment_variable(self) -> None:
        environment = launcher.build_environment("python", [], reset_state=True)

        self.assertEqual(environment["SPECTRIX_RESET_STATE"], "1")

    def test_console_feature_is_enabled_for_release_logging(self) -> None:
        command = launcher.cargo_command("cargo", debug_build=False, console=True)

        self.assertEqual(
            command,
            ["cargo", "run", "--locked", "--features", "console", "--release"],
        )

    def test_plain_release_keeps_the_gui_subsystem(self) -> None:
        command = launcher.cargo_command("cargo", debug_build=False)

        self.assertEqual(command, ["cargo", "run", "--locked", "--release"])


if __name__ == "__main__":
    unittest.main()
