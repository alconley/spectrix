use crate::histoer::histogrammer;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Write as _};

// UUID to (energy, uncertainty) mapping for fits
#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct FitUUID {
    pub uuid: usize,
    pub energy: (f64, f64),
}

impl FitUUID {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(&mut self.uuid).speed(1));
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.energy.0));
            ui.label("±");
            ui.add(egui::DragValue::new(&mut self.energy.1));
            ui.label("keV");
        });
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct FitUUIDMap {
    pub map: Vec<FitUUID>,

    // UI sorting flags (unchanged)
    #[serde(skip)]
    sort_uuid_asc: bool,
    #[serde(skip)]
    sort_energy_asc: bool,

    // background sync state
    #[serde(skip)]
    syncing: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[serde(skip)]
    abort: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[serde(skip)]
    progress: std::sync::Arc<std::sync::Mutex<f32>>,
}

impl FitUUIDMap {
    /// Simple table + actions. Call this from your panel.
    pub fn ui(&mut self, ui: &mut egui::Ui, histogrammer: Option<&mut histogrammer::Histogrammer>) {
        ui.horizontal(|ui| {

            if ui.button("Load CSV").on_hover_text("Format->UUID,Energy,Uncertainty (with header)").clicked() {
                self.load_fit_uuid_csv();
            }

            if ui.button("Save CSV").clicked() {
                self.save_fit_uuid_csv();
            }

            if let Some(hist) = histogrammer {
                ui.horizontal(|ui| {
                    if self.syncing.load(std::sync::atomic::Ordering::Relaxed) {
                        ui.label("Syncing…");
                        ui.add(egui::widgets::Spinner::default());
                        if let Ok(p) = self.progress.lock() {
                            ui.add(egui::ProgressBar::new(*p).show_percentage().desired_width(100.0));
                        }
                        if ui.button("Cancel").clicked() {
                            self.abort.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    } else if ui.button("Sync ➡ Histograms")
                        .on_hover_text("Runs off-thread; writes energies into lmfit results for each histogram")
                        .clicked()
                    {
                        self.sync_uuid_with_histogrammer_bg(hist);
                    }
                });
            }
        });

        let mut to_remove: Option<usize> = None;

        egui::Grid::new("fit_uuid_map_grid")
            .striped(true)
            .num_columns(3)
            .show(ui, |ui| {
                // ---- Header row with sort buttons ----
                ui.horizontal(|ui| {
                    ui.label("UUID");
                    let label = if self.sort_uuid_asc { "⬆" } else { "⬇" };
                    if ui
                        .small_button(label)
                        .on_hover_text("Sort by UUID")
                        .clicked()
                    {
                        self.map.sort_by_key(|r| r.uuid);
                        if !self.sort_uuid_asc {
                            self.map.reverse();
                        }
                        self.sort_uuid_asc = !self.sort_uuid_asc;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Energy");
                    let label = if self.sort_energy_asc { "⬆" } else { "⬇" };
                    if ui
                        .small_button(label)
                        .on_hover_text("Sort by Energy value")
                        .clicked()
                    {
                        // Sort by energy.0, then by uncertainty to stabilize
                        self.map.sort_by(|a, b| {
                            match a
                                .energy
                                .0
                                .partial_cmp(&b.energy.0)
                                .unwrap_or(Ordering::Equal)
                            {
                                Ordering::Equal => a
                                    .energy
                                    .1
                                    .partial_cmp(&b.energy.1)
                                    .unwrap_or(Ordering::Equal),
                                ord => ord,
                            }
                        });
                        if !self.sort_energy_asc {
                            self.map.reverse();
                        }
                        self.sort_energy_asc = !self.sort_energy_asc;
                    }
                });
                ui.label("");
                ui.end_row();

                for (i, row) in self.map.iter_mut().enumerate() {
                    row.ui(ui);
                    if ui.button("X").clicked() {
                        to_remove = Some(i);
                    }
                    ui.end_row();
                }
            });

        if let Some(i) = to_remove {
            self.map.remove(i);
        }

        if ui.button("Add Row").clicked() {
            self.map.push(FitUUID::default());
        }
    }

    pub fn sync_uuid_with_histogrammer_bg(
        &mut self,
        histogrammer: &mut crate::histoer::histogrammer::Histogrammer,
    ) {
        use egui_tiles::Tile;
        use std::sync::atomic::Ordering;
        use std::sync::{Arc, Mutex};
        use std::thread;

        // prevent re-entry
        if self.syncing.load(Ordering::Relaxed) {
            log::warn!("Sync already running");
            return;
        }

        // Collect handles ON THE UI THREAD
        let mut handles: Vec<Arc<Mutex<Box<crate::histoer::histo1d::histogram1d::Histogram>>>> =
            Vec::new();
        for (_id, tile) in histogrammer.tree.tiles.iter_mut() {
            if let Tile::Pane(pane) = tile
                && let crate::histoer::pane::Pane::Histogram(h_arc) = pane
            {
                handles.push(h_arc.clone()); // Arc<Mutex<..>>
            }
        }

        // Clone the data to move into the worker
        let map = self.map.clone();
        let syncing = self.syncing.clone();
        let abort = self.abort.clone();
        let progress = self.progress.clone();

        syncing.store(true, Ordering::Relaxed);
        abort.store(false, Ordering::Relaxed);
        if let Ok(mut p) = progress.lock() {
            *p = 0.0;
        }

        thread::spawn(move || {
            let total = handles.len().max(1);
            for (i, h) in handles.into_iter().enumerate() {
                if abort.load(Ordering::Relaxed) {
                    log::info!("Sync aborted");
                    break;
                }
                if let Ok(mut hist) = h.lock() {
                    hist.fits.sync_uuid(&map);
                }
                if let Ok(mut p) = progress.lock() {
                    *p = (i + 1) as f32 / total as f32;
                }
            }
            syncing.store(false, Ordering::Relaxed);
        });
    }

    pub fn save_fit_uuid_csv(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("fit_uuid_map.csv")
            .save_file()
        {
            let mut f = match File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create CSV: {e}");
                    return;
                }
            };

            // Header exactly as requested
            if let Err(e) = writeln!(f, "UUID,Energy,Uncertainity") {
                log::error!("Failed to write header: {e}");
                return;
            }

            for row in &self.map {
                if let Err(e) = writeln!(f, "{},{},{}", row.uuid, row.energy.0, row.energy.1) {
                    log::error!("Failed to write row for UUID {}: {e}", row.uuid);
                    return;
                }
            }

            log::info!("Saved Fit UUID CSV to {path:?}");
        }
    }

    pub fn load_fit_uuid_csv(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .pick_file()
        {
            let f = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to open CSV: {e}");
                    return;
                }
            };
            let mut rdr = BufReader::new(f);

            // Read first line (header). Accept both "Uncertainity" and "Uncertainty".
            let mut header = String::new();
            if let Err(e) = rdr.read_line(&mut header) {
                log::error!("Failed to read header: {e}");
                return;
            }
            let normalized = header.trim().replace(' ', "");
            let ok_header = normalized.eq_ignore_ascii_case("UUID,Energy,Uncertainity")
                || normalized.eq_ignore_ascii_case("UUID,Energy,Uncertainty");
            if !ok_header {
                log::warn!("Unexpected header '{}'; continuing anyway.", header.trim());
            }

            let mut out: Vec<FitUUID> = Vec::new();
            for (lineno, line_res) in rdr.lines().enumerate() {
                let line_no = lineno + 2; // header already read
                let line = match line_res {
                    Ok(s) => s.trim().to_owned(),
                    Err(e) => {
                        log::error!("Read error on line {line_no}: {e}");
                        break;
                    }
                };
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let parts: Vec<_> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() < 3 {
                    log::warn!("Skipping malformed line {line_no}: '{line}'");
                    continue;
                }

                let uuid: usize = match parts[0].parse() {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("Bad UUID on line {line_no}: {e}");
                        continue;
                    }
                };
                let energy: f64 = match parts[1].parse() {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("Bad Energy on line {line_no}: {e}");
                        continue;
                    }
                };
                let uncert: f64 = match parts[2].parse() {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("Bad Uncertainity on line {line_no}: {e}");
                        continue;
                    }
                };

                out.push(FitUUID {
                    uuid,
                    energy: (energy, uncert),
                });
            }

            self.map = out;
            log::info!("Loaded {} UUID rows from {:?}", self.map.len(), path);
        }
    }
}
