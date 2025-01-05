use super::cuts::{Cut, Cuts};
use super::histogrammer::Histogrammer;

use egui_extras::{Column, TableBuilder};

// Enum to encapsulate 1D and 2D histogram configurations
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum Config {
    Hist1D(Hist1DConfig),
    Hist2D(Hist2DConfig),
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct Configs {
    pub configs: Vec<Config>,
    pub columns: Vec<(String, String)>,
    pub cuts: Cuts,
}

impl Configs {
    pub fn hist1d(
        &mut self,
        name: &str,
        column_name: &str,
        range: (f64, f64),
        bins: usize,
        cuts: Option<Cuts>,
    ) {
        let mut config = Hist1DConfig::new(name, column_name, range, bins);

        if let Some(cuts) = cuts {
            config.cuts = cuts;
        }

        self.configs.push(Config::Hist1D(config))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn hist2d(
        &mut self,
        name: &str,
        x_column_name: &str,
        y_column_name: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        bins: (usize, usize),
        cuts: Option<Cuts>,
    ) {
        let mut config =
            Hist2DConfig::new(name, x_column_name, y_column_name, x_range, y_range, bins);

        if let Some(cuts) = cuts {
            config.cuts = cuts;
        }

        self.configs.push(Config::Hist2D(config));
    }

    pub fn merge(&mut self, other: Configs) -> &mut Self {
        // Merge configurations
        for config in other.configs {
            match &config {
                Config::Hist1D(other_hist1d) => {
                    if let Some(existing) = self
                        .configs
                        .iter()
                        .filter_map(|c| match c {
                            Config::Hist1D(h) if h.name == other_hist1d.name => Some(h),
                            _ => None,
                        })
                        .next()
                    {
                        if existing.column_name != other_hist1d.column_name
                            && existing.range == other_hist1d.range
                            && existing.bins == other_hist1d.bins
                        {
                            self.configs.push(Config::Hist1D(other_hist1d.clone()));
                        } else {
                            log::error!(
                                "Conflict detected for Hist1D '{}' with column '{}' or range/bin mismatch.",
                                other_hist1d.name,
                                other_hist1d.column_name
                            );
                        }
                    } else {
                        self.configs.push(Config::Hist1D(other_hist1d.clone()));
                    }
                }
                Config::Hist2D(other_hist2d) => {
                    if let Some(existing) = self
                        .configs
                        .iter()
                        .filter_map(|c| match c {
                            Config::Hist2D(h) if h.name == other_hist2d.name => Some(h),
                            _ => None,
                        })
                        .next()
                    {
                        if (existing.x_column_name != other_hist2d.x_column_name
                            || existing.y_column_name != other_hist2d.y_column_name)
                            && existing.x_range == other_hist2d.x_range
                            && existing.y_range == other_hist2d.y_range
                            && existing.bins == other_hist2d.bins
                        {
                            self.configs.push(Config::Hist2D(other_hist2d.clone()));
                        } else {
                            log::error!(
                                "Conflict detected for Hist2D '{}' with columns ('{}', '{}') or range/bin mismatch.",
                                other_hist2d.name,
                                other_hist2d.x_column_name,
                                other_hist2d.y_column_name
                            );
                        }
                    } else {
                        self.configs.push(Config::Hist2D(other_hist2d.clone()));
                    }
                }
            }
        }

        // Merge columns
        for (expression, alias) in other.columns {
            if let Some(existing) = self.columns.iter().find(|(_, a)| a == &alias) {
                if existing.0 != expression {
                    log::error!(
                        "Conflict detected for column alias '{}': Existing expression '{}', New expression '{}'.",
                        alias,
                        existing.0,
                        expression
                    );
                }
            } else {
                self.columns.push((expression, alias));
            }
        }

        // Merge cuts
        self.cuts.merge(&other.cuts);

        self
    }

    pub fn valid_configs(&mut self, lf: &mut LazyFrame) -> Configs {
        // Add new computed columns to the LazyFrame
        for (expression, alias) in &self.columns {
            if let Err(e) = add_computed_column(lf, expression, alias) {
                log::error!("Error adding computed column '{}': {}", alias, e);
            }
        }

        // Get the column names from the LazyFrame
        let column_names = get_column_names_from_lazyframe(lf);

        // Ensure 1D cuts have their expressions parsed
        self.cuts.parse_conditions();

        // Expand the configurations (to account for patterns)
        let expanded_configs = self.expand();

        // Validate configurations and cuts
        let mut valid_configs = Vec::new();
        let mut valid_cuts = Cuts::default();

        for config in &expanded_configs.configs {
            match config {
                Config::Hist1D(hist1d) => {
                    if hist1d.calculate {
                        if column_names.contains(&hist1d.column_name) {
                            // Validate cuts for the histogram
                            let valid_hist_cuts: Vec<Cut> = hist1d
                                .cuts
                                .cuts
                                .iter()
                                .filter(|cut| {
                                    let required_columns = cut.required_columns();
                                    for column in required_columns {
                                        if !column_names.contains(&column) {
                                            log::error!(
                                                "Invalid cut '{}' for 1D histogram '{}': Missing column '{}'",
                                                cut.name(),
                                                hist1d.name,
                                                column
                                            );
                                            return false;
                                        }
                                    }
                                    true
                                })
                                .cloned()
                                .collect();

                            let mut validated_hist1d = hist1d.clone();
                            validated_hist1d.cuts = Cuts::new(valid_hist_cuts);
                            validated_hist1d.cuts.parse_conditions();
                            valid_configs.push(Config::Hist1D(validated_hist1d));
                        } else {
                            log::error!(
                                "Invalid 1D histogram '{}': Missing column '{}'",
                                hist1d.name,
                                hist1d.column_name
                            );
                        }
                    }
                }
                Config::Hist2D(hist2d) => {
                    if hist2d.calculate {
                        if column_names.contains(&hist2d.x_column_name)
                            && column_names.contains(&hist2d.y_column_name)
                        {
                            // Validate cuts for the histogram
                            let valid_hist_cuts: Vec<Cut> = hist2d
                                .cuts
                                .cuts
                                .iter()
                                .filter(|cut| {
                                    let required_columns = cut.required_columns();
                                    for column in required_columns {
                                        if !column_names.contains(&column) {
                                            log::error!(
                                                "Invalid cut '{}' for 2D histogram '{}': Missing column '{}'",
                                                cut.name(),
                                                hist2d.name,
                                                column
                                            );
                                            return false;
                                        }
                                    }
                                    true
                                })
                                .cloned()
                                .collect();

                            let mut validated_hist2d = hist2d.clone();
                            validated_hist2d.cuts.cuts = valid_hist_cuts;
                            validated_hist2d.cuts.parse_conditions();
                            valid_configs.push(Config::Hist2D(validated_hist2d));
                        } else {
                            log::error!(
                                "Invalid 2D histogram '{}': Missing column(s) '{}', '{}'",
                                hist2d.name,
                                hist2d.x_column_name,
                                hist2d.y_column_name
                            );
                        }
                    }
                }
            }
        }

        // Validate cuts not associated with histograms
        for cut in &self.cuts.cuts {
            let required_columns = cut.required_columns();
            if required_columns
                .iter()
                .all(|col| column_names.contains(col))
            {
                valid_cuts.add_cut(cut.clone());
            } else {
                for column in required_columns {
                    if !column_names.contains(&column) {
                        log::error!(
                            "Invalid cut '{}': Missing required column '{}'",
                            cut.name(),
                            column
                        );
                    }
                }
            }
        }

        // Return a new Configs instance with validated configurations and cuts
        Configs {
            configs: valid_configs,
            columns: self.columns.clone(),
            cuts: valid_cuts,
        }
    }

    pub fn check_and_add_panes(&self, h: &mut Histogrammer) {
        // reset all existings panes
        h.reset_histograms();

        // add panes that do not already exist in the histogrammer
        for config in &self.configs {
            match config {
                Config::Hist1D(hist1d) => {
                    if let Some(_id) = h.find_existing_histogram(&hist1d.name) {
                        log::info!("Histogram {} already exists", hist1d.name);
                    } else {
                        h.add_hist1d(&hist1d.name, hist1d.bins, (hist1d.range.0, hist1d.range.1));
                    }
                }
                Config::Hist2D(hist2d) => {
                    if let Some(_id) = h.find_existing_histogram(&hist2d.name) {
                        log::info!("Histogram {} already exists", hist2d.name);
                    } else {
                        h.add_hist2d(
                            &hist2d.name,
                            hist2d.bins,
                            (
                                (hist2d.x_range.0, hist2d.x_range.1),
                                (hist2d.y_range.0, hist2d.y_range.1),
                            ),
                        );
                    }
                }
            }
        }
    }

    pub fn get_used_columns(&self) -> Vec<String> {
        // Collect all column names currently used in the configurations and cuts
        let mut used_column_names = Vec::new();
        for config in &self.configs {
            match config {
                Config::Hist1D(hist1d) => {
                    used_column_names.push(hist1d.column_name.clone());
                    used_column_names.extend(hist1d.cuts.required_columns());
                }
                Config::Hist2D(hist2d) => {
                    used_column_names.push(hist2d.x_column_name.clone());
                    used_column_names.push(hist2d.y_column_name.clone());
                    used_column_names.extend(hist2d.cuts.required_columns());
                }
            }
        }

        // Remove duplicates
        used_column_names.sort();
        used_column_names.dedup();

        used_column_names
    }

    fn expand(&self) -> Configs {
        let mut expanded_configs: Vec<Config> = Vec::new();

        for config in &self.configs {
            match config {
                Config::Hist1D(config) => {
                    let expanded_1d = config.expand();
                    for expanded_config in expanded_1d {
                        expanded_configs.push(Config::Hist1D(expanded_config));
                    }
                }
                Config::Hist2D(config) => {
                    let expanded_2d = config.expand();
                    for expanded_config in expanded_2d {
                        expanded_configs.push(Config::Hist2D(expanded_config));
                    }
                }
            }
        }

        Configs {
            configs: expanded_configs,
            columns: self.columns.clone(),
            cuts: self.cuts.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    pub fn config_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Histograms");

            if ui.button("+1D").clicked() {
                self.configs.push(Config::Hist1D(Hist1DConfig {
                    name: "".to_string(),
                    column_name: "".to_string(),
                    range: (0.0, 4096.0),
                    bins: 512,
                    cuts: Cuts::default(),
                    calculate: true,
                    enabled: true,
                }));
            }

            if ui.button("+2D").clicked() {
                self.configs.push(Config::Hist2D(Hist2DConfig {
                    name: "".to_string(),
                    x_column_name: "".to_string(),
                    y_column_name: "".to_string(),
                    x_range: (0.0, 4096.0),
                    y_range: (0.0, 4096.0),
                    bins: (512, 512),
                    cuts: Cuts::default(),
                    calculate: true,
                    enabled: true,
                }));
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.configs.clear();
            }
        });

        let mut indices_to_remove = Vec::new();

        // Create the table
        TableBuilder::new(ui)
            .id_salt("hist_configs")
            .column(Column::auto()) // Type
            .column(Column::auto()) // Name
            .column(Column::auto()) // Columns
            .column(Column::auto()) // Ranges
            .column(Column::auto()) // Bins
            .column(Column::auto()) // cuts
            .column(Column::auto()) // Actions
            .column(Column::remainder()) // remove
            .striped(true)
            .vscroll(false)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(" # ");
                });
                header.col(|ui| {
                    ui.label("Name");
                });
                header.col(|ui| {
                    ui.label("Column(s)");
                });
                header.col(|ui| {
                    ui.label("Range(s)");
                });
                header.col(|ui| {
                    ui.label("Bins");
                });
                header.col(|ui| {
                    ui.label("Cuts");
                });
            })
            .body(|mut body| {
                for (index, config) in self.configs.iter_mut().enumerate() {
                    body.row(18.0, |mut row| {
                        row.col(|ui| match config {
                            Config::Hist1D(_) => {
                                ui.label(format!("{index}"));
                            }
                            Config::Hist2D(_) => {
                                ui.label(format!("{index}"));
                            }
                        });

                        match config {
                            Config::Hist1D(config) => config.table_row(&mut row, &mut self.cuts),
                            Config::Hist2D(config) => config.table_row(&mut row, &mut self.cuts),
                        }

                        row.col(|ui| {
                            if ui.button("X").clicked() {
                                indices_to_remove.push(index);
                            }
                        });
                    });
                }
            });

        // Remove indices in reverse order to prevent shifting issues
        for &index in indices_to_remove.iter().rev() {
            self.configs.remove(index);
        }
    }

    pub fn column_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Column Creation");

            if ui.button("+").clicked() {
                self.columns.push(("".to_string(), "".to_string()));
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.columns.clear();
            }
        });

        if !self.columns.is_empty() {
            let mut indices_to_remove_column = Vec::new();

            TableBuilder::new(ui)
                .id_salt("new_columns")
                .column(Column::auto()) // expression
                .column(Column::auto()) // alias
                .column(Column::remainder()) // Actions
                .striped(true)
                .vscroll(false)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label("Alias");
                    });
                    header.col(|ui| {
                        ui.label("Expression");
                    });
                })
                .body(|mut body| {
                    for (index, (expression, alias)) in self.columns.iter_mut().enumerate() {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(alias)
                                        .hint_text("Alias")
                                        .clip_text(false),
                                );
                            });

                            row.col(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(expression)
                                        .hint_text("Expression")
                                        .clip_text(false),
                                );
                            });

                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("X").clicked() {
                                        indices_to_remove_column.push(index);
                                    }
                                });
                            });
                        });
                    }
                });

            // Remove indices in reverse order to prevent shifting issues
            for &index in indices_to_remove_column.iter().rev() {
                self.columns.remove(index);
            }
        }
    }

    pub fn cut_ui(&mut self, ui: &mut egui::Ui) {
        self.cuts.ui(ui);

        // verify/sync cuts with histograms
        for hist_config in &mut self.configs {
            match hist_config {
                Config::Hist1D(hist1d) => {
                    for hist_cut in &mut hist1d.cuts.cuts {
                        if let Some(updated_cut) = self
                            .cuts
                            .cuts
                            .iter()
                            .find(|cut| cut.name() == hist_cut.name())
                        {
                            // Replace the cut if the operation or content has changed
                            *hist_cut = updated_cut.clone();
                        }
                    }

                    // Remove cuts that no longer exist in `self.cuts`
                    hist1d
                        .cuts
                        .cuts
                        .retain(|cut| self.cuts.cuts.iter().any(|c| c.name() == cut.name()));
                }
                Config::Hist2D(hist2d) => {
                    for hist_cut in &mut hist2d.cuts.cuts {
                        if let Some(updated_cut) = self
                            .cuts
                            .cuts
                            .iter()
                            .find(|cut| cut.name() == hist_cut.name())
                        {
                            // Replace the cut if the operation or content has changed
                            *hist_cut = updated_cut.clone();
                        }
                    }

                    // Remove cuts that no longer exist in `self.cuts`
                    hist2d
                        .cuts
                        .cuts
                        .retain(|cut| self.cuts.cuts.iter().any(|c| c.name() == cut.name()));
                }
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.column_ui(ui);

        ui.separator();

        self.cut_ui(ui);

        ui.separator();

        self.config_ui(ui);
    }
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Hist1DConfig {
    pub name: String,        // Histogram display name
    pub column_name: String, // Data column to fill from
    pub range: (f64, f64),   // Range for the histogram
    pub bins: usize,         // Number of bins
    pub cuts: Cuts,          // Cuts for the histogram
    pub calculate: bool,     // Whether to calculate the histogram
    pub enabled: bool,       // Whether to let the user interact with the histogram
}

impl Hist1DConfig {
    pub fn new(name: &str, column_name: &str, range: (f64, f64), bins: usize) -> Self {
        Self {
            name: name.to_string(),
            column_name: column_name.to_string(),
            range,
            bins,
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }
    }

    pub fn table_row(&mut self, row: &mut egui_extras::TableRow<'_, '_>, cuts: &mut Cuts) {
        row.col(|ui| {
            ui.add_enabled(
                self.enabled,
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Name")
                    .clip_text(false),
            );
        });

        row.col(|ui| {
            ui.add_enabled(
                self.enabled,
                egui::TextEdit::singleline(&mut self.column_name)
                    .hint_text("Column Name")
                    .clip_text(false),
            );
        });

        row.col(|ui| {
            ui.horizontal(|ui| {
                ui.add_enabled(
                    self.enabled,
                    egui::DragValue::new(&mut self.range.0)
                        .speed(0.1)
                        .prefix("(")
                        .suffix(","),
                );
                ui.add_enabled(
                    self.enabled,
                    egui::DragValue::new(&mut self.range.1)
                        .speed(0.1)
                        .prefix(" ")
                        .suffix(")"),
                );
            });
        });

        row.col(|ui| {
            ui.add_enabled(self.enabled, egui::DragValue::new(&mut self.bins).speed(1));
        });

        row.col(|ui| {
            egui::ComboBox::from_id_salt(format!("cut_select_1d_{}", self.name))
                .selected_text("Select cuts")
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for cut in &cuts.cuts {
                        let mut is_selected = self
                            .cuts
                            .cuts
                            .iter()
                            .any(|selected_cut| selected_cut == cut);

                        match cut {
                            Cut::Cut1D(cut1d) => {
                                if ui.checkbox(&mut is_selected, &cut1d.name).clicked() {
                                    if is_selected && !self.cuts.cuts.contains(cut) {
                                        self.cuts.cuts.push(cut.clone());
                                    } else if !is_selected {
                                        self.cuts.cuts.retain(|selected_cut| selected_cut != cut);
                                    }
                                }
                            }
                            Cut::Cut2D(cut2d) => {
                                if ui.checkbox(&mut is_selected, &cut2d.polygon.name).clicked() {
                                    if is_selected && !self.cuts.cuts.contains(cut) {
                                        self.cuts.cuts.push(cut.clone());
                                    } else if !is_selected {
                                        self.cuts.cuts.retain(|selected_cut| selected_cut != cut);
                                    }
                                }
                            }
                        }
                    }
                });
        });

        row.col(|ui| {
            ui.add_enabled(self.enabled, egui::Checkbox::new(&mut self.calculate, ""));
        });
    }

    pub fn expand(&self) -> Vec<Self> {
        // Regex for range pattern `{start-end}`
        let range_re = regex::Regex::new(r"\{(\d+)-(\d+)\}").unwrap();

        // Regex for discrete comma-separated values `{val1,val2,...}`
        let list_re = regex::Regex::new(r"\{([\d,]+)\}").unwrap();

        let mut configs = Vec::new();

        if self.calculate {
            if self.name.contains("{}") {
                // name has {} and column_name has a range pattern
                if let Some(caps) = range_re.captures(&self.column_name) {
                    let start: usize = caps[1].parse().unwrap();
                    let end: usize = caps[2].parse().unwrap();

                    // Loop through start and end values
                    for i in start..=end {
                        let mut new_config = self.clone();
                        new_config.name = self.name.replace("{}", &i.to_string()).to_string();
                        new_config.column_name = range_re
                            .replace(&self.column_name, i.to_string())
                            .to_string();
                        configs.push(new_config);
                    }
                }
                // name has {} and column_name has a list pattern
                else if let Some(caps) = list_re.captures(&self.column_name) {
                    // Split comma-separated values and loop over them
                    let values: Vec<&str> = caps[1].split(',').collect();
                    for val in values {
                        let mut new_config = self.clone();
                        new_config.name = self.name.replace("{}", val).to_string();
                        new_config.column_name =
                            list_re.replace(&self.column_name, val).to_string();
                        configs.push(new_config);
                    }
                // Unsupported pattern
                } else {
                    log::error!(
                        "Warning: Unsupported pattern for 1D histogram with name '{}', column '{}'",
                        self.name,
                        self.column_name
                    );
                }
            } else {
                // No {} in name, but column_name has a range pattern
                if let Some(caps) = range_re.captures(&self.column_name) {
                    let start: usize = caps[1].parse().unwrap();
                    let end: usize = caps[2].parse().unwrap();

                    for i in start..=end {
                        let mut new_config = self.clone();
                        new_config.column_name = range_re
                            .replace(&self.column_name, i.to_string())
                            .to_string();
                        configs.push(new_config);
                    }
                }
                // No {} in name, but column_name has a list pattern
                else if let Some(caps) = list_re.captures(&self.column_name) {
                    let values: Vec<&str> = caps[1].split(',').collect();
                    for val in values {
                        let mut new_config = self.clone();
                        new_config.column_name =
                            list_re.replace(&self.column_name, val).to_string();
                        configs.push(new_config);
                    }
                // No {} in name or column_name i.e. a normal configuration
                } else {
                    configs.push(self.clone());
                }
            }
        }
        configs
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Hist2DConfig {
    pub name: String,          // Histogram display name
    pub x_column_name: String, // Data column for X-axis
    pub y_column_name: String, // Data column for Y-axis
    pub x_range: (f64, f64),   // Range for X-axis
    pub y_range: (f64, f64),   // Range for Y-axis
    pub bins: (usize, usize),  // Number of bins for X and Y axes
    pub cuts: Cuts,            // Cuts for the histogram
    pub calculate: bool,       // Whether to calculate the histogram
    pub enabled: bool,         // Whether to let the user interact with the histogram
}

impl Hist2DConfig {
    pub fn new(
        name: &str,
        x_column_name: &str,
        y_column_name: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        bins: (usize, usize),
    ) -> Self {
        Self {
            name: name.to_string(),
            x_column_name: x_column_name.to_string(),
            y_column_name: y_column_name.to_string(),
            x_range,
            y_range,
            bins,
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }
    }

    pub fn table_row(&mut self, row: &mut egui_extras::TableRow<'_, '_>, cuts: &mut Cuts) {
        row.col(|ui| {
            ui.add_enabled(
                self.enabled,
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Name")
                    .clip_text(false),
            );
        });

        row.col(|ui| {
            ui.vertical(|ui| {
                ui.add_enabled(
                    self.enabled,
                    egui::TextEdit::singleline(&mut self.x_column_name)
                        .hint_text("X Column Name")
                        .clip_text(false),
                );
                ui.add_enabled(
                    self.enabled,
                    egui::TextEdit::singleline(&mut self.y_column_name)
                        .hint_text("Y Column Name")
                        .clip_text(false),
                );
            });
        });

        row.col(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        self.enabled,
                        egui::DragValue::new(&mut self.x_range.0)
                            .speed(0.1)
                            .prefix("(")
                            .suffix(","),
                    );
                    ui.add_enabled(
                        self.enabled,
                        egui::DragValue::new(&mut self.x_range.1)
                            .speed(0.1)
                            .prefix(" ")
                            .suffix(")"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        self.enabled,
                        egui::DragValue::new(&mut self.y_range.0)
                            .speed(0.1)
                            .prefix("(")
                            .suffix(","),
                    );
                    ui.add_enabled(
                        self.enabled,
                        egui::DragValue::new(&mut self.y_range.1)
                            .speed(0.1)
                            .prefix(" ")
                            .suffix(")"),
                    );
                });
            });
        });

        row.col(|ui| {
            ui.vertical(|ui| {
                ui.add_enabled(
                    self.enabled,
                    egui::DragValue::new(&mut self.bins.0).speed(1),
                );
                ui.add_enabled(
                    self.enabled,
                    egui::DragValue::new(&mut self.bins.1).speed(1),
                );
            });
        });

        row.col(|ui| {
            egui::ComboBox::from_id_salt(format!("cut_select_2d_{}", self.name))
                .selected_text("Select cuts")
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for cut in &cuts.cuts {
                        let mut is_selected = self
                            .cuts
                            .cuts
                            .iter()
                            .any(|selected_cut| selected_cut == cut);

                        match cut {
                            Cut::Cut1D(cut1d) => {
                                if ui.checkbox(&mut is_selected, &cut1d.name).clicked() {
                                    if is_selected && !self.cuts.cuts.contains(cut) {
                                        self.cuts.cuts.push(cut.clone());
                                    } else if !is_selected {
                                        self.cuts.cuts.retain(|selected_cut| selected_cut != cut);
                                    }
                                }
                            }
                            Cut::Cut2D(cut2d) => {
                                if ui.checkbox(&mut is_selected, &cut2d.polygon.name).clicked() {
                                    if is_selected && !self.cuts.cuts.contains(cut) {
                                        self.cuts.cuts.push(cut.clone());
                                    } else if !is_selected {
                                        self.cuts.cuts.retain(|selected_cut| selected_cut != cut);
                                    }
                                }
                            }
                        }
                    }
                });
        });

        row.col(|ui| {
            ui.add_enabled(self.enabled, egui::Checkbox::new(&mut self.calculate, ""));
        });
    }

    pub fn expand(&self) -> Vec<Self> {
        // Regex for range pattern `{start-end}`
        let range_re = regex::Regex::new(r"\{(\d+)-(\d+)\}").unwrap();

        // Regex for discrete comma-separated values `{val1,val2,...}`
        let list_re = regex::Regex::new(r"\{([\d,]+)\}").unwrap();

        let mut configs = Vec::new();

        if self.calculate {
            if self.name.contains("{}") {
                // Case 1: `{}` in `name`, `x_column_name` has a pattern
                if let Some(caps) = range_re.captures(&self.x_column_name) {
                    let start: usize = caps[1].parse().unwrap();
                    let end: usize = caps[2].parse().unwrap();
                    for i in start..=end {
                        let mut new_config = self.clone();
                        new_config.name = self.name.replace("{}", &i.to_string());
                        new_config.x_column_name = range_re
                            .replace(&self.x_column_name, i.to_string())
                            .to_string();
                        new_config.y_column_name = self.y_column_name.clone();
                        configs.push(new_config);
                    }
                } else if let Some(caps) = list_re.captures(&self.x_column_name) {
                    let values: Vec<&str> = caps[1].split(',').collect();
                    for val in values {
                        let mut new_config = self.clone();
                        new_config.name = self.name.replace("{}", val);
                        new_config.x_column_name =
                            list_re.replace(&self.x_column_name, val).to_string();
                        new_config.y_column_name = self.y_column_name.clone();
                        configs.push(new_config);
                    }
                }
                // Case 2: `{}` in `name`, `y_column_name` has a pattern
                else if let Some(caps) = range_re.captures(&self.y_column_name) {
                    let start: usize = caps[1].parse().unwrap();
                    let end: usize = caps[2].parse().unwrap();
                    for i in start..=end {
                        let mut new_config = self.clone();
                        new_config.name = self.name.replace("{}", &i.to_string());
                        new_config.x_column_name = self.x_column_name.clone();
                        new_config.y_column_name = range_re
                            .replace(&self.y_column_name, i.to_string())
                            .to_string();
                        configs.push(new_config);
                    }
                } else if let Some(caps) = list_re.captures(&self.y_column_name) {
                    let values: Vec<&str> = caps[1].split(',').collect();
                    for val in values {
                        let mut new_config = self.clone();
                        new_config.name = self.name.replace("{}", val);
                        new_config.x_column_name = self.x_column_name.clone();
                        new_config.y_column_name =
                            list_re.replace(&self.y_column_name, val).to_string();
                        configs.push(new_config);
                    }
                } else {
                    log::error!(
                        "Warning: Unsupported pattern for 2D histogram with name '{}', x_column '{}', y_column '{}'",
                        self.name, self.x_column_name, self.y_column_name
                    );
                }
            } else {
                // Static `name`, expand `x_column_name` or `y_column_name` with range or list patterns
                if let Some(caps) = range_re.captures(&self.x_column_name) {
                    let start: usize = caps[1].parse().unwrap();
                    let end: usize = caps[2].parse().unwrap();
                    for i in start..=end {
                        let mut new_config = self.clone();
                        new_config.x_column_name = range_re
                            .replace(&self.x_column_name, i.to_string())
                            .to_string();
                        configs.push(new_config);
                    }
                } else if let Some(caps) = list_re.captures(&self.x_column_name) {
                    let values: Vec<&str> = caps[1].split(',').collect();
                    for val in values {
                        let mut new_config = self.clone();
                        new_config.x_column_name =
                            list_re.replace(&self.x_column_name, val).to_string();
                        configs.push(new_config);
                    }
                } else if let Some(caps) = range_re.captures(&self.y_column_name) {
                    let start: usize = caps[1].parse().unwrap();
                    let end: usize = caps[2].parse().unwrap();
                    for i in start..=end {
                        let mut new_config = self.clone();
                        new_config.y_column_name = range_re
                            .replace(&self.y_column_name, i.to_string())
                            .to_string();
                        configs.push(new_config);
                    }
                } else if let Some(caps) = list_re.captures(&self.y_column_name) {
                    let values: Vec<&str> = caps[1].split(',').collect();
                    for val in values {
                        let mut new_config = self.clone();
                        new_config.y_column_name =
                            list_re.replace(&self.y_column_name, val).to_string();
                        configs.push(new_config);
                    }
                } else {
                    configs.push(self.clone());
                }
            }
        }
        configs
    }
}

use polars::prelude::*;
use regex::Regex;

fn expr_from_string(expression: &str) -> Result<Expr, PolarsError> {
    let re = Regex::new(r"(-?\d+\.?\d*|\w+|\*\*|[+*/()-])").unwrap();
    let tokens: Vec<String> = re
        .find_iter(expression)
        .map(|m| m.as_str().to_string())
        .collect();

    let mut expr_stack: Vec<Expr> = Vec::new();
    let mut op_stack: Vec<String> = Vec::new();

    log::debug!("Starting evaluation of expression: '{}'", expression);
    log::debug!("Tokens: {:?}", tokens);

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token.as_str() {
            "+" | "-" | "*" | "/" | "**" => {
                // Handle consecutive operators like "- -" or "- +"
                if i < tokens.len() - 1 && (tokens[i + 1] == "-" || tokens[i + 1] == "+") {
                    // Collapse consecutive operators into one
                    let mut sign = if *token == "-" { -1.0 } else { 1.0 };
                    let mut j = i + 1;
                    while j < tokens.len() && (tokens[j] == "-" || tokens[j] == "+") {
                        sign *= if tokens[j] == "-" { -1.0 } else { 1.0 };
                        j += 1;
                    }

                    if j < tokens.len() && tokens[j].parse::<f64>().is_ok() {
                        // Combine the collapsed operators with the number
                        let number = tokens[j].parse::<f64>().unwrap();
                        expr_stack.push(lit(sign * number));
                        i = j; // Skip to the next token after the number
                        continue;
                    }
                }

                // Normal operator precedence handling
                while let Some(op) = op_stack.last() {
                    if precedence(op) > precedence(token)
                        || (precedence(op) == precedence(token) && is_left_associative(token))
                    {
                        apply_op(&mut expr_stack, op_stack.pop().unwrap().as_str());
                    } else {
                        break;
                    }
                }
                op_stack.push(token.clone());
            }
            "(" => {
                op_stack.push(token.clone());
            }
            ")" => {
                while let Some(op) = op_stack.pop() {
                    if op == "(" {
                        break;
                    }
                    apply_op(&mut expr_stack, &op);
                }
            }
            _ if token.parse::<f64>().is_ok() => {
                let number = token.parse::<f64>().unwrap();
                expr_stack.push(lit(number));
            }
            _ => {
                expr_stack.push(col(token));
            }
        }
        i += 1;
    }

    while let Some(op) = op_stack.pop() {
        apply_op(&mut expr_stack, &op);
    }

    if expr_stack.len() == 1 {
        Ok(expr_stack.pop().unwrap())
    } else {
        log::error!("Error: Stack ended with more than one expression, invalid expression");
        Err(PolarsError::ComputeError("Invalid expression".into()))
    }
}

fn precedence(op: &str) -> i32 {
    match op {
        "+" | "-" => 1,
        "*" | "/" => 2,
        "**" => 3,
        _ => 0,
    }
}

fn is_left_associative(op: &str) -> bool {
    match op {
        "+" | "-" | "*" | "/" => true,
        "**" => false, // Exponentiation is right-associative
        _ => false,
    }
}

fn apply_op(expr_stack: &mut Vec<Expr>, operator: &str) {
    if expr_stack.len() < 2 {
        log::warn!("Error: Not enough operands for '{}'", operator);
        return;
    }

    let right = expr_stack.pop().unwrap();
    let left = expr_stack.pop().unwrap();

    let result = match operator {
        "+" => left + right,
        "-" => left - right,
        "*" => left * right,
        "/" => left / right,
        "**" => left.pow(right),
        _ => {
            log::error!("Unknown operator: '{}'", operator);
            return;
        }
    };

    expr_stack.push(result);
}

fn add_computed_column(
    lf: &mut LazyFrame,
    expression: &str,
    alias: &str,
) -> Result<(), PolarsError> {
    let computed_expr = expr_from_string(expression)?;
    log::info!("Computed expression: {:?}", computed_expr);
    *lf = lf.clone().with_column(computed_expr.alias(alias)); // Use alias for the new column name
    Ok(())
}

pub fn get_column_names_from_lazyframe(lf: &LazyFrame) -> Vec<String> {
    let lf: LazyFrame = lf.clone().limit(1);
    let df: DataFrame = lf.collect().unwrap();
    let columns: Vec<String> = df
        .get_column_names_owned()
        .into_iter()
        .map(|name| name.to_string())
        .collect();

    columns
}
