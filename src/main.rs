#![cfg_attr(
    all(not(debug_assertions), not(feature = "console")),
    windows_subsystem = "windows"
)] // hide the Windows console in normal release builds

use spectrix::ui::Spectrix;

const APP_NAME: &str = "Spectrix";
const PERSISTENCE_PATH_ENV: &str = "SPECTRIX_PERSISTENCE_PATH";
const RESET_STATE_ENV: &str = "SPECTRIX_RESET_STATE";

fn unused_path(directory: &std::path::Path, file_name: &str) -> std::path::PathBuf {
    let first_choice = directory.join(file_name);
    if !first_choice.exists() {
        return first_choice;
    }

    (2..)
        .map(|index| directory.join(format!("{file_name}.{index}")))
        .find(|candidate| !candidate.exists())
        .unwrap_or(first_choice)
}

fn reset_persistence_path() -> Option<std::path::PathBuf> {
    let data_directory = eframe::storage_dir(APP_NAME)?;
    if let Err(error) = std::fs::create_dir_all(&data_directory) {
        eprintln!(
            "Unable to create Spectrix's state directory '{}': {error}",
            data_directory.display()
        );
        return None;
    }

    let state_path = data_directory.join("app.ron");
    if !state_path.exists() {
        eprintln!("No saved Spectrix state was found; starting with a clean session.");
        return Some(state_path);
    }

    let backup_path = unused_path(&data_directory, "app.ron.backup");
    match std::fs::rename(&state_path, &backup_path) {
        Ok(()) => {
            eprintln!(
                "Backed up the previous Spectrix state to '{}'.",
                backup_path.display()
            );
            Some(state_path)
        }
        Err(error) => {
            let fresh_path = unused_path(&data_directory, "app-reset.ron");
            eprintln!(
                "Unable to back up '{}': {error}. Starting with the separate clean state file '{}'.",
                state_path.display(),
                fresh_path.display()
            );
            Some(fresh_path)
        }
    }
}

fn persistence_path_from_environment() -> Option<std::path::PathBuf> {
    std::env::var_os(PERSISTENCE_PATH_ENV)
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os(RESET_STATE_ENV).and_then(|_| reset_persistence_path()))
}

fn main() -> eframe::Result {
    env_logger::init(); // The launcher sets RUST_LOG for --info and --debug.

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        persistence_path: persistence_path_from_environment(),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(Spectrix::new(cc)))),
    )
}
