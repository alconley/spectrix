use log::{error, info};
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use eframe::egui::{self, Color32, RichText};
use eframe::App;

use super::channel_map::{Board, ChannelType};
use super::compass_run::{process_runs, ProcessParams};
use super::error::EVBError;
use super::kinematics::KineParameters;
use super::nuclear_data::MassMap;
use super::scaler_list::ScalerEntryUI;
use super::shift_map::ShiftMapEntry;
use super::ws::{Workspace, WorkspaceError};

#[derive(Debug, Serialize, Deserialize)]
struct EvbAppParams {
    pub workspace: Option<Workspace>,
    pub kinematics: KineParameters,
    pub coincidence_window: f64,
    pub run_min: i32,
    pub run_max: i32,
    pub channel_map_entries: Vec<Board>,
    pub shift_map_entries: Vec<ShiftMapEntry>,
    pub scaler_list_entries: Vec<ScalerEntryUI>,
}

impl Default for EvbAppParams {
    fn default() -> Self {
        EvbAppParams {
            workspace: None,
            kinematics: KineParameters::default(),
            coincidence_window: 3.0e3,
            run_min: 0,
            run_max: 0,
            channel_map_entries: Vec::new(),
            shift_map_entries: Vec::new(),
            scaler_list_entries: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum ActiveTab {
    MainTab,
    Kinematics,
    ChannelMap,
    ShiftMap,
    ScalerList,
}

impl Default for ActiveTab {
    fn default() -> Self {
        Self::MainTab
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
pub struct EVBApp {
    #[serde(skip)]
    progress: Arc<Mutex<f32>>,

    parameters: EvbAppParams,
    rxn_eqn: String,
    active_tab: ActiveTab,

    mass_map: MassMap,

    #[serde(skip)]
    thread_handle: Option<JoinHandle<Result<(), EVBError>>>,

    window: bool,
}

impl EVBApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, window: bool) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        EVBApp {
            progress: Arc::new(Mutex::new(0.0)),
            parameters: EvbAppParams::default(),
            active_tab: ActiveTab::MainTab,
            rxn_eqn: String::from("None"),
            mass_map: MassMap::new().expect("Could not open amdc data, shutting down!"),
            thread_handle: None,
            window,
        }
    }

    fn check_and_startup_processing_thread(&mut self) -> Result<(), WorkspaceError> {
        if self.thread_handle.is_none()
            && self.parameters.workspace.is_some()
            && !self.parameters.channel_map_entries.is_empty()
        {
            let prog = self.progress.clone();
            let r_params = ProcessParams {
                archive_dir: self
                    .parameters
                    .workspace
                    .as_ref()
                    .unwrap()
                    .get_archive_dir()?,
                unpack_dir: self
                    .parameters
                    .workspace
                    .as_ref()
                    .unwrap()
                    .get_unpack_dir()?,
                output_dir: self
                    .parameters
                    .workspace
                    .as_ref()
                    .unwrap()
                    .get_output_dir()?,
                channel_map: self.parameters.channel_map_entries.clone(),
                scaler_list: self.parameters.scaler_list_entries.clone(),
                shift_map: self.parameters.shift_map_entries.clone(),
                coincidence_window: self.parameters.coincidence_window,
                run_min: self.parameters.run_min,
                run_max: self.parameters.run_max + 1, //Make it [run_min, run_max]
            };

            match self.progress.lock() {
                Ok(mut x) => *x = 0.0,
                Err(_) => error!("Could not aquire lock at starting processor..."),
            };
            let k_params = self.parameters.kinematics.clone();
            self.thread_handle = Some(std::thread::spawn(|| {
                process_runs(r_params, k_params, prog)
            }));
        } else {
            error!("Cannot run event builder without all filepaths specified");
        }
        Ok(())
    }

    fn check_and_shutdown_processing_thread(&mut self) {
        if self.thread_handle.is_some() && self.thread_handle.as_ref().unwrap().is_finished() {
            match self.thread_handle.take().unwrap().join() {
                Ok(result) => {
                    match result {
                        Ok(_) => info!("Finished processing the run"),
                        Err(x) => {
                            error!("An error occured while processing the run: {x}. Job stopped.")
                        }
                    };
                }
                Err(_) => error!("An error occured in joining the processing thread!"),
            };
        }
    }

    fn write_params_to_file(&self, path: &Path) {
        if let Ok(mut config) = File::create(path) {
            match serde_yaml::to_string(&self.parameters) {
                Ok(yaml_str) => match config.write(yaml_str.as_bytes()) {
                    Ok(_) => (),
                    Err(x) => error!("Error writing config to file{}: {}", path.display(), x),
                },
                Err(x) => error!(
                    "Unable to write configuration to file, serializer error: {}",
                    x
                ),
            };
        } else {
            error!("Could not open file {} for config write", path.display());
        }
    }

    fn read_params_from_file(&mut self, path: &Path) {
        let yaml_str = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(x) => {
                error!(
                    "Unable to open and read config file {} with error {}",
                    path.display(),
                    x
                );
                return;
            }
        };

        match serde_yaml::from_str::<EvbAppParams>(&yaml_str) {
            Ok(params) => self.parameters = params,
            Err(x) => error!(
                "Unable to write configuration to file, serializer error: {}",
                x
            ),
        };
    }

    fn channel_map_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Channel Map")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );

        if ui.button("Add Board").clicked() {
            self.parameters.channel_map_entries.push(Board::default()); // This line seems correct, assuming boards is a Vec<Board>
        }

        // Use a DragValue to adjust the desired number of boards
        ui.horizontal(|ui| {
            ui.label("Number of Boards:");
            let mut desired_board_count = self.parameters.channel_map_entries.len();
            if ui
                .add(egui::DragValue::new(&mut desired_board_count).clamp_range(0..=16))
                .changed()
            {
                self.parameters
                    .channel_map_entries
                    .resize_with(desired_board_count, Board::default);
            }
        });

        // Use a horizontal scroll area to contain all the boards
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for (board_idx, board) in self.parameters.channel_map_entries.iter_mut().enumerate()
                {
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            egui::Grid::new(format!("board_{}", board_idx))
                                .num_columns(2)
                                .spacing([20.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label(format!("Board {}", board_idx));
                                    ui.label("Channel Number");
                                    ui.end_row();

                                    for (channel_idx, channel_type) in
                                        board.channels.iter_mut().enumerate()
                                    {
                                        ui.label(format!("{}", channel_idx));
                                        egui::ComboBox::from_id_source(format!(
                                            "channel_type_{}_{}",
                                            board_idx, channel_idx
                                        ))
                                        .selected_text(format!("{:?}", channel_type))
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                // Populate ComboBox with channel types

                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::AnodeFront,
                                                    "AnodeFront",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::AnodeBack,
                                                    "AnodeBack",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::ScintLeft,
                                                    "ScintLeft",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::ScintRight,
                                                    "ScintRight",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::Cathode,
                                                    "Cathode",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::DelayFrontLeft,
                                                    "DelayFrontLeft",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::DelayFrontRight,
                                                    "DelayFrontRight",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::DelayBackLeft,
                                                    "DelayBackLeft",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::DelayBackRight,
                                                    "DelayBackRight",
                                                );
                                                ui.selectable_value(
                                                    channel_type,
                                                    ChannelType::None,
                                                    "None",
                                                );
                                            },
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    });
                    ui.add_space(1.0); // Add space between boards
                }
            });
        });
    }

    fn shift_map_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Time Shift Map")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );

        // Assuming `self.shift_map_entries` is a Vec<ShiftMapEntry>
        if ui.button("Add Entry").clicked() {
            // Add a new entry with default values
            self.parameters.shift_map_entries.push(ShiftMapEntry {
                board_number: 0,
                channel_number: 0,
                time_shift: 0.0,
            });
        }

        // Iterate over each entry with its index
        let mut to_remove = Vec::new(); // Collect indices of entries to remove
        for (index, entry) in self.parameters.shift_map_entries.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                // Allow the user to input board number, channel number, and time shift
                ui.label("Board:");
                ui.add(egui::DragValue::new(&mut entry.board_number));
                ui.label("Channel:");
                ui.add(egui::DragValue::new(&mut entry.channel_number));
                ui.label("Time Shift:");
                ui.add(egui::DragValue::new(&mut entry.time_shift).suffix(" ns"));

                // Button to remove the current entry
                if ui.button("❌").clicked() {
                    to_remove.push(index);
                }
            });
        }

        // Remove entries marked for removal
        // Iterate in reverse to ensure indices remain valid after removals
        for &index in to_remove.iter().rev() {
            self.parameters.shift_map_entries.remove(index);
        }
    }

    fn scaler_list_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Scalar List")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );

        if ui.button("Add Scaler Entry").clicked() {
            // Add a new entry with default values
            self.parameters.scaler_list_entries.push(ScalerEntryUI {
                file_pattern: "".to_string(),
                scaler_name: "".to_string(),
            });
        }

        // Use a `ScrollArea` to ensure the UI can handle many entries
        // egui::ScrollArea::horizontal().show(ui, |ui| {
        let mut to_remove = Vec::new(); // Indices of entries to remove
        for (index, entry) in self.parameters.scaler_list_entries.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label("File Pattern:")
                    .on_hover_text("Data_CH<channel_number>@<board_type>_<board_serial_number>");
                ui.text_edit_singleline(&mut entry.file_pattern);
                ui.label("Scaler Name:");
                ui.text_edit_singleline(&mut entry.scaler_name);

                // Button to remove the current entry
                if ui.button("❌").clicked() {
                    to_remove.push(index);
                }
            });
        }

        // Remove entries marked for removal, in reverse order to maintain correct indices
        for &index in to_remove.iter().rev() {
            self.parameters.scaler_list_entries.remove(index);
        }
        // });
    }

    fn kinematics_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Kinematics")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );

        egui::Grid::new("KineGrid").show(ui, |ui| {
            ui.label("Target Z     ");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.target_z).speed(1),
            );
            ui.label("Target A     ");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.target_a).speed(1),
            );
            ui.end_row();

            ui.label("Projectile Z");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.projectile_z)
                    .speed(1),
            );
            ui.label("Projectile A");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.projectile_a)
                    .speed(1),
            );
            ui.end_row();

            ui.label("Ejectile Z   ");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.ejectile_z).speed(1),
            );
            ui.label("Ejectile A   ");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.ejectile_a).speed(1),
            );
            ui.end_row();

            ui.label("Magnetic Field(kG)");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.b_field).speed(10.0),
            );
            ui.label("SPS Angle(deg)");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.sps_angle).speed(1.0),
            );
            ui.label("Projectile KE(MeV)");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.kinematics.projectile_ke)
                    .speed(0.01),
            );
            ui.end_row();

            ui.label("Reaction Equation");
            ui.label(&self.rxn_eqn);
            if ui.button("Set Kinematics").clicked() {
                self.rxn_eqn = self.parameters.kinematics.generate_rxn_eqn(&self.mass_map);
            }
        });
    }

    fn main_tab_ui(&mut self, ui: &mut egui::Ui) {
        //Files/Workspace
        ui.separator();
        ui.label(
            RichText::new("Run Information")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );
        egui::Grid::new("RunGrid").show(ui, |ui| {
            ui.label("Workspace: ");
            ui.label(match &self.parameters.workspace {
                Some(ws) => ws.get_parent_str(),
                None => "None",
            });

            if ui.button("Open").clicked() {
                let result = rfd::FileDialog::new()
                    .set_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .pick_folder();

                if let Some(real_path) = result {
                    self.parameters.workspace = match Workspace::new(&real_path) {
                        Ok(ws) => Some(ws),
                        Err(e) => {
                            eprintln!("Error creating workspace: {}", e);
                            None
                        }
                    }
                }
            }

            ui.end_row();

            ui.label("Coincidence Window (ns)");
            ui.add(
                egui::widgets::DragValue::new(&mut self.parameters.coincidence_window)
                    .speed(100)
                    .custom_formatter(|n, _| format!("{:e}", n)),
            );
            ui.end_row();

            ui.label("Run Min");
            ui.add(egui::widgets::DragValue::new(&mut self.parameters.run_min).speed(1));
            ui.end_row();

            ui.label("Run Max");
            ui.add(egui::widgets::DragValue::new(&mut self.parameters.run_max).speed(1));
        });
    }

    fn ui_tabs(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("sps_cebra_top_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        matches!(self.active_tab, ActiveTab::MainTab),
                        "Eventbuilder",
                    )
                    .clicked()
                {
                    self.active_tab = ActiveTab::MainTab;
                }
                if ui
                    .selectable_label(
                        matches!(self.active_tab, ActiveTab::Kinematics),
                        "Kinematics",
                    )
                    .clicked()
                {
                    self.active_tab = ActiveTab::Kinematics;
                }
                if ui
                    .selectable_label(
                        matches!(self.active_tab, ActiveTab::ChannelMap),
                        "Channel Map",
                    )
                    .clicked()
                {
                    self.active_tab = ActiveTab::ChannelMap;
                }
                if ui
                    .selectable_label(matches!(self.active_tab, ActiveTab::ShiftMap), "Shift Map")
                    .clicked()
                {
                    self.active_tab = ActiveTab::ShiftMap;
                }
                if ui
                    .selectable_label(
                        matches!(self.active_tab, ActiveTab::ScalerList),
                        "Scaler List",
                    )
                    .clicked()
                {
                    self.active_tab = ActiveTab::ScalerList;
                }
            });
        });

        match self.active_tab {
            ActiveTab::MainTab => self.main_tab_ui(ui),
            ActiveTab::Kinematics => self.kinematics_ui(ui),
            ActiveTab::ChannelMap => self.channel_map_ui(ui),
            ActiveTab::ShiftMap => self.shift_map_ui(ui),
            ActiveTab::ScalerList => self.scaler_list_ui(ui),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("Open Config...").clicked() {
                let result = rfd::FileDialog::new()
                    .set_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .add_filter("YAML file", &["yaml"])
                    .pick_file();

                if let Some(real_path) = result {
                    self.read_params_from_file(&real_path)
                }
            }
            if ui.button("Save Config...").clicked() {
                let result = rfd::FileDialog::new()
                    .set_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
                    .add_filter("YAML file", &["yaml"])
                    .save_file();

                if let Some(real_path) = result {
                    self.write_params_to_file(&real_path)
                }
            }
        });

        ui.separator();

        self.ui_tabs(ui);

        ui.separator();

        ui.add(
            egui::widgets::ProgressBar::new(match self.progress.lock() {
                Ok(x) => *x,
                Err(_) => 0.0,
            })
            .show_percentage(),
        );

        // Check if the thread handle exists to determine if the process is running
        let is_running = self.thread_handle.is_some();
        if is_running {
            ui.add(egui::Spinner::new());
        }

        if ui
            .add_enabled(
                self.thread_handle.is_none(),
                egui::widgets::Button::new("Run"),
            )
            .clicked()
        {
            info!("Starting processor...");
            match self.check_and_startup_processing_thread() {
                Ok(_) => (),
                Err(e) => error!(
                    "Could not start processor, recieved the following error: {}",
                    e
                ),
            };
        } else {
            self.check_and_shutdown_processing_thread();
        }
    }
}

impl App for EVBApp {
    #[cfg(not(target_arch = "wasm32"))]
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if self.window {
            egui::Window::new("SE-SPS Event Builder")
                .min_width(200.0)
                .max_width(600.0)
                .show(ctx, |ui| {
                    self.ui(ui);
                });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.ui(ui);
            });
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("SPS Eventbuilder is not supported in the browser.");
        });
    }
}
