use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::fitter::main_fitter::FitResult;
use crate::histoer::histogrammer;
use egui::{Color32, Vec2b};
use egui_plot::MarkerShape;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Write as _};

use crate::egui_plot_stuff::egui_points::EguiPoints;
use crate::fitter::models::gaussian::GaussianParameters;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Run {
    file_name: String,
    bci_scale: u32,
    bci_scaler: f64,
    bci_uncertainty: f64,   // percentage, e.g. 10 for 10%
    angle: f64,             // lab degrees
    slits: f64,             // msr
    slits_uncertainty: f64, // percentage, e.g. 10 for 10%
    magnetic_field: f64,    // kG
    normalization_factor: Option<(f64, f64)>,
    color: egui::Color32,
    markershape: String,
}

impl Run {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        changed |= ui
            .add(
                egui::DragValue::new(&mut self.bci_scaler)
                    .speed(1)
                    .range(0.0..=f64::INFINITY),
            )
            .on_hover_text("Beam current integrator scaler value from the run")
            .changed();

        changed |= ui
            .add(
                egui::DragValue::new(&mut self.bci_uncertainty)
                    .speed(1)
                    .range(0.0..=100.0)
                    .suffix(" %"),
            )
            .changed();

        changed |= egui::ComboBox::from_id_salt(format!("bci_scale_combo_{}", self.file_name))
            .selected_text(format!("{} nA", self.bci_scale))
            .show_ui(ui, |ui| {
                let mut any = false;
                for &option in &[1, 3, 10, 30, 100, 300] {
                    if ui
                        .selectable_value(&mut self.bci_scale, option, format!("{option} nA"))
                        .changed()
                    {
                        any = true;
                    }
                }
                any
            })
            .inner
            .unwrap_or(false);

        changed |= ui
            .add(egui::DragValue::new(&mut self.angle).speed(1.0).suffix("°"))
            .changed();

        changed |= ui
            .add(
                egui::DragValue::new(&mut self.slits)
                    .speed(0.1)
                    .suffix(" msr"),
            )
            .changed();

        changed |= ui
            .add(
                egui::DragValue::new(&mut self.slits_uncertainty)
                    .speed(1)
                    .range(0.0..=100.0)
                    .suffix(" %"),
            )
            .changed();

        changed |= ui
            .add(
                egui::DragValue::new(&mut self.magnetic_field)
                    .speed(1.0)
                    .suffix(" kG"),
            )
            .changed();

        // Normalization column (read-only)
        if let Some((val, unc)) = self.normalization_factor {
            ui.monospace(format!("{val:.3} ± {unc:.3}"));
        } else {
            ui.monospace("—");
        }

        changed |= ui.color_edit_button_srgba(&mut self.color).changed();

        changed |= egui::ComboBox::from_id_salt(format!("marker_shape_combo_{}", self.file_name))
            .selected_text(self.markershape.clone())
            .show_ui(ui, |ui| {
                let mut any = false;
                for option in &[
                    "Circle", "Diamond", "Square", "Cross", "Plus", "Up", "Down", "Left", "Right",
                    "Asterisk",
                ] {
                    if ui
                        .selectable_value(&mut self.markershape, (*option).to_owned(), *option)
                        .changed()
                    {
                        any = true;
                    }
                }
                any
            })
            .inner
            .unwrap_or(false);

        changed
    }

    /// Extract the closest number immediately before the given unit (case-insensitive).
    /// e.g. "`foo_27.5deg_bar`" -> `find_number_before_unit("deg`") == Some(27.5)
    fn find_number_before_unit(name: &str, unit: &str) -> Option<f64> {
        let lname = name.to_ascii_lowercase();
        let lunit = unit.to_ascii_lowercase();

        // Find the last occurrence to prefer the rightmost tag if multiple exist.
        let idx = lname.rfind(&lunit)?;
        if idx == 0 {
            return None;
        }

        // Walk left collecting characters that could be part of a number.
        // Allow digits, one dot, optional leading sign, and optional spaces/underscores.
        let bytes = lname.as_bytes();
        let end = idx; // exclusive
        let mut start = idx;

        // Skip whitespace/underscores right before unit
        while start > 0 {
            let b = bytes[start - 1];
            if b == b'_' || (b as char).is_ascii_whitespace() {
                start -= 1;
            } else {
                break;
            }
        }

        // Now collect the numeric token (digits, dot, sign) possibly with separators
        let mut seen_digit = false;
        let mut seen_dot = false;

        while start > 0 {
            let c = bytes[start - 1] as char;
            if c.is_ascii_digit() {
                seen_digit = true;
                start -= 1;
            } else if c == '.' {
                if seen_dot {
                    break;
                }
                seen_dot = true;
                start -= 1;
            } else if c == '-' || c == '+' {
                // sign only allowed at the very front of the token
                start -= 1;
                break;
            } else if c.is_ascii_whitespace() || c == '_' {
                // allow separators inside, but stop if we haven't seen any digit yet and next left is non-number
                start -= 1;
            } else {
                break;
            }
        }

        if !seen_digit {
            return None;
        }
        let token =
            &lname[start..end].trim_matches(|ch: char| ch.is_ascii_whitespace() || ch == '_');
        token.parse::<f64>().ok()
    }

    pub fn from_path(path: &Path) -> Self {
        let file_name_full = path.to_string_lossy().into_owned();
        let short = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Parse values from the short filename (e.g., "run_15deg_4.6msr_2.0kG.root")
        let angle = Self::find_number_before_unit(short, "deg").unwrap_or(0.0);
        let slits = Self::find_number_before_unit(short, "msr").unwrap_or(0.0);
        let magnetic_field = Self::find_number_before_unit(short, "kg").unwrap_or(0.0); // kG

        Self {
            file_name: file_name_full,
            bci_scale: 300,
            bci_uncertainty: 10.0,
            bci_scaler: 1.0,
            angle,
            slits,
            slits_uncertainty: 10.0,
            color: egui::Color32::from_rgb(255, 0, 0),
            magnetic_field,
            normalization_factor: None,
            markershape: "Circle".to_owned(),
        }
    }

    pub fn get_egui_markershape(&self) -> MarkerShape {
        match self.markershape.as_str() {
            // "Circle" => MarkerShape::Circle,
            "Diamond" => MarkerShape::Diamond,
            "Square" => MarkerShape::Square,
            "Cross" => MarkerShape::Cross,
            "Plus" => MarkerShape::Plus,
            "Up" => MarkerShape::Up,
            "Down" => MarkerShape::Down,
            "Left" => MarkerShape::Left,
            "Right" => MarkerShape::Right,
            "Asterisk" => MarkerShape::Asterisk,
            _ => MarkerShape::Circle,
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Runs {
    runs: Vec<Run>,

    // Sorting state
    #[serde(skip)]
    sort_column: Option<SortColumn>,
    #[serde(skip)]
    sort_ascending: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum SortColumn {
    File,
    BciScale,
    Angle,
    Slits,
    Field,
}

impl Runs {
    /// Draw the Runs table. Returns `true` if any field changed or a row was removed.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let mut index_to_remove: Option<usize> = None;

        ui.horizontal(|ui| {
            if ui
                .button("Load")
                .on_hover_text("Add any new files to the runs list")
                .clicked()
            {
                self.load_csv();
                changed = true;
            }

            if ui
                .button("Save")
                .on_hover_text("Save current runs to CSV")
                .clicked()
            {
                self.save_csv();
            }
        });

        ui.separator();

        egui::Grid::new("sps_runs_grid")
            .striped(true)
            .num_columns(10) // File | BCI Scaler | BCI Unc | BCI Scale | Angle | Slits | Slits Unc | Field | Normalization | Color
            .show(ui, |ui| {
                // ---- Header row with clickable sort labels ----
                self.sortable_header(ui, "File", SortColumn::File);
                ui.label("BCI Scaler"); // not sortable
                ui.label("BCI Unc"); // not sortable
                self.sortable_header(ui, "BCI Scale", SortColumn::BciScale);
                self.sortable_header(ui, "Angle", SortColumn::Angle);
                self.sortable_header(ui, "Slits", SortColumn::Slits);
                ui.label("Slits Unc"); // not sortable
                self.sortable_header(ui, "Field", SortColumn::Field);
                ui.label("Normalization")
                    .on_hover_text("Divide area by this to get cross section [μb/sr]");
                ui.label("Color");
                ui.label("Marker Shape");
                ui.end_row();

                for (i, run) in self.runs.iter_mut().enumerate() {
                    // File name (short) with full path on hover
                    let full_path = &run.file_name;
                    let display_name = std::path::Path::new(full_path)
                        .file_name()
                        .map(|f| f.to_string_lossy())
                        .unwrap_or_else(|| full_path.into());
                    ui.label(display_name).on_hover_text(full_path);

                    // Run editable cells (+ normalization read-only & color)
                    if run.ui(ui) {
                        changed = true;
                    }

                    // Remove button
                    if ui.button("X").on_hover_text("Remove run").clicked() {
                        index_to_remove = Some(i);
                    }
                    ui.end_row();
                }
            });

        if let Some(i) = index_to_remove {
            self.runs.remove(i);
            changed = true;
        }

        changed
    }

    /// Sort the vec according to the active sort column and direction.
    fn apply_sort(&mut self) {
        if let Some(col) = self.sort_column {
            match col {
                SortColumn::File => {
                    self.runs.sort_by(|a, b| a.file_name.cmp(&b.file_name));
                }
                SortColumn::BciScale => {
                    self.runs.sort_by(|a, b| a.bci_scale.cmp(&b.bci_scale));
                }
                SortColumn::Angle => {
                    self.runs
                        .sort_by(|a, b| a.angle.partial_cmp(&b.angle).unwrap_or(Ordering::Equal));
                }
                SortColumn::Slits => {
                    self.runs
                        .sort_by(|a, b| a.slits.partial_cmp(&b.slits).unwrap_or(Ordering::Equal));
                }
                SortColumn::Field => {
                    self.runs.sort_by(|a, b| {
                        a.magnetic_field
                            .partial_cmp(&b.magnetic_field)
                            .unwrap_or(Ordering::Equal)
                    });
                }
            }
            if !self.sort_ascending {
                self.runs.reverse();
            }
        }
    }

    /// Helper: clickable header with arrow
    fn sortable_header(&mut self, ui: &mut egui::Ui, label: &str, col: SortColumn) {
        ui.horizontal(|ui| {
            let is_active = self.sort_column == Some(col);
            let arrow = if is_active {
                if self.sort_ascending { "⬆" } else { "⬇" }
            } else {
                ""
            };
            if ui.button(format!("{label} {arrow}")).clicked() {
                if is_active {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(col);
                    self.sort_ascending = true;
                }
                self.apply_sort();
            }
        });
    }

    /// Save current runs to CSV.
    pub fn save_csv(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("runs.csv")
            .save_file()
        {
            let mut f = match File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create CSV: {e}");
                    return;
                }
            };

            // Header
            if let Err(e) = writeln!(
                f,
                "file_name,bci_scale,bci_scaler,bci_uncertainty,angle,slits,slits_uncertainty,magnetic_field,color_r,color_g,color_b,color_a,markershape"
            ) {
                log::error!("Failed to write header: {e}");
                return;
            }

            for run in &self.runs {
                let (r, g, b, a) = (run.color.r(), run.color.g(), run.color.b(), run.color.a());
                if let Err(e) = writeln!(
                    f,
                    "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{}",
                    Self::escape_csv(&run.file_name),
                    run.bci_scale,
                    run.bci_scaler,
                    run.bci_uncertainty,
                    run.angle,
                    run.slits,
                    run.slits_uncertainty,
                    run.magnetic_field,
                    r,
                    g,
                    b,
                    a,
                    run.markershape,
                ) {
                    log::error!("Failed to write row for {}: {e}", run.file_name);
                    return;
                }
            }

            log::info!("Saved Runs CSV to {path:?}");
        }
    }

    /// Load values from CSV and apply to existing runs by matching file name.
    /// Rows whose file is not present in `self.runs` are skipped.
    pub fn load_csv(&mut self) {
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

            // Read header
            let mut header = String::new();
            if let Err(e) = rdr.read_line(&mut header) {
                log::error!("Failed to read header: {e}");
                return;
            }

            use std::collections::HashMap;
            use std::path::Path;

            // build maps with OWNED keys so we don't borrow from self.runs
            let mut by_full: HashMap<String, usize> = HashMap::new();
            let mut by_base: HashMap<String, usize> = HashMap::new();

            for (i, run) in self.runs.iter().enumerate() {
                by_full.insert(run.file_name.clone(), i);
                let base = Path::new(&run.file_name)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !base.is_empty() {
                    by_base.insert(base, i);
                }
            }

            let mut updated = 0usize;
            for (lineno, line_res) in rdr.lines().enumerate() {
                let line_no = lineno + 2;
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

                let cols = Self::split_csv(&line);
                if cols.len() < 12 {
                    log::warn!(
                        "Skipping malformed line {line_no}: expected 12 columns, got {}",
                        cols.len()
                    );
                    continue;
                }

                // borrow &str when calling unescape_csv
                let fname = Self::unescape_csv(&cols[0]);

                // look up by owned keys (no borrow of self.runs)
                let idx = by_full.get(&fname).copied().or_else(|| {
                    let base = Path::new(&fname)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    by_base.get(&base).copied()
                });

                if let Some(i) = idx {
                    if let Some(run) = self.runs.get_mut(i) {
                        run.bci_scale = cols[1].parse().unwrap_or(run.bci_scale);
                        run.bci_scaler = cols[2].parse().unwrap_or(run.bci_scaler);
                        run.bci_uncertainty = cols[3].parse().unwrap_or(run.bci_uncertainty);
                        run.angle = cols[4].parse().unwrap_or(run.angle);
                        run.slits = cols[5].parse().unwrap_or(run.slits);
                        run.slits_uncertainty = cols[6].parse().unwrap_or(run.slits_uncertainty);
                        run.magnetic_field = cols[7].parse().unwrap_or(run.magnetic_field);
                        let r: u8 = cols[8].parse().unwrap_or(255);
                        let g: u8 = cols[9].parse().unwrap_or(0);
                        let b: u8 = cols[10].parse().unwrap_or(0);
                        let a: u8 = cols[11].parse().unwrap_or(255);
                        run.color = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
                        if cols.len() >= 13 {
                            run.markershape = cols[12].clone();
                        }
                        updated += 1;
                    }
                } else {
                    log::debug!("Skipping unmatched run '{fname}'");
                }
            }

            log::info!("Loaded & matched {updated} run rows from CSV");
        }
    }

    /// Escape commas/quotes in file names for CSV
    fn escape_csv(s: &str) -> String {
        if s.contains(',') || s.contains('"') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_owned()
        }
    }

    /// Undo escaping
    fn unescape_csv(s: &str) -> String {
        let s = s.trim();
        if s.starts_with('"') && s.ends_with('"') {
            s[1..s.len() - 1].replace("\"\"", "\"")
        } else {
            s.to_owned()
        }
    }

    /// Split a CSV line into cells (very simple, handles quotes)
    fn split_csv(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    if in_quotes && chars.peek() == Some(&'"') {
                        cur.push('"');
                        chars.next();
                    } else {
                        in_quotes = !in_quotes;
                    }
                }
                ',' if !in_quotes => {
                    out.push(cur.trim().to_owned());
                    cur.clear();
                }
                _ => cur.push(c),
            }
        }
        out.push(cur.trim().to_owned());
        out
    }
}

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

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SPSAnalysisSettings {
    panel_open: bool,
    n_columns: usize,
    log_scale: bool,
    view_aspect: f32,
    markersize: f32,
}

impl Default for SPSAnalysisSettings {
    fn default() -> Self {
        Self {
            panel_open: true,
            n_columns: 3,
            log_scale: true,
            view_aspect: 2.0,
            markersize: 3.0,
        }
    }
}

impl SPSAnalysisSettings {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("sps_settings_grid")
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Number of Columns:");
                ui.add(
                    egui::DragValue::new(&mut self.n_columns)
                        .speed(1)
                        .range(1..=10),
                );
                ui.end_row();
                ui.label("Log Scale:");
                ui.checkbox(&mut self.log_scale, "");
                ui.end_row();
                ui.label("View Aspect Ratio:");
                ui.add(
                    egui::DragValue::new(&mut self.view_aspect)
                        .speed(0.1)
                        .range(0.1..=10.0),
                );
                ui.end_row();
                ui.label("Marker Size:");
                ui.add(
                    egui::DragValue::new(&mut self.markersize)
                        .speed(0.1)
                        .range(0.1..=10.0),
                );
                ui.end_row();
            });
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SPSAnalysis {
    target_thickness: (f64, f64), // in ug/cm^2, uncertainty
    target_molar_mass: f64,       // in g/mol
    beam_energy: f64,             // in MeV
    beam_z: u32,                  // atomic number
    runs: Runs,
    fit_uuid_map: FitUUIDMap,
    settings: SPSAnalysisSettings,
}

impl Default for SPSAnalysis {
    fn default() -> Self {
        Self {
            target_thickness: (130.0, 13.0),
            target_molar_mass: 149.920887,
            beam_energy: 16.0,
            beam_z: 1,
            runs: Runs::default(),
            fit_uuid_map: FitUUIDMap::default(),
            settings: SPSAnalysisSettings::default(),
        }
    }
}

impl SPSAnalysis {
    pub fn ui(
        &mut self,
        ctx: &mut egui::Ui,
        files: &[(PathBuf, bool)],
        histogrammer: &mut histogrammer::Histogrammer,
    ) {
        self.ensure_runs_for_files(files);

        // left panel
        egui::SidePanel::left("sps_left_panel")
            .resizable(true)
            .default_width(300.0)
            .show_animated(ctx.ctx(), self.settings.panel_open, |ui| {
                egui::ScrollArea::both()
                    .id_salt("sps_left_scroll_area")
                    .show(ui, |ui| {
                        self.left_panel(ui, histogrammer);
                    });
            });

        self.panel_toggle_button(ctx.ctx());

        egui::CentralPanel::default().show(ctx.ctx(), |ui| {
            self.cross_section_ui(ui, histogrammer);
        });
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
                        .small_button(if self.settings.panel_open {
                            "◀"
                        } else {
                            "▶"
                        })
                        .clicked()
                    {
                        self.settings.panel_open = !self.settings.panel_open;
                    }
                });
            });
    }

    pub fn left_panel(&mut self, ui: &mut egui::Ui, histogrammer: &mut histogrammer::Histogrammer) {
        let mut dirty = false; // <- track changes

        egui::CollapsingHeader::new("General Settings")
            .default_open(true)
            .show(ui, |ui| {
                if self.general_settings_ui(ui) {
                    dirty = true;
                }
            });

        egui::CollapsingHeader::new("Runs")
            .default_open(true)
            .show(ui, |ui| {
                if self.runs.ui(ui) {
                    dirty = true;
                }
            });

        egui::CollapsingHeader::new("Fit UUID Map")
            .default_open(true)
            .show(ui, |ui| {
                self.fit_uuid_map.ui(ui, Some(histogrammer));
            });

        egui::CollapsingHeader::new("Settings")
            .default_open(false)
            .show(ui, |ui| {
                self.settings.ui(ui);
                if ui.button("Export [.csv]").clicked() {
                    self.export_cross_section_csv(histogrammer);
                }
            });

        // Recompute once if anything changed
        if dirty {
            self.calculate_normalization_factor();
        }
    }

    pub fn general_settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("sps_target_grid")
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Target Thickness:");
                ui.horizontal(|ui| {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.target_thickness.0)
                                .speed(1.0)
                                .range(0.0..=f64::INFINITY),
                        )
                        .changed();
                    ui.label("±");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.target_thickness.1)
                                .speed(0.1)
                                .range(0.0..=f64::INFINITY)
                                .suffix(" ug/cm²"),
                        )
                        .changed();
                });
                ui.end_row();

                ui.label("Target Molar Mass:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.target_molar_mass)
                            .speed(0.1)
                            .range(0.0..=f64::INFINITY)
                            .suffix(" g/mol"),
                    )
                    .changed();
                ui.end_row();

                ui.label("Beam Energy:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.beam_energy)
                            .speed(0.1)
                            .range(0.0..=f64::INFINITY)
                            .suffix(" MeV"),
                    )
                    .changed();
                ui.end_row();

                ui.label("Beam Z:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut self.beam_z)
                            .speed(1)
                            .range(1..=92),
                    )
                    .changed();
                ui.end_row();
            });

        changed
    }

    /// Ensure each *checked* file appears once in `runs` (add missing; keep existing).
    fn ensure_runs_for_files(&mut self, files: &[(PathBuf, bool)]) {
        for (path, checked) in files {
            if !*checked {
                continue;
            }
            let key = path.to_string_lossy();
            let exists = self.runs.runs.iter().any(|r| r.file_name == key);

            if !exists {
                self.runs.runs.push(Run::from_path(path));
            }
        }
    }

    fn calculate_normalization_factor(&mut self) {
        for run in &mut self.runs.runs {
            let q_b = run.bci_scaler * (run.bci_scale as f64) * 1e-9 / 100.0; // total charge of particles incident on the target
            let n_b = q_b / ((self.beam_z as f64) * 1.602e-19); // number of incident particles
            let f_target =
                1.0e-24 * 6.023e23 * (self.target_thickness.0 / 1.0e6) / self.target_molar_mass; // areal density of target atoms in atoms/cm^2
            let solid_angle = run.slits * 1.0e-3; // convert msr to sr

            let normalization_factor = n_b * f_target * solid_angle * 1.0e-6; // for units of ub/sr
            let normalization_factor_uncertainty = normalization_factor
                * ((run.bci_uncertainty / 100.0).powi(2)
                    + (self.target_thickness.1 / self.target_thickness.0).powi(2)
                    + (run.slits_uncertainty / 100.0).powi(2))
                .sqrt();

            run.normalization_factor =
                Some((normalization_factor, normalization_factor_uncertainty));
        }
    }

    #[expect(clippy::type_complexity)]
    pub fn get_data_from_histogrammer(
        &self,
        histogrammer: &histogrammer::Histogrammer,
    ) -> HashMap<usize, Vec<(f64, f64, f64, f64, GaussianParameters, Color32, MarkerShape)>> {
        let mut map: HashMap<
            usize,
            Vec<(f64, f64, f64, f64, GaussianParameters, Color32, MarkerShape)>,
        > = HashMap::new();

        for run in &self.runs.runs {
            let angle = run.angle;
            let color = run.color;
            let markershape = run.get_egui_markershape();
            let field = run.magnetic_field;

            let (norm, norm_unc) = match run.normalization_factor {
                Some((n, dn)) if n != 0.0 => (n, dn),
                _ => continue,
            };

            // get base file name without extension
            let base_name = std::path::Path::new(&run.file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            for (_id, tile) in histogrammer.tree.tiles.iter() {
                if let egui_tiles::Tile::Pane(pane) = tile
                    && let crate::histoer::pane::Pane::Histogram(h_arc) = pane
                    && let Ok(h) = h_arc.lock()
                {
                    let hist_name = h.name.to_ascii_lowercase();
                    if !hist_name.starts_with(&format!("{base_name}/")) {
                        continue;
                    }
                    for fit in &h.fits.stored_fits {
                        if let Some(fit_result) = &fit.fit_result {
                            let FitResult::Gaussian(gauss_fit) = fit_result;
                            for params in &gauss_fit.fit_result {
                                let uuid = params.uuid;
                                if uuid == 0 {
                                    continue; // skip unset
                                }
                                if let Some(area) = params.area.value {
                                    let cross_section_ub_sr = area / norm;
                                    let area_uncertainty = params.area.uncertainty.unwrap_or(0.0);
                                    let cross_section_uncertainty = cross_section_ub_sr
                                        * (area_uncertainty / area).hypot(norm_unc / norm);

                                    map.entry(uuid).or_default().push((
                                        angle,
                                        cross_section_ub_sr,
                                        cross_section_uncertainty,
                                        field,
                                        params.clone(),
                                        color,
                                        markershape,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        map
    }

    pub fn export_cross_section_csv(
        &self,
        histogrammer: &crate::histoer::histogrammer::Histogrammer,
    ) {
        let data = self.get_data_from_histogrammer(histogrammer);
        if data.is_empty() {
            log::warn!("No cross-section data to export.");
            return;
        }

        let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("cross_sections.csv")
            .save_file()
        else {
            return;
        };

        let mut f = match std::fs::File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to create CSV: {e}");
                return;
            }
        };

        // Header
        if let Err(e) = writeln!(
            f,
            "UUID,Angle,Magnetic Field,Cross Section,Cross Section Uncertainty,\
             Mean,Mean Unc,Sigma,Sigma Unc,FWHM,FWHM Unc,Amplitude,Amplitude Unc,\
             Area,Area Unc,Assigned Energy,Energy Unc,Calibrated Mean,Calibrated Mean Unc,\
             Calibrated FWHM,Calibrated FWHM Unc"
        ) {
            log::error!("Failed to write header: {e}");
            return;
        }

        // Sort UUIDs
        let mut uuids: Vec<_> = data.keys().copied().collect();
        uuids.sort_unstable();

        for uuid in uuids {
            for (angle, cs, dcs, field, params, _, _) in &data[&uuid] {
                let fmt = |v: Option<f64>| v.map(|x| format!("{x:.6}")).unwrap_or_default();
                let fmt_u = |u: Option<f64>| u.map(|x| format!("{x:.6}")).unwrap_or_default();

                if let Err(e) = writeln!(
                    f,
                    "{},{:.6},{:.6},{:.6},{:.6},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{},\
                     {},{}",
                    uuid,
                    angle,
                    field,
                    cs,
                    dcs,
                    fmt(params.mean.value),
                    fmt_u(params.mean.uncertainty),
                    fmt(params.sigma.value),
                    fmt_u(params.sigma.uncertainty),
                    fmt(params.fwhm.value),
                    fmt_u(params.fwhm.uncertainty),
                    fmt(params.amplitude.value),
                    fmt_u(params.amplitude.uncertainty),
                    fmt(params.area.value),
                    fmt_u(params.area.uncertainty),
                    fmt(params.energy.value),
                    fmt_u(params.energy.uncertainty),
                    fmt(params.mean.calibrated_value),
                    fmt_u(params.mean.calibrated_uncertainty),
                    fmt(params.fwhm.calibrated_value),
                    fmt_u(params.fwhm.calibrated_uncertainty),
                ) {
                    log::error!("Failed to write row for UUID {uuid}: {e}");
                    return;
                }
            }
        }

        log::info!("Saved full cross sections CSV to {path:?}");
    }

    pub fn cross_section_ui(
        &self,
        ui: &mut egui::Ui,
        histogrammer: &mut histogrammer::Histogrammer,
    ) {
        use egui_extras::{Column, TableBuilder};
        use egui_plot::Plot;
        const EPS: f64 = 1e-12;

        let data = self.get_data_from_histogrammer(histogrammer);
        if data.is_empty() {
            ui.label("No cross-section data found.");
            return;
        }

        // Sort UUIDs for stable layout
        let mut uuids: Vec<usize> = data.keys().copied().collect();
        uuids.sort_unstable();

        let ncols = self.settings.n_columns.max(1);
        let nrows = uuids.len().div_ceil(ncols);

        let available_w = ui.available_width() - 10.0;
        let col_w = available_w / (ncols as f32);

        // Build dynamic table: ncols equal plot columns
        let mut table = TableBuilder::new(ui)
            .striped(false)
            .resizable(false) // keep widths stable as the window resizes
            .vscroll(true);

        // Create ncols equal-width, growable columns with a reasonable minimum
        table = table.columns(Column::exact(col_w), ncols);

        let row_h: f32 = col_w / self.settings.view_aspect;

        // Optional header with UUIDs for the first row positions
        table.body(|mut body| {
            for r in 0..nrows {
                body.row(row_h, |mut row| {
                    for c in 0..ncols {
                        let idx = r * ncols + c;
                        row.col(|ui| {
                            if idx >= uuids.len() {
                                return;
                            }
                            let uuid = uuids[idx];
                            let pts: &Vec<(f64, f64, f64, f64, GaussianParameters, Color32, MarkerShape)> = &data[&uuid];

                            // ---------- Build a per-UUID label from params ----------
                            fn auto_fmt(value: Option<f64>, unc: Option<f64>, units: &str) -> String {
                                match value {
                                    Some(val) => {
                                        let unc = unc.unwrap_or(0.0);
                                        if unc > 0.0 && unc.is_finite() {
                                            // 2 sig figs in the uncertainty → decimals from its magnitude
                                            let exp = unc.abs().log10().floor() as i32;
                                            let digits = (-(exp) + 1).max(0) as usize;
                                            format!("{val:.digits$} ± {unc:.digits$} {units}")
                                        } else {
                                            format!("{val:.3} {units}")
                                        }
                                    }
                                    None => "—".to_owned(),
                                }
                            }

                            // Assigned energy (expect same for all points; show “(mixed)” if not)
                            let mut energy_val: Option<f64> = None;
                            let mut energy_unc: Option<f64> = None;
                            let mut energy_mixed = false;

                            for &(_ang, _y, _dy, _field, ref params, _col, _shape) in pts {
                                if let Some(e) = params.energy.value && e != -1.0 {
                                    match energy_val {
                                        None => {
                                            energy_val = Some(e);
                                            energy_unc = params.energy.uncertainty;
                                        }
                                        Some(prev) if (prev - e).abs() > f64::EPSILON => {
                                            energy_mixed = true;
                                        }
                                        _ => {}
                                    }
                                }
                            }



                            // Compose the label shown on-plot (top-left)
                            let mut label = format!("UUID {uuid}");
                            if let Some(e) = energy_val {
                                let mut line = format!("\nAssigned Energy: {}", auto_fmt(Some(e), energy_unc, "keV"));
                                if energy_mixed {
                                    line.push_str(" (mixed)");
                                }
                                label.push_str(&line);
                            }
                            let (avg_cal_mean, avg_cal_mean_unc) = avg_calibrated_mean(pts);
                            if avg_cal_mean.is_some() {
                                label.push_str(&format!(
                                    "\nAverage Calibrated Mean: {}",
                                    auto_fmt(avg_cal_mean, avg_cal_mean_unc, "keV")
                                ));
                            }
                            // --------------------------------------------------------
                            // ---- Bounds (linear space first) ----
                            let xmin_lin = 0.0;
                            let xmax_lin = 65.0;

                            let mut y_max_lin = 1e-9;
                            let mut y_min_lin = f64::INFINITY;

                            for &(_ang, y, dy, _field, ref _params, _col, _markershape) in pts {
                                // Upper always includes the +unc
                                let hi = (y + dy).max(EPS);

                                // LOWER:
                                // If the bar would go <= 0, use one decade below the datum (y/10)
                                // instead of (y - dy) or EPS.
                                let lo = if y - dy > 0.0 {
                                    y - dy
                                } else {
                                    (y / 10.0).max(EPS)
                                };

                                if hi.is_finite() && hi > y_max_lin {
                                    y_max_lin = hi;
                                }
                                if lo.is_finite() && lo < y_min_lin {
                                    y_min_lin = lo;
                                }
                            }

                            if !y_max_lin.is_finite() || y_max_lin <= 0.0 {
                                y_max_lin = 1.0;
                            }
                            if !y_min_lin.is_finite() || y_min_lin <= 0.0 {
                                y_min_lin = EPS;
                            }

                            // ---- Convert bounds to plot-space ----
                            let (ymin_plot, ymax_plot) = if self.settings.log_scale {
                                let ymin_nice = nice_log_floor(y_min_lin.max(EPS));
                                let ymax_nice = nice_log_ceil(y_max_lin.max(EPS));
                                (ymin_nice.log10(), ymax_nice.log10())
                            } else {
                                (0.0, y_max_lin * 1.1)
                            };

                            let plot_id = ui.id().with(("cs_plot_uuid", uuid));
                            let available_w = ui.available_width();

                            let padding = 4.0;
                            ui.add_space(padding);

                            let mut plot = Plot::new(plot_id)
                                .width(available_w)
                                .height(row_h - padding * 2.0)
                                .allow_zoom(false)
                                .allow_drag(false)
                                .allow_scroll(false)
                                .allow_double_click_reset(false)
                                .auto_bounds(Vec2b::new(false, false))
                                .label_formatter({
                                    let log_y = self.settings.log_scale;
                                    move |name, value| {
                                        let x = value.x;
                                        let y = if log_y {
                                            10.0f64.powf(value.y)
                                        } else {
                                            value.y
                                        };
                                        if !name.is_empty() {
                                            name.to_owned()
                                        } else {
                                            format!("{x:.2}, {y:.2}")
                                        }
                                    }
                                });

                            if c == 0 {
                                plot = plot.y_axis_label("dΩ/dσ [μb/sr]");
                            }

                            if r == nrows - 1 {
                                plot = plot.x_axis_label("θ [°]");
                            }

                            // Log Y ticks/formatter if requested
                            if self.settings.log_scale {
                                let max_size = 4;
                                plot = plot.y_grid_spacer(log_axis_spacer).y_axis_formatter(
                                    move |gm, bounds| log_axis_formatter(gm, bounds, max_size),
                                );
                            }


                            let label_color = if ui.visuals().dark_mode {
                                egui::Color32::LIGHT_BLUE
                            } else {
                                egui::Color32::BLACK
                            };

                            // Render
                            plot.show(ui, |pui| {
                                // One marker per datum
                                for &(ang, y, dy, field, ref params, col, markershape) in pts {
                                    let uuid: usize = params.uuid;

                                    // Base label
                                    let mut name = format!(
                                        "UUID {uuid}\nAngle: {ang:.1}°\ndΩ/dσ: {y:.2} ± {dy:.2} [μb/sr]\nMagnetic Field: {field:.2} kG"
                                    );

                                    let gaussian_summary = params.summary_string(Some("keV"), Some("mm"));
                                    if !gaussian_summary.is_empty() {
                                        name.push_str(&format!("\n{gaussian_summary}"));
                                    }

                                    let mut point = EguiPoints::new_cross_section(
                                        &name,
                                        ang,
                                        y,
                                        dy,
                                        col,
                                    );
                                    point.log_y = self.settings.log_scale;
                                    point.radius = self.settings.markersize;
                                    point.shape = Some(markershape);
                                    point.draw(pui);
                                }

                                // Put the per-UUID label in the top-left corner
                                let label_item = egui_plot::Text::new(
                                    format!("uuid_label_{uuid}"),              // item name/id
                                    egui_plot::PlotPoint::new(xmin_lin + 1.0, ymax_plot),
                                    label,                                     // the string we built
                                )
                                .anchor(egui::Align2::LEFT_TOP).color(label_color);

                                pui.text(label_item);

                                // X bounds are linear in both modes
                                pui.set_plot_bounds_x(xmin_lin..=xmax_lin);
                                // Y bounds are in plot coords (log-transformed if log mode)
                                pui.set_plot_bounds_y(ymin_plot..=ymax_plot);
                            });
                        });
                    }
                });
            }
        });
    }
}

#[expect(clippy::needless_pass_by_value)]
fn log_axis_spacer(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
    let (min, max) = input.bounds;
    let mut marks = vec![];
    for i in min.floor() as i32..=max.ceil() as i32 {
        marks.extend(
            (10..100)
                .map(|j| {
                    let value = i as f64 + (j as f64).log10() - 1.0;
                    let step_size = if j == 10 {
                        1.0
                    } else if j % 10 == 0 {
                        0.1
                    } else {
                        0.01
                    };
                    egui_plot::GridMark { value, step_size }
                })
                .filter(|gm| (min..=max).contains(&gm.value)),
        );
    }
    marks
}

fn log_axis_formatter(
    gm: egui_plot::GridMark,
    _bounds: &std::ops::RangeInclusive<f64>,
    max_size: usize,
) -> String {
    let min_precision = (-gm.value + 1.0).ceil().clamp(1.0, 10.0) as usize;
    let digits = (gm.value).ceil().max(1.0) as usize;
    let size = digits + min_precision + 1;
    let value = 10.0f64.powf(gm.value);
    if size < max_size {
        let precision = max_size.saturating_sub(digits + 1);
        format!("{value:.precision$}")
    } else {
        let exp_digits = (digits as f64).log10() as usize;
        let precision = max_size.saturating_sub(exp_digits).saturating_sub(3);
        format!("{value:.precision$e}")
    }
}

// Keep “nice” log rounding to 1–3–10 decades.
fn nice_log_ceil(x: f64) -> f64 {
    if !x.is_finite() || x <= 0.0 {
        return 1.0;
    }
    let exp = x.log10().floor();
    let base = 10f64.powf(exp);
    let mant = x / base; // in [1,10)
    let m = if mant <= 1.0 {
        1.0
    } else if mant <= 3.0 {
        3.0
    } else {
        10.0
    };
    m * base
}

fn nice_log_floor(x: f64) -> f64 {
    if !x.is_finite() || x <= 0.0 {
        return 1.0;
    }
    let exp = x.log10().floor();
    let base = 10f64.powf(exp);
    let mant = x / base; // in [1,10)
    let m = if mant >= 3.0 { 3.0 } else { 1.0 };
    m * base
}

/// Compute inverse-variance weighted average of calibrated means (and its uncertainty).
/// Falls back to unweighted mean + standard error if uncertainties are missing/invalid.
/// Returns (mean, uncertainty). Both None if no calibrated means found.
fn avg_calibrated_mean(
    pts: &[(
        f64,
        f64,
        f64,
        f64,
        GaussianParameters,
        egui::Color32,
        egui_plot::MarkerShape,
    )],
) -> (Option<f64>, Option<f64>) {
    // Collect (mean, sigma) pairs where calibrated mean exists
    let mut with_unc: Vec<(f64, f64)> = Vec::new(); // (m_i, σ_i)
    let mut bare: Vec<f64> = Vec::new(); // m_i when σ_i missing

    for (_, _, _, _, params, _, _) in pts {
        if let Some(m) = params.mean.calibrated_value {
            if let Some(dm) = params.mean.calibrated_uncertainty
                && dm.is_finite()
                && dm > 0.0
            {
                with_unc.push((m, dm));
                continue;
            }
            bare.push(m);
        }
    }

    if !with_unc.is_empty() {
        // Inverse-variance weighted mean
        let mut wsum = 0.0;
        let mut wmsum = 0.0;
        for (m, dm) in with_unc {
            let w = 1.0 / (dm * dm);
            wsum += w;
            wmsum += w * m;
        }
        if wsum > 0.0 {
            let mean = wmsum / wsum;
            let unc = 1.0 / wsum.sqrt();
            return (Some(mean), Some(unc));
        }
    }

    // Fallback: unweighted mean + standard error (if possible)
    let all: Vec<f64> = if !bare.is_empty() {
        bare
    } else {
        // No valid values at all
        return (None, None);
    };

    let n = all.len() as f64;
    let mean = all.iter().copied().sum::<f64>() / n;
    if all.len() >= 2 {
        let var = all.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let se = (var / n).sqrt();
        (Some(mean), Some(se))
    } else {
        (Some(mean), None)
    }
}
