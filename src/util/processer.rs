use crate::custom_analysis::analysis::AnalysisScripts;
use crate::histoer::histogrammer::Histogrammer;

use crate::histogram_scripter::histogram_script::HistogramScript;
use pyo3::ffi::c_str;
use pyo3::{prelude::*, types::PyModule};

use egui_file_dialog::{FileDialog, Filter};
use polars::prelude::*;

use std::cmp::Ordering as CmpOrdering;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use std::sync::Arc;
use std::thread;

use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ProcessorSettings {
    pub name: String,
    pub left_panel_open: bool,
    pub histogram_script_open: bool,
    pub column_names: Vec<String>,
    pub estimated_memory: f64,
    pub saved_cut_suffix: String,
    pub calculate_histograms_seperately: bool,
    #[serde(skip)]
    pub saving_in_progress: Arc<AtomicBool>,
    #[serde(skip)]
    pub combining_in_progress: Arc<AtomicBool>,
    #[serde(skip)]
    pub save_progress: Arc<Mutex<f32>>,
}

impl Default for ProcessorSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            left_panel_open: true,
            histogram_script_open: true,
            column_names: Vec::new(),
            estimated_memory: 4.0,
            saved_cut_suffix: String::new(),
            calculate_histograms_seperately: false,
            saving_in_progress: Arc::new(AtomicBool::new(false)),
            combining_in_progress: Arc::new(AtomicBool::new(false)),
            save_progress: Arc::new(Mutex::new(0.0)),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
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
    pub analysis: AnalysisScripts,
    pub file_sort: FileSortState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, Default)]
pub enum FileSortKey {
    #[default]
    Name,
    Size,
    Modified,
}

impl FileSortKey {
    fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Size => "Size",
            Self::Modified => "Modified",
        }
    }

    fn short_label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Size => "Size",
            Self::Modified => "Time",
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct FileSortState {
    pub key: FileSortKey,
    pub ascending: bool,
}

impl Default for FileSortState {
    fn default() -> Self {
        Self {
            key: FileSortKey::Name,
            ascending: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectedFileFormat {
    None,
    Parquet,
    Root,
    Mixed,
}

impl Processor {
    pub fn new(name: impl Into<String>) -> Self {
        let settings = ProcessorSettings {
            name: name.into(),
            ..ProcessorSettings::default()
        };

        Self {
            file_dialog: FileDialog::new()
                .add_file_filter(
                    "Root files",
                    Filter::new(|p: &std::path::Path| p.extension().unwrap_or_default() == "root"),
                )
                .add_file_filter(
                    "Parquet files",
                    Filter::new(|p: &std::path::Path| {
                        p.extension().unwrap_or_default() == "parquet"
                    }),
                ),
            selected_files: Vec::new(),
            lazyframe: None,
            histogrammer: Histogrammer::default(),
            histogram_script: HistogramScript::new(),
            settings,
            analysis: AnalysisScripts::default(),
            file_sort: FileSortState::default(),
        }
    }

    pub fn reset(&mut self) {
        let name = self.settings.name.clone();
        *self = Self::new(name);
    }

    fn checked_files(&self) -> Vec<PathBuf> {
        self.selected_files
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(file, _)| file.clone())
            .collect()
    }

    fn checked_file_format(&self) -> SelectedFileFormat {
        let mut has_parquet = false;
        let mut has_root = false;

        for (file, checked) in &self.selected_files {
            if !*checked {
                continue;
            }

            match file.extension().and_then(|ext| ext.to_str()) {
                Some("parquet") => has_parquet = true,
                Some("root") => has_root = true,
                _ => {}
            }
        }

        match (has_parquet, has_root) {
            (false, false) => SelectedFileFormat::None,
            (true, false) => SelectedFileFormat::Parquet,
            (false, true) => SelectedFileFormat::Root,
            (true, true) => SelectedFileFormat::Mixed,
        }
    }

    fn checked_parquet_files(&self) -> Vec<PathBuf> {
        self.checked_files()
            .into_iter()
            .filter(|file| file.extension().is_some_and(|ext| ext == "parquet"))
            .collect()
    }

    fn active_filter_cut_count(&self) -> usize {
        self.histogram_script
            .active_filter_cut_count(&self.histogrammer)
    }

    fn save_filtered_files_button_enabled(&self, active_filter_cut_count: usize) -> bool {
        matches!(self.checked_file_format(), SelectedFileFormat::Parquet)
            && active_filter_cut_count > 0
    }

    fn save_filtered_files_hover_text(&self, active_filter_cut_count: usize) -> String {
        if active_filter_cut_count == 1 {
            "Apply the 1 active cut from the Histogram Script to each selected parquet file and save the result as filename_{suffix}.parquet. Only active cuts are used here.".to_owned()
        } else {
            format!(
                "Apply the {active_filter_cut_count} active cuts from the Histogram Script to each selected parquet file and save the result as filename_{{suffix}}.parquet. Only active cuts are used here."
            )
        }
    }

    fn save_filtered_files_disabled_reason(
        &self,
        saving: bool,
        active_filter_cut_count: usize,
    ) -> &'static str {
        if saving {
            return "Filtered parquet saving is already in progress.";
        }

        match self.checked_file_format() {
            SelectedFileFormat::None => {
                "Select one or more parquet files to save filtered outputs."
            }
            SelectedFileFormat::Mixed | SelectedFileFormat::Root => {
                "Select only parquet files to save filtered outputs."
            }
            SelectedFileFormat::Parquet if active_filter_cut_count == 0 => {
                "Enable one or more cuts in the Histogram Script to save filtered parquet outputs."
            }
            SelectedFileFormat::Parquet => "",
        }
    }

    fn combine_selected_files_button_enabled(&self, checked_parquet_file_count: usize) -> bool {
        matches!(self.checked_file_format(), SelectedFileFormat::Parquet)
            && checked_parquet_file_count >= 2
    }

    fn combine_selected_files_hover_text(&self) -> &'static str {
        "Combine all selected parquet files into a single parquet file. Spectrix scans the inputs lazily and streams the result to the output parquet sink, so it does not collect the full combined dataset into memory first."
    }

    fn combine_selected_files_disabled_reason(
        &self,
        combining: bool,
        checked_parquet_file_count: usize,
    ) -> &'static str {
        if combining {
            return "Parquet combine is already in progress.";
        }

        match self.checked_file_format() {
            SelectedFileFormat::None => "Select at least two parquet files to combine.",
            SelectedFileFormat::Mixed | SelectedFileFormat::Root => {
                "Select only parquet files to combine them."
            }
            SelectedFileFormat::Parquet if checked_parquet_file_count < 2 => {
                "Select at least two parquet files to combine."
            }
            SelectedFileFormat::Parquet => "",
        }
    }

    fn ensure_parquet_extension(path: PathBuf) -> PathBuf {
        if path.extension().is_some_and(|ext| ext == "parquet") {
            path
        } else {
            path.with_extension("parquet")
        }
    }

    fn histogram_action_label(&self) -> &'static str {
        match (
            self.checked_file_format(),
            self.settings.calculate_histograms_seperately,
        ) {
            (SelectedFileFormat::Root, true) => "Get Histograms Separately",
            (SelectedFileFormat::Root, false) => "Get Histograms",
            (SelectedFileFormat::Parquet | SelectedFileFormat::None, true) => {
                "Calculate Histograms Separately"
            }
            (SelectedFileFormat::Parquet | SelectedFileFormat::None, false) => {
                "Calculate Histograms"
            }
            (SelectedFileFormat::Mixed, true) => "Calculate/Get Histograms Separately",
            (SelectedFileFormat::Mixed, false) => "Calculate/Get Histograms",
        }
    }

    fn histogram_action_disabled_reason(&self) -> &'static str {
        match self.checked_file_format() {
            SelectedFileFormat::None => "No files selected.",
            SelectedFileFormat::Mixed => {
                "Select only parquet files or only root files before starting."
            }
            SelectedFileFormat::Parquet | SelectedFileFormat::Root => "",
        }
    }

    pub fn session_processor_menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Processor");

        ui.horizontal(|ui| {
            ui.label("Estimated Memory:");
            ui.add(
                egui::DragValue::new(&mut self.settings.estimated_memory)
                    .range(0.1..=f64::INFINITY)
                    .speed(1)
                    .suffix(" GB"),
            )
            .on_hover_text(
                "Estimated memory in GB. This is an approximation based off the rows and columns in a lazyframe, so set it lower than the actual memory to avoid crashes.",
            );
        });

        ui.checkbox(
            &mut self.settings.calculate_histograms_seperately,
            "Calculate histograms separately",
        );

        ui.separator();
        ui.label("Analysis");
        ui.checkbox(&mut self.analysis.open, "Open SE-SPS Analysis");
        ui.label(
            egui::RichText::new("Under development for SE-SPS experiments.")
                .weak()
                .small(),
        );
    }

    pub fn get_histograms_from_root_files(&mut self, checked_files: &[PathBuf]) -> PyResult<()> {
        Python::attach(|py| {
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
        let parquet_files: Vec<PathBuf> = checked_files
            .iter()
            .filter(|file| file.extension().is_some_and(|ext| ext == "parquet"))
            .cloned()
            .collect();

        if parquet_files.is_empty() {
            log::warn!("No selected Parquet files to process.");
            return;
        }

        log::info!("Processing Parquet files: {parquet_files:?}");
        let args = ScanArgsParquet::default();

        let paths: Result<polars_buffer::Buffer<PlRefPath>, PolarsError> = parquet_files
            .into_iter()
            .map(PlRefPath::try_from_pathbuf)
            .collect();

        let paths = match paths {
            Ok(paths) => paths,
            Err(e) => {
                self.lazyframe = None;
                log::error!("Failed to convert parquet paths: {e}");
                return;
            }
        };

        match LazyFrame::scan_parquet_files(paths, args) {
            Ok(lf) => {
                log::info!("Successfully loaded selected Parquet files.");
                let column_names = Self::get_column_names_from_lazyframe(&lf);

                self.lazyframe = Some(lf);
                self.settings.column_names = column_names;
            }
            Err(e) => {
                self.lazyframe = None;
                log::error!("Failed to load selected Parquet files: {e}");
            }
        }
    }

    pub fn filter_selected_files_and_save(&self) {
        // Gather checked files
        let checked_files = self.checked_files();

        if checked_files.is_empty() {
            log::error!("No files selected for filtering.");
            return;
        }

        if !matches!(self.checked_file_format(), SelectedFileFormat::Parquet) {
            log::error!("Save Filtered Files only supports selected Parquet inputs.");
            return;
        }

        // Keep only parquet files
        let parquet_files = self.checked_parquet_files();

        if parquet_files.is_empty() {
            log::warn!("No selected Parquet files to process.");
            return;
        }

        let cut = self.histogram_script.active_filter_cuts(&self.histogrammer);
        if cut.is_empty() {
            log::warn!(
                "No active cuts are enabled in the Histogram Script. Nothing to apply while saving filtered files."
            );
            return;
        }

        log::info!(
            "Saving filtered parquet files using {} active cut(s) from the Histogram Script.",
            cut.cuts.len()
        );

        let saved_cut_suffix = self.settings.saved_cut_suffix.clone();

        // Initialize UI state
        self.settings
            .saving_in_progress
            .store(true, Ordering::Relaxed);
        if let Ok(mut p) = self.settings.save_progress.lock() {
            *p = 0.0;
        }

        // Clone Arcs for worker thread
        let saving_flag = self.settings.saving_in_progress.clone();
        let save_progress = self.settings.save_progress.clone();

        let total_files = parquet_files.len().max(1); // avoid div-by-zero

        // Spawn the filtering task on a new thread
        thread::spawn(move || {
            for (i, file) in parquet_files.into_iter().enumerate() {
                eprintln!("Processing file: {file:?}");
                let file_stem = file
                    .file_stem()
                    .expect("Failed to get file stem")
                    .to_string_lossy();
                let new_file_name = if saved_cut_suffix.is_empty() {
                    format!("{file_stem}_filtered")
                } else {
                    format!("{file_stem}_{saved_cut_suffix}")
                };
                let new_file_path = file.with_file_name(format!("{new_file_name}.parquet"));

                log::info!("Processing file: {file:?}");
                log::info!("Saving filtered file as: {new_file_path:?}");

                // Load and collect one file at a time
                match PlRefPath::try_from_pathbuf(file.clone()) {
                    Ok(path) => match LazyFrame::scan_parquet(path, Default::default()) {
                        Ok(lf) => match lf.collect() {
                            Ok(df) => {
                                if let Err(e) = cut.filter_df_and_save(
                                    &df,
                                    new_file_path
                                        .to_str()
                                        .expect("Failed to convert path to str"),
                                ) {
                                    log::error!(
                                        "Failed to save filtered DataFrame for {file:?}: {e}"
                                    );
                                } else {
                                    log::info!(
                                        "Successfully saved filtered DataFrame: {new_file_path:?}"
                                    );
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to collect DataFrame from LazyFrame: {e} ({file:?})"
                                );
                            }
                        },
                        Err(e) => log::error!("Failed to read Parquet file {file:?}: {e}"),
                    },
                    Err(e) => log::error!("Failed to convert Parquet path {file:?}: {e}"),
                }

                // Update progress after each file
                if let Ok(mut p) = save_progress.lock() {
                    *p = ((i + 1) as f32) / (total_files as f32);
                }
            }

            // Finalize state
            if let Ok(mut p) = save_progress.lock() {
                *p = 1.0;
            }
            saving_flag.store(false, Ordering::Relaxed);
        });
    }

    pub fn combine_and_save_selected_files(&self) {
        // Gather checked files
        let checked_files = self.checked_files();

        if checked_files.is_empty() {
            log::error!("No files selected for combination.");
            return;
        }

        if matches!(
            self.checked_file_format(),
            SelectedFileFormat::Mixed | SelectedFileFormat::Root
        ) {
            log::error!(
                "Combine Selected Files only supports Parquet inputs. Select only .parquet files before starting."
            );
            return;
        }

        // Keep only parquet files
        let parquet_files = self.checked_parquet_files();

        if parquet_files.is_empty() {
            log::warn!("No selected Parquet files to combine.");
            return;
        }

        self.start_combine_and_save_selected_files(parquet_files);
    }

    fn start_combine_and_save_selected_files(&self, parquet_files: Vec<PathBuf>) {
        // Ask where to save the combined parquet
        if let Some(output_file) = rfd::FileDialog::new()
            .set_title("Save Combined Parquet File")
            .add_filter("Parquet Files", &["parquet"])
            .save_file()
        {
            let output_file = Self::ensure_parquet_extension(output_file);

            if parquet_files.iter().any(|file| file == &output_file) {
                log::error!(
                    "The output file cannot overwrite one of the selected input Parquet files."
                );
                return;
            }

            // Initialize UI state
            self.settings
                .combining_in_progress
                .store(true, Ordering::Relaxed);

            // Clone Arcs for thread
            let combining_flag = self.settings.combining_in_progress.clone();
            let output_file_clone = output_file.clone();

            // Capture inputs for the worker thread
            let total_files = parquet_files.len().max(1);

            std::thread::spawn(move || {
                let input_paths: Result<polars_buffer::Buffer<PlRefPath>, PolarsError> =
                    parquet_files
                        .into_iter()
                        .map(PlRefPath::try_from_pathbuf)
                        .collect();

                let input_paths = match input_paths {
                    Ok(paths) => paths,
                    Err(e) => {
                        log::error!("Failed to convert selected Parquet paths: {e}");
                        combining_flag.store(false, Ordering::Relaxed);
                        return;
                    }
                };

                let scan_args = ScanArgsParquet {
                    low_memory: true,
                    cache: false,
                    ..Default::default()
                };

                let combined_lazyframe = match LazyFrame::scan_parquet_files(input_paths, scan_args)
                {
                    Ok(lf) => lf,
                    Err(e) => {
                        log::error!("Failed to scan selected Parquet files for combine: {e}");
                        combining_flag.store(false, Ordering::Relaxed);
                        return;
                    }
                };

                let output_path = match PlRefPath::try_from_pathbuf(output_file_clone.clone()) {
                    Ok(path) => path,
                    Err(e) => {
                        log::error!("Failed to convert output path {output_file_clone:?}: {e}");
                        combining_flag.store(false, Ordering::Relaxed);
                        return;
                    }
                };

                let parquet_write_options = Arc::new(ParquetWriteOptions::default());
                let sink = match combined_lazyframe.sink(
                    SinkDestination::File {
                        target: SinkTarget::Path(output_path),
                    },
                    FileWriteFormat::Parquet(parquet_write_options),
                    UnifiedSinkArgs::default(),
                ) {
                    Ok(sink) => sink,
                    Err(e) => {
                        log::error!("Failed to prepare streamed Parquet combine: {e}");
                        combining_flag.store(false, Ordering::Relaxed);
                        return;
                    }
                };

                match sink.collect_with_engine(Engine::Auto) {
                    Ok(_) => {
                        log::info!(
                            "Successfully combined {total_files} Parquet files into {output_file_clone:?} using the streamed writer."
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to stream combined Parquet output: {e}");
                    }
                }

                // Clear in-progress state
                combining_flag.store(false, Ordering::Relaxed);
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
        let checked_files = self.checked_files();

        if checked_files.is_empty() {
            log::error!("No files selected for histogram calculation.");
            return;
        }

        if self.checked_file_format() == SelectedFileFormat::Mixed {
            log::error!("Select only parquet files or only root files before starting.");
            return;
        }

        if self.settings.calculate_histograms_seperately {
            for file in &checked_files {
                let prefix = file
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string());

                if file.extension().is_some_and(|ext| ext == "parquet") {
                    self.create_lazyframe(std::slice::from_ref(file));
                    self.perform_histogrammer_from_lazyframe(prefix);
                } else if file.extension().is_some_and(|ext| ext == "root") {
                    self.get_histograms_from_root_files(std::slice::from_ref(file))
                        .unwrap_or_else(|e| {
                            log::error!("Error processing ROOT file {file:?}: {e:?}");
                        });
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

    pub fn left_side_panels_ui(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("spectrix_processor_left_panel").show_animated_inside(
            ui,
            self.settings.left_panel_open,
            |ui| {
                if let Some(paths) = self.file_dialog.take_picked_multiple() {
                    for path in paths {
                        if path.is_dir() {
                            self.file_dialog = FileDialog::new().initial_directory(path.clone());
                            if let Ok(entries) = std::fs::read_dir(&path) {
                                for entry in entries.flatten() {
                                    let file_path = entry.path();
                                    if let Some(ext) = file_path.extension()
                                        && (ext == "parquet" || ext == "root")
                                    {
                                        self.add_selected_file(file_path, true);
                                    }
                                }
                            }
                        } else if let Some(ext) = path.extension() {
                            // If it's a file, check if it's a valid type
                            if ext == "parquet" || ext == "root" {
                                self.add_selected_file(path, true);
                            }
                        }
                    }
                    self.sort_selected_files();
                }

                if ui
                    .add_enabled(
                        !self.selected_files.is_empty()
                            && self.checked_file_format() != SelectedFileFormat::Mixed,
                        egui::Button::new(self.histogram_action_label())
                            .min_size(egui::vec2(ui.available_width(), 0.0)),
                    )
                    .on_disabled_hover_text(self.histogram_action_disabled_reason())
                    .clicked()
                {
                    self.calculate_histograms();
                }

                if ui
                    .add(
                        egui::Button::selectable(
                            self.settings.histogram_script_open,
                            "Open Histogram Script",
                        )
                        .min_size(egui::vec2(ui.available_width(), 0.0)),
                    )
                    .clicked()
                {
                    self.settings.histogram_script_open = !self.settings.histogram_script_open;
                }

                if self.histogrammer.calculating.load(Ordering::Relaxed) {
                    ui.separator();

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

                ui.separator();

                self.selected_files_ui(ui);
            },
        );

        egui::Panel::left("spectrix_histogram_panel").show_animated_inside(
            ui,
            self.settings.histogram_script_open && self.settings.left_panel_open,
            |ui| {
                self.histogram_script.ui(ui, &self.histogrammer);
            },
        );

        self.panel_toggle_button(ui);
    }

    pub fn panel_toggle_button(&mut self, ui: &mut egui::Ui) {
        // Secondary left panel for the toggle button
        egui::Panel::left("spectrix_toggle_left_panel")
            .resizable(false)
            .show_separator_line(false)
            .min_size(1.0)
            .show_inside(ui, |ui| {
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

    fn add_selected_file(&mut self, path: PathBuf, checked: bool) {
        if !self.selected_files.iter().any(|(file, _)| file == &path) {
            self.selected_files.push((path, checked));
        }
    }

    fn refresh_selected_files_from_directory(&mut self, directory: &std::path::Path) {
        let previous_selection = self
            .selected_files
            .iter()
            .map(|(path, checked)| (path.clone(), *checked))
            .collect::<std::collections::HashMap<_, _>>();

        let mut refreshed_files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(directory) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension()
                    && (ext == "parquet" || ext == "root")
                {
                    let checked = previous_selection.get(&file_path).copied().unwrap_or(true);
                    refreshed_files.push((file_path, checked));
                }
            }
        }

        self.selected_files = refreshed_files;
        self.sort_selected_files();
    }

    fn sort_selected_files(&mut self) {
        let sort_state = self.file_sort;
        self.selected_files.sort_by(|a, b| {
            let ordering = match sort_state.key {
                FileSortKey::Name => natural_path_cmp(&a.0, &b.0),
                FileSortKey::Size => compare_file_size(&a.0, &b.0),
                FileSortKey::Modified => compare_file_modified(&a.0, &b.0),
            };

            let ordering = if ordering == CmpOrdering::Equal {
                natural_path_cmp(&a.0, &b.0)
            } else {
                ordering
            };

            if sort_state.ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
    }

    pub fn selected_files_ui(&mut self, ui: &mut egui::Ui) {
        if ui
            .add(
                egui::Button::new("Get Files/Directory")
                    .min_size(egui::vec2(ui.available_width(), 0.0)),
            )
            .clicked()
        {
            self.file_dialog.pick_multiple();
        }

        if !self.selected_files.is_empty() {
            ui.add_space(4.0);
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

                egui::ComboBox::from_id_salt("selected_files_sort_key")
                    .selected_text(format!("Sort: {}", self.file_sort.key.short_label()))
                    .show_ui(ui, |ui| {
                        for key in [FileSortKey::Name, FileSortKey::Size, FileSortKey::Modified] {
                            if ui
                                .selectable_value(&mut self.file_sort.key, key, key.label())
                                .changed()
                            {
                                self.sort_selected_files();
                            }
                        }
                    });

                if ui
                    .button(if self.file_sort.ascending {
                        "Ascending"
                    } else {
                        "Descending"
                    })
                    .on_hover_text("Toggle file sort direction")
                    .clicked()
                {
                    self.file_sort.ascending = !self.file_sort.ascending;
                    self.sort_selected_files();
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
                            self.refresh_selected_files_from_directory(common_dir);
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
                    // button to get the column names from the selected files
                    if ui
                        .button("Get Column Names")
                        .on_hover_text("Get the column names from the selected files")
                        .clicked()
                    {
                        let checked_files: Vec<PathBuf> = self
                            .selected_files
                            .iter()
                            .filter(|(_, checked)| *checked)
                            .map(|(file, _)| file.clone())
                            .collect();
                        self.create_lazyframe(&checked_files);
                    }

                    if self.lazyframe.is_some() {
                        ui.label(format!(
                            "Columns: {}",
                            self.settings.column_names.join(", ")
                        ));
                    } else {
                        ui.label("Columns: (none loaded)");
                    }

                    ui.separator();

                    // Save Filtered Files controls + live status
                    ui.horizontal_wrapped(|ui| {
                        let saving = self.settings.saving_in_progress.load(Ordering::Relaxed);
                        let active_filter_cut_count = self.active_filter_cut_count();
                        let save_enabled = !saving
                            && self.save_filtered_files_button_enabled(active_filter_cut_count);
                        let save_hover_text =
                            self.save_filtered_files_hover_text(active_filter_cut_count);
                        let save_disabled_reason = self
                            .save_filtered_files_disabled_reason(saving, active_filter_cut_count);

                        let save_response =
                            ui.add_enabled(save_enabled, egui::Button::new("Save Filtered Files"));
                        let save_response = if save_enabled {
                            save_response.on_hover_text(save_hover_text)
                        } else {
                            save_response.on_disabled_hover_text(save_disabled_reason)
                        };

                        if save_response.clicked() {
                            self.filter_selected_files_and_save();
                        }

                        if saving {
                            ui.label("Saving…");
                            ui.add(egui::widgets::Spinner::default());
                            let p = self
                                .settings
                                .save_progress
                                .lock()
                                .map(|g| *g)
                                .unwrap_or(0.0);
                            ui.add(
                                egui::widgets::ProgressBar::new(p)
                                    .animate(true)
                                    .show_percentage()
                                    .desired_width(100.0),
                            );
                        }

                        ui.horizontal(|ui| {
                            ui.label("Suffix:");
                            ui.add_enabled(
                                !saving,
                                egui::TextEdit::singleline(&mut self.settings.saved_cut_suffix),
                            );
                        });
                    });

                    // Combine Selected Files controls + live status
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        let combining = self.settings.combining_in_progress.load(Ordering::Relaxed);
                        let checked_parquet_file_count = self.checked_parquet_files().len();
                        let combine_enabled = !combining
                            && self
                                .combine_selected_files_button_enabled(checked_parquet_file_count);
                        let combine_disabled_reason = self.combine_selected_files_disabled_reason(
                            combining,
                            checked_parquet_file_count,
                        );

                        let combine_response = ui.add_enabled(
                            combine_enabled,
                            egui::Button::new("Combine Selected Files"),
                        );
                        let combine_response = if combine_enabled {
                            combine_response.on_hover_text(self.combine_selected_files_hover_text())
                        } else {
                            combine_response.on_disabled_hover_text(combine_disabled_reason)
                        };

                        if combine_response.clicked() {
                            self.combine_and_save_selected_files();
                        }

                        if combining {
                            ui.label("Combining…");
                            ui.add(egui::widgets::Spinner::default());
                        }
                    });
                });
            }
        });
    }

    pub fn bottom_panel(&mut self, ui: &mut egui::Ui) {
        if self.histogrammer.calculating.load(Ordering::Relaxed)
            || self.settings.saving_in_progress.load(Ordering::Relaxed)
            || self.settings.combining_in_progress.load(Ordering::Relaxed)
        {
            egui::Panel::bottom("spectrix_bottom_panel").show_inside(ui, |ui| {
                // existing histogrammer progress bar...
                if self.histogrammer.calculating.load(Ordering::Relaxed) {
                    ui.add(
                        egui::widgets::ProgressBar::new(
                            self.histogrammer.progress.lock().map(|x| *x).unwrap_or(0.0),
                        )
                        .animate(true)
                        .show_percentage(),
                    );
                }

                // ADD save progress
                if self.settings.saving_in_progress.load(Ordering::Relaxed) {
                    let p = self
                        .settings
                        .save_progress
                        .lock()
                        .map(|g| *g)
                        .unwrap_or(0.0);
                    ui.label("Saving filtered files…");
                    ui.add(
                        egui::widgets::ProgressBar::new(p)
                            .animate(true)
                            .show_percentage(),
                    );
                }

                // ADD combine progress
                if self.settings.combining_in_progress.load(Ordering::Relaxed) {
                    ui.horizontal(|ui| {
                        ui.label("Combining files…");
                        ui.add(egui::widgets::Spinner::default());
                    });
                }
            });
        }
    }

    fn central_panel_ui(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.histogrammer.ui(ui);
        });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.left_side_panels_ui(ui);
        self.bottom_panel(ui);
        self.central_panel_ui(ui);

        self.analysis
            .ui(ui, &self.selected_files, &mut self.histogrammer);

        self.file_dialog.update(ui);
    }
}

impl Default for Processor {
    fn default() -> Self {
        Self::new(String::new())
    }
}

fn compare_file_size(path_a: &std::path::Path, path_b: &std::path::Path) -> CmpOrdering {
    let value_a = std::fs::metadata(path_a)
        .ok()
        .map(|metadata| metadata.len());
    let value_b = std::fs::metadata(path_b)
        .ok()
        .map(|metadata| metadata.len());
    value_a.cmp(&value_b)
}

fn compare_file_modified(path_a: &std::path::Path, path_b: &std::path::Path) -> CmpOrdering {
    let value_a = std::fs::metadata(path_a)
        .ok()
        .and_then(|metadata| metadata.modified().ok());
    let value_b = std::fs::metadata(path_b)
        .ok()
        .and_then(|metadata| metadata.modified().ok());

    match (value_a.as_ref(), value_b.as_ref()) {
        (Some(a), Some(b)) => a.cmp(b),
        (None, Some(_)) => CmpOrdering::Less,
        (Some(_), None) => CmpOrdering::Greater,
        (None, None) => CmpOrdering::Equal,
    }
}

fn natural_path_cmp(path_a: &std::path::Path, path_b: &std::path::Path) -> CmpOrdering {
    let name_a = path_a
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let name_b = path_b
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    natural_str_cmp(name_a, name_b).then_with(|| path_a.cmp(path_b))
}

fn natural_str_cmp(a: &str, b: &str) -> CmpOrdering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (Some(a_char), Some(b_char)) if a_char.is_ascii_digit() && b_char.is_ascii_digit() => {
                let a_number = take_numeric_chunk(&mut a_chars);
                let b_number = take_numeric_chunk(&mut b_chars);

                let ordering = compare_numeric_chunks(&a_number, &b_number);
                if ordering != CmpOrdering::Equal {
                    return ordering;
                }
            }
            (Some(_), Some(_)) => {
                let ordering = a_chars
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .cmp(&b_chars.next().unwrap_or_default().to_ascii_lowercase());

                if ordering != CmpOrdering::Equal {
                    return ordering;
                }
            }
            (Some(_), None) => return CmpOrdering::Greater,
            (None, Some(_)) => return CmpOrdering::Less,
            (None, None) => return CmpOrdering::Equal,
        }
    }
}

fn take_numeric_chunk(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut chunk = String::new();

    while let Some(ch) = chars.peek() {
        if ch.is_ascii_digit() {
            chunk.push(*ch);
            chars.next();
        } else {
            break;
        }
    }

    chunk
}

fn compare_numeric_chunks(a: &str, b: &str) -> CmpOrdering {
    let trimmed_a = a.trim_start_matches('0');
    let trimmed_b = b.trim_start_matches('0');
    let normalized_a = if trimmed_a.is_empty() { "0" } else { trimmed_a };
    let normalized_b = if trimmed_b.is_empty() { "0" } else { trimmed_b };

    normalized_a
        .len()
        .cmp(&normalized_b.len())
        .then_with(|| normalized_a.cmp(normalized_b))
        .then_with(|| a.len().cmp(&b.len()))
}
