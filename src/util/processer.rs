use super::lazyframer::LazyFramer;
use super::workspacer::Workspacer;
use crate::cutter::cut_handler::CutHandler;
use crate::histoer::histogrammer::Histogrammer;
use crate::histogram_scripter::histogram_script::HistogramScript;
use pyo3::{prelude::*, types::PyModule};

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Processer {
    pub workspacer: Workspacer,
    #[serde(skip)]
    pub lazyframer: Option<LazyFramer>,
    pub cut_handler: CutHandler,
    pub histogrammer: Histogrammer,
    pub histogram_script: HistogramScript,
    pub save_with_scanning: bool,
    pub suffix: String,
}

impl Processer {
    pub fn new() -> Self {
        Self {
            workspacer: Workspacer::default(),
            lazyframer: None,
            cut_handler: CutHandler::default(),
            histogrammer: Histogrammer::default(),
            histogram_script: HistogramScript::new(),
            save_with_scanning: false,
            suffix: "filtered".to_string(),
        }
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

            for file in self.workspacer.selected_files.iter() {
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

                    // Split the path to get the grid name and the last part as the histogram name
                    let parts: Vec<&str> = full_name.split('/').collect();
                    let grid_name = parts[..parts.len() - 1].join("/"); // Join the folder parts as the grid name
                                                                        // let grid_name = full_name.clone(); // Join the folder parts as the grid name
                    log::info!("Grid name: {}", grid_name);
                    let hist_name = parts.last().unwrap(); // The last part is the histogram name

                    let mut counts: Vec<f64> = item.get_item(1)?.extract()?;
                    let underflow = counts.remove(0);
                    let overflow = counts.pop().unwrap();
                    let bin_edges: Vec<f64> = item.get_item(2)?.extract()?;
                    let range = (bin_edges[0], bin_edges[bin_edges.len() - 1]);

                    let counts_u64 = counts.iter().map(|&x| x as u64).collect::<Vec<u64>>();

                    self.histogrammer.add_hist1d_with_bin_values(
                        hist_name,
                        counts_u64,
                        underflow as u64,
                        overflow as u64,
                        range,
                        Some(grid_name.as_str()),
                    );
                }

                let result_2d = module.getattr("get_2d_histograms")?.call1((file_name,))?;
                let length_2d: usize = result_2d.len()?;

                for i in 0..length_2d {
                    let item = result_2d.get_item(i)?;

                    let full_name: String = item.get_item(0)?.extract()?;
                    let parts: Vec<&str> = full_name.split('/').collect();
                    let grid_name = parts[..parts.len() - 1].join("/");
                    let hist_name = parts.last().unwrap();

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

                    self.histogrammer.add_hist2d_with_bin_values(
                        hist_name,
                        counts_u64,
                        range,
                        Some(grid_name.as_str()),
                    );
                }
            }

            Ok(())
        })
    }

    pub fn reset(&mut self) {
        self.lazyframer = None;
        self.histogrammer = Histogrammer::default();
    }

    fn create_lazyframe(&mut self) {
        self.lazyframer = Some(LazyFramer::new(self.workspacer.selected_files.clone()));
    }

    fn perform_histogrammer_from_lazyframe(&mut self) {
        if let Some(lazyframer) = &self.lazyframer {
            if let Some(lf) = &lazyframer.lazyframe {
                self.histogram_script
                    .add_histograms(&mut self.histogrammer, lf.clone());
            } else {
                log::error!("LazyFrame is not loaded");
            }
        } else {
            log::error!("LazyFramer is not initialized");
        }
    }

    pub fn calculate_histograms(&mut self) {
        self.create_lazyframe();
        self.perform_histogrammer_from_lazyframe();
    }

    pub fn calculate_histograms_with_cuts(&mut self) {
        self.create_lazyframe();
        if let Some(ref mut lazyframer) = self.lazyframer {
            if let Some(ref lazyframe) = lazyframer.lazyframe {
                match self.cut_handler.filter_lf_with_selected_cuts(lazyframe) {
                    Ok(filtered_lf) => {
                        lazyframer.set_lazyframe(filtered_lf);
                        self.perform_histogrammer_from_lazyframe();
                    }
                    Err(e) => {
                        log::error!("Failed to filter LazyFrame with cuts: {}", e);
                    }
                }
            }
        }
    }

    pub fn save_selected_files_to_single_file(&mut self) {
        let scan = self.save_with_scanning;
        if let Some(output_path) = rfd::FileDialog::new()
            .set_title("Save the selected files to a single file")
            .add_filter("Parquet file", &["parquet"])
            .save_file()
        {
            match self
                .workspacer
                .save_selected_files_to_single_file(&output_path, scan)
            {
                Ok(_) => println!("Selected files saved successfully."),
                Err(e) => log::error!("Failed to save selected files: {}", e),
            }
        }
    }

    pub fn save_filtered_files_to_single_file(&mut self) {
        let scan = self.save_with_scanning;

        if let Some(output_path) = rfd::FileDialog::new()
            .set_title("Filter the files with the selected cuts and save to a single file")
            .add_filter("Parquet file", &["parquet"])
            .save_file()
        {
            match self.workspacer.save_filtered_files_to_single_file(
                &output_path,
                &mut self.cut_handler,
                scan,
            ) {
                Ok(_) => println!("Filtered files saved successfully."),
                Err(e) => log::error!("Failed to save filtered files: {}", e),
            }
        }
    }

    pub fn save_filtered_files_individually(&mut self, suffix: &str) {
        let scan = self.save_with_scanning;

        if let Some(output_dir) = rfd::FileDialog::new()
            .set_title("Select Output Directory to Save Filtered Files Individually")
            .pick_folder()
        {
            if !suffix.is_empty() {
                match self.workspacer.save_individually_filtered_files(
                    &output_dir,
                    &mut self.cut_handler,
                    suffix,
                    scan,
                ) {
                    Ok(_) => println!("Filtered files saved individually."),
                    Err(e) => log::error!("Failed to save filtered files individually: {}", e),
                }
            } else {
                log::error!("No suffix provided, operation canceled.");
            }
        } else {
            log::error!("No output directory selected, operation canceled.");
        }
    }

    pub fn saving_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Parquet Writer", |ui| {
            ui.checkbox(&mut self.save_with_scanning, "Save with Scanning")
                .on_hover_text(
                    "This can save files that are larger than memory at the cost of being slower.",
                );

            egui::Grid::new("parquet_writer_grid")
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("");
                    ui.label("Single File");
                    ui.label("Individually");
                    ui.label("Suffix     ");
                    ui.end_row();

                    ui.label("Non-Filtered");

                    if ui
                        .add_enabled(
                            self.workspacer.selected_files.len() > 1,
                            egui::Button::new("Save"),
                        )
                        .clicked()
                    {
                        self.save_selected_files_to_single_file();
                    }

                    ui.end_row();

                    ui.label("Cut Filtered");

                    if ui
                        .add_enabled(
                            self.cut_handler.cuts_are_selected()
                                && !self.workspacer.selected_files.is_empty(),
                            egui::Button::new("Save"),
                        )
                        .on_disabled_hover_text("No cuts selected.")
                        .clicked()
                    {
                        self.save_filtered_files_to_single_file();
                    }

                    if ui
                        .add_enabled(
                            self.cut_handler.cuts_are_selected()
                                && !self.workspacer.selected_files.is_empty(),
                            egui::Button::new("Save"),
                        )
                        .on_disabled_hover_text("No cuts selected.")
                        .clicked()
                    {
                        self.save_filtered_files_individually(&self.suffix.clone());
                    }

                    ui.text_edit_singleline(&mut self.suffix);

                    ui.end_row();
                });
        });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if !self.workspacer.options.root {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !self.workspacer.selected_files.is_empty(),
                        egui::Button::new("Calculate Histograms"),
                    )
                    .on_disabled_hover_text("No files selected.")
                    .clicked()
                {
                    self.calculate_histograms();
                }

                if ui
                    .add_enabled(
                        !self.workspacer.selected_files.is_empty()
                            && self.cut_handler.cuts_are_selected(),
                        egui::Button::new("with Cuts"),
                    )
                    .on_disabled_hover_text("No files selected or cuts selected.")
                    .clicked()
                {
                    self.calculate_histograms_with_cuts();
                }

                // add a spinner
                if !self.histogrammer.handles.is_empty() {
                    ui.separator();
                    ui.label("Calculating Histograms");
                    ui.add(egui::widgets::Spinner::default());
                }
            });

            ui.separator();
        } else if ui
            .add_enabled(
                !self.workspacer.selected_files.is_empty(),
                egui::Button::new("Get Histograms"),
            )
            .on_disabled_hover_text("No files selected.")
            .clicked()
        {
            let _ = self.get_histograms_from_root_files();
        }

        self.workspacer.workspace_ui(ui);

        ui.separator();

        if !self.workspacer.options.root {
            self.cut_handler.cut_ui(ui, &mut self.histogrammer);

            ui.separator();

            self.saving_ui(ui);

            ui.separator();

            if let Some(lazyframer) = &mut self.lazyframer {
                lazyframer.ui(ui);

                ui.separator();
            }
        }

        self.histogrammer.side_panel_ui(ui);
    }

    pub fn histogram_script_ui(&mut self, ui: &mut egui::Ui) {
        self.histogram_script.ui(ui);
    }
}
