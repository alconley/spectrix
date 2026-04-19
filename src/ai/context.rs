use crate::histoer::configs::Config;
use crate::histoer::histogrammer::Histogrammer;
use crate::histogram_scripter::histogram_script::HistogramScript;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistogramKind {
    OneD,
    TwoD,
}

impl HistogramKind {
    fn label(self) -> &'static str {
        match self {
            Self::OneD => "1D",
            Self::TwoD => "2D",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExistingHistogramSummary {
    pub name: String,
    pub kind: HistogramKind,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExistingDerivedColumn {
    pub alias: String,
    pub expression: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiContextSnapshot {
    pub selected_files: Vec<String>,
    pub loaded_columns: Vec<String>,
    pub existing_derived_columns: Vec<ExistingDerivedColumn>,
    pub general_cut_names: Vec<String>,
    pub active_histogram_cut_names: Vec<String>,
    pub existing_histograms: Vec<ExistingHistogramSummary>,
}

impl AiContextSnapshot {
    pub fn from_state(
        selected_files: &[(PathBuf, bool)],
        loaded_columns: &[String],
        histogram_script: &HistogramScript,
        histogrammer: &Histogrammer,
    ) -> Self {
        let mut general_cut_names = histogram_script
            .configs
            .cuts
            .cuts
            .iter()
            .map(|cut| cut.name().to_owned())
            .collect::<Vec<_>>();
        general_cut_names.sort();
        general_cut_names.dedup();

        let mut active_histogram_cut_names = histogrammer
            .retrieve_active_histogram_cuts()
            .into_iter()
            .map(|active_cut| active_cut.cut.name().to_owned())
            .collect::<Vec<_>>();
        active_histogram_cut_names.sort();
        active_histogram_cut_names.dedup();

        let existing_histograms = histogram_script
            .configs
            .configs
            .iter()
            .map(|config| match config {
                Config::Hist1D(hist) => ExistingHistogramSummary {
                    name: hist.name.clone(),
                    kind: HistogramKind::OneD,
                },
                Config::Hist2D(hist) => ExistingHistogramSummary {
                    name: hist.name.clone(),
                    kind: HistogramKind::TwoD,
                },
            })
            .collect();

        let existing_derived_columns = histogram_script
            .configs
            .columns
            .iter()
            .map(|(expression, alias)| ExistingDerivedColumn {
                alias: alias.clone(),
                expression: expression.clone(),
            })
            .collect();

        let selected_files = selected_files
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(path, _)| path.display().to_string())
            .collect();

        Self {
            selected_files,
            loaded_columns: loaded_columns.to_vec(),
            existing_derived_columns,
            general_cut_names,
            active_histogram_cut_names,
            existing_histograms,
        }
    }

    fn available_columns(&self) -> BTreeSet<String> {
        let mut columns = self.loaded_columns.iter().cloned().collect::<BTreeSet<_>>();
        columns.extend(
            self.existing_derived_columns
                .iter()
                .map(|column| column.alias.clone()),
        );
        columns
    }

    pub(crate) fn to_prompt_section(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "- Selected files: {}",
            summarize_list(&self.selected_files, "none")
        ));
        lines.push(format!(
            "- Loaded parquet columns: {}",
            summarize_list(&self.loaded_columns, "none")
        ));

        let derived_columns = self
            .existing_derived_columns
            .iter()
            .map(|column| format!("{} = {}", column.alias, column.expression))
            .collect::<Vec<_>>();
        lines.push(format!(
            "- Derived columns: {}",
            summarize_list(&derived_columns, "none")
        ));
        lines.push(format!(
            "- Histogram Script cuts: {}",
            summarize_list(&self.general_cut_names, "none")
        ));
        lines.push(format!(
            "- Active histogram cuts: {}",
            summarize_list(&self.active_histogram_cut_names, "none")
        ));

        let histograms = self
            .existing_histograms
            .iter()
            .map(|histogram| format!("{} ({})", histogram.name, histogram.kind.label()))
            .collect::<Vec<_>>();
        lines.push(format!(
            "- Defined histograms: {}",
            summarize_list(&histograms, "none")
        ));

        let available_columns = self.available_columns().into_iter().collect::<Vec<_>>();
        lines.push(format!(
            "- Columns available for new cuts/histograms: {}",
            summarize_list(&available_columns, "none")
        ));

        lines.join("\n")
    }
}

fn summarize_list(values: &[String], empty_label: &str) -> String {
    const MAX_ITEMS: usize = 24;

    if values.is_empty() {
        return empty_label.to_owned();
    }

    let mut shown = values.iter().take(MAX_ITEMS).cloned().collect::<Vec<_>>();
    if values.len() > MAX_ITEMS {
        shown.push(format!("and {} more", values.len() - MAX_ITEMS));
    }
    shown.join(", ")
}
