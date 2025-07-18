use crate::histoer::cuts::Cuts;
use crate::histoer::histogrammer::Histogrammer;

use crate::histogram_scripter::histogram_script::HistogramScript;
use pyo3::ffi::c_str;
use pyo3::{prelude::*, types::PyModule};

use egui_file_dialog::FileDialog;
use polars::prelude::*;

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use std::sync::Arc;
use std::thread;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ProcessorSettings {
    pub left_panel_open: bool,
    pub histogram_script_open: bool,
    pub column_names: Vec<String>,
    pub estimated_memory: f64,
    pub cuts: Cuts,
    pub saved_cut_suffix: String,
    pub calculate_histograms_seperately: bool,
}

impl Default for ProcessorSettings {
    fn default() -> Self {
        Self {
            left_panel_open: true,
            histogram_script_open: true,
            column_names: Vec::new(),
            estimated_memory: 4.0,
            cuts: Cuts::default(),
            saved_cut_suffix: String::new(),
            calculate_histograms_seperately: false,
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Processor {
    #[serde(skip)]
    pub file_dialog: FileDialog,
    pub selected_files: Vec<(PathBuf, bool)>, // Vec preserves order
    #[serde(skip)]
    pub lazyframe: Option<LazyFrame>,
    pub histogrammer: Histogrammer,
    pub histogram_script: HistogramScript,
    pub settings: ProcessorSettings,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            file_dialog: FileDialog::new()
                .add_file_filter(
                    "Root files",
                    Arc::new(|p| p.extension().unwrap_or_default() == "root"),
                )
                .add_file_filter(
                    "Parquet files",
                    Arc::new(|p| p.extension().unwrap_or_default() == "parquet"),
                ),
            selected_files: Vec::new(),
            lazyframe: None,
            histogrammer: Histogrammer::default(),
            histogram_script: HistogramScript::new(),
            settings: ProcessorSettings::default(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn get_histograms_from_root_files(&mut self, checked_files: &[PathBuf]) -> PyResult<()> {
        Python::with_gil(|py| {
            // Attempt to import Python modules and handle errors
            let sys = py.import("sys").map_err(|e| {
                eprintln!("Error importing `sys` module: {e:?}");
                e
            })?;
            let version: String = sys
                .getattr("version")
                .map_err(|e| {
                    eprintln!("Error retrieving Python version: {e:?}");
                    e
                })?
                .extract()
                .map_err(|e| {
                    eprintln!("Error extracting Python version as a string: {e:?}");
                    e
                })?;
            let executable: String = sys
                .getattr("executable")
                .map_err(|e| {
                    eprintln!("Error retrieving Python executable: {e:?}");
                    e
                })?
                .extract()
                .map_err(|e| {
                    eprintln!("Error extracting Python executable as a string: {e:?}");
                    e
                })?;
            println!("Using Python version: {version}");
            println!("Python executable: {executable}");

            // Check if the `uproot` module can be imported
            if let Err(e) = py.import("uproot") {
                eprintln!(
                    "Error: `uproot` module could not be found. Ensure you have the correct Python environment with `uproot` installed."
                );
                return Err(e);
            }

            // Define the Python code as a module
            let code = c_str!("
import uproot

def get_1d_histogram(file, name):
    hist = file[name]
    bin_edges = hist.axis().edges().tolist()
    counts = hist.counts(flow=True).tolist()

    return name, counts, bin_edges

def get_2d_histogram(file, name):
    hist = file[name]
    bin_edges_x = hist.axis('x').edges().tolist()
    bin_edges_y = hist.axis('y').edges().tolist()
    counts = hist.counts(flow=False).tolist()

    return name, counts, bin_edges_x, bin_edges_y

def get_1d_histograms(file_name):
    with uproot.open(file_name) as file:
        hist1d_names = [key for key, value in file.classnames().items() if value in ['TH1D', 'TH1F', 'TH1I']]
        histograms = []
        for name in hist1d_names:
            name, counts, bin_edges = get_1d_histogram(file, name)
            histograms.append([name, counts, bin_edges])
        return histograms

def get_2d_histograms(file_name):
    with uproot.open(file_name) as file:
        hist2d_names = [key for key, value in file.classnames().items() if value in ['TH2D', 'TH2F', 'TH2I']]
        histograms = []
        for name in hist2d_names:
            name, counts, bin_edges_x, bin_edges_y = get_2d_histogram(file, name)
            histograms.append([name, counts, bin_edges_x, bin_edges_y])
        return histograms
");
            let module = PyModule::from_code(
                py,
                code,
                c_str!("uproot_functions.py"),
                c_str!("uproot_functions"),
            )
            .map_err(|e| {
                eprintln!("Error compiling Python code into a module: {e:?}");
                e
            })?;

            let root_files: Vec<_> = checked_files
                .iter()
                .filter(|file| file.extension().is_some_and(|ext| ext == "root"))
                .collect();

            for file in &root_files {
                let file_name = file.to_str().expect("Failed to convert path to str");

                let result_1d = module
                    .getattr("get_1d_histograms")
                    .map_err(|e| {
                        eprintln!("Error accessing `get_1d_histograms` function: {e:?}");
                        e
                    })?
                    .call1((file_name,))
                    .map_err(|e| {
                        eprintln!("Error calling `get_1d_histograms` with file {file_name}: {e:?}");
                        e
                    })?;

                let length_1d: usize = result_1d.len()?;

                for i in 0..length_1d {
                    let item = result_1d.get_item(i)?;
                    let full_name: String = item.get_item(0)?.extract()?;
                    let mut counts: Vec<f64> = item.get_item(1)?.extract()?;
                    let underflow = counts.remove(0);
                    let overflow = counts.pop().unwrap_or(0.0);
                    let bin_edges: Vec<f64> = item.get_item(2)?.extract()?;
                    let range = (bin_edges[0], bin_edges[bin_edges.len() - 1]);

                    let counts_u64 = counts.iter().map(|&x| x as u64).collect::<Vec<u64>>();

                    self.histogrammer.add_hist1d_with_bin_values(
                        &full_name,
                        counts_u64,
                        underflow as u64,
                        overflow as u64,
                        range,
                    );
                }

                // let result_2d = module.getattr("get_2d_histograms")?.call1((file_name,))?;
                let result_2d = module
                    .getattr("get_2d_histograms")
                    .map_err(|e| {
                        eprintln!("Error accessing `get_2d_histograms` function: {e:?}");
                        e
                    })?
                    .call1((file_name,))
                    .map_err(|e| {
                        eprintln!("Error calling `get_2d_histograms` with file {file_name}: {e:?}");
                        e
                    })?;

                let length_2d: usize = result_2d.len()?;

                for i in 0..length_2d {
                    let item = result_2d.get_item(i)?;

                    let full_name: String = item.get_item(0)?.extract()?;
                    let counts: Vec<Vec<f64>> = item.get_item(1)?.extract()?;
                    let bin_edges_x: Vec<f64> = item.get_item(2)?.extract()?;
                    let bin_edges_y: Vec<f64> = item.get_item(3)?.extract()?;
                    let range = (
                        (bin_edges_x[0], bin_edges_x[bin_edges_x.len() - 1]),
                        (bin_edges_y[0], bin_edges_y[bin_edges_y.len() - 1]),
                    );

                    let counts_u64 = counts
                        .iter()
                        .map(|row| row.iter().map(|&x| x as u64).collect::<Vec<u64>>())
                        .collect::<Vec<Vec<u64>>>();

                    self.histogrammer
                        .add_hist2d_with_bin_values(&full_name, &counts_u64, range);
                }
            }

            Ok(())
        })
    }

    fn create_lazyframe(&mut self, checked_files: &[PathBuf]) {
        // Get all the checked parquet files
        let parquet_files: Vec<PathBuf> = checked_files
            .iter()
            .filter(|file| file.extension().is_some_and(|ext| ext == "parquet"))
            .cloned()
            .collect();

        // Warn if no checked parquet files are selected
        if parquet_files.is_empty() {
            log::warn!("No selected Parquet files to process.");
            return;
        }

        let files_arc: Arc<[PathBuf]> = Arc::from(parquet_files);
        let args = ScanArgsParquet::default();
        log::info!("Processing Parquet files: {files_arc:?}");

        match LazyFrame::scan_parquet_files(files_arc, args) {
            Ok(lf) => {
                log::info!("Successfully loaded selected Parquet files.");
                let column_names = Self::get_column_names_from_lazyframe(&lf);

                self.lazyframe = Some(lf);
                self.settings.column_names = column_names;
            }
            Err(e) => {
                self.lazyframe = None; // Indicates that loading failed
                log::error!("Failed to load selected Parquet files: {e}");
            }
        }
    }

    pub fn filter_selected_files_and_save(&self) {
        let checked_files: Vec<PathBuf> = self
            .selected_files
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(file, _)| file.clone())
            .collect();

        if checked_files.is_empty() {
            log::error!("No files selected for filtering.");
            return;
        }

        let parquet_files: Vec<PathBuf> = checked_files
            .into_iter()
            .filter(|file| file.extension().is_some_and(|ext| ext == "parquet"))
            .collect();

        if parquet_files.is_empty() {
            log::warn!("No selected Parquet files to process.");
            return;
        }

        // Clone necessary data for the thread
        let cut = self.settings.cuts.clone();
        let saved_cut_suffix = self.settings.saved_cut_suffix.clone();

        // Spawn the filtering task on a new thread
        thread::spawn(move || {
            for file in parquet_files {
                eprintln!("Processing file: {file:?}");
                let file_stem = file
                    .file_stem()
                    .expect("Failed to get file stem")
                    .to_string_lossy();
                let new_file_name = format!("{file_stem}_{saved_cut_suffix}");
                let new_file_path = file.with_file_name(format!("{new_file_name}.parquet"));

                log::info!("Processing file: {file:?}");
                log::info!("Saving filtered file as: {new_file_path:?}");

                // Load and collect one file at a time
                match LazyFrame::scan_parquet(
                    file.to_str().expect("Failed to convert path to str"),
                    Default::default(),
                ) {
                    Ok(lf) => {
                        if let Ok(df) = lf.collect() {
                            if let Err(e) = cut.filter_df_and_save(
                                &df,
                                new_file_path
                                    .to_str()
                                    .expect("Failed to convert path to str"),
                            ) {
                                log::error!("Failed to save filtered DataFrame for {file:?}: {e}");
                            } else {
                                log::info!(
                                    "Successfully saved filtered DataFrame: {new_file_path:?}"
                                );
                            }
                        } else {
                            log::error!("Failed to collect DataFrame from LazyFrame: {file:?}");
                        }
                    }
                    Err(e) => log::error!("Failed to read Parquet file {file:?}: {e}"),
                }
            }
        });
    }

    pub fn combine_and_save_selected_files(&self) {
        let checked_files: Vec<PathBuf> = self
            .selected_files
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(file, _)| file.clone())
            .collect();

        if checked_files.is_empty() {
            log::error!("No files selected for combination.");
            return;
        }

        let parquet_files: Vec<PathBuf> = checked_files
            .into_iter()
            .filter(|file| file.extension().is_some_and(|ext| ext == "parquet"))
            .collect();

        if parquet_files.is_empty() {
            log::warn!("No selected Parquet files to combine.");
            return;
        }

        // Ask the user to select a file name and path
        if let Some(output_file) = rfd::FileDialog::new()
            .set_title("Save Combined Parquet File")
            .add_filter("Parquet Files", &["parquet"])
            .save_file()
        {
            let output_file_clone = output_file.clone();

            // Spawn a new thread for processing
            thread::spawn(move || {
                let mut lazyframes = Vec::new();

                for file in &parquet_files {
                    log::info!("Reading file: {file:?}");
                    match LazyFrame::scan_parquet(
                        file.to_str().expect("Failed to convert path to str"),
                        Default::default(),
                    ) {
                        Ok(lf) => lazyframes.push(lf),
                        Err(e) => log::error!("Failed to read Parquet file {file:?}: {e}"),
                    }
                }

                if lazyframes.is_empty() {
                    log::error!("No valid Parquet files loaded.");
                    return;
                }

                let combined_lazyframe = concat(lazyframes, UnionArgs::default())
                    .expect("Failed to concatenate LazyFrames");
                match combined_lazyframe.collect() {
                    Ok(mut df) => {
                        if let Err(e) = ParquetWriter::new(
                            &mut std::fs::File::create(&output_file_clone)
                                .expect("Failed to create output file"),
                        )
                        .finish(&mut df)
                        {
                            log::error!(
                                "Failed to save combined DataFrame to {output_file_clone:?}: {e}"
                            );
                        } else {
                            log::info!(
                                "Successfully saved combined Parquet file: {output_file_clone:?}"
                            );
                        }
                    }
                    Err(e) => log::error!("Failed to collect combined LazyFrame: {e}"),
                }
            });
        } else {
            log::warn!("User canceled the save operation.");
        }
    }

    fn get_column_names_from_lazyframe(lazyframe: &LazyFrame) -> Vec<String> {
        let lf = lazyframe.clone().limit(1);

        match lf.collect() {
            Ok(df) => df
                .get_column_names_owned()
                .into_iter()
                .map(|name| name.to_string())
                .collect(),
            Err(e) => {
                eprintln!("Error collecting DataFrame: {e:?}");
                Vec::new() // Return an empty vector on error
            }
        }
    }

    fn perform_histogrammer_from_lazyframe(&mut self, prefix: Option<String>) {
        if let Some(lf) = &self.lazyframe {
            self.histogram_script.add_histograms(
                &mut self.histogrammer,
                &lf.clone(),
                self.settings.estimated_memory,
                prefix,
            );
        } else {
            log::error!("Failed to preform histogrammer: LazyFrame is None.");
        }
    }

    pub fn calculate_histograms(&mut self) {
        let checked_files: Vec<_> = self
            .selected_files
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(file, _)| file.clone())
            .collect();

        if checked_files.is_empty() {
            log::error!("No files selected for histogram calculation.");
            return;
        }

        if self.settings.calculate_histograms_seperately {
            for file in &checked_files {
                let prefix = file
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string());

                if file.extension().is_some_and(|ext| ext == "parquet") {
                    self.create_lazyframe(&[file.clone()]);
                    self.perform_histogrammer_from_lazyframe(prefix);
                }
            }
        } else if checked_files
            .iter()
            .any(|file| file.extension().is_some_and(|ext| ext == "parquet"))
        {
            self.create_lazyframe(&checked_files);
            self.perform_histogrammer_from_lazyframe(None);
        } else if checked_files
            .iter()
            .any(|file| file.extension().is_some_and(|ext| ext == "root"))
        {
            self.get_histograms_from_root_files(&checked_files)
                .unwrap_or_else(|e| {
                    log::error!("Error processing ROOT files: {e:?}");
                });
        }
    }

    pub fn left_side_panels_ui(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("spectrix_processor_left_panel").show_animated(
            ctx,
            self.settings.left_panel_open,
            |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        if ui.button("Get Files").clicked() {
                            self.file_dialog.pick_multiple();
                        }
                        if ui
                            .selectable_label(self.settings.histogram_script_open, "Histograms")
                            .clicked()
                            {
                                self.settings.histogram_script_open = !self.settings.histogram_script_open;
                            }
                    });

                    if let Some(paths) = self.file_dialog.take_picked_multiple() {
                        for path in paths {
                            if path.is_dir() {
                                self.file_dialog = FileDialog::new().initial_directory(path.clone());
                                if let Ok(entries) = std::fs::read_dir(&path) {
                                    for entry in entries.flatten() {
                                        let file_path = entry.path();
                                        if let Some(ext) = file_path.extension() {
                                            if (ext == "parquet" || ext == "root") && !self.selected_files.iter().any(|(f, _)| f == &file_path) {
                                                self.selected_files.push((file_path, true)); // Default to selected
                                            }
                                        }
                                    }
                                }
                            } else if let Some(ext) = path.extension() {
                                // If it's a file, check if it's a valid type
                                if (ext == "parquet" || ext == "root") && !self.selected_files.iter().any(|(f, _)| f == &path) {
                                    self.selected_files.push((path, true)); // Default to selected
                                }
                            }
                        }
                        // Sort the selected files by name
                        self.selected_files.sort_by(|a, b| a.0.cmp(&b.0));
                    }

                    ui.separator();

                    ui.vertical(|ui| {

                        ui.label("Processor");

                        if ui
                            .add_enabled(
                                !self.selected_files.is_empty(),
                                egui::Button::new("Calculate/Get Histograms"),
                            )
                            .on_disabled_hover_text("No files selected.")
                            .clicked()
                        {
                            self.calculate_histograms();
                        }

                        ui.add(
                            egui::DragValue::new(&mut self.settings.estimated_memory)
                                .range(0.1..=f64::INFINITY)
                                .speed(1)
                                .prefix("Estimated Memory: ")
                                .suffix(" GB"),
                        ).on_hover_text("Estimated memory in GB. This is an approximation based off the rows and columns in a lazyframe, so set it lower that the actual memory to avoid crashes.");

                        ui.add(
                            egui::Checkbox::new(
                                &mut self.settings.calculate_histograms_seperately,
                                "Calculate histograms separately",
                            )
                        );

                        if self.histogrammer.calculating.load(Ordering::Relaxed) {
                            // Show spinner while `calculating` is true
                            ui.horizontal(|ui| {
                                ui.label("Calculating");
                                ui.add(egui::widgets::Spinner::default());
                                ui.separator();
                                if ui.button("Cancel").clicked() {
                                    self.histogrammer.abort_flag.store(true, Ordering::Relaxed);
                                }
                            });
                        }
                    });
                });

                ui.separator();

                self.selected_files_ui(ui);
            },
        );

        egui::SidePanel::left("spectrix_histogram_panel").show_animated(
            ctx,
            self.settings.histogram_script_open && self.settings.left_panel_open,
            |ui| {
                self.histogram_script.ui(ui);
            },
        );

        self.panel_toggle_button(ctx);
    }

    pub fn panel_toggle_button(&mut self, ctx: &egui::Context) {
        // Secondary left panel for the toggle button
        egui::SidePanel::left("spectrix_toggle_left_panel")
            .resizable(false)
            .show_separator_line(false)
            .min_width(1.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 10.0); // Center the button vertically
                    if ui
                        .small_button(if self.settings.left_panel_open {
                            "◀"
                        } else {
                            "▶"
                        })
                        .clicked()
                    {
                        self.settings.left_panel_open = !self.settings.left_panel_open;
                    }
                });
            });
    }

    pub fn selected_files_ui(&mut self, ui: &mut egui::Ui) {
        if !self.selected_files.is_empty() {
            ui.label("Selected files:");
            ui.horizontal_wrapped(|ui| {
                if ui.button("De/Select All").clicked() {
                    let all_selected = self.selected_files.iter().all(|(_, checked)| *checked);
                    for (_, checked) in &mut self.selected_files {
                        *checked = !all_selected;
                    }
                }
                if ui.button("Clear").clicked() {
                    self.selected_files.clear();
                }
            });
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            if !self.selected_files.is_empty() {
                // Clone the paths beforehand to avoid borrowing conflicts
                let file_parents: Vec<_> = self
                    .selected_files
                    .iter()
                    .filter_map(|(file, _)| file.parent().map(|p| p.to_path_buf()))
                    .collect();
                // Check if all parent directories are the same
                let common_path = file_parents
                    .first()
                    .filter(|&first_path| file_parents.iter().all(|p| p == first_path))
                    .cloned();
                // Show common directory label if applicable
                if let Some(ref common_dir) = common_path {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("Directory: {}", common_dir.to_string_lossy())); // Show common directory

                        // add a refresh button to update the files in the directory
                        if ui.button("⟳").clicked() {
                            self.selected_files.clear();
                            if let Ok(entries) = std::fs::read_dir(common_dir) {
                                for entry in entries.flatten() {
                                    let file_path = entry.path();
                                    if let Some(ext) = file_path.extension() {
                                        if (ext == "parquet" || ext == "root")
                                            && !self
                                                .selected_files
                                                .iter()
                                                .any(|(f, _)| f == &file_path)
                                        {
                                            self.selected_files.push((file_path, false));
                                            // Default to selected
                                        }
                                    }
                                }
                            }
                            // sort the selected files by name
                            self.selected_files.sort_by(|a, b| a.0.cmp(&b.0));
                        }
                    });
                }
                // Track indices of files to remove
                let mut to_remove = Vec::new();
                // Iterate over files and track index
                for (index, (file, checked)) in self.selected_files.iter_mut().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        let display_text = if let Some(ref common_dir) = common_path {
                            if file.parent() == Some(common_dir.as_path()) {
                                file.file_name().unwrap_or_default().to_string_lossy()
                            // Show only filename
                            } else {
                                file.to_string_lossy() // Show full path for outliers
                            }
                        } else {
                            file.to_string_lossy() // Show full path if no common directory
                        };
                        if ui.selectable_label(*checked, display_text).clicked() {
                            *checked = !*checked; // Toggle selection
                        }

                        // "❌" button to mark for removal
                        if ui.button("❌").clicked() {
                            to_remove.push(index);
                        }
                    });
                }
                // Remove files after iteration (to avoid borrowing issues)
                for &index in to_remove.iter().rev() {
                    self.selected_files.remove(index);
                }
            }

            // if there are no selected files, show a message
            if !self.selected_files.is_empty() {
                ui.separator();

                ui.collapsing("Selected File Settings", |ui| {
                    ui.label("Save Filtered Files:");
                    self.settings.cuts.ui(ui);
                    ui.horizontal(|ui| {
                        ui.label("Suffix:");
                        ui.text_edit_singleline(&mut self.settings.saved_cut_suffix);
                    });
                    ui.separator();
                    if ui.button("Save Filtered Files").clicked() {
                        self.filter_selected_files_and_save();
                    }
                    ui.add_space(10.0);
                    if ui.button("Combine Selected Files").clicked() {
                        self.combine_and_save_selected_files();
                    }
                });
            }
        });
    }

    pub fn bottom_panel(&mut self, ctx: &egui::Context) {
        if self.histogrammer.calculating.load(Ordering::Relaxed) {
            egui::TopBottomPanel::bottom("spectrix_bottom_panel").show(ctx, |ui| {
                ui.add(
                    egui::widgets::ProgressBar::new(match self.histogrammer.progress.lock() {
                        Ok(x) => *x,
                        Err(_) => 0.0,
                    })
                    .animate(true)
                    .show_percentage(),
                );
            });
        }
    }

    fn central_panel_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.histogrammer.ui(ui);
        });
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        self.left_side_panels_ui(ctx);
        self.bottom_panel(ctx);
        self.central_panel_ui(ctx);

        self.file_dialog.update(ctx);
    }
}
