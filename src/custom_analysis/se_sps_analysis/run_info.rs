use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::Path;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Run {
    pub file_name: String,
    pub bci_scale: u32,
    pub bci_scaler: f64,
    pub bci_uncertainty: f64,   // percentage, e.g. 10 for 10%
    pub angle: f64,             // lab degrees
    pub slits: f64,             // msr
    pub slits_uncertainty: f64, // percentage, e.g. 10 for 10%
    pub magnetic_field: f64,    // kG
    pub normalization_factor: Option<(f64, f64)>,
    pub color: egui::Color32,
    pub markershape: String,
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

    pub fn get_egui_markershape(&self) -> egui_plot::MarkerShape {
        match self.markershape.as_str() {
            // "Circle" => MarkerShape::Circle,
            "Diamond" => egui_plot::MarkerShape::Diamond,
            "Square" => egui_plot::MarkerShape::Square,
            "Cross" => egui_plot::MarkerShape::Cross,
            "Plus" => egui_plot::MarkerShape::Plus,
            "Up" => egui_plot::MarkerShape::Up,
            "Down" => egui_plot::MarkerShape::Down,
            "Left" => egui_plot::MarkerShape::Left,
            "Right" => egui_plot::MarkerShape::Right,
            "Asterisk" => egui_plot::MarkerShape::Asterisk,
            _ => egui_plot::MarkerShape::Circle,
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Runs {
    pub runs: Vec<Run>,

    // Sorting state
    #[serde(skip)]
    pub sort_column: Option<SortColumn>,
    #[serde(skip)]
    pub sort_ascending: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SortColumn {
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
