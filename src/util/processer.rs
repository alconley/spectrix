use super::lazyframer::LazyFramer;
use super::workspacer::Workspacer;
use crate::histoer::histogrammer::Histogrammer;
use crate::histogram_scripter::histogram_script::HistogramScript;
use pyo3::{prelude::*, types::PyModule};

use std::sync::atomic::Ordering;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Processer {
    #[serde(skip)]
    pub workspacer: Workspacer,
    #[serde(skip)]
    pub lazyframer: Option<LazyFramer>,
    pub histogrammer: Histogrammer,
    pub histogram_script: HistogramScript,
    pub show_histogram_script: bool,
}

impl Processer {
    pub fn new() -> Self {
        Self {
            workspacer: Workspacer::new(),
            lazyframer: None,
            histogrammer: Histogrammer::default(),
            histogram_script: HistogramScript::new(),
            show_histogram_script: true,
        }
    }

    pub fn reset(&mut self) {
        self.lazyframer = None;
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

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if self.workspacer.options.root {
            self.show_histogram_script = false;
        }

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

                if self.histogrammer.calculating.load(Ordering::Relaxed) {
                    // Show spinner while `calculating` is true
                    ui.add(egui::widgets::Spinner::default());
                }

                ui.separator();

                if ui
                    .selectable_label(self.show_histogram_script, "Histograms")
                    .clicked()
                {
                    self.show_histogram_script = !self.show_histogram_script;
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
