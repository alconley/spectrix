use super::cuts::{ActiveHistogramCut, Cut, Cuts};
use super::histogrammer::Histogrammer;
use super::ui_helpers::{precise_drag_value, searchable_column_picker_ui};

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
    #[serde(default)]
    pub variables: Vec<(String, f64)>,
    pub cuts: Cuts,
    #[serde(skip)]
    column_ui_state: Vec<ComputedColumnUiState>,
    #[serde(skip)]
    selected_column_index: Option<usize>,
}

type Hist1DShapeLock = Option<((f64, f64), usize)>;
type Hist2DShapeLock = Option<(((f64, f64), (f64, f64)), (usize, usize))>;
type DuplicateShapeLock = (Hist1DShapeLock, Hist2DShapeLock);

#[derive(Clone, Debug, Default)]
struct ComputedColumnUiState {
    builder: ComputedColumnBuilder,
    unsupported_expression: Option<String>,
    expression_fingerprint: String,
    initialized: bool,
}

#[derive(Clone, Debug)]
struct ComputedColumnBuilder {
    terms: Vec<ComputedColumnTerm>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ComputedColumnTermSign {
    #[default]
    Add,
    Subtract,
}

impl ComputedColumnTermSign {
    fn label(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ComputedColumnTermKind {
    #[default]
    Column,
    Constant,
}

impl ComputedColumnTermKind {
    fn label(self) -> &'static str {
        match self {
            Self::Column => "Column",
            Self::Constant => "Constant",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ComputedColumnValueKind {
    #[default]
    Literal,
    Variable,
    Column,
}

impl ComputedColumnValueKind {
    fn label(self) -> &'static str {
        match self {
            Self::Literal => "Value",
            Self::Variable => "Variable",
            Self::Column => "Column",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ComputedColumnTerm {
    sign: ComputedColumnTermSign,
    kind: ComputedColumnTermKind,
    coefficient_kind: ComputedColumnValueKind,
    coefficient_literal: f64,
    coefficient_column: String,
    column: String,
    power: f64,
}

impl Default for ComputedColumnTerm {
    fn default() -> Self {
        Self {
            sign: ComputedColumnTermSign::Add,
            kind: ComputedColumnTermKind::Column,
            coefficient_kind: ComputedColumnValueKind::Literal,
            coefficient_literal: 1.0,
            coefficient_column: String::new(),
            column: String::new(),
            power: 1.0,
        }
    }
}

impl Default for ComputedColumnBuilder {
    fn default() -> Self {
        Self {
            terms: vec![ComputedColumnTerm::default()],
        }
    }
}

impl Configs {
    pub fn hist1d(
        &mut self,
        name: &str,
        column_name: &str,
        range: (f64, f64),
        bins: usize,
        cuts: &Option<Cuts>,
    ) {
        let mut config = Hist1DConfig::new(name, column_name, range, bins);

        if let Some(cuts) = cuts {
            config.cuts = cuts.clone();
        }

        self.configs.push(Config::Hist1D(config));
    }

    #[expect(clippy::too_many_arguments)]
    pub fn hist2d(
        &mut self,
        name: &str,
        x_column_name: &str,
        y_column_name: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        bins: (usize, usize),
        cuts: &Option<Cuts>,
    ) {
        let mut config =
            Hist2DConfig::new(name, x_column_name, y_column_name, x_range, y_range, bins);

        if let Some(cuts) = cuts {
            config.cuts = cuts.clone();
        }

        self.configs.push(Config::Hist2D(config));
    }

    pub fn merge(&mut self, other: Self) -> &mut Self {
        // Merge configurations
        for config in other.configs {
            match &config {
                Config::Hist1D(other_hist1d) => {
                    if let Some(existing) = self.configs.iter().find_map(|c| match c {
                        Config::Hist1D(h) if h.name == other_hist1d.name => Some(h),
                        _ => None,
                    }) {
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
                    if let Some(existing) = self.configs.iter().find_map(|c| match c {
                        Config::Hist2D(h) if h.name == other_hist2d.name => Some(h),
                        _ => None,
                    }) {
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

        for (name, value) in other.variables {
            if let Some(existing) = self
                .variables
                .iter()
                .find(|(existing_name, _)| existing_name == &name)
            {
                if (existing.1 - value).abs() > f64::EPSILON {
                    log::error!(
                        "Conflict detected for variable '{}': Existing value '{}', New value '{}'.",
                        name,
                        existing.1,
                        value
                    );
                }
            } else {
                self.variables.push((name, value));
            }
        }

        // Merge cuts
        self.cuts.merge(&other.cuts);

        self
    }

    pub fn valid_configs(&mut self, lf: &mut LazyFrame) -> Self {
        let mut column_names = match get_column_names_from_lazyframe(lf) {
            Ok(names) => names,
            Err(e) => {
                log::error!("Failed to retrieve column names: {e:?}");
                return Self::default();
            }
        };

        let mut valid_variables = Vec::new();
        let mut variable_aliases = std::collections::BTreeSet::new();
        let future_computed_aliases = self
            .columns
            .iter()
            .map(|(_, alias)| alias.clone())
            .filter(|alias| !alias.trim().is_empty())
            .collect::<std::collections::BTreeSet<_>>();

        for (name, value) in &self.variables {
            if !is_valid_identifier_name(name) {
                log::error!("Invalid variable name '{name}'");
                continue;
            }

            if column_names.contains(name) || future_computed_aliases.contains(name) {
                log::error!(
                    "Variable '{name}' conflicts with an existing or computed column name."
                );
                continue;
            }

            if !variable_aliases.insert(name.clone()) {
                log::error!("Duplicate variable name '{name}'");
                continue;
            }

            valid_variables.push((name.clone(), *value));
        }

        // Add new computed columns to the LazyFrame
        for (expression, alias) in &self.columns {
            if column_names.contains(alias) {
                log::info!(
                    "Skipping computed column '{alias}' because it already exists in the LazyFrame"
                );
                continue;
            }

            if let Err(e) = add_computed_column(lf, expression, alias, &valid_variables) {
                log::error!("Error adding computed column '{alias}': {e}");
            } else {
                column_names.push(alias.clone());
            }
        }

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
                                    let has_columns = required_columns.iter().all(|col| column_names.contains(col));
                                    let is_active = match cut {
                                        Cut::Cut1D(cut1d) => cut1d.active,
                                        Cut::Cut2D(cut2d) => cut2d.active,
                                    };
                                    if !has_columns {
                                        for column in required_columns {
                                            if !column_names.contains(&column) {
                                                log::error!(
                                                    "Invalid cut '{}' for 1D histogram '{}': Missing column '{}'",
                                                    cut.name(),
                                                    hist1d.name,
                                                    column
                                                );
                                            }
                                        }
                                    }
                                    has_columns && is_active
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
                                    let has_columns = required_columns.iter().all(|col| column_names.contains(col));
                                    let is_active = match cut {
                                        Cut::Cut1D(cut1d) => cut1d.active,
                                        Cut::Cut2D(cut2d) => cut2d.active,
                                    };
                                    if !has_columns {
                                        for column in required_columns {
                                            if !column_names.contains(&column) {
                                                log::error!(
                                                    "Invalid cut '{}' for 2D histogram '{}': Missing column '{}'",
                                                    cut.name(),
                                                    hist2d.name,
                                                    column
                                                );
                                            }
                                        }
                                    }
                                    has_columns && is_active
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
        Self {
            configs: valid_configs,
            columns: self.columns.clone(),
            variables: valid_variables,
            cuts: valid_cuts,
            column_ui_state: self.column_ui_state.clone(),
            selected_column_index: self.selected_column_index,
        }
    }

    pub fn check_and_add_panes(&self, h: &mut Histogrammer) {
        for config in &self.configs {
            match config {
                Config::Hist1D(hist1d) => {
                    h.add_hist1d(&hist1d.name, hist1d.bins, (hist1d.range.0, hist1d.range.1));
                }
                Config::Hist2D(hist2d) => {
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

    fn expand(&self) -> Self {
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

        Self {
            configs: expanded_configs,
            columns: self.columns.clone(),
            variables: self.variables.clone(),
            cuts: self.cuts.clone(),
            column_ui_state: self.column_ui_state.clone(),
            selected_column_index: self.selected_column_index,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    fn sync_column_ui_state(&mut self, base_columns: &[String]) {
        self.column_ui_state
            .resize_with(self.columns.len(), ComputedColumnUiState::default);

        if self.column_ui_state.len() > self.columns.len() {
            self.column_ui_state.truncate(self.columns.len());
        }

        let available_variables = self.available_variables_for_ui();
        let available_columns_per_row = (0..self.columns.len())
            .map(|index| self.available_columns_for_builder_row(base_columns, index))
            .collect::<Vec<_>>();

        for (index, ((expression, _), state)) in self
            .columns
            .iter()
            .zip(self.column_ui_state.iter_mut())
            .enumerate()
        {
            if state.initialized && state.expression_fingerprint == *expression {
                continue;
            }

            if let Some(builder) = parse_simple_computed_column_builder(
                expression,
                &available_columns_per_row[index],
                &available_variables,
            ) {
                state.builder = builder;
                state.unsupported_expression = None;
            } else {
                state.builder = ComputedColumnBuilder::default();
                state.unsupported_expression = if expression.trim().is_empty() {
                    None
                } else {
                    Some(expression.clone())
                };
            }

            state.expression_fingerprint = expression.clone();
            state.initialized = true;
        }
    }

    fn available_columns_for_ui(&self, base_columns: &[String]) -> Vec<String> {
        let mut available_columns = base_columns.to_vec();
        available_columns.extend(
            self.columns
                .iter()
                .map(|(_, alias)| alias.clone())
                .filter(|alias| !alias.trim().is_empty()),
        );
        available_columns.sort();
        available_columns.dedup();
        available_columns
    }

    fn available_variables_for_ui(&self) -> Vec<String> {
        let mut available_variables = self
            .variables
            .iter()
            .map(|(name, _)| name.clone())
            .filter(|name| !name.trim().is_empty())
            .collect::<Vec<_>>();
        available_variables.sort();
        available_variables.dedup();
        available_variables
    }

    fn available_columns_for_builder_row(
        &self,
        base_columns: &[String],
        row_index: usize,
    ) -> Vec<String> {
        let mut available_columns = base_columns.to_vec();
        available_columns.extend(
            self.columns
                .iter()
                .take(row_index)
                .map(|(_, alias)| alias.clone())
                .filter(|alias| !alias.trim().is_empty()),
        );
        available_columns.sort();
        available_columns.dedup();
        available_columns
    }

    pub fn variable_ui(&mut self, ui: &mut egui::Ui, base_columns: &[String]) {
        ui.horizontal(|ui| {
            ui.label("Variables");

            if ui.button("+").clicked() {
                self.variables.push((String::new(), 0.0));
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.variables.clear();
            }
        });

        if self.variables.is_empty() {
            return;
        }

        let existing_column_aliases = self
            .columns
            .iter()
            .map(|(_, alias)| alias.as_str())
            .filter(|alias| !alias.trim().is_empty())
            .collect::<std::collections::BTreeSet<_>>();
        let mut indices_to_remove = Vec::new();

        TableBuilder::new(ui)
            .id_salt("named_variables")
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .striped(true)
            .vscroll(false)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Name");
                });
                header.col(|ui| {
                    ui.label("Value");
                });
            })
            .body(|mut body| {
                for (index, (name, value)) in self.variables.iter_mut().enumerate() {
                    body.row(28.0, |mut row| {
                        row.col(|ui| {
                            let response = ui.add(
                                egui::TextEdit::singleline(name)
                                    .hint_text("Variable")
                                    .clip_text(false),
                            );
                            let changed = response.changed();
                            response.on_hover_text(
                                "Variables may only contain letters, numbers, and underscores.",
                            );
                            if changed {
                                *name = sanitize_identifier_name(name);
                            }

                            let trimmed = name.trim();
                            if !trimmed.is_empty()
                                && (base_columns.iter().any(|column| column == trimmed)
                                    || existing_column_aliases.contains(trimmed))
                            {
                                ui.colored_label(
                                    egui::Color32::LIGHT_YELLOW,
                                    "Conflicts with a column name.",
                                );
                            }
                        });

                        row.col(|ui| {
                            ui.add(precise_drag_value(value).speed(0.1));
                        });

                        row.col(|ui| {
                            if ui.button("X").clicked() {
                                indices_to_remove.push(index);
                            }
                        });
                    });
                }
            });

        for &index in indices_to_remove.iter().rev() {
            self.variables.remove(index);
        }
    }

    pub fn config_ui(
        &mut self,
        ui: &mut egui::Ui,
        available_cuts: &mut Cuts,
        available_columns: &[String],
    ) {
        ui.horizontal(|ui| {
            ui.label("Histograms");

            if ui.button("+1D").clicked() {
                self.configs.push(Config::Hist1D(Hist1DConfig {
                    name: String::new(),
                    column_name: String::new(),
                    range: (0.0, 4096.0),
                    bins: 512,
                    cuts: Cuts::default(),
                    calculate: true,
                    enabled: true,
                }));
            }

            if ui.button("+2D").clicked() {
                self.configs.push(Config::Hist2D(Hist2DConfig {
                    name: String::new(),
                    x_column_name: String::new(),
                    y_column_name: String::new(),
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
        let mut pending_clone = None;
        let mut pending_move = None;
        let duplicate_shape_locks: Vec<DuplicateShapeLock> = self
            .configs
            .iter()
            .enumerate()
            .map(|(index, config)| match config {
                Config::Hist1D(current) => {
                    let lock = if current.name.trim().is_empty() {
                        None
                    } else {
                        self.configs[..index].iter().find_map(|prior| match prior {
                            Config::Hist1D(prior_1d) if prior_1d.name == current.name => {
                                Some((prior_1d.range, prior_1d.bins))
                            }
                            _ => None,
                        })
                    };

                    (lock, None)
                }
                Config::Hist2D(current) => {
                    let lock = if current.name.trim().is_empty() {
                        None
                    } else {
                        self.configs[..index].iter().find_map(|prior| match prior {
                            Config::Hist2D(prior_2d) if prior_2d.name == current.name => {
                                Some(((prior_2d.x_range, prior_2d.y_range), prior_2d.bins))
                            }
                            _ => None,
                        })
                    };

                    (None, lock)
                }
            })
            .collect();

        // Create the table
        TableBuilder::new(ui)
            .id_salt("hist_configs")
            .column(Column::auto()) // Type
            .column(Column::auto()) // Name
            .column(Column::auto()) // Columns
            .column(Column::auto()) // Ranges
            .column(Column::auto()) // Bins
            .column(Column::auto()) // cuts
            .column(Column::auto()) // enabled
            .column(Column::auto()) // clone
            .column(Column::remainder()) // remove
            .striped(true)
            .vscroll(false)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(" ↕ ");
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
                header.col(|ui| {
                    ui.label("On");
                });
                header.col(|ui| {
                    ui.label("Clone");
                });
            })
            .body(|mut body| {
                for (index, (hist1d_lock, hist2d_lock)) in
                    duplicate_shape_locks.iter().copied().enumerate()
                {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            let response = ui
                                .add(
                                    egui::Button::new(format!("↕ {index}"))
                                        .sense(egui::Sense::click_and_drag()),
                                )
                                .on_hover_text("Drag to reorder this histogram config");

                            response.dnd_set_drag_payload(index);

                            if let Some(dragged_index) = response.dnd_release_payload::<usize>() {
                                let insert_index = ui.pointer_interact_pos().map_or(index, |pos| {
                                    if pos.y >= response.rect.center().y {
                                        index + 1
                                    } else {
                                        index
                                    }
                                });
                                pending_move = Some((*dragged_index, insert_index));
                            }
                        });

                        match &mut self.configs[index] {
                            Config::Hist1D(config) => {
                                config.table_row(
                                    &mut row,
                                    available_cuts,
                                    hist1d_lock,
                                    available_columns,
                                    index,
                                );
                            }
                            Config::Hist2D(config) => {
                                config.table_row(
                                    &mut row,
                                    available_cuts,
                                    hist2d_lock,
                                    available_columns,
                                    index,
                                );
                            }
                        }

                        row.col(|ui| {
                            if ui
                                .button("\u{2935}")
                                .on_hover_text("Clone this histogram config")
                                .clicked()
                            {
                                pending_clone = Some(index);
                            }
                        });

                        row.col(|ui| {
                            if ui.button("X").clicked() {
                                indices_to_remove.push(index);
                            }
                        });
                    });
                }
            });

        // Remove indices in reverse order to prevent shifting issues
        if !indices_to_remove.is_empty() {
            for &index in indices_to_remove.iter().rev() {
                self.configs.remove(index);
            }
        } else if let Some(index) = pending_clone {
            if let Some(config) = self.configs.get(index).cloned() {
                self.configs.insert(index + 1, config);
            }
        } else if let Some((from, to)) = pending_move {
            let len = self.configs.len();
            if from < len && to <= len {
                let target = if from < to { to - 1 } else { to };
                if from != target {
                    let config = self.configs.remove(from);
                    self.configs.insert(target, config);
                }
            }
        }
    }

    pub fn column_ui(&mut self, ui: &mut egui::Ui, base_columns: &[String]) {
        self.sync_column_ui_state(base_columns);

        if self
            .selected_column_index
            .is_some_and(|index| index >= self.columns.len())
        {
            self.selected_column_index = None;
        }

        ui.horizontal(|ui| {
            ui.label("Column Creation");

            if ui.button("+").clicked() {
                self.columns.push((String::new(), String::new()));
                self.column_ui_state.push(ComputedColumnUiState::default());
                self.selected_column_index = Some(self.columns.len() - 1);
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.columns.clear();
                self.column_ui_state.clear();
                self.selected_column_index = None;
            }
        });

        if !self.columns.is_empty() {
            let mut indices_to_remove_column = Vec::new();
            let mut pending_selected_column = None;

            TableBuilder::new(ui)
                .id_salt("new_columns")
                .column(Column::auto()) // alias
                .column(Column::remainder()) // expression
                .column(Column::auto()) // actions
                .striped(true)
                .vscroll(false)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label("Alias");
                    });
                    header.col(|ui| {
                        ui.label("Expression");
                    });
                    header.col(|ui| {
                        ui.label("");
                    });
                })
                .body(|mut body| {
                    for (index, (expression, alias)) in self.columns.iter_mut().enumerate() {
                        let state = &mut self.column_ui_state[index];
                        state.builder.ensure_terms();
                        let is_selected = self.selected_column_index == Some(index);

                        body.row(ComputedColumnBuilder::table_row_height(), |mut row| {
                            row.col(|ui| {
                                let response = ui.add(
                                    egui::TextEdit::singleline(alias)
                                        .hint_text("Alias")
                                        .clip_text(false),
                                );
                                let changed = response.changed();
                                response.on_hover_text(
                                    "Aliases may only contain letters, numbers, and underscores.",
                                );
                                if changed {
                                    *alias = sanitize_computed_column_alias(alias);
                                }
                            });

                            row.col(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .small_button(if is_selected {
                                                "Hide Builder"
                                            } else {
                                                "Builder"
                                            })
                                            .clicked()
                                        {
                                            pending_selected_column =
                                                Some(if is_selected { None } else { Some(index) });
                                        }

                                        ui.label(format!(
                                            "{} term(s)",
                                            state.builder.active_term_count()
                                        ));
                                    });

                                    ui.label(
                                        egui::RichText::new(summarize_column_expression_line(
                                            expression,
                                        ))
                                        .weak()
                                        .small(),
                                    );
                                });
                            });

                            row.col(|ui| {
                                if ui.button("X").clicked() {
                                    indices_to_remove_column.push(index);
                                }
                            });
                        });
                    }
                });

            if let Some(selected_column_index) = pending_selected_column {
                self.selected_column_index = selected_column_index;
            }

            // Remove indices in reverse order to prevent shifting issues
            for &index in indices_to_remove_column.iter().rev() {
                self.columns.remove(index);
                self.column_ui_state.remove(index);
                match self.selected_column_index {
                    Some(selected) if selected == index => self.selected_column_index = None,
                    Some(selected) if selected > index => {
                        self.selected_column_index = Some(selected - 1);
                    }
                    _ => {}
                }
            }

            if let Some(selected_index) = self.selected_column_index
                && selected_index < self.columns.len()
                && selected_index < self.column_ui_state.len()
            {
                ui.separator();
                ui.group(|ui| {
                    let alias = self.columns[selected_index].1.trim();
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Column Builder: {}",
                                if alias.is_empty() {
                                    "Unnamed Column"
                                } else {
                                    alias
                                }
                            ))
                            .strong(),
                        );
                        if ui.small_button("Close").clicked() {
                            self.selected_column_index = None;
                        }
                    });
                    ui.label(
                        egui::RichText::new(
                            "Build the computed-column expression here. The line above updates automatically.",
                        )
                        .weak()
                        .small(),
                    );
                    ui.separator();
                    self.column_builder_editor_ui(ui, base_columns, selected_index);
                });
            } else {
                ui.label(
                    egui::RichText::new("Click Builder on a computed column to edit its terms.")
                        .weak()
                        .small(),
                );
            }
        }
    }

    fn column_builder_editor_ui(
        &mut self,
        ui: &mut egui::Ui,
        base_columns: &[String],
        index: usize,
    ) {
        let available_columns = self.available_columns_for_builder_row(base_columns, index);
        let available_variables = self.available_variables_for_ui();
        let Some((expression, _alias)) = self.columns.get_mut(index) else {
            return;
        };
        let Some(state) = self.column_ui_state.get_mut(index) else {
            return;
        };

        state.builder.ensure_terms();

        let mut changed = false;
        let mut term_indices_to_remove = Vec::new();
        let total_terms = state.builder.terms.len();

        ui.label(format!("Terms: {}", state.builder.active_term_count()));
        ui.separator();

        for (term_index, term) in state.builder.terms.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if term_index > 0 {
                        egui::ComboBox::from_id_salt(format!(
                            "computed_column_term_sign_{index}_{term_index}"
                        ))
                        .selected_text(term.sign.label())
                        .show_ui(ui, |ui| {
                            changed |= ui
                                .selectable_value(
                                    &mut term.sign,
                                    ComputedColumnTermSign::Add,
                                    ComputedColumnTermSign::Add.label(),
                                )
                                .changed();
                            changed |= ui
                                .selectable_value(
                                    &mut term.sign,
                                    ComputedColumnTermSign::Subtract,
                                    ComputedColumnTermSign::Subtract.label(),
                                )
                                .changed();
                        });
                    } else {
                        ui.label("First term");
                    }

                    if total_terms > 1 && ui.small_button("Remove").clicked() {
                        term_indices_to_remove.push(term_index);
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("Type");
                    egui::ComboBox::from_id_salt(format!(
                        "computed_column_term_kind_{index}_{term_index}"
                    ))
                    .selected_text(term.kind.label())
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut term.kind,
                                ComputedColumnTermKind::Column,
                                ComputedColumnTermKind::Column.label(),
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut term.kind,
                                ComputedColumnTermKind::Constant,
                                ComputedColumnTermKind::Constant.label(),
                            )
                            .changed();
                    });

                    ui.label("Coeff");
                    egui::ComboBox::from_id_salt(format!(
                        "computed_column_coeff_kind_{index}_{term_index}"
                    ))
                    .selected_text(term.coefficient_kind.label())
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut term.coefficient_kind,
                                ComputedColumnValueKind::Literal,
                                ComputedColumnValueKind::Literal.label(),
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut term.coefficient_kind,
                                ComputedColumnValueKind::Variable,
                                ComputedColumnValueKind::Variable.label(),
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut term.coefficient_kind,
                                ComputedColumnValueKind::Column,
                                ComputedColumnValueKind::Column.label(),
                            )
                            .changed();
                    });

                    match term.coefficient_kind {
                        ComputedColumnValueKind::Literal => {
                            changed |= ui
                                .add(precise_drag_value(&mut term.coefficient_literal).speed(0.1))
                                .changed();
                        }
                        ComputedColumnValueKind::Variable => {
                            changed |= searchable_column_picker_ui(
                                ui,
                                format!("computed_column_coeff_variable_{index}_{term_index}"),
                                &mut term.coefficient_column,
                                &available_variables,
                                "Variable",
                                true,
                            );
                        }
                        ComputedColumnValueKind::Column => {
                            changed |= searchable_column_picker_ui(
                                ui,
                                format!("computed_column_coeff_column_{index}_{term_index}"),
                                &mut term.coefficient_column,
                                &available_columns,
                                "Coeff Column",
                                true,
                            );
                        }
                    }

                    if term.kind == ComputedColumnTermKind::Column {
                        ui.label("*");
                        changed |= searchable_column_picker_ui(
                            ui,
                            format!("computed_column_term_column_{index}_{term_index}"),
                            &mut term.column,
                            &available_columns,
                            "Column",
                            true,
                        );

                        ui.label("Power");
                        changed |= ui
                            .add(
                                precise_drag_value(&mut term.power)
                                    .range(0.0..=16.0)
                                    .speed(0.1),
                            )
                            .changed();
                        term.power = term.power.clamp(0.0, 16.0);
                    }
                });
            });
        }

        for &term_index in term_indices_to_remove.iter().rev() {
            state.builder.terms.remove(term_index);
            changed = true;
        }

        state.builder.ensure_terms();

        if ui.button("Add Term").clicked() {
            state.builder.terms.push(ComputedColumnTerm::default());
            changed = true;
            ui.ctx().request_repaint();
        }

        if let Some(unsupported_expression) = &state.unsupported_expression {
            ui.colored_label(
                egui::Color32::LIGHT_YELLOW,
                "Existing expression is not builder-compatible yet. Changing builder fields will replace it.",
            );
            ui.monospace(unsupported_expression);
        }

        let built_expression = state.builder.to_expression_string();
        if changed || state.expression_fingerprint != *expression {
            *expression = built_expression;
            state.expression_fingerprint = expression.clone();
            state.unsupported_expression = None;
        }
    }

    pub fn sync_histogram_cuts(&mut self, available_cuts: &Cuts) {
        let available_cut_names: Vec<String> = available_cuts
            .cuts
            .iter()
            .map(|cut| cut.name().to_owned())
            .collect();

        for hist_config in &mut self.configs {
            match hist_config {
                Config::Hist1D(hist1d) => {
                    for hist_cut in &mut hist1d.cuts.cuts {
                        if let Some(updated_cut) = available_cuts
                            .cuts
                            .iter()
                            .find(|cut| cut.name() == hist_cut.name())
                        {
                            *hist_cut = updated_cut.clone();
                        }
                    }

                    hist1d
                        .cuts
                        .cuts
                        .retain(|cut| available_cut_names.iter().any(|name| name == cut.name()));
                }
                Config::Hist2D(hist2d) => {
                    for hist_cut in &mut hist2d.cuts.cuts {
                        if let Some(updated_cut) = available_cuts
                            .cuts
                            .iter()
                            .find(|cut| cut.name() == hist_cut.name())
                        {
                            *hist_cut = updated_cut.clone();
                        }
                    }

                    hist2d
                        .cuts
                        .cuts
                        .retain(|cut| available_cut_names.iter().any(|name| name == cut.name()));
                }
            }
        }
    }

    pub fn cut_ui(
        &mut self,
        ui: &mut egui::Ui,
        mut active_cuts: Option<&mut [ActiveHistogramCut]>,
        available_columns: &[String],
    ) {
        self.cuts
            .ui(ui, active_cuts.as_deref_mut(), available_columns, "general");
        let merged_cuts = self.cuts.merged_with_active_cuts(active_cuts.as_deref());

        self.sync_histogram_cuts(&merged_cuts);
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        active_cuts: Option<&mut [ActiveHistogramCut]>,
        base_columns: &[String],
    ) {
        let mut merged_cuts = self.cuts.merged_with_active_cuts(active_cuts.as_deref());
        let available_columns = self.available_columns_for_ui(base_columns);

        self.variable_ui(ui, base_columns);

        ui.separator();

        self.column_ui(ui, base_columns);

        ui.separator();

        self.cut_ui(ui, active_cuts, &available_columns);

        ui.separator();

        self.config_ui(ui, &mut merged_cuts, &available_columns);
    }

    pub fn set_prefix(&mut self, prefix: &str) {
        for config in &mut self.configs {
            match config {
                Config::Hist1D(hist1d) => {
                    hist1d.name = format!("{}/{}", prefix, hist1d.name);
                }
                Config::Hist2D(hist2d) => {
                    hist2d.name = format!("{}/{}", prefix, hist2d.name);
                }
            }
        }
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
            name: name.to_owned(),
            column_name: column_name.to_owned(),
            range,
            bins,
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }
    }

    pub fn table_row(
        &mut self,
        row: &mut egui_extras::TableRow<'_, '_>,
        cuts: &mut Cuts,
        shape_lock: Hist1DShapeLock,
        available_columns: &[String],
        row_index: usize,
    ) {
        if let Some((range, bins)) = shape_lock {
            self.range = range;
            self.bins = bins;
        }

        let shape_editable = self.enabled && shape_lock.is_none();

        row.col(|ui| {
            ui.add_enabled(
                self.enabled,
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Name")
                    .clip_text(false),
            );
        });

        row.col(|ui| {
            searchable_column_picker_ui(
                ui,
                format!("hist1d_column_picker_{row_index}"),
                &mut self.column_name,
                available_columns,
                "Column Name",
                self.enabled,
            );
        });

        row.col(|ui| {
            ui.horizontal(|ui| {
                ui.add_enabled(
                    shape_editable,
                    precise_drag_value(&mut self.range.0)
                        .speed(0.1)
                        .prefix("(")
                        .suffix(","),
                )
                .on_disabled_hover_text("Locked to the first histogram declared with this name.");
                ui.add_enabled(
                    shape_editable,
                    precise_drag_value(&mut self.range.1)
                        .speed(0.1)
                        .prefix(" ")
                        .suffix(")"),
                )
                .on_disabled_hover_text("Locked to the first histogram declared with this name.");
            });
        });

        row.col(|ui| {
            ui.add_enabled(
                shape_editable,
                egui::DragValue::new(&mut self.bins).speed(1),
            )
            .on_disabled_hover_text("Locked to the first histogram declared with this name.");
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
        let range_re = regex::Regex::new(r"\{(\d+)-(\d+)\}").expect("Failed to create range regex");

        // Regex for discrete comma-separated values `{val1,val2,...}`
        let list_re = regex::Regex::new(r"\{([\d,]+)\}").expect("Failed to create list regex");

        let mut configs = Vec::new();

        if self.calculate {
            if self.name.contains("{}") {
                // name has {} and column_name has a range pattern
                if let Some(caps) = range_re.captures(&self.column_name) {
                    let start: usize = caps[1].parse().expect("Failed to parse start range");
                    let end: usize = caps[2].parse().expect("Failed to parse end range");

                    // Loop through start and end values
                    for i in start..=end {
                        let mut new_config = self.clone();
                        new_config.name = self.name.replace("{}", &i.to_string());
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
                        new_config.name = self.name.replace("{}", val);
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
                    let start: usize = caps[1].parse().expect("Failed to parse start range");
                    let end: usize = caps[2].parse().expect("Failed to parse end range");

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
            name: name.to_owned(),
            x_column_name: x_column_name.to_owned(),
            y_column_name: y_column_name.to_owned(),
            x_range,
            y_range,
            bins,
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }
    }

    pub fn table_row(
        &mut self,
        row: &mut egui_extras::TableRow<'_, '_>,
        cuts: &mut Cuts,
        shape_lock: Hist2DShapeLock,
        available_columns: &[String],
        row_index: usize,
    ) {
        if let Some(((x_range, y_range), bins)) = shape_lock {
            self.x_range = x_range;
            self.y_range = y_range;
            self.bins = bins;
        }

        let shape_editable = self.enabled && shape_lock.is_none();

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
                searchable_column_picker_ui(
                    ui,
                    format!("hist2d_x_column_picker_{row_index}"),
                    &mut self.x_column_name,
                    available_columns,
                    "X Column Name",
                    self.enabled,
                );
                searchable_column_picker_ui(
                    ui,
                    format!("hist2d_y_column_picker_{row_index}"),
                    &mut self.y_column_name,
                    available_columns,
                    "Y Column Name",
                    self.enabled,
                );
            });
        });

        row.col(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        shape_editable,
                        precise_drag_value(&mut self.x_range.0)
                            .speed(0.1)
                            .prefix("(")
                            .suffix(","),
                    )
                    .on_disabled_hover_text(
                        "Locked to the first histogram declared with this name.",
                    );
                    ui.add_enabled(
                        shape_editable,
                        precise_drag_value(&mut self.x_range.1)
                            .speed(0.1)
                            .prefix(" ")
                            .suffix(")"),
                    )
                    .on_disabled_hover_text(
                        "Locked to the first histogram declared with this name.",
                    );
                });
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        shape_editable,
                        precise_drag_value(&mut self.y_range.0)
                            .speed(0.1)
                            .prefix("(")
                            .suffix(","),
                    )
                    .on_disabled_hover_text(
                        "Locked to the first histogram declared with this name.",
                    );
                    ui.add_enabled(
                        shape_editable,
                        precise_drag_value(&mut self.y_range.1)
                            .speed(0.1)
                            .prefix(" ")
                            .suffix(")"),
                    )
                    .on_disabled_hover_text(
                        "Locked to the first histogram declared with this name.",
                    );
                });
            });
        });

        row.col(|ui| {
            ui.vertical(|ui| {
                ui.add_enabled(
                    shape_editable,
                    egui::DragValue::new(&mut self.bins.0).speed(1),
                )
                .on_disabled_hover_text("Locked to the first histogram declared with this name.");
                ui.add_enabled(
                    shape_editable,
                    egui::DragValue::new(&mut self.bins.1).speed(1),
                )
                .on_disabled_hover_text("Locked to the first histogram declared with this name.");
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
        let range_re = regex::Regex::new(r"\{(\d+)-(\d+)\}").expect("Failed to create range regex");

        // Regex for discrete comma-separated values `{val1,val2,...}`
        let list_re = regex::Regex::new(r"\{([\d,]+)\}").expect("Failed to create list regex");

        let mut configs = Vec::new();

        if self.calculate {
            if self.name.contains("{}") {
                // Case 1: `{}` in `name`, `x_column_name` has a pattern
                if let Some(caps) = range_re.captures(&self.x_column_name) {
                    let start: usize = caps[1].parse().expect("Failed to parse start range");
                    let end: usize = caps[2].parse().expect("Failed to parse end range");
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
                    let start: usize = caps[1].parse().expect("Failed to parse start range");
                    let end: usize = caps[2].parse().expect("Failed to parse end range");
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
                        self.name,
                        self.x_column_name,
                        self.y_column_name
                    );
                }
            } else {
                // Static `name`, expand `x_column_name` or `y_column_name` with range or list patterns
                if let Some(caps) = range_re.captures(&self.x_column_name) {
                    let start: usize = caps[1].parse().expect("Failed to parse start range");
                    let end: usize = caps[2].parse().expect("Failed to parse end range");
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
                    let start: usize = caps[1].parse().expect("Failed to parse start range");
                    let end: usize = caps[2].parse().expect("Failed to parse end range");
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

impl ComputedColumnBuilder {
    fn table_row_height() -> f32 {
        44.0
    }

    fn active_term_count(&self) -> usize {
        self.terms
            .iter()
            .filter(|term| Self::term_body_string(term).is_some())
            .count()
    }

    fn ensure_terms(&mut self) {
        if self.terms.is_empty() {
            self.terms.push(ComputedColumnTerm::default());
        }
    }

    fn term_body_string(term: &ComputedColumnTerm) -> Option<String> {
        let coefficient = match term.coefficient_kind {
            ComputedColumnValueKind::Literal => format_literal(term.coefficient_literal),
            ComputedColumnValueKind::Variable | ComputedColumnValueKind::Column => {
                let coefficient_name = term.coefficient_column.trim();
                if coefficient_name.is_empty() {
                    return None;
                }
                coefficient_name.to_owned()
            }
        };

        match term.kind {
            ComputedColumnTermKind::Constant => Some(coefficient),
            ComputedColumnTermKind::Column => {
                let column = term.column.trim();
                if column.is_empty() {
                    return None;
                }

                let base = if (term.power - 1.0).abs() < f64::EPSILON {
                    column.to_owned()
                } else {
                    format!("{column} ** {}", format_literal(term.power))
                };

                if term.coefficient_kind == ComputedColumnValueKind::Literal
                    && (term.coefficient_literal - 1.0).abs() < f64::EPSILON
                {
                    Some(base)
                } else {
                    Some(format!("{coefficient} * {base}"))
                }
            }
        }
    }

    fn to_expression_string(&self) -> String {
        let mut valid_terms = self
            .terms
            .iter()
            .filter_map(|term| Some((term.sign, Self::term_body_string(term)?)))
            .collect::<Vec<_>>();

        if valid_terms.is_empty() {
            return String::new();
        }

        let mut expression = String::new();
        for (index, (sign, body)) in valid_terms.drain(..).enumerate() {
            if index == 0 {
                match sign {
                    ComputedColumnTermSign::Add => {
                        expression.push_str(&format!("({body})"));
                    }
                    ComputedColumnTermSign::Subtract => {
                        expression.push_str(&format!("0 - ({body})"));
                    }
                }
            } else {
                expression.push_str(match sign {
                    ComputedColumnTermSign::Add => " + ",
                    ComputedColumnTermSign::Subtract => " - ",
                });
                expression.push_str(&format!("({body})"));
            }
        }

        expression
    }
}

fn format_literal(value: f64) -> String {
    let mut formatted = format!("{value:.15}");
    if formatted.contains('.') {
        formatted = formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned();
    }

    if formatted == "-0" {
        "0".to_owned()
    } else {
        formatted
    }
}

fn summarize_column_expression_line(expression: &str) -> String {
    let trimmed = expression.trim();
    if trimmed.is_empty() {
        return "Incomplete expression".to_owned();
    }

    const MAX_CHARS: usize = 84;
    let char_count = trimmed.chars().count();
    if char_count <= MAX_CHARS {
        return trimmed.to_owned();
    }

    let mut summary = trimmed.chars().take(MAX_CHARS - 1).collect::<String>();
    summary.push('…');
    summary
}

fn is_valid_identifier_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first_character) = characters.next() else {
        return false;
    };

    if !(first_character.is_ascii_alphabetic() || first_character == '_') {
        return false;
    }

    characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn sanitize_identifier_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut sanitized = String::with_capacity(trimmed.len());
    let mut previous_was_underscore = false;

    for character in trimmed.chars() {
        let mapped = if character.is_ascii_alphanumeric() || character == '_' {
            character
        } else {
            '_'
        };

        if mapped == '_' {
            if !previous_was_underscore {
                sanitized.push(mapped);
            }
            previous_was_underscore = true;
        } else {
            sanitized.push(mapped);
            previous_was_underscore = false;
        }
    }

    if sanitized.is_empty() {
        return String::new();
    }

    if sanitized
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        sanitized.insert(0, '_');
    }

    sanitized
}

fn is_valid_computed_column_alias(alias: &str) -> bool {
    is_valid_identifier_name(alias)
}

fn sanitize_computed_column_alias(alias: &str) -> String {
    sanitize_identifier_name(alias)
}

fn trim_wrapping_parentheses(expression: &str) -> String {
    let mut trimmed = expression.trim().to_owned();

    loop {
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
            break;
        }

        let mut depth = 0usize;
        let mut wraps_entire_expression = true;

        for (index, character) in trimmed.char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && index + character.len_utf8() < trimmed.len() {
                        wraps_entire_expression = false;
                        break;
                    }
                }
                _ => {}
            }
        }

        if wraps_entire_expression && depth == 0 {
            trimmed = trimmed[1..trimmed.len() - 1].trim().to_owned();
        } else {
            break;
        }
    }

    trimmed
}

fn split_top_level_additive_terms(expression: &str) -> Vec<(ComputedColumnTermSign, String)> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Vec::new();
    }

    let mut terms = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut current_sign = ComputedColumnTermSign::Add;
    let mut first_token_seen = false;

    for (index, character) in expression.char_indices() {
        if !first_token_seen {
            if character.is_whitespace() {
                continue;
            }

            first_token_seen = true;
            if character == '+' || character == '-' {
                current_sign = if character == '+' {
                    ComputedColumnTermSign::Add
                } else {
                    ComputedColumnTermSign::Subtract
                };
                start = index + character.len_utf8();
                continue;
            }

            start = index;
        }

        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '+' | '-' if depth == 0 => {
                let term = expression[start..index].trim();
                if !term.is_empty() {
                    terms.push((current_sign, term.to_owned()));
                }
                current_sign = if character == '+' {
                    ComputedColumnTermSign::Add
                } else {
                    ComputedColumnTermSign::Subtract
                };
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }

    let tail = expression[start..].trim();
    if !tail.is_empty() {
        terms.push((current_sign, tail.to_owned()));
    }

    terms
}

fn parse_simple_computed_column_builder(
    expression: &str,
    available_columns: &[String],
    available_variables: &[String],
) -> Option<ComputedColumnBuilder> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Some(ComputedColumnBuilder::default());
    }

    let identifier_re =
        Regex::new(r"^[A-Za-z_]\w*$").expect("failed to create computed-column identifier regex");
    let number_pattern = r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?";
    let number_re = Regex::new(&format!("^{number_pattern}$"))
        .expect("failed to create computed-column number regex");
    let coeff_times_column_re = Regex::new(&format!(
        r"^(?P<coeff>[A-Za-z_]\w*|{number_pattern})\s*\*\s*(?P<column>[A-Za-z_]\w*)(?:\s*\*\*\s*(?P<power>{number_pattern}))?$"
    ))
    .expect("failed to create coefficient-times-column regex");
    let column_term_re = Regex::new(&format!(
        r"^(?P<column>[A-Za-z_]\w*)(?:\s*\*\*\s*(?P<power>{number_pattern}))?$"
    ))
    .expect("failed to create column-term regex");

    let mut terms = Vec::new();
    for (sign, raw_term) in split_top_level_additive_terms(expression) {
        let term = trim_wrapping_parentheses(&raw_term);
        let term = term.trim();

        if let Some(captures) = coeff_times_column_re.captures(term) {
            let coefficient_token = captures.name("coeff")?.as_str();
            let mut parsed_term = ComputedColumnTerm {
                sign,
                kind: ComputedColumnTermKind::Column,
                column: captures.name("column")?.as_str().to_owned(),
                power: captures
                    .name("power")
                    .and_then(|power| power.as_str().parse().ok())
                    .unwrap_or(1.0),
                ..ComputedColumnTerm::default()
            };

            if number_re.is_match(coefficient_token) {
                parsed_term.coefficient_kind = ComputedColumnValueKind::Literal;
                parsed_term.coefficient_literal = coefficient_token.parse().ok()?;
            } else if available_variables
                .iter()
                .any(|variable_name| variable_name == coefficient_token)
            {
                parsed_term.coefficient_kind = ComputedColumnValueKind::Variable;
                parsed_term.coefficient_column = coefficient_token.to_owned();
            } else if identifier_re.is_match(coefficient_token) {
                parsed_term.coefficient_kind = ComputedColumnValueKind::Column;
                parsed_term.coefficient_column = coefficient_token.to_owned();
            } else {
                return None;
            }

            terms.push(parsed_term);
            continue;
        }

        if let Some(captures) = column_term_re.captures(term) {
            let identifier = captures.name("column")?.as_str();
            if available_variables
                .iter()
                .any(|variable_name| variable_name == identifier)
            {
                terms.push(ComputedColumnTerm {
                    sign,
                    kind: ComputedColumnTermKind::Constant,
                    coefficient_kind: ComputedColumnValueKind::Variable,
                    coefficient_column: identifier.to_owned(),
                    ..ComputedColumnTerm::default()
                });
                continue;
            }

            terms.push(ComputedColumnTerm {
                sign,
                kind: ComputedColumnTermKind::Column,
                coefficient_kind: ComputedColumnValueKind::Literal,
                coefficient_literal: 1.0,
                coefficient_column: String::new(),
                column: if available_columns
                    .iter()
                    .any(|column_name| column_name == identifier)
                    || identifier_re.is_match(identifier)
                {
                    identifier.to_owned()
                } else {
                    return None;
                },
                power: captures
                    .name("power")
                    .and_then(|power| power.as_str().parse().ok())
                    .unwrap_or(1.0),
            });
            continue;
        }

        if number_re.is_match(term) {
            terms.push(ComputedColumnTerm {
                sign,
                kind: ComputedColumnTermKind::Constant,
                coefficient_kind: ComputedColumnValueKind::Literal,
                coefficient_literal: term.parse().ok()?,
                ..ComputedColumnTerm::default()
            });
            continue;
        }

        return None;
    }

    if terms.is_empty() {
        None
    } else {
        Some(ComputedColumnBuilder { terms })
    }
}

fn expr_from_string(expression: &str, variables: &[(String, f64)]) -> Result<Expr, PolarsError> {
    let re = Regex::new(r"(-?\d+\.?\d*|\w+|\*\*|[+*/()-])").expect("Failed to create regex");
    let tokens: Vec<String> = re
        .find_iter(expression)
        .map(|m| m.as_str().to_owned())
        .collect();
    let variable_lookup = variables
        .iter()
        .map(|(name, value)| (name.as_str(), *value))
        .collect::<std::collections::HashMap<_, _>>();

    let mut expr_stack: Vec<Expr> = Vec::new();
    let mut op_stack: Vec<String> = Vec::new();

    log::debug!("Starting evaluation of expression: '{expression}'");
    log::debug!("Tokens: {tokens:?}");

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
                        let number = tokens[j].parse::<f64>().expect("Failed to parse number");
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
                        apply_op(
                            &mut expr_stack,
                            op_stack.pop().expect("Failed to pop operator").as_str(),
                        );
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
                let number = token.parse::<f64>().expect("Failed to parse number");
                expr_stack.push(lit(number));
            }
            _ => {
                if let Some(value) = variable_lookup.get(token.as_str()) {
                    expr_stack.push(lit(*value));
                } else {
                    expr_stack.push(col(token));
                }
            }
        }
        i += 1;
    }

    while let Some(op) = op_stack.pop() {
        apply_op(&mut expr_stack, &op);
    }

    if expr_stack.len() == 1 {
        Ok(expr_stack.pop().expect("Failed to pop final expression"))
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
    matches!(op, "+" | "-" | "*" | "/")
}

fn apply_op(expr_stack: &mut Vec<Expr>, operator: &str) {
    if expr_stack.len() < 2 {
        log::warn!("Error: Not enough operands for '{operator}'");
        return;
    }

    let right = expr_stack.pop().expect("Failed to pop right operand");
    let left = expr_stack.pop().expect("Failed to pop left operand");

    let result = match operator {
        "+" => left + right,
        "-" => left - right,
        "*" => left * right,
        "/" => left / right,
        "**" => left.pow(right),
        _ => {
            log::error!("Unknown operator: '{operator}'");
            return;
        }
    };

    expr_stack.push(result);
}

fn add_computed_column(
    lf: &mut LazyFrame,
    expression: &str,
    alias: &str,
    variables: &[(String, f64)],
) -> Result<(), PolarsError> {
    if !is_valid_computed_column_alias(alias) {
        log::error!("Invalid computed column alias '{alias}'");
        return Err(PolarsError::ComputeError(
            format!("Invalid computed column alias: {alias}").into(),
        ));
    }

    // Attempt to create the computed expression
    let computed_expr = expr_from_string(expression, variables).map_err(|err| {
        log::error!("Failed to parse expression: {expression}. Error: {err:?}");
        PolarsError::ComputeError(format!("Error parsing expression: {expression}").into())
    })?;

    // Safely add the computed column to the LazyFrame
    log::info!("Adding computed column '{alias}' with expression '{expression}'");
    *lf = lf.clone().with_column(computed_expr.alias(alias));
    log::info!("Successfully added computed column '{alias}'");

    Ok(())
}

pub fn get_column_names_from_lazyframe(lf: &LazyFrame) -> Result<Vec<String>, PolarsError> {
    let lf = lf.clone().limit(1);

    // Attempt to collect the LazyFrame into a DataFrame
    let df = lf.collect().map_err(|err| {
        log::error!("Failed to collect LazyFrame: {err:?}");
        err
    })?;

    // Get column names
    let columns: Vec<String> = df
        .get_column_names_owned()
        .into_iter()
        .map(|name| name.to_string())
        .collect();

    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computed_column_builder_supports_quadratic_forms() {
        let builder = parse_simple_computed_column_builder(
            "a*column**2 + m*column + c",
            &["column".to_owned()],
            &["a".to_owned(), "m".to_owned(), "c".to_owned()],
        )
        .expect("quadratic expression should parse");

        assert_eq!(builder.terms.len(), 3);

        assert_eq!(builder.terms[0].kind, ComputedColumnTermKind::Column);
        assert_eq!(
            builder.terms[0].coefficient_kind,
            ComputedColumnValueKind::Variable
        );
        assert_eq!(builder.terms[0].coefficient_column, "a");
        assert_eq!(builder.terms[0].column, "column");
        assert!((builder.terms[0].power - 2.0).abs() < f64::EPSILON);

        assert_eq!(builder.terms[1].kind, ComputedColumnTermKind::Column);
        assert_eq!(
            builder.terms[1].coefficient_kind,
            ComputedColumnValueKind::Variable
        );
        assert_eq!(builder.terms[1].coefficient_column, "m");
        assert_eq!(builder.terms[1].column, "column");
        assert!((builder.terms[1].power - 1.0).abs() < f64::EPSILON);

        assert_eq!(builder.terms[2].kind, ComputedColumnTermKind::Constant);
        assert_eq!(
            builder.terms[2].coefficient_kind,
            ComputedColumnValueKind::Variable
        );
        assert_eq!(builder.terms[2].coefficient_column, "c");
    }

    #[test]
    fn computed_column_builder_writes_multi_term_expression() {
        let builder = ComputedColumnBuilder {
            terms: vec![
                ComputedColumnTerm {
                    coefficient_kind: ComputedColumnValueKind::Variable,
                    coefficient_column: "a".to_owned(),
                    column: "column".to_owned(),
                    power: 2.0,
                    ..ComputedColumnTerm::default()
                },
                ComputedColumnTerm {
                    sign: ComputedColumnTermSign::Add,
                    coefficient_kind: ComputedColumnValueKind::Variable,
                    coefficient_column: "m".to_owned(),
                    column: "column".to_owned(),
                    power: 1.0,
                    ..ComputedColumnTerm::default()
                },
                ComputedColumnTerm {
                    sign: ComputedColumnTermSign::Add,
                    kind: ComputedColumnTermKind::Constant,
                    coefficient_kind: ComputedColumnValueKind::Variable,
                    coefficient_column: "c".to_owned(),
                    ..ComputedColumnTerm::default()
                },
            ],
        };

        assert_eq!(
            builder.to_expression_string(),
            "(a * column ** 2) + (m * column) + (c)"
        );
    }

    #[test]
    fn computed_column_builder_supports_fractional_powers() {
        let builder = parse_simple_computed_column_builder(
            "a*column**0.5 + c",
            &["column".to_owned()],
            &["a".to_owned(), "c".to_owned()],
        )
        .expect("fractional-power expression should parse");

        assert_eq!(builder.terms.len(), 2);
        assert_eq!(builder.terms[0].kind, ComputedColumnTermKind::Column);
        assert_eq!(
            builder.terms[0].coefficient_kind,
            ComputedColumnValueKind::Variable
        );
        assert_eq!(builder.terms[0].coefficient_column, "a");
        assert_eq!(builder.terms[0].column, "column");
        assert!((builder.terms[0].power - 0.5).abs() < f64::EPSILON);

        assert_eq!(builder.to_expression_string(), "(a * column ** 0.5) + (c)");
    }

    #[test]
    fn computed_column_aliases_are_sanitized_to_identifiers() {
        assert_eq!(
            sanitize_computed_column_alias(" 12 bad alias!? "),
            "_12_bad_alias_"
        );
        assert!(is_valid_computed_column_alias("_12_bad_alias_"));
        assert!(!is_valid_computed_column_alias("bad-alias"));
    }

    #[test]
    fn expr_parser_replaces_variables_with_literals() {
        let expr = expr_from_string(
            "(a * column ** 2) + (m * column) + (c)",
            &[
                ("a".to_owned(), 2.0),
                ("m".to_owned(), 3.0),
                ("c".to_owned(), 4.0),
            ],
        );

        assert!(expr.is_ok());
    }

    #[test]
    fn expr_parser_accepts_fractional_powers() {
        let expr = expr_from_string(
            "(a * column ** 0.5) + (c)",
            &[("a".to_owned(), 2.0), ("c".to_owned(), 4.0)],
        );

        assert!(expr.is_ok());
    }
}
