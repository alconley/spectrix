use crate::histoer::histogrammer::Histogrammer;
use crate::histogram_scripter::histogram_script::HistogramScript;
use pyo3::{prelude::*, types::PyModule};

use super::egui_file_dialog::FileDialog;
use polars::prelude::*;

use std::sync::atomic::Ordering;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ProcessorSettings {
    pub dialog_open: bool,
    pub histogram_script_open: bool,
    pub column_names: Vec<String>,
    pub test: Vec<String>,
}

impl Default for ProcessorSettings {
    fn default() -> Self {
        Self {
            dialog_open: true,
            histogram_script_open: true,
            column_names: Vec::new(),
            test: Vec::new(),
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Processor {
    #[serde(skip)]
    pub file_dialog: Option<FileDialog>,
    pub selected_files: Vec<std::path::PathBuf>,
    #[serde(skip)]
    pub lazyframe: Option<LazyFrame>,
    pub histogrammer: Histogrammer,
    pub histogram_script: HistogramScript,
    pub settings: ProcessorSettings,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            file_dialog: None,
            selected_files: Vec::new(),
            lazyframe: None,
            histogrammer: Histogrammer::default(),
            histogram_script: HistogramScript::new(),
            settings: ProcessorSettings::default(),
        }
    }

    pub fn reset(&mut self) {
        self.lazyframe = None;
        self.histogrammer = Histogrammer::default();
    }

    pub fn get_histograms_from_root_files(&mut self) -> PyResult<()> {
        // python3 -m venv .venv
        // source .venv/bin/activate
        // export PYO3_PYTHON=$(pwd)/.venv/bin/python
        // export PYTHONPATH=$(pwd)/.venv/lib/python3.12/site-packages
        // cargo run --release

        Python::with_gil(|py| {
            let sys = py.import_bound("sys")?;
            let version: String = sys.getattr("version")?.extract()?;
            let executable: String = sys.getattr("executable")?.extract()?;
            println!("Using Python version: {}", version);
            println!("Python executable: {}", executable);

            // Check if the `uproot` module can be imported
            match py.import_bound("uproot") {
                Ok(_) => {
                    println!("Successfully imported `uproot` module.");
                }
                Err(_) => {
                    eprintln!("Error: `uproot` module could not be found. Make sure you are using the correct Python environment with `uproot` installed.");
                    return Err(PyErr::new::<pyo3::exceptions::PyImportError, _>(
                        "`uproot` module not available",
                    ));
                }
            }

            // Define the Python code as a module
            let code = r#"
import uproot

def get_1d_histogram(file, name):
    """Get a 1D histogram from the ROOT file."""
    hist = file[name]
    bin_edges = hist.axis().edges().tolist()
    counts = hist.counts(flow=True).tolist()

    return name, counts, bin_edges

def get_2d_histogram(file, name):
    """Get a 2D histogram from the ROOT file."""
    hist = file[name]
    bin_edges_x = hist.axis("x").edges().tolist()
    bin_edges_y = hist.axis("y").edges().tolist()
    counts = hist.counts(flow=False).tolist()

    return name, counts, bin_edges_x, bin_edges_y

def get_1d_histograms(file_name):
    """Get all 1D histograms from the ROOT file."""
    with uproot.open(file_name) as file:
        hist1d_names = [key for key, value in file.classnames().items() if value in ['TH1D', 'TH1F', 'TH1I']]
        histograms = []
        for name in hist1d_names:
            name, counts, bin_edges = get_1d_histogram(file, name)
            histograms.append([name, counts, bin_edges])
        return histograms

def get_2d_histograms(file_name):
    """Get all 2D histograms from the ROOT file."""
    with uproot.open(file_name) as file:
        hist2d_names = [key for key, value in file.classnames().items() if value in ['TH2D', 'TH2F', 'TH2I']]
        histograms = []
        for name in hist2d_names:
            name, counts, bin_edges_x, bin_edges_y = get_2d_histogram(file, name)
            histograms.append([name, counts, bin_edges_x, bin_edges_y])
        return histograms
"#;

            // Compile the Python code into a module
            let module =
                PyModule::from_code_bound(py, code, "uproot_functions.py", "uproot_functions")?;

            let root_files = self
                .selected_files
                .iter()
                .filter(|file| file.extension().unwrap() == "root")
                .collect::<Vec<_>>();

            for file in root_files.iter() {
                let file_name = file.to_str().unwrap();

                let result_1d = module.getattr("get_1d_histograms")?.call1((file_name,))?;

                // log::info!("File: {}", file_name);
                // log::info!("Result 1D: {:?}", result_1d);

                let length_1d: usize = result_1d.len()?;
                // log::info!("Number of histograms: {}", length_1d);

                for i in 0..length_1d {
                    let item = result_1d.get_item(i)?;

                    // Extract the full path of the histogram
                    let full_name: String = item.get_item(0)?.extract()?;
                    // let grid_name = full_name.clone(); // Join the folder parts as the grid name
                    let mut counts: Vec<f64> = item.get_item(1)?.extract()?;
                    let underflow = counts.remove(0);
                    let overflow = counts.pop().unwrap();
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

                let result_2d = module.getattr("get_2d_histograms")?.call1((file_name,))?;
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
                        .add_hist2d_with_bin_values(&full_name, counts_u64, range);
                }
            }

            Ok(())
        })
    }
    
    fn create_lazyframe(&mut self) {
        // get all the parquet files from the selected files
        let parquet_files: Vec<std::path::PathBuf> = self
            .selected_files
            .iter()
            .filter(|file| file.extension().unwrap() == "parquet")
            .cloned()
            .collect();

        // warn if no parquet files are selected
        if parquet_files.is_empty() {
            log::warn!("No Parquet files selected.");
            return;
        }

        let files_arc: Arc<[std::path::PathBuf]> = Arc::from(parquet_files);
        let args = ScanArgsParquet::default();
        log::info!("Files {:?}", files_arc);

        match LazyFrame::scan_parquet_files(files_arc, args) {
            Ok(lf) => {
                log::info!("Loaded Parquet files");
                let column_names = Self::get_column_names_from_lazyframe(&lf);

                self.lazyframe = Some(lf);
                self.settings.column_names = column_names;
            }
            Err(e) => {
                self.lazyframe = None; // Indicates that loading failed
                log::error!("Failed to load Parquet files: {}", e);
            }
        }
    }

    fn get_column_names_from_lazyframe(lazyframe: &LazyFrame) -> Vec<String> {
        let lf: LazyFrame = lazyframe.clone().limit(1);
        let df: DataFrame = lf.collect().unwrap();
        let columns: Vec<String> = df
            .get_column_names_owned()
            .into_iter()
            .map(|name| name.to_string())
            .collect();

        columns
    }

    fn perform_histogrammer_from_lazyframe(&mut self) {
        if let Some(lf) = &self.lazyframe {
            self.histogram_script
                .add_histograms(&mut self.histogrammer, lf.clone());
        } else {
            log::error!("Failed to preform histogrammer: LazyFrame is None.");
        }
    }

    pub fn calculate_histograms(&mut self) {
        // Check if the files are Parquet files
        if self
            .selected_files
            .iter()
            .any(|file| match file.extension() {
                Some(ext) => ext == "parquet",
                None => false,
            })
        {
            self.create_lazyframe();
            self.perform_histogrammer_from_lazyframe();
        }
        // Check if the files are ROOT files
        else if self
            .selected_files
            .iter()
            .any(|file| match file.extension() {
                Some(ext) => ext == "root",
                None => false,
            })
        {
            let _ = self.get_histograms_from_root_files();
        }
        // No valid files selected
        else {
            log::error!("No Parquet files or ROOT files selected.");
        }
    }

    fn open_file_dialog(&mut self) {
        self.file_dialog = Some(FileDialog::open_file(None).multi_select(true));
        if let Some(dialog) = &mut self.file_dialog {
            dialog.set_filter(".parquet");
            dialog.open(); // Modify the dialog in-place to open it
        }
    }

    pub fn left_side_panels_ui(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("spectrix_processor_left_panel").show_animated(
            ctx,
            self.settings.dialog_open,
            |ui| {
                ui.horizontal(|ui| {
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

                    if self.histogrammer.calculating.load(Ordering::Relaxed) {
                        // Show spinner while `calculating` is true
                        ui.add(egui::widgets::Spinner::default());
                    }

                    ui.separator();

                    if ui
                        .selectable_label(self.settings.histogram_script_open, "Histograms")
                        .clicked()
                    {
                        self.settings.histogram_script_open = !self.settings.histogram_script_open;
                    }
                });

                ui.separator();

                if self.file_dialog.is_none() {
                    self.open_file_dialog();
                }

                if let Some(dialog) = &mut self.file_dialog {
                    dialog.ui_embeded(ui);
                    self.selected_files = dialog.selected_file_paths();
                }
            },
        );

        egui::SidePanel::left("spectrix_histogram_panel").show_animated(
            ctx,
            self.settings.histogram_script_open && self.settings.dialog_open,
            |ui| {
                self.histogram_script.ui(ui);
            },
        );

        // Secondary left panel for the toggle button
        egui::SidePanel::left("spectrix_toggle_left_panel")
            .resizable(false)
            .show_separator_line(false)
            .min_width(1.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 10.0); // Center the button vertically
                    if ui
                        .small_button(if self.settings.dialog_open {
                            "◀"
                        } else {
                            "▶"
                        })
                        .clicked()
                    {
                        self.settings.dialog_open = !self.settings.dialog_open;
                    }
                });
            });
    }

    fn central_panel_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.histogrammer.ui(ui);
        });
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        self.left_side_panels_ui(ctx);
        self.central_panel_ui(ctx);
    }
}
