//! Minimal Windows entry point for Spectrix's PowerShell bootstrapper.

use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, ExitCode},
};

const HELP: &str = "\
Usage: .\\spectrix.exe [options]

Options:
  --info          Show info-level Rust logs.
  --debug         Show debug-level Rust logs and use a debug build.
  --debug-build   Use a debug build without changing RUST_LOG.
  --reset-state   Back up persisted state and start clean.
  --no-sync       Skip Python dependency synchronization.
  --check         Check prerequisites without installing or launching.
  --dry-run       Print the bootstrap plan without changing the machine.
  -h, --help      Show this help.
";

fn powershell_argument(argument: &OsStr) -> OsString {
    match argument.to_str() {
        Some("--info") => "-Info".into(),
        Some("--debug") => "-Debug".into(),
        Some("--debug-build") => "-DebugBuild".into(),
        Some("--reset-state") => "-ResetState".into(),
        Some("--no-sync") => "-NoSync".into(),
        Some("--check") => "-CheckOnly".into(),
        Some("--dry-run") => "-DryRun".into(),
        _ => argument.to_os_string(),
    }
}

fn run(script_path: &Path, arguments: impl Iterator<Item = OsString>) -> std::io::Result<ExitCode> {
    let status = Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(script_path)
        .args(arguments)
        .status()?;

    let exit_code = status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1);
    Ok(ExitCode::from(exit_code))
}

fn main() -> ExitCode {
    let raw_arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if raw_arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }

    let executable_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Spectrix could not locate its launcher: {error}");
            return ExitCode::FAILURE;
        }
    };
    let Some(install_directory) = executable_path.parent() else {
        eprintln!("Spectrix could not determine its installation directory.");
        return ExitCode::FAILURE;
    };
    let script_path = install_directory.join("spectrix.ps1");
    if !script_path.is_file() {
        eprintln!(
            "Spectrix's bootstrap script was not found at '{}'. Keep spectrix.exe and spectrix.ps1 in the same folder.",
            script_path.display()
        );
        return ExitCode::FAILURE;
    }

    let arguments = raw_arguments
        .iter()
        .map(|argument| powershell_argument(argument));
    match run(&script_path, arguments) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("Spectrix could not start PowerShell: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::powershell_argument;
    use std::ffi::{OsStr, OsString};

    #[test]
    fn translates_cross_platform_long_options() {
        let cases = [
            ("--info", "-Info"),
            ("--debug", "-Debug"),
            ("--debug-build", "-DebugBuild"),
            ("--reset-state", "-ResetState"),
            ("--no-sync", "-NoSync"),
            ("--check", "-CheckOnly"),
            ("--dry-run", "-DryRun"),
        ];

        for (argument, expected) in cases {
            assert_eq!(
                powershell_argument(OsStr::new(argument)),
                OsString::from(expected)
            );
        }
    }

    #[test]
    fn preserves_unrecognized_arguments() {
        assert_eq!(
            powershell_argument(OsStr::new("example")),
            OsString::from("example")
        );
    }
}
