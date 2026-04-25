use super::cuts::{ActiveHistogramCut, Cut, Cuts};
use super::histogrammer::Histogrammer;
use super::ui_helpers::{
    precise_drag_value, searchable_column_picker_ui, searchable_multi_column_picker_ui,
};

use egui_extras::{Column, TableBuilder};
use std::hash::{Hash, Hasher as _};

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
    #[serde(default)]
    pub column_groups: Vec<ColumnGroup>,
    pub cuts: Cuts,
    #[serde(skip)]
    column_ui_state: Vec<ComputedColumnUiState>,
    #[serde(skip)]
    selected_column_index: Option<usize>,
    #[serde(skip)]
    expanded_column_ui_cache: ExpandedColumnUiCache,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColumnGroup {
    pub alias: String,
    #[serde(default)]
    pub column_names: Vec<String>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedSourceSelection {
    column_name: String,
    name_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct SourceGroupResolver {
    groups: std::collections::HashMap<String, Vec<ResolvedSourceSelection>>,
}

#[derive(Clone, Debug, Default)]
struct ExpandedColumnUiCache {
    key: u64,
    expanded_alias_prefix_lengths: Vec<usize>,
    all_expanded_aliases: Vec<String>,
    available_columns: Vec<String>,
    available_source_names: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct VariableRowDragPayload(usize);

#[derive(Clone, Copy, Debug)]
struct ColumnGroupRowDragPayload(usize);

#[derive(Clone, Copy, Debug)]
struct ComputedColumnRowDragPayload(usize);

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

fn reorder_vec_item<T>(items: &mut Vec<T>, from: usize, to: usize) -> Option<usize> {
    let len = items.len();
    if from >= len || to > len {
        return None;
    }

    let target = if from < to { to.saturating_sub(1) } else { to };
    if from == target {
        return None;
    }

    let item = items.remove(from);
    items.insert(target, item);
    Some(target)
}

fn reordered_selected_index(selected: Option<usize>, from: usize, target: usize) -> Option<usize> {
    let selected_index = selected?;

    if selected_index == from {
        return Some(target);
    }

    if from < target && selected_index > from && selected_index <= target {
        return Some(selected_index - 1);
    }

    if target < from && selected_index >= target && selected_index < from {
        return Some(selected_index + 1);
    }

    Some(selected_index)
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
                        let existing_columns = existing.fill_columns();
                        let other_columns = other_hist1d.fill_columns();
                        let shares_existing_column = other_columns
                            .iter()
                            .any(|column_name| existing_columns.contains(column_name));

                        if !shares_existing_column
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
                        let existing_pairs = existing.fill_pairs_for_merge();
                        let other_pairs = other_hist2d.fill_pairs_for_merge();
                        let shares_existing_pair =
                            other_pairs.iter().any(|pair| existing_pairs.contains(pair));

                        if !shares_existing_pair
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

        for other_group in other.column_groups {
            let normalized_other_columns = other_group.normalized_column_names();

            if let Some(existing) = self
                .column_groups
                .iter()
                .find(|group| group.alias == other_group.alias)
            {
                if existing.normalized_column_names() != normalized_other_columns {
                    log::error!(
                        "Conflict detected for column group alias '{}': Existing members '{:?}', New members '{:?}'.",
                        other_group.alias,
                        existing.normalized_column_names(),
                        normalized_other_columns
                    );
                }
            } else {
                self.column_groups.push(ColumnGroup {
                    alias: other_group.alias,
                    column_names: normalized_other_columns,
                });
            }
        }

        // Merge cuts
        self.cuts.merge(&other.cuts);

        self
    }

    pub fn prepend_computed_columns(&mut self, computed_columns: Vec<(String, String)>) {
        if computed_columns.is_empty() {
            return;
        }

        let replacement_aliases = computed_columns
            .iter()
            .map(|(_, alias)| alias.as_str())
            .filter(|alias| !alias.trim().is_empty())
            .collect::<std::collections::BTreeSet<_>>();

        self.columns.retain(|(_, alias)| {
            let keep = !replacement_aliases.contains(alias.as_str());
            if !keep {
                log::warn!(
                    "Replacing computed column '{alias}' with an earlier calibration column definition."
                );
            }
            keep
        });

        let mut prepended_columns = computed_columns
            .into_iter()
            .filter(|(_, alias)| !alias.trim().is_empty())
            .collect::<Vec<_>>();
        prepended_columns.append(&mut self.columns);
        self.columns = prepended_columns;
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
        let future_computed_aliases = collect_expanded_computed_column_aliases(
            &column_names,
            &self.columns,
            &self.column_groups,
            &self.variables,
        )
        .1
        .into_iter()
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

        apply_computed_columns_to_lazyframe(
            lf,
            &mut column_names,
            &self.columns,
            &self.column_groups,
            &valid_variables,
        );

        // Ensure 1D cuts have their expressions parsed
        self.cuts.parse_conditions();

        // Expand the configurations (to account for patterns)
        let expanded_configs = self.expand(&column_names);

        // Validate configurations and cuts
        let mut valid_configs = Vec::new();
        let mut valid_cuts = Cuts::default();

        for config in &expanded_configs.configs {
            match config {
                Config::Hist1D(hist1d) => {
                    let mut parsed_hist1d = hist1d.clone();
                    parsed_hist1d.cuts.parse_conditions();

                    if hist1d.calculate {
                        if column_names.contains(&parsed_hist1d.column_name) {
                            // Validate cuts for the histogram
                            let valid_hist_cuts: Vec<Cut> = parsed_hist1d
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
                                                log::warn!(
                                                    "Invalid cut '{}' for 1D histogram '{}': Missing column '{}'",
                                                    cut.name(),
                                                    parsed_hist1d.name,
                                                    column
                                                );
                                            }
                                        }
                                    }
                                    has_columns && is_active
                                })
                                .cloned()
                                .collect();

                            let mut validated_hist1d = parsed_hist1d;
                            validated_hist1d.cuts = Cuts::new(valid_hist_cuts);
                            validated_hist1d.cuts.parse_conditions();
                            valid_configs.push(Config::Hist1D(validated_hist1d));
                        } else {
                            log::warn!(
                                "Invalid 1D histogram '{}': Missing column '{}'",
                                parsed_hist1d.name,
                                parsed_hist1d.column_name
                            );
                        }
                    }
                }
                Config::Hist2D(hist2d) => {
                    let mut parsed_hist2d = hist2d.clone();
                    parsed_hist2d.cuts.parse_conditions();

                    if hist2d.calculate {
                        if column_names.contains(&parsed_hist2d.x_column_name)
                            && column_names.contains(&parsed_hist2d.y_column_name)
                        {
                            // Validate cuts for the histogram
                            let valid_hist_cuts: Vec<Cut> = parsed_hist2d
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
                                                log::warn!(
                                                    "Invalid cut '{}' for 2D histogram '{}': Missing column '{}'",
                                                    cut.name(),
                                                    parsed_hist2d.name,
                                                    column
                                                );
                                            }
                                        }
                                    }
                                    has_columns && is_active
                                })
                                .cloned()
                                .collect();

                            let mut validated_hist2d = parsed_hist2d;
                            validated_hist2d.cuts.cuts = valid_hist_cuts;
                            validated_hist2d.cuts.parse_conditions();
                            valid_configs.push(Config::Hist2D(validated_hist2d));
                        } else {
                            log::warn!(
                                "Invalid 2D histogram '{}': Missing column(s) '{}', '{}'",
                                parsed_hist2d.name,
                                parsed_hist2d.x_column_name,
                                parsed_hist2d.y_column_name
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
                        log::warn!(
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
            column_groups: self.column_groups.clone(),
            cuts: valid_cuts,
            column_ui_state: self.column_ui_state.clone(),
            selected_column_index: self.selected_column_index,
            expanded_column_ui_cache: ExpandedColumnUiCache::default(),
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
                    used_column_names.extend(hist1d.fill_columns());
                    used_column_names.extend(hist1d.cuts.required_columns());
                }
                Config::Hist2D(hist2d) => {
                    used_column_names.extend(hist2d.x_fill_columns());
                    used_column_names.extend(hist2d.y_fill_columns());
                    used_column_names.extend(hist2d.cuts.required_columns());
                }
            }
        }

        // Remove duplicates
        used_column_names.sort();
        used_column_names.dedup();

        used_column_names
    }

    fn expand(&self, available_columns: &[String]) -> Self {
        let source_group_resolver = self.build_source_group_resolver(available_columns);
        let mut expanded_configs: Vec<Config> = Vec::new();

        for config in &self.configs {
            match config {
                Config::Hist1D(config) => {
                    let expanded_1d = config.expand(&source_group_resolver);
                    for expanded_config in expanded_1d {
                        expanded_configs.push(Config::Hist1D(expanded_config));
                    }
                }
                Config::Hist2D(config) => {
                    let expanded_2d = config.expand(&source_group_resolver);
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
            column_groups: self.column_groups.clone(),
            cuts: self.cuts.clone(),
            column_ui_state: self.column_ui_state.clone(),
            selected_column_index: self.selected_column_index,
            expanded_column_ui_cache: ExpandedColumnUiCache::default(),
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

    fn ui_expansion_cache_key(&self, base_columns: &[String]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        base_columns.hash(&mut hasher);
        self.columns.hash(&mut hasher);
        self.column_groups.hash(&mut hasher);
        for (name, value) in &self.variables {
            name.hash(&mut hasher);
            value.to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }

    fn ensure_expanded_column_ui_cache(&mut self, base_columns: &[String]) {
        let key = self.ui_expansion_cache_key(base_columns);
        if self.expanded_column_ui_cache.key == key {
            return;
        }

        let (expanded_alias_prefix_lengths, all_expanded_aliases) =
            collect_expanded_computed_column_aliases(
                base_columns,
                &self.columns,
                &self.column_groups,
                &self.variables,
            );

        let mut available_columns = base_columns.to_vec();
        available_columns.extend(all_expanded_aliases.iter().cloned());
        available_columns.sort();
        available_columns.dedup();

        let mut available_source_names = available_columns.clone();
        available_source_names
            .extend(self.available_column_group_aliases_for_available_columns(&available_columns));
        available_source_names.sort();
        available_source_names.dedup();

        self.expanded_column_ui_cache = ExpandedColumnUiCache {
            key,
            expanded_alias_prefix_lengths,
            all_expanded_aliases,
            available_columns,
            available_source_names,
        };
    }

    fn available_columns_for_ui(&mut self, base_columns: &[String]) -> Vec<String> {
        self.ensure_expanded_column_ui_cache(base_columns);
        self.expanded_column_ui_cache.available_columns.clone()
    }

    fn available_histogram_source_names_for_ui(&mut self, base_columns: &[String]) -> Vec<String> {
        self.ensure_expanded_column_ui_cache(base_columns);
        self.expanded_column_ui_cache.available_source_names.clone()
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
        &mut self,
        base_columns: &[String],
        row_index: usize,
    ) -> Vec<String> {
        self.ensure_expanded_column_ui_cache(base_columns);
        let mut available_columns = base_columns.to_vec();
        let expanded_alias_count = self
            .expanded_column_ui_cache
            .expanded_alias_prefix_lengths
            .get(row_index)
            .copied()
            .unwrap_or(self.expanded_column_ui_cache.all_expanded_aliases.len());
        available_columns.extend(
            self.expanded_column_ui_cache
                .all_expanded_aliases
                .iter()
                .take(expanded_alias_count)
                .cloned(),
        );
        available_columns
            .extend(self.available_column_group_aliases_for_available_columns(&available_columns));
        available_columns.sort();
        available_columns.dedup();
        available_columns
    }

    fn grouped_computed_column_preview_for_row(
        &mut self,
        base_columns: &[String],
        row_index: usize,
    ) -> Vec<String> {
        let available_columns = self.available_columns_for_builder_row(base_columns, row_index);
        let Some((expression, alias)) = self.columns.get(row_index) else {
            return Vec::new();
        };

        expand_grouped_computed_columns_for_available_columns(
            expression,
            alias,
            &available_columns,
            &self.column_groups,
            &self.variables,
        )
        .into_iter()
        .map(|(_, expanded_alias)| expanded_alias)
        .collect()
    }

    fn available_column_group_aliases_for_available_columns(
        &self,
        available_columns: &[String],
    ) -> Vec<String> {
        let available_column_names = available_columns
            .iter()
            .map(String::as_str)
            .collect::<std::collections::HashSet<_>>();
        let mut seen_aliases = std::collections::HashSet::new();
        let mut aliases = Vec::new();

        for group in &self.column_groups {
            let alias = group.alias.trim();
            if alias.is_empty()
                || !is_valid_identifier_name(alias)
                || available_column_names.contains(alias)
                || !seen_aliases.insert(alias.to_owned())
                || group.normalized_column_names().is_empty()
            {
                continue;
            }

            aliases.push(alias.to_owned());
        }

        aliases
    }

    fn build_source_group_resolver(&self, available_columns: &[String]) -> SourceGroupResolver {
        build_source_group_resolver_for_columns(&self.column_groups, available_columns)
    }

    pub fn variable_ui(&mut self, ui: &mut egui::Ui, base_columns: &[String]) {
        ui.horizontal(|ui| {
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
        let mut pending_move = None;

        TableBuilder::new(ui)
            .id_salt("named_variables")
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
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
                    ui.label("Value");
                });
            })
            .body(|mut body| {
                for (index, (name, value)) in self.variables.iter_mut().enumerate() {
                    body.row(28.0, |mut row| {
                        row.col(|ui| {
                            let response = ui
                                .add(
                                    egui::Button::new(format!("↕ {index}"))
                                        .sense(egui::Sense::click_and_drag()),
                                )
                                .on_hover_text("Drag to reorder this variable");

                            response.dnd_set_drag_payload(VariableRowDragPayload(index));

                            if let Some(dragged_payload) =
                                response.dnd_release_payload::<VariableRowDragPayload>()
                            {
                                let insert_index = ui.pointer_interact_pos().map_or(index, |pos| {
                                    if pos.y >= response.rect.center().y {
                                        index + 1
                                    } else {
                                        index
                                    }
                                });
                                pending_move = Some((dragged_payload.0, insert_index));
                            }
                        });

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
                                    egui::Color32::RED,
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

        if !indices_to_remove.is_empty() {
            for &index in indices_to_remove.iter().rev() {
                self.variables.remove(index);
            }
        } else if let Some((from, to)) = pending_move {
            let _reordered_index: Option<usize> = reorder_vec_item(&mut self.variables, from, to);
        }
    }

    pub fn column_group_ui(&mut self, ui: &mut egui::Ui, base_columns: &[String]) {
        let available_columns = self.available_columns_for_ui(base_columns);
        let existing_columns = available_columns
            .iter()
            .map(String::as_str)
            .collect::<std::collections::HashSet<_>>();
        let duplicate_alias_counts = self
            .column_groups
            .iter()
            .map(|group| group.alias.trim())
            .filter(|alias| !alias.is_empty())
            .fold(
                std::collections::HashMap::<String, usize>::new(),
                |mut counts, alias| {
                    *counts.entry(alias.to_owned()).or_insert(0) += 1;
                    counts
                },
            );

        ui.horizontal(|ui| {
            if ui.button("+").clicked() {
                self.column_groups.push(ColumnGroup::default());
            }

            ui.separator();

            if ui.button("Remove All").clicked() {
                self.column_groups.clear();
            }
        });

        if self.column_groups.is_empty() {
            return;
        }

        let mut indices_to_remove = Vec::new();
        let mut pending_move = None;

        TableBuilder::new(ui)
            .id_salt("column_groups")
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .column(Column::auto())
            .striped(true)
            .vscroll(false)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(" ↕ ");
                });
                header.col(|ui| {
                    ui.label("Alias");
                });
                header.col(|ui| {
                    ui.label("Columns");
                });
                header.col(|ui| {
                    ui.label("");
                });
            })
            .body(|mut body| {
                for (index, group) in self.column_groups.iter_mut().enumerate() {
                    body.row(36.0, |mut row| {
                        row.col(|ui| {
                            let response = ui
                                .add(
                                    egui::Button::new(format!("↕ {index}"))
                                        .sense(egui::Sense::click_and_drag()),
                                )
                                .on_hover_text("Drag to reorder this column group");

                            response.dnd_set_drag_payload(ColumnGroupRowDragPayload(index));

                            if let Some(dragged_payload) =
                                response.dnd_release_payload::<ColumnGroupRowDragPayload>()
                            {
                                let insert_index = ui.pointer_interact_pos().map_or(index, |pos| {
                                    if pos.y >= response.rect.center().y {
                                        index + 1
                                    } else {
                                        index
                                    }
                                });
                                pending_move = Some((dragged_payload.0, insert_index));
                            }
                        });

                        row.col(|ui| {
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut group.alias)
                                    .hint_text("Group Alias")
                                    .clip_text(false),
                            );
                            let changed = response.changed();
                            response.on_hover_text(
                                "Group aliases may only contain letters, numbers, and underscores.",
                            );
                            if changed {
                                group.alias = sanitize_identifier_name(&group.alias);
                            }

                            let trimmed = group.alias.trim();
                            if !trimmed.is_empty() {
                                if existing_columns.contains(trimmed) {
                                    ui.colored_label(
                                        egui::Color32::RED,
                                        "Conflicts with a column name.",
                                    );
                                } else if duplicate_alias_counts
                                    .get(trimmed)
                                    .copied()
                                    .unwrap_or_default()
                                    > 1
                                {
                                    ui.colored_label(egui::Color32::RED, "Duplicate group alias.");
                                }
                            }
                        });

                        row.col(|ui| {
                            let mut selected_columns = group.normalized_column_names();
                            if searchable_multi_column_picker_ui(
                                ui,
                                format!("column_group_picker_{index}"),
                                &mut selected_columns,
                                &available_columns,
                                "Columns",
                                true,
                            ) {
                                group.set_column_names(selected_columns);
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

        if !indices_to_remove.is_empty() {
            for &index in indices_to_remove.iter().rev() {
                self.column_groups.remove(index);
            }
        } else if let Some((from, to)) = pending_move {
            let _reordered_index: Option<usize> =
                reorder_vec_item(&mut self.column_groups, from, to);
        }
    }

    pub fn config_ui(
        &mut self,
        ui: &mut egui::Ui,
        available_cuts: &mut Cuts,
        available_source_names: &[String],
    ) {
        ui.horizontal(|ui| {
            ui.label("Histograms");

            if ui.button("+1D").clicked() {
                self.configs.push(Config::Hist1D(Hist1DConfig {
                    name: String::new(),
                    column_name: String::new(),
                    additional_column_names: Vec::new(),
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
                    additional_x_column_names: Vec::new(),
                    additional_y_column_names: Vec::new(),
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
                                    available_source_names,
                                    index,
                                );
                            }
                            Config::Hist2D(config) => {
                                config.table_row(
                                    &mut row,
                                    available_cuts,
                                    hist2d_lock,
                                    available_source_names,
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
            let mut pending_move = None;

            TableBuilder::new(ui)
                .id_salt("new_columns")
                .column(Column::auto()) // reorder
                .column(Column::auto()) // alias
                .column(Column::remainder()) // expression
                .column(Column::auto()) // actions
                .striped(true)
                .vscroll(false)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label(" ↕ ");
                    });
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
                                let response = ui
                                    .add(
                                        egui::Button::new(format!("↕ {index}"))
                                            .sense(egui::Sense::click_and_drag()),
                                    )
                                    .on_hover_text("Drag to reorder this computed column");

                                response.dnd_set_drag_payload(ComputedColumnRowDragPayload(index));

                                if let Some(dragged_payload) =
                                    response.dnd_release_payload::<ComputedColumnRowDragPayload>()
                                {
                                    let insert_index =
                                        ui.pointer_interact_pos().map_or(index, |pos| {
                                            if pos.y >= response.rect.center().y {
                                                index + 1
                                            } else {
                                                index
                                            }
                                        });
                                    pending_move = Some((dragged_payload.0, insert_index));
                                }
                            });

                            row.col(|ui| {
                                let response = ui.add(
                                    egui::TextEdit::singleline(alias)
                                        .hint_text("Alias")
                                        .clip_text(false),
                                );
                                let changed = response.changed();
                                response.on_hover_text(
                                    "Aliases may only contain letters, numbers, and underscores. Grouped column creation also supports {} to place the grouped source column names in the generated name.",
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

            if indices_to_remove_column.is_empty()
                && let Some((from, to)) = pending_move
                && let Some(target) = reorder_vec_item(&mut self.columns, from, to)
            {
                let state = self.column_ui_state.remove(from);
                self.column_ui_state.insert(target, state);
                self.selected_column_index =
                    reordered_selected_index(self.selected_column_index, from, target);
            }

            if let Some(selected_index) = self.selected_column_index
                && selected_index < self.columns.len()
                && selected_index < self.column_ui_state.len()
            {
                let grouped_column_preview =
                    self.grouped_computed_column_preview_for_row(base_columns, selected_index);
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
                    if grouped_column_preview.len() > 1 {
                        let preview_limit = 6usize;
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "This grouped expression will create {} columns.",
                                grouped_column_preview.len()
                            ))
                            .strong(),
                        );
                        ui.label(
                            egui::RichText::new(
                                "Change the output name template below if you want different generated names. Use {} to insert the grouped source column names directly.",
                            )
                            .weak()
                            .small(),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for output_name in grouped_column_preview.iter().take(preview_limit) {
                                ui.monospace(output_name);
                            }
                            if grouped_column_preview.len() > preview_limit {
                                ui.label(format!(
                                    "… and {} more",
                                    grouped_column_preview.len() - preview_limit
                                ));
                            }
                        });
                    }
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
        let Some((expression, alias)) = self.columns.get_mut(index) else {
            return;
        };
        let Some(state) = self.column_ui_state.get_mut(index) else {
            return;
        };

        state.builder.ensure_terms();

        let mut changed = false;
        let mut term_indices_to_remove = Vec::new();
        let total_terms = state.builder.terms.len();

        ui.horizontal(|ui| {
            ui.label("Output Name");
            let response = ui.add(
                egui::TextEdit::singleline(alias)
                    .hint_text("Alias")
                    .clip_text(false),
            );
            let alias_changed = response.changed();
            response.on_hover_text(
                "Use letters, numbers, and underscores. For grouped columns, you can also use {} to place the grouped source column names in the generated names.",
            );
            if alias_changed {
                *alias = sanitize_computed_column_alias(alias);
                ui.ctx().request_repaint();
            }
        });
        ui.label(
            egui::RichText::new(
                "Grouped expressions can use {} in the output name. If you leave {} out, Spectrix appends the grouped source column names automatically.",
            )
            .weak()
            .small(),
        );
        ui.separator();
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
                egui::Color32::RED,
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
        self.cuts.ui(
            ui,
            active_cuts.as_deref_mut(),
            available_columns,
            "general",
            false,
        );
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
        let available_source_names = self.available_histogram_source_names_for_ui(base_columns);

        egui::CollapsingHeader::new("Variables")
            .id_salt("general_variables_section")
            .default_open(true)
            .show(ui, |ui| {
                self.variable_ui(ui, base_columns);
            });

        ui.separator();

        egui::CollapsingHeader::new("Column Creation")
            .id_salt("general_column_creation_section")
            .default_open(true)
            .show(ui, |ui| {
                self.column_ui(ui, base_columns);
            });

        ui.separator();

        egui::CollapsingHeader::new("Column Groups")
            .id_salt("general_column_groups_section")
            .default_open(false)
            .show(ui, |ui| {
                self.column_group_ui(ui, base_columns);
            });

        ui.separator();

        egui::CollapsingHeader::new("Cuts")
            .id_salt("general_cuts_section")
            .default_open(true)
            .show(ui, |ui| {
                self.cut_ui(ui, active_cuts, &available_columns);
            });

        ui.separator();

        self.config_ui(ui, &mut merged_cuts, &available_source_names);
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

impl ColumnGroup {
    fn normalized_column_names(&self) -> Vec<String> {
        let mut normalized_columns = Vec::new();
        let mut seen_columns = std::collections::HashSet::new();

        for column_name in &self.column_names {
            let trimmed = column_name.trim();
            if trimmed.is_empty() {
                continue;
            }

            let normalized = trimmed.to_owned();
            if seen_columns.insert(normalized.clone()) {
                normalized_columns.push(normalized);
            }
        }

        normalized_columns
    }

    fn set_column_names(&mut self, column_names: Vec<String>) {
        self.column_names = column_names
            .into_iter()
            .map(|column_name| column_name.trim().to_owned())
            .filter(|column_name| !column_name.is_empty())
            .fold(
                (Vec::new(), std::collections::HashSet::<String>::new()),
                |(mut normalized_columns, mut seen_columns), column_name| {
                    if seen_columns.insert(column_name.clone()) {
                        normalized_columns.push(column_name);
                    }
                    (normalized_columns, seen_columns)
                },
            )
            .0;
    }
}

impl SourceGroupResolver {
    fn resolve_selection(&self, selection: &str) -> Vec<ResolvedSourceSelection> {
        let trimmed = selection.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        if let Some(group_members) = self.groups.get(trimmed) {
            return group_members.clone();
        }

        expand_pattern_source_selection(trimmed)
    }
}

fn build_source_group_resolver_for_columns(
    column_groups: &[ColumnGroup],
    available_columns: &[String],
) -> SourceGroupResolver {
    let available_column_names = available_columns
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    let mut resolver = SourceGroupResolver::default();
    let mut seen_aliases = std::collections::HashSet::new();

    for group in column_groups {
        let alias = group.alias.trim();
        if alias.is_empty() {
            continue;
        }

        if !is_valid_identifier_name(alias) {
            log::warn!("Skipping invalid column group alias '{alias}'.");
            continue;
        }

        if available_column_names.contains(alias) {
            log::warn!(
                "Skipping column group alias '{alias}' because it conflicts with an existing column name."
            );
            continue;
        }

        if !seen_aliases.insert(alias.to_owned()) {
            log::warn!("Skipping duplicate column group alias '{alias}'.");
            continue;
        }

        let mut resolved_columns = Vec::new();
        let mut seen_columns = std::collections::HashSet::new();

        for member in group.normalized_column_names() {
            for resolved_member in expand_pattern_source_selection(member.as_str()) {
                if !available_column_names.contains(resolved_member.column_name.as_str()) {
                    log::warn!(
                        "Skipping missing column '{}' in column group '{}'.",
                        resolved_member.column_name,
                        alias
                    );
                    continue;
                }

                if seen_columns.insert(resolved_member.column_name.clone()) {
                    resolved_columns.push(resolved_member.column_name);
                }
            }
        }

        if resolved_columns.is_empty() {
            log::warn!(
                "Skipping column group alias '{alias}' because it does not resolve to any available columns."
            );
            continue;
        }

        let name_tokens = derive_group_name_tokens(&resolved_columns);
        let resolved_group = resolved_columns
            .into_iter()
            .zip(name_tokens)
            .map(|(column_name, name_token)| ResolvedSourceSelection {
                column_name,
                name_token,
            })
            .collect::<Vec<_>>();

        resolver.groups.insert(alias.to_owned(), resolved_group);
    }

    resolver
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Hist1DConfig {
    pub name: String,        // Histogram display name
    pub column_name: String, // Primary data column to fill from
    #[serde(default)]
    pub additional_column_names: Vec<String>, // Additional columns to also fill into this histogram
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
            additional_column_names: Vec::new(),
            range,
            bins,
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }
    }

    fn fill_columns(&self) -> Vec<String> {
        let mut fill_columns = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for column_name in
            std::iter::once(&self.column_name).chain(self.additional_column_names.iter())
        {
            let trimmed = column_name.trim();
            if trimmed.is_empty() {
                continue;
            }

            let normalized = trimmed.to_owned();
            if seen.insert(normalized.clone()) {
                fill_columns.push(normalized);
            }
        }

        fill_columns
    }

    fn set_fill_columns(&mut self, columns: Vec<String>) {
        let mut fill_columns = columns
            .into_iter()
            .map(|column_name| column_name.trim().to_owned())
            .filter(|column_name| !column_name.is_empty())
            .collect::<Vec<_>>();

        if fill_columns.is_empty() {
            self.column_name.clear();
            self.additional_column_names.clear();
            return;
        }

        self.column_name = fill_columns.remove(0);
        self.additional_column_names = fill_columns;
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
            let mut fill_columns = self.fill_columns();
            if searchable_multi_column_picker_ui(
                ui,
                format!("hist1d_column_picker_{row_index}"),
                &mut fill_columns,
                available_columns,
                "Column Names",
                self.enabled,
            ) {
                self.set_fill_columns(fill_columns);
            }
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

    fn expand(&self, source_group_resolver: &SourceGroupResolver) -> Vec<Self> {
        let mut configs = Vec::new();

        if self.calculate {
            let mut seen_columns = std::collections::HashSet::new();

            for resolved_source in self
                .fill_columns()
                .into_iter()
                .flat_map(|fill_column| source_group_resolver.resolve_selection(&fill_column))
            {
                if !seen_columns.insert(resolved_source.column_name.clone()) {
                    continue;
                }

                if self.name.contains("{}") {
                    if let Some(name_token) = resolved_source.name_token.as_deref() {
                        let mut new_config = self.clone();
                        new_config.name = self.name.replace("{}", name_token);
                        new_config.column_name = resolved_source.column_name;
                        new_config.additional_column_names.clear();
                        configs.push(new_config);
                    } else {
                        log::error!(
                            "Warning: Unsupported pattern for 1D histogram with name '{}', column '{}'",
                            self.name,
                            resolved_source.column_name
                        );
                    }
                } else {
                    let mut new_config = self.clone();
                    new_config.column_name = resolved_source.column_name;
                    new_config.additional_column_names.clear();
                    configs.push(new_config);
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
    #[serde(default)]
    pub additional_x_column_names: Vec<String>,
    #[serde(default)]
    pub additional_y_column_names: Vec<String>,
    pub x_range: (f64, f64),  // Range for X-axis
    pub y_range: (f64, f64),  // Range for Y-axis
    pub bins: (usize, usize), // Number of bins for X and Y axes
    pub cuts: Cuts,           // Cuts for the histogram
    pub calculate: bool,      // Whether to calculate the histogram
    pub enabled: bool,        // Whether to let the user interact with the histogram
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
            additional_x_column_names: Vec::new(),
            additional_y_column_names: Vec::new(),
            x_range,
            y_range,
            bins,
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }
    }

    fn normalized_axis_columns(
        primary_column_name: &str,
        additional_column_names: &[String],
    ) -> Vec<String> {
        let mut columns = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for column_name in std::iter::once(primary_column_name)
            .chain(additional_column_names.iter().map(String::as_str))
        {
            let trimmed = column_name.trim();
            if trimmed.is_empty() {
                continue;
            }

            let normalized = trimmed.to_owned();
            if seen.insert(normalized.clone()) {
                columns.push(normalized);
            }
        }

        columns
    }

    fn x_fill_columns(&self) -> Vec<String> {
        Self::normalized_axis_columns(&self.x_column_name, &self.additional_x_column_names)
    }

    fn y_fill_columns(&self) -> Vec<String> {
        Self::normalized_axis_columns(&self.y_column_name, &self.additional_y_column_names)
    }

    fn set_x_fill_columns(&mut self, columns: Vec<String>) {
        let mut columns = columns
            .into_iter()
            .map(|column_name| column_name.trim().to_owned())
            .filter(|column_name| !column_name.is_empty())
            .collect::<Vec<_>>();

        if columns.is_empty() {
            self.x_column_name.clear();
            self.additional_x_column_names.clear();
            return;
        }

        self.x_column_name = columns.remove(0);
        self.additional_x_column_names = columns;
    }

    fn set_y_fill_columns(&mut self, columns: Vec<String>) {
        let mut columns = columns
            .into_iter()
            .map(|column_name| column_name.trim().to_owned())
            .filter(|column_name| !column_name.is_empty())
            .collect::<Vec<_>>();

        if columns.is_empty() {
            self.y_column_name.clear();
            self.additional_y_column_names.clear();
            return;
        }

        self.y_column_name = columns.remove(0);
        self.additional_y_column_names = columns;
    }

    fn fill_pairs_for_merge(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        let mut seen_pairs = std::collections::HashSet::new();
        let mut seen_unordered_pairs = std::collections::HashSet::new();

        for x_column_name in self.x_fill_columns() {
            for y_column_name in self.y_fill_columns() {
                if x_column_name == y_column_name {
                    continue;
                }

                let pair = (x_column_name.clone(), y_column_name.clone());
                if !seen_pairs.insert(pair.clone()) {
                    continue;
                }

                let unordered_pair = if x_column_name <= y_column_name {
                    (x_column_name.clone(), y_column_name.clone())
                } else {
                    (y_column_name.clone(), x_column_name.clone())
                };

                if !seen_unordered_pairs.insert(unordered_pair) {
                    continue;
                }

                pairs.push(pair);
            }
        }

        pairs
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
                ui.horizontal(|ui| {
                    ui.label("X");
                    let mut x_fill_columns = self.x_fill_columns();
                    if searchable_multi_column_picker_ui(
                        ui,
                        format!("hist2d_x_column_picker_{row_index}"),
                        &mut x_fill_columns,
                        available_columns,
                        "X Column Names",
                        self.enabled,
                    ) {
                        self.set_x_fill_columns(x_fill_columns);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Y");
                    let mut y_fill_columns = self.y_fill_columns();
                    if searchable_multi_column_picker_ui(
                        ui,
                        format!("hist2d_y_column_picker_{row_index}"),
                        &mut y_fill_columns,
                        available_columns,
                        "Y Column Names",
                        self.enabled,
                    ) {
                        self.set_y_fill_columns(y_fill_columns);
                    }
                });
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

    fn expand(&self, source_group_resolver: &SourceGroupResolver) -> Vec<Self> {
        let mut configs = Vec::new();

        if self.calculate {
            let expanded_x_columns = self
                .x_fill_columns()
                .into_iter()
                .flat_map(|column_name| source_group_resolver.resolve_selection(&column_name))
                .collect::<Vec<_>>();
            let expanded_y_columns = self
                .y_fill_columns()
                .into_iter()
                .flat_map(|column_name| source_group_resolver.resolve_selection(&column_name))
                .collect::<Vec<_>>();
            let mut seen_pairs = std::collections::HashSet::new();
            let mut seen_unordered_pairs = std::collections::HashSet::new();

            for x_source in &expanded_x_columns {
                for y_source in &expanded_y_columns {
                    if x_source.column_name == y_source.column_name {
                        continue;
                    }

                    let pair = (x_source.column_name.clone(), y_source.column_name.clone());
                    if !seen_pairs.insert(pair.clone()) {
                        continue;
                    }

                    let unordered_pair = if x_source.column_name <= y_source.column_name {
                        (x_source.column_name.clone(), y_source.column_name.clone())
                    } else {
                        (y_source.column_name.clone(), x_source.column_name.clone())
                    };

                    if !seen_unordered_pairs.insert(unordered_pair) {
                        continue;
                    }

                    let mut new_config = self.clone();
                    new_config.x_column_name = x_source.column_name.clone();
                    new_config.y_column_name = y_source.column_name.clone();
                    new_config.additional_x_column_names.clear();
                    new_config.additional_y_column_names.clear();

                    if self.name.contains("{}") {
                        match (
                            x_source.name_token.as_deref(),
                            y_source.name_token.as_deref(),
                        ) {
                            (Some(x_token), None) => {
                                new_config.name = self.name.replace("{}", x_token);
                            }
                            (None, Some(y_token)) => {
                                new_config.name = self.name.replace("{}", y_token);
                            }
                            (Some(x_token), Some(y_token)) => {
                                new_config.name =
                                    self.name.replace("{}", &format!("{x_token}_{y_token}"));
                            }
                            (None, None) => {
                                log::error!(
                                    "Warning: Unsupported pattern for 2D histogram with name '{}', x_column '{}', y_column '{}'",
                                    self.name,
                                    self.x_column_name,
                                    self.y_column_name
                                );
                                continue;
                            }
                        }
                    }

                    configs.push(new_config);
                }
            }
        }
        configs
    }
}

use polars::prelude::*;
use regex::Regex;

fn expand_pattern_source_selection(selection: &str) -> Vec<ResolvedSourceSelection> {
    let trimmed = selection.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let range_re = regex::Regex::new(r"\{(\d+)-(\d+)\}").expect("Failed to create range regex");
    let list_re = regex::Regex::new(r"\{([\d,]+)\}").expect("Failed to create list regex");

    if let Some(caps) = range_re.captures(trimmed) {
        let start: usize = caps[1].parse().expect("Failed to parse start range");
        let end: usize = caps[2].parse().expect("Failed to parse end range");

        return (start..=end)
            .map(|index| ResolvedSourceSelection {
                column_name: range_re.replace(trimmed, index.to_string()).to_string(),
                name_token: Some(index.to_string()),
            })
            .collect();
    }

    if let Some(caps) = list_re.captures(trimmed) {
        return caps[1]
            .split(',')
            .map(|value| ResolvedSourceSelection {
                column_name: list_re.replace(trimmed, value).to_string(),
                name_token: Some(value.to_owned()),
            })
            .collect();
    }

    vec![ResolvedSourceSelection {
        column_name: trimmed.to_owned(),
        name_token: None,
    }]
}

fn longest_common_prefix(values: &[String]) -> String {
    let Some(first) = values.first() else {
        return String::new();
    };

    let first_chars = first.chars().collect::<Vec<_>>();
    let mut prefix_len = first_chars.len();

    for value in values.iter().skip(1) {
        let shared = first_chars
            .iter()
            .zip(value.chars())
            .take_while(|(left, right)| left == &right)
            .count();
        prefix_len = prefix_len.min(shared);
    }

    first_chars.into_iter().take(prefix_len).collect()
}

fn longest_common_suffix(values: &[String], prefix: &str) -> String {
    let Some(first) = values.first() else {
        return String::new();
    };

    let prefix_chars = prefix.chars().count();
    let max_suffix_len = values
        .iter()
        .map(|value| value.chars().count().saturating_sub(prefix_chars))
        .min()
        .unwrap_or(0);

    let first_reversed = first.chars().rev().collect::<Vec<_>>();
    let mut suffix_len = max_suffix_len.min(first_reversed.len());

    for value in values.iter().skip(1) {
        let shared = first_reversed
            .iter()
            .zip(value.chars().rev())
            .take_while(|(left, right)| left == &right)
            .count();
        suffix_len = suffix_len.min(shared).min(max_suffix_len);
    }

    first_reversed
        .into_iter()
        .take(suffix_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn extract_middle_token(value: &str, prefix: &str, suffix: &str) -> Option<String> {
    let total_chars = value.chars().count();
    let prefix_chars = prefix.chars().count();
    let suffix_chars = suffix.chars().count();

    if prefix_chars + suffix_chars > total_chars {
        return None;
    }

    let middle = value
        .chars()
        .skip(prefix_chars)
        .take(total_chars.saturating_sub(prefix_chars + suffix_chars))
        .collect::<String>();

    if middle.is_empty() {
        None
    } else {
        Some(middle)
    }
}

fn derive_group_name_tokens(column_names: &[String]) -> Vec<Option<String>> {
    if column_names.is_empty() {
        return Vec::new();
    }

    let prefix = longest_common_prefix(column_names);
    let suffix = longest_common_suffix(column_names, &prefix);
    let mut tokens = column_names
        .iter()
        .map(|column_name| extract_middle_token(column_name, &prefix, &suffix))
        .collect::<Vec<_>>();

    let unique_tokens = tokens
        .iter()
        .all(|token| token.as_ref().is_some_and(|value| !value.is_empty()))
        && tokens
            .iter()
            .flatten()
            .cloned()
            .collect::<std::collections::HashSet<_>>()
            .len()
            == tokens.len();

    if unique_tokens {
        return tokens;
    }

    tokens.clear();
    tokens.extend(
        column_names
            .iter()
            .map(|column_name| Some(column_name.clone())),
    );
    tokens
}

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
    let trimmed = alias.trim();
    !trimmed.is_empty()
        && (is_valid_identifier_name(trimmed)
            || is_valid_identifier_name(&trimmed.replace("{}", "GroupToken")))
}

fn sanitize_computed_column_alias(alias: &str) -> String {
    let trimmed = alias.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut sanitized = String::with_capacity(trimmed.len());
    let mut previous_was_underscore = false;
    let mut characters = trimmed.chars().peekable();

    while let Some(character) = characters.next() {
        if character == '{' && characters.peek().is_some_and(|next| *next == '}') {
            characters.next();
            sanitized.push_str("{}");
            previous_was_underscore = false;
            continue;
        }

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

fn computed_column_dependencies(expression: &str, variables: &[(String, f64)]) -> Vec<String> {
    let number_pattern = r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?";
    let token_re = regex::Regex::new(&format!(r"{number_pattern}|[A-Za-z_]\w*|\*\*|[+*/()-]"))
        .expect("failed to create computed-column token regex");
    let identifier_re =
        regex::Regex::new(r"^[A-Za-z_]\w*$").expect("failed to create identifier regex");
    let variable_names = variables
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut seen_dependencies = std::collections::HashSet::new();

    token_re
        .find_iter(expression)
        .map(|token| token.as_str())
        .filter(|token| identifier_re.is_match(token))
        .filter(|token| {
            !variable_names.contains(token)
                && !matches!(token.to_ascii_lowercase().as_str(), "inf" | "nan")
        })
        .filter(|token| seen_dependencies.insert((*token).to_owned()))
        .map(ToOwned::to_owned)
        .collect()
}

fn replace_group_alias_in_expression(expression: &str, alias: &str, column_name: &str) -> String {
    let pattern = regex::Regex::new(&format!(r"\b{}\b", regex::escape(alias)))
        .expect("failed to create grouped computed-column replacement regex");
    pattern.replace_all(expression, column_name).to_string()
}

fn sanitize_identifier_fragment(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    let mut previous_was_underscore = false;

    for character in name.chars() {
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

    sanitized.trim_matches('_').to_owned()
}

fn expanded_computed_column_alias(
    alias: &str,
    grouped_column_names: &[String],
    expansion_count: usize,
) -> String {
    let trimmed_alias = alias.trim();
    let sanitized_token = grouped_column_names
        .iter()
        .map(|column_name| sanitize_identifier_fragment(column_name))
        .filter(|fragment| !fragment.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    let expanded_alias = if trimmed_alias.contains("{}") {
        trimmed_alias.replace("{}", &sanitized_token)
    } else if expansion_count > 1 && !sanitized_token.is_empty() {
        format!("{trimmed_alias}_{sanitized_token}")
    } else {
        trimmed_alias.to_owned()
    };

    sanitize_identifier_name(&expanded_alias)
}

fn expand_grouped_computed_columns(
    expression: &str,
    alias: &str,
    variables: &[(String, f64)],
    source_group_resolver: &SourceGroupResolver,
) -> Vec<(String, String)> {
    let grouped_dependencies = computed_column_dependencies(expression, variables)
        .into_iter()
        .filter_map(|dependency| {
            source_group_resolver
                .groups
                .get(&dependency)
                .cloned()
                .map(|members| (dependency, members))
        })
        .collect::<Vec<_>>();

    if grouped_dependencies.is_empty() {
        return vec![(expression.to_owned(), alias.to_owned())];
    }

    let mut expanded_variants = vec![(expression.to_owned(), Vec::<String>::new())];

    for (group_alias, members) in &grouped_dependencies {
        if members.is_empty() {
            return Vec::new();
        }

        let mut next_variants = Vec::new();
        for (current_expression, current_grouped_column_names) in expanded_variants {
            for member in members {
                let mut grouped_column_names = current_grouped_column_names.clone();
                grouped_column_names.push(member.column_name.clone());
                next_variants.push((
                    replace_group_alias_in_expression(
                        &current_expression,
                        group_alias,
                        &member.column_name,
                    ),
                    grouped_column_names,
                ));
            }
        }

        expanded_variants = next_variants;
    }

    let expansion_count = expanded_variants.len();
    expanded_variants
        .iter()
        .enumerate()
        .map(|(index, (expanded_expression, grouped_column_names))| {
            let expanded_alias =
                expanded_computed_column_alias(alias, grouped_column_names, expansion_count);
            let expanded_alias = if expanded_alias.trim().is_empty() {
                format!("{alias}_{index}")
            } else {
                expanded_alias
            };

            (expanded_expression.clone(), expanded_alias)
        })
        .collect()
}

fn expand_grouped_computed_columns_for_available_columns(
    expression: &str,
    alias: &str,
    available_columns: &[String],
    column_groups: &[ColumnGroup],
    variables: &[(String, f64)],
) -> Vec<(String, String)> {
    let available_column_names = available_columns
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    let source_group_resolver =
        build_source_group_resolver_for_columns(column_groups, available_columns);

    expand_grouped_computed_columns(expression, alias, variables, &source_group_resolver)
        .into_iter()
        .filter(|(expanded_expression, expanded_alias)| {
            !expanded_alias.trim().is_empty()
                && computed_column_dependencies(expanded_expression, variables)
                    .iter()
                    .all(|dependency| available_column_names.contains(dependency.as_str()))
        })
        .collect()
}

fn collect_expanded_computed_column_aliases(
    base_columns: &[String],
    computed_columns: &[(String, String)],
    column_groups: &[ColumnGroup],
    variables: &[(String, f64)],
) -> (Vec<usize>, Vec<String>) {
    let mut current_columns = base_columns.to_vec();
    let mut current_column_names = current_columns
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let mut expanded_alias_prefix_lengths = Vec::with_capacity(computed_columns.len());
    let mut all_expanded_aliases = Vec::new();
    let mut seen_aliases = std::collections::HashSet::new();

    for (expression, alias) in computed_columns {
        expanded_alias_prefix_lengths.push(all_expanded_aliases.len());

        for (_, expanded_alias) in expand_grouped_computed_columns_for_available_columns(
            expression,
            alias,
            &current_columns,
            column_groups,
            variables,
        ) {
            if seen_aliases.insert(expanded_alias.clone()) {
                all_expanded_aliases.push(expanded_alias.clone());
            }

            if current_column_names.insert(expanded_alias.clone()) {
                current_columns.push(expanded_alias);
            }
        }
    }

    (expanded_alias_prefix_lengths, all_expanded_aliases)
}

pub(crate) fn apply_computed_columns_to_lazyframe(
    lf: &mut LazyFrame,
    column_names: &mut Vec<String>,
    computed_columns: &[(String, String)],
    column_groups: &[ColumnGroup],
    variables: &[(String, f64)],
) {
    for (expression, alias) in computed_columns {
        if alias.trim().is_empty() {
            log::error!("Skipping computed column with an empty alias.");
            continue;
        }

        let source_group_resolver =
            build_source_group_resolver_for_columns(column_groups, column_names);
        let expanded_columns =
            expand_grouped_computed_columns(expression, alias, variables, &source_group_resolver);

        if expanded_columns.is_empty() {
            log::warn!(
                "Skipping computed column '{alias}': grouped dependencies did not resolve to any columns."
            );
            continue;
        }

        for (expanded_expression, expanded_alias) in expanded_columns {
            let replacing_existing = column_names
                .iter()
                .any(|column_name| column_name == &expanded_alias);
            if replacing_existing {
                log::info!(
                    "Overwriting existing column '{expanded_alias}' with a computed expression."
                );
            }

            let missing_dependencies =
                computed_column_dependencies(&expanded_expression, variables)
                    .into_iter()
                    .filter(|dependency| !column_names.contains(dependency))
                    .collect::<Vec<_>>();
            if !missing_dependencies.is_empty() {
                log::warn!(
                    "Skipping computed column '{expanded_alias}': missing dependency column(s): {}",
                    missing_dependencies.join(", ")
                );
                continue;
            }

            if let Err(error) =
                add_computed_column(lf, &expanded_expression, &expanded_alias, variables)
            {
                log::error!("Error adding computed column '{expanded_alias}': {error}");
            } else if !replacing_existing {
                column_names.push(expanded_alias.clone());
            }
        }
    }
}

pub fn get_column_names_from_lazyframe(lf: &LazyFrame) -> Result<Vec<String>, PolarsError> {
    let mut lf = lf.clone();

    // Use the logical schema instead of collecting rows from the source files.
    let schema = lf.collect_schema().map_err(|err| {
        log::error!("Failed to collect LazyFrame schema: {err:?}");
        let err_text = err.to_string();
        if err_text.contains("Operation timed out") || err_text.contains("os error 60") {
            log::error!(
                "This looks like a file availability issue outside Spectrix. One or more selected parquet files may still be cloud-only in OneDrive or another synced folder. Download them locally and mark them available offline, then retry."
            );
        }
        err
    })?;

    // Get column names
    let columns: Vec<String> = schema
        .iter_names_cloned()
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
        assert_eq!(
            sanitize_computed_column_alias(" RF Minus Ring {} "),
            "RF_Minus_Ring_{}"
        );
        assert!(is_valid_computed_column_alias("RFMinusRing{}"));
        assert!(!is_valid_computed_column_alias("bad-alias"));
    }

    #[test]
    fn reorder_selected_index_tracks_dragged_row() {
        assert_eq!(reordered_selected_index(Some(2), 2, 0), Some(0));
        assert_eq!(reordered_selected_index(Some(1), 1, 3), Some(3));
    }

    #[test]
    fn reorder_selected_index_tracks_rows_around_dragged_row() {
        assert_eq!(reordered_selected_index(Some(2), 0, 2), Some(1));
        assert_eq!(reordered_selected_index(Some(1), 3, 0), Some(2));
    }

    #[test]
    fn column_group_alias_expands_into_many_1d_histograms() {
        let mut configs = Configs::default();
        configs.column_groups.push(ColumnGroup {
            alias: "S1RingEnergy".to_owned(),
            column_names: vec!["S1Ring0Energy".to_owned(), "S1Ring1Energy".to_owned()],
        });
        configs.configs.push(Config::Hist1D(Hist1DConfig {
            name: "S1/Ring {}".to_owned(),
            column_name: "S1RingEnergy".to_owned(),
            additional_column_names: Vec::new(),
            range: (0.0, 4096.0),
            bins: 512,
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }));

        let expanded = configs.expand(&["S1Ring0Energy".to_owned(), "S1Ring1Energy".to_owned()]);

        let expanded_histograms = expanded
            .configs
            .iter()
            .filter_map(|config| match config {
                Config::Hist1D(hist) => Some((hist.name.as_str(), hist.column_name.as_str())),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            expanded_histograms,
            vec![
                ("S1/Ring 0", "S1Ring0Energy"),
                ("S1/Ring 1", "S1Ring1Energy"),
            ]
        );
    }

    #[test]
    fn column_group_alias_fills_one_1d_histogram_from_many_columns() {
        let mut configs = Configs::default();
        configs.column_groups.push(ColumnGroup {
            alias: "S1RingEnergy".to_owned(),
            column_names: vec!["S1Ring0Energy".to_owned(), "S1Ring1Energy".to_owned()],
        });
        configs.configs.push(Config::Hist1D(Hist1DConfig {
            name: "S1/Combined".to_owned(),
            column_name: "S1RingEnergy".to_owned(),
            additional_column_names: Vec::new(),
            range: (0.0, 4096.0),
            bins: 512,
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }));

        let expanded = configs.expand(&["S1Ring0Energy".to_owned(), "S1Ring1Energy".to_owned()]);

        let expanded_histograms = expanded
            .configs
            .iter()
            .filter_map(|config| match config {
                Config::Hist1D(hist) => Some((hist.name.as_str(), hist.column_name.as_str())),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            expanded_histograms,
            vec![
                ("S1/Combined", "S1Ring0Energy"),
                ("S1/Combined", "S1Ring1Energy"),
            ]
        );
    }

    #[test]
    fn column_group_alias_expands_2d_histograms_from_grouped_x_columns() {
        let mut configs = Configs::default();
        configs.column_groups.push(ColumnGroup {
            alias: "S1RingEnergy".to_owned(),
            column_names: vec!["S1Ring0Energy".to_owned(), "S1Ring1Energy".to_owned()],
        });
        configs.configs.push(Config::Hist2D(Hist2DConfig {
            name: "S1/ToF {}".to_owned(),
            x_column_name: "S1RingEnergy".to_owned(),
            y_column_name: "ToF".to_owned(),
            additional_x_column_names: Vec::new(),
            additional_y_column_names: Vec::new(),
            x_range: (0.0, 4096.0),
            y_range: (-1000.0, 1000.0),
            bins: (512, 512),
            cuts: Cuts::default(),
            calculate: true,
            enabled: true,
        }));

        let expanded = configs.expand(&[
            "S1Ring0Energy".to_owned(),
            "S1Ring1Energy".to_owned(),
            "ToF".to_owned(),
        ]);

        let expanded_histograms = expanded
            .configs
            .iter()
            .filter_map(|config| match config {
                Config::Hist2D(hist) => Some((
                    hist.name.as_str(),
                    hist.x_column_name.as_str(),
                    hist.y_column_name.as_str(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            expanded_histograms,
            vec![
                ("S1/ToF 0", "S1Ring0Energy", "ToF"),
                ("S1/ToF 1", "S1Ring1Energy", "ToF"),
            ]
        );
    }

    #[test]
    fn grouped_computed_column_expression_expands_to_many_columns() {
        let resolver = build_source_group_resolver_for_columns(
            &[ColumnGroup {
                alias: "S1RingTime".to_owned(),
                column_names: vec!["S1Ring0Time".to_owned(), "S1Ring1Time".to_owned()],
            }],
            &[
                "RF".to_owned(),
                "S1Ring0Time".to_owned(),
                "S1Ring1Time".to_owned(),
            ],
        );

        let expanded_columns =
            expand_grouped_computed_columns("RF - S1RingTime", "RFMinusS1RingTime", &[], &resolver);

        assert_eq!(
            expanded_columns,
            vec![
                (
                    "RF - S1Ring0Time".to_owned(),
                    "RFMinusS1RingTime_S1Ring0Time".to_owned(),
                ),
                (
                    "RF - S1Ring1Time".to_owned(),
                    "RFMinusS1RingTime_S1Ring1Time".to_owned(),
                ),
            ]
        );
    }

    #[test]
    fn grouped_computed_column_alias_template_uses_group_token() {
        let resolver = build_source_group_resolver_for_columns(
            &[ColumnGroup {
                alias: "S1RingTime".to_owned(),
                column_names: vec!["S1Ring0Time".to_owned(), "S1Ring1Time".to_owned()],
            }],
            &[
                "RF".to_owned(),
                "S1Ring0Time".to_owned(),
                "S1Ring1Time".to_owned(),
            ],
        );

        let expanded_columns =
            expand_grouped_computed_columns("RF - S1RingTime", "RFMinusRing{}", &[], &resolver);

        assert_eq!(
            expanded_columns,
            vec![
                (
                    "RF - S1Ring0Time".to_owned(),
                    "RFMinusRingS1Ring0Time".to_owned(),
                ),
                (
                    "RF - S1Ring1Time".to_owned(),
                    "RFMinusRingS1Ring1Time".to_owned(),
                ),
            ]
        );
    }

    #[test]
    fn grouped_computed_columns_expand_across_all_group_combinations() {
        let resolver = build_source_group_resolver_for_columns(
            &[
                ColumnGroup {
                    alias: "S1RingTime".to_owned(),
                    column_names: vec!["S1Ring0Time".to_owned(), "S1Ring1Time".to_owned()],
                },
                ColumnGroup {
                    alias: "S1RingEnergy".to_owned(),
                    column_names: vec!["S1Ring0Energy".to_owned(), "S1Ring1Energy".to_owned()],
                },
            ],
            &[
                "S1Ring0Time".to_owned(),
                "S1Ring1Time".to_owned(),
                "S1Ring0Energy".to_owned(),
                "S1Ring1Energy".to_owned(),
            ],
        );

        let expanded_columns = expand_grouped_computed_columns(
            "S1RingTime - S1RingEnergy",
            "S1RingDelta",
            &[],
            &resolver,
        );

        assert_eq!(
            expanded_columns,
            vec![
                (
                    "S1Ring0Time - S1Ring0Energy".to_owned(),
                    "S1RingDelta_S1Ring0Time_S1Ring0Energy".to_owned(),
                ),
                (
                    "S1Ring0Time - S1Ring1Energy".to_owned(),
                    "S1RingDelta_S1Ring0Time_S1Ring1Energy".to_owned(),
                ),
                (
                    "S1Ring1Time - S1Ring0Energy".to_owned(),
                    "S1RingDelta_S1Ring1Time_S1Ring0Energy".to_owned(),
                ),
                (
                    "S1Ring1Time - S1Ring1Energy".to_owned(),
                    "S1RingDelta_S1Ring1Time_S1Ring1Energy".to_owned(),
                ),
            ]
        );
    }

    #[test]
    fn grouped_computed_columns_appear_as_expanded_outputs_in_available_columns() {
        let mut configs = Configs::default();
        configs.column_groups.push(ColumnGroup {
            alias: "S1RingTime".to_owned(),
            column_names: vec!["S1Ring0Time".to_owned(), "S1Ring1Time".to_owned()],
        });
        configs
            .columns
            .push(("RF - S1RingTime".to_owned(), "RFMinusS1RingTime".to_owned()));

        let available_columns = configs.available_columns_for_ui(&[
            "RF".to_owned(),
            "S1Ring0Time".to_owned(),
            "S1Ring1Time".to_owned(),
        ]);

        assert!(available_columns.contains(&"RFMinusS1RingTime_S1Ring0Time".to_owned()));
        assert!(available_columns.contains(&"RFMinusS1RingTime_S1Ring1Time".to_owned()));
    }

    #[test]
    fn apply_grouped_computed_columns_to_lazyframe_creates_expanded_columns() {
        let dataframe = df!(
            "RF" => &[10.0, 20.0],
            "S1Ring0Time" => &[1.0, 2.0],
            "S1Ring1Time" => &[3.0, 4.0]
        )
        .expect("failed to create test dataframe");
        let mut lazyframe = dataframe.lazy();
        let mut column_names = get_column_names_from_lazyframe(&lazyframe)
            .expect("failed to collect initial test column names");

        apply_computed_columns_to_lazyframe(
            &mut lazyframe,
            &mut column_names,
            &[("RF - S1RingTime".to_owned(), "RFMinusS1RingTime".to_owned())],
            &[ColumnGroup {
                alias: "S1RingTime".to_owned(),
                column_names: vec!["S1Ring0Time".to_owned(), "S1Ring1Time".to_owned()],
            }],
            &[],
        );

        let collected = lazyframe
            .collect()
            .expect("failed to collect grouped computed columns");

        let ring0 = collected
            .column("RFMinusS1RingTime_S1Ring0Time")
            .expect("missing expanded grouped computed column 0")
            .f64()
            .expect("column 0 should be f64")
            .into_no_null_iter()
            .collect::<Vec<_>>();
        let ring1 = collected
            .column("RFMinusS1RingTime_S1Ring1Time")
            .expect("missing expanded grouped computed column 1")
            .f64()
            .expect("column 1 should be f64")
            .into_no_null_iter()
            .collect::<Vec<_>>();

        assert_eq!(ring0, vec![9.0, 18.0]);
        assert_eq!(ring1, vec![7.0, 16.0]);
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
