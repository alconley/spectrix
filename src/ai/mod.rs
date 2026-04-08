#![allow(dead_code)]

use crate::histoer::configs::{Config, Configs, Hist1DConfig, Hist2DConfig};
use crate::histoer::cuts::{ActiveHistogramCut, Cut, Cut1D};
use crate::histoer::histogrammer::Histogrammer;
use crate::histogram_scripter::histogram_script::HistogramScript;
use pyo3::ffi::c_str;
use pyo3::{prelude::*, types::PyModule};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

const DEFAULT_SYSTEM_PROMPT: &str = r#"
You are Spectrix's in-app help assistant.

Your job is to answer questions about how to use Spectrix as it exists today. Return only valid JSON that matches the provided schema.

Response rules:
- Put the main answer in `summary`.
- Use `clarification_questions` only if a short follow-up is required to answer correctly.
- Use `notes` for short extra tips, caveats, or UI labels that may help the user.
- Always return `actions` as an empty array.

Behavior rules:
- If the user asks how to do something in Spectrix, answer with practical UI steps.
- Use the provided README context as your primary source of truth for Spectrix behavior and UI names.
- Refer to real Spectrix UI names when helpful, such as Processor, Histogram Script, Variables, Column Creation, 1D Cuts, Builder, Calculate Histograms, Selected File Settings, and fitting controls.
- If the user asks you to create, apply, edit, calculate, configure, or otherwise do something for them, clearly say that AI action-taking in Spectrix is currently under development.
- If Spectrix does not currently support what the user wants, say so plainly.
- Keep answers concise, concrete, and focused on the current app.
"#;

const README_HELP_TEXT: &str = include_str!("../../README.md");

const DEFAULT_1D_BINS: usize = 512;
const DEFAULT_2D_BINS: usize = 512;
const DEFAULT_1D_RANGE: [f64; 2] = [0.0, 4096.0];
const DEFAULT_2D_RANGE: [f64; 2] = [0.0, 4096.0];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    Ollama,
    OpenAiCompatible,
    OpenAi,
}

impl AiProvider {
    fn label(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::OpenAiCompatible => "OpenAI-Compatible",
            Self::OpenAi => "OpenAI Cloud",
        }
    }

    fn default_base_url(self) -> &'static str {
        match self {
            Self::Ollama => "http://127.0.0.1:11434",
            Self::OpenAiCompatible => "http://127.0.0.1:1234/v1",
            Self::OpenAi => "https://api.openai.com/v1",
        }
    }

    fn default_model(self) -> &'static str {
        match self {
            Self::Ollama => "gpt-oss",
            Self::OpenAiCompatible => "local-model",
            Self::OpenAi => "gpt-5-mini",
        }
    }

    fn requires_api_key(self) -> bool {
        matches!(self, Self::OpenAi)
    }

    fn help_text(self) -> &'static str {
        match self {
            Self::Ollama => {
                "Local-first. Expects an Ollama server on localhost and uses Ollama's native API."
            }
            Self::OpenAiCompatible => {
                "For a local or self-hosted OpenAI-compatible server such as LM Studio."
            }
            Self::OpenAi => "Uses OpenAI's hosted Responses API and requires OPENAI_API_KEY.",
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

    fn histogram_kind_by_name(&self, name: &str) -> Option<HistogramKind> {
        self.existing_histograms
            .iter()
            .find(|hist| hist.name == name)
            .map(|hist| hist.kind)
    }

    fn has_loaded_context(&self) -> bool {
        !self.available_columns().is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum AiActionType {
    #[serde(rename = "add_column")]
    AddColumn,
    #[serde(rename = "add_cut_1d")]
    AddCut1D,
    #[serde(rename = "add_hist_1d")]
    AddHist1D,
    #[serde(rename = "add_hist_2d")]
    AddHist2D,
}

impl Default for AiActionType {
    fn default() -> Self {
        Self::AddHist1D
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct AiAction {
    #[serde(rename = "type")]
    pub action_type: AiActionType,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub alias: String,
    #[serde(default)]
    pub expression: String,
    #[serde(default)]
    pub column_name: String,
    #[serde(default)]
    pub x_column_name: String,
    #[serde(default)]
    pub y_column_name: String,
    #[serde(default)]
    pub range: [f64; 2],
    #[serde(default)]
    pub x_range: [f64; 2],
    #[serde(default)]
    pub y_range: [f64; 2],
    #[serde(default)]
    pub bins: usize,
    #[serde(default)]
    pub x_bins: usize,
    #[serde(default)]
    pub y_bins: usize,
    #[serde(default)]
    pub cuts: Vec<String>,
}

impl AiAction {
    fn preview_label(&self) -> String {
        match self.action_type {
            AiActionType::AddColumn => {
                format!(
                    "Add column '{}' = {}",
                    self.alias.trim(),
                    self.expression.trim()
                )
            }
            AiActionType::AddCut1D => {
                format!(
                    "Add 1D cut '{}' with {}",
                    self.name.trim(),
                    self.expression.trim()
                )
            }
            AiActionType::AddHist1D => format!(
                "Add 1D histogram '{}' from '{}' with range [{}, {}], bins {}, cuts [{}]",
                self.name.trim(),
                self.column_name.trim(),
                self.range[0],
                self.range[1],
                self.bins,
                self.cuts.join(", ")
            ),
            AiActionType::AddHist2D => format!(
                "Add 2D histogram '{}' from '{}' vs '{}' with X range [{}, {}], Y range [{}, {}], bins [{}, {}], cuts [{}]",
                self.name.trim(),
                self.x_column_name.trim(),
                self.y_column_name.trim(),
                self.x_range[0],
                self.x_range[1],
                self.y_range[0],
                self.y_range[1],
                self.x_bins,
                self.y_bins,
                self.cuts.join(", ")
            ),
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct AiResponse {
    pub summary: String,
    #[serde(default)]
    pub clarification_questions: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub actions: Vec<AiAction>,
}

#[derive(Debug)]
struct PendingGeneration {
    response: AiResponse,
    raw_json: String,
}

#[derive(Debug, Default)]
struct ValidationReport {
    clarification_questions: Vec<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl ValidationReport {
    fn can_apply(&self) -> bool {
        self.errors.is_empty() && self.clarification_questions.is_empty()
    }
}

#[derive(Debug, Default)]
struct ApplyReport {
    added_columns: usize,
    replaced_columns: usize,
    added_cuts: usize,
    replaced_cuts: usize,
    added_histograms: usize,
    replaced_histograms: usize,
}

impl ApplyReport {
    fn status_message(&self) -> String {
        format!(
            "Applied AI actions: {} new column(s), {} replaced column(s), {} new cut(s), {} replaced cut(s), {} new histogram(s), {} replaced histogram(s).",
            self.added_columns,
            self.replaced_columns,
            self.added_cuts,
            self.replaced_cuts,
            self.added_histograms,
            self.replaced_histograms
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ActionCounts {
    columns: usize,
    cuts_1d: usize,
    histograms_1d: usize,
    histograms_2d: usize,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AiAssistant {
    pub provider: AiProvider,
    pub base_url: String,
    pub model: String,
    pub system_prompt: String,
    pub prompt: String,
    #[serde(skip)]
    discovered_models: Vec<String>,
    #[serde(skip)]
    preview: Option<AiResponse>,
    #[serde(skip)]
    preview_raw_json: Option<String>,
    #[serde(skip)]
    last_error: Option<String>,
    #[serde(skip)]
    last_apply_status: Option<String>,
    #[serde(skip)]
    last_model_error: Option<String>,
    #[serde(skip)]
    generating: Arc<AtomicBool>,
    #[serde(skip)]
    pending_result: Arc<Mutex<Option<Result<PendingGeneration, String>>>>,
    #[serde(skip)]
    listing_models: Arc<AtomicBool>,
    #[serde(skip)]
    pending_models: Arc<Mutex<Option<Result<Vec<String>, String>>>>,
}

impl Default for AiAssistant {
    fn default() -> Self {
        let provider = AiProvider::Ollama;
        Self {
            provider,
            base_url: provider.default_base_url().to_owned(),
            model: provider.default_model().to_owned(),
            system_prompt: default_system_prompt().to_owned(),
            prompt: String::new(),
            discovered_models: Vec::new(),
            preview: None,
            preview_raw_json: None,
            last_error: None,
            last_apply_status: None,
            last_model_error: None,
            generating: Arc::new(AtomicBool::new(false)),
            pending_result: Arc::new(Mutex::new(None)),
            listing_models: Arc::new(AtomicBool::new(false)),
            pending_models: Arc::new(Mutex::new(None)),
        }
    }
}

impl AiAssistant {
    pub fn is_generating(&self) -> bool {
        self.generating.load(Ordering::Relaxed)
    }

    pub fn is_busy(&self) -> bool {
        self.generating.load(Ordering::Relaxed) || self.listing_models.load(Ordering::Relaxed)
    }

    fn take_pending_result(&mut self) {
        let result = self
            .pending_result
            .lock()
            .ok()
            .and_then(|mut pending| pending.take());

        match result {
            Some(Ok(pending)) => {
                self.preview = Some(pending.response);
                self.preview_raw_json = Some(pending.raw_json);
                self.last_error = None;
            }
            Some(Err(error)) => {
                self.last_error = Some(error);
            }
            None => {}
        }
    }

    fn take_pending_models(&mut self) {
        let result = self
            .pending_models
            .lock()
            .ok()
            .and_then(|mut pending| pending.take());

        match result {
            Some(Ok(models)) => {
                self.discovered_models = models;
                self.last_model_error = None;
            }
            Some(Err(error)) => {
                self.last_model_error = Some(error);
            }
            None => {}
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.take_pending_result();
        self.take_pending_models();

        egui::ScrollArea::vertical()
            .id_salt("spectrix_ai_assistant_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading("AI Spectrix Assistant");
                ui.label(
                    egui::RichText::new(
                        "Ask how to do things in Spectrix. For now, the assistant is help-only.",
                    )
                    .small(),
                );
                ui.label(
                    egui::RichText::new(
                        "If you ask it to perform actions for you, it should say that AI action-taking is still under development.",
                    )
                    .weak()
                    .small(),
                );

                ui.add_space(4.0);
                let previous_provider = self.provider;
                egui::ComboBox::from_id_salt("spectrix_ai_provider")
                    .selected_text(self.provider.label())
                    .show_ui(ui, |ui| {
                        for provider in [
                            AiProvider::Ollama,
                            AiProvider::OpenAiCompatible,
                            AiProvider::OpenAi,
                        ] {
                            ui.selectable_value(&mut self.provider, provider, provider.label());
                        }
                    });

                if self.provider != previous_provider {
                    self.base_url = self.provider.default_base_url().to_owned();
                    self.model = self.provider.default_model().to_owned();
                    self.discovered_models.clear();
                    self.last_model_error = None;
                }

                ui.label(
                    egui::RichText::new(self.provider.help_text())
                        .weak()
                        .small(),
                );

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Base URL:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.base_url)
                            .hint_text(self.provider.default_base_url())
                            .desired_width(280.0),
                    );
                });

                ui.add_space(4.0);
                let list_models_enabled = self.provider != AiProvider::OpenAi
                    && !self.base_url.trim().is_empty()
                    && !self.listing_models.load(Ordering::Relaxed);
                ui.horizontal_wrapped(|ui| {
                    let response =
                        ui.add_enabled(list_models_enabled, egui::Button::new("List Local Models"));
                    let response = if list_models_enabled {
                        response.on_hover_text(
                            "Query the configured local model server for available models.",
                        )
                    } else if self.provider == AiProvider::OpenAi {
                        response.on_disabled_hover_text(
                            "Local model listing is for local providers such as Ollama or LM Studio.",
                        )
                    } else if self.base_url.trim().is_empty() {
                        response.on_disabled_hover_text("Enter the local server base URL first.")
                    } else {
                        response.on_disabled_hover_text("A model list request is already running.")
                    };

                    if response.clicked() {
                        self.start_model_listing();
                    }

                    if self.listing_models.load(Ordering::Relaxed) {
                        ui.label("Loading models...");
                        ui.add(egui::widgets::Spinner::default());
                    } else if !self.discovered_models.is_empty() {
                        ui.label(format!("{} found", self.discovered_models.len()));
                    }
                });

                if let Some(model_error) = &self.last_model_error {
                    ui.colored_label(egui::Color32::LIGHT_RED, model_error);
                }

                if !self.discovered_models.is_empty() {
                    ui.collapsing("Discovered Models", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for discovered_model in self.discovered_models.clone() {
                                if ui.button(&discovered_model).clicked() {
                                    self.model = discovered_model;
                                }
                            }
                        });
                    });
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Model:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.model)
                            .hint_text(self.provider.default_model())
                            .desired_width(180.0),
                    );
                });

                ui.add_space(4.0);
                ui.label("Question or Request:");
                ui.add_sized(
                    [ui.available_width(), 110.0],
                    egui::TextEdit::multiline(&mut self.prompt).hint_text(
                        "Example: How do I create a 1D cut with OR conditions in Spectrix?",
                    ),
                );

                ui.add_space(4.0);
                ui.collapsing("System Prompt", |ui| {
                    ui.label(
                        egui::RichText::new(
                            "This prompt is sent with every request. Edit it here until the assistant behaves the way you want.",
                        )
                        .weak()
                        .small(),
                    );
                    ui.add_sized(
                        [ui.available_width(), 220.0],
                        egui::TextEdit::multiline(&mut self.system_prompt)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Reset System Prompt").clicked() {
                            self.system_prompt = default_system_prompt().to_owned();
                        }
                        ui.label(
                            egui::RichText::new("Changes persist with the Spectrix app state.")
                                .weak()
                                .small(),
                        );
                    });
                });

                let api_key_present = if self.provider.requires_api_key() {
                    std::env::var("OPENAI_API_KEY")
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
                } else {
                    true
                };
                let can_generate = api_key_present
                    && !self.prompt.trim().is_empty()
                    && !self.system_prompt.trim().is_empty()
                    && !self.base_url.trim().is_empty()
                    && !self.model.trim().is_empty()
                    && !self.is_generating();

                let disabled_reason = if self.is_generating() {
                    "An AI request is already running."
                } else if !api_key_present {
                    "Set OPENAI_API_KEY before using the OpenAI cloud provider."
                } else if self.prompt.trim().is_empty() {
                    "Ask a question about how to use Spectrix."
                } else if self.system_prompt.trim().is_empty() {
                    "Enter a system prompt or reset it to the default."
                } else if self.base_url.trim().is_empty() {
                    "Enter a local or remote AI server base URL."
                } else if self.model.trim().is_empty() {
                    "Enter a model name."
                } else {
                    ""
                };

                let response = ui.add_enabled(can_generate, egui::Button::new("Ask Spectrix AI"));
                let response = if can_generate {
                    response.on_hover_text("Ask a usage question about Spectrix.")
                } else {
                    response.on_disabled_hover_text(disabled_reason)
                };

                if response.clicked() {
                    self.start_generation();
                }

                if self.is_generating() {
                    ui.horizontal(|ui| {
                        ui.label("Thinking...");
                        ui.add(egui::widgets::Spinner::default());
                    });
                }

                if let Some(error) = &self.last_error {
                    ui.add_space(6.0);
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }

                if let Some(preview) = self.preview.clone() {
                    ui.separator();
                    ui.label(egui::RichText::new("Answer").strong());
                    ui.label(preview.summary.trim());

                    if !preview.notes.is_empty() {
                        ui.add_space(4.0);
                        ui.label("Helpful Notes:");
                        for note in &preview.notes {
                            ui.label(format!("• {note}"));
                        }
                    }

                    if !preview.clarification_questions.is_empty() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Clarification Needed").strong());
                        ui.label(
                            "Type your answers in the Question or Request box above, then ask again.",
                        );
                        for question in &preview.clarification_questions {
                            ui.colored_label(egui::Color32::LIGHT_BLUE, format!("• {question}"));
                        }
                        ui.add_space(4.0);
                        if ui.button("Prepare Follow-Up In Request").clicked() {
                            self.prompt = build_follow_up_prompt_template(
                                &self.prompt,
                                &preview.clarification_questions,
                            );
                        }
                    }

                    ui.add_space(6.0);
                    let mut should_clear_preview = false;
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Clear Answer").clicked() {
                            should_clear_preview = true;
                        }
                    });

                    if should_clear_preview {
                        self.preview = None;
                        self.preview_raw_json = None;
                    }

                    if let Some(raw_json) = &self.preview_raw_json {
                        ui.add_space(4.0);
                        ui.collapsing("Raw JSON", |ui| {
                            let mut raw_json_display = raw_json.clone();
                            ui.add_sized(
                                [ui.available_width(), 180.0],
                                egui::TextEdit::multiline(&mut raw_json_display)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .interactive(false),
                            );
                        });
                    }
                }
            });
    }

    fn start_generation(&mut self) {
        self.last_error = None;

        let provider = self.provider;
        let base_url = self.base_url.trim().to_owned();
        let model = self.model.trim().to_owned();
        let system_prompt = self.system_prompt.trim().to_owned();
        let prompt = self.prompt.trim().to_owned();
        let generating = Arc::clone(&self.generating);
        let pending_result = Arc::clone(&self.pending_result);
        generating.store(true, Ordering::Relaxed);

        thread::spawn(move || {
            let result =
                Self::generate_response(provider, &base_url, &model, &system_prompt, &prompt);

            if let Ok(mut pending) = pending_result.lock() {
                *pending = Some(result);
            }

            generating.store(false, Ordering::Relaxed);
        });
    }

    fn start_model_listing(&mut self) {
        self.last_model_error = None;

        let provider = self.provider;
        let base_url = self.base_url.trim().to_owned();
        let listing_models = Arc::clone(&self.listing_models);
        let pending_models = Arc::clone(&self.pending_models);
        listing_models.store(true, Ordering::Relaxed);

        thread::spawn(move || {
            let result = list_models(provider, &base_url);

            if let Ok(mut pending) = pending_models.lock() {
                *pending = Some(result);
            }

            listing_models.store(false, Ordering::Relaxed);
        });
    }

    fn generate_response(
        provider: AiProvider,
        base_url: &str,
        model: &str,
        system_prompt: &str,
        prompt: &str,
    ) -> Result<PendingGeneration, String> {
        let user_prompt = build_user_prompt(prompt)?;
        let schema = response_schema();

        let output_text = match provider {
            AiProvider::Ollama => {
                call_ollama_generate_api(base_url, model, system_prompt, &user_prompt, &schema)
                    .map_err(|error| format!("Ollama request failed: {error}"))?
            }
            AiProvider::OpenAiCompatible => {
                let response_body = call_openai_responses_api(
                    base_url,
                    None,
                    model,
                    system_prompt,
                    &user_prompt,
                    &schema,
                )
                .map_err(|error| format!("OpenAI-compatible request failed: {error}"))?;

                let response_json: Value =
                    serde_json::from_str(&response_body).map_err(|error| {
                        format!("Failed to parse the OpenAI-compatible API response JSON: {error}")
                    })?;
                extract_output_text(&response_json)?
            }
            AiProvider::OpenAi => {
                let api_key = std::env::var("OPENAI_API_KEY")
                    .map_err(|error| format!("OPENAI_API_KEY is not set: {error}"))?;
                let response_body = call_openai_responses_api(
                    base_url,
                    Some(&api_key),
                    model,
                    system_prompt,
                    &user_prompt,
                    &schema,
                )
                .map_err(|error| format!("OpenAI request failed: {error}"))?;

                let response_json: Value =
                    serde_json::from_str(&response_body).map_err(|error| {
                        format!("Failed to parse the OpenAI API response JSON: {error}")
                    })?;
                extract_output_text(&response_json)?
            }
        };

        let response: AiResponse = serde_json::from_str(&output_text)
            .map_err(|error| format!("The model returned invalid Spectrix action JSON: {error}"))?;
        let response = normalize_help_response(response, prompt);

        Ok(PendingGeneration {
            response,
            raw_json: prettify_json(&output_text),
        })
    }
}

impl AiResponse {
    fn action_counts(&self) -> ActionCounts {
        let mut counts = ActionCounts::default();
        for action in &self.actions {
            match action.action_type {
                AiActionType::AddColumn => counts.columns += 1,
                AiActionType::AddCut1D => counts.cuts_1d += 1,
                AiActionType::AddHist1D => counts.histograms_1d += 1,
                AiActionType::AddHist2D => counts.histograms_2d += 1,
            }
        }
        counts
    }

    fn validate(&self, snapshot: &AiContextSnapshot) -> ValidationReport {
        let mut report = ValidationReport::default();
        report.clarification_questions = dedupe_strings(self.clarification_questions.clone());

        let mut available_columns = snapshot.available_columns();
        let base_columns = snapshot
            .loaded_columns
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let existing_derived_aliases = snapshot
            .existing_derived_columns
            .iter()
            .map(|column| column.alias.clone())
            .collect::<BTreeSet<_>>();
        let active_cut_names = snapshot
            .active_histogram_cut_names
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let general_cut_names = snapshot
            .general_cut_names
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();

        let mut seen_aliases = BTreeSet::new();
        for action in self
            .actions
            .iter()
            .filter(|action| action.action_type == AiActionType::AddColumn)
        {
            let alias = action.alias.trim();
            if alias.is_empty() {
                report
                    .errors
                    .push("Derived columns need a non-empty alias.".to_owned());
                continue;
            }

            if !seen_aliases.insert(alias.to_owned()) {
                report.errors.push(format!(
                    "Derived column alias '{alias}' appears more than once."
                ));
                continue;
            }

            if base_columns.contains(alias) {
                report.errors.push(format!(
                    "Derived column alias '{alias}' matches an existing loaded column."
                ));
                continue;
            }

            if existing_derived_aliases.contains(alias) {
                report.warnings.push(format!(
                    "Derived column alias '{alias}' already exists and will be replaced."
                ));
            }

            if action.expression.trim().is_empty() {
                report.errors.push(format!(
                    "Derived column '{alias}' needs a non-empty expression."
                ));
                continue;
            }

            let referenced_identifiers = extract_expression_identifiers(&action.expression);
            let unknown_identifiers = referenced_identifiers
                .into_iter()
                .filter(|identifier| !available_columns.contains(identifier))
                .collect::<Vec<_>>();
            if !unknown_identifiers.is_empty() {
                report.errors.push(format!(
                    "Derived column '{alias}' references unknown column(s): {}.",
                    unknown_identifiers.join(", ")
                ));
                continue;
            }

            available_columns.insert(alias.to_owned());
        }

        let mut known_cut_names = general_cut_names.clone();
        let mut seen_new_cut_names = BTreeSet::new();
        for action in self
            .actions
            .iter()
            .filter(|action| action.action_type == AiActionType::AddCut1D)
        {
            let name = action.name.trim();
            if name.is_empty() {
                report
                    .errors
                    .push("1D cuts need a non-empty name.".to_owned());
                continue;
            }

            if !seen_new_cut_names.insert(name.to_owned()) {
                report
                    .errors
                    .push(format!("1D cut '{name}' appears more than once."));
                continue;
            }

            if active_cut_names.contains(name) {
                report.errors.push(format!(
                    "1D cut '{name}' conflicts with an existing active histogram cut name."
                ));
                continue;
            }

            if general_cut_names.contains(name) {
                report.warnings.push(format!(
                    "1D cut '{name}' already exists and will be replaced."
                ));
            }

            if action.expression.trim().is_empty() {
                report
                    .errors
                    .push(format!("1D cut '{name}' needs a non-empty expression."));
                continue;
            }

            let mut parsed_cut = Cut1D::new(name, action.expression.trim());
            parsed_cut.parse_conditions();
            let required_columns = parsed_cut.required_columns();
            if required_columns.is_empty() {
                report.errors.push(format!(
                    "1D cut '{name}' could not be parsed. Use comparisons joined with '&'."
                ));
                continue;
            }

            let missing_columns = required_columns
                .into_iter()
                .filter(|column| !available_columns.contains(column))
                .collect::<Vec<_>>();
            if !missing_columns.is_empty() {
                report.errors.push(format!(
                    "1D cut '{name}' references unknown column(s): {}.",
                    missing_columns.join(", ")
                ));
                continue;
            }

            known_cut_names.insert(name.to_owned());
        }

        let mut seen_histogram_names = BTreeSet::new();
        for action in self
            .actions
            .iter()
            .filter(|action| action.action_type == AiActionType::AddHist1D)
        {
            validate_histogram_name(
                action.name.trim(),
                HistogramKind::OneD,
                snapshot,
                &mut seen_histogram_names,
                &mut report,
            );

            if action.column_name.trim().is_empty() {
                report
                    .errors
                    .push("1D histogram actions need a non-empty column name.".to_owned());
            } else if !available_columns.contains(action.column_name.trim()) {
                report.errors.push(format!(
                    "1D histogram '{}' references unknown column '{}'.",
                    action.name, action.column_name
                ));
            }

            validate_range(&action.name, "1D histogram", action.range, &mut report);
            if action.bins == 0 {
                report
                    .errors
                    .push(format!("1D histogram '{}' needs bins > 0.", action.name));
            }

            validate_histogram_cut_names(&action.name, &action.cuts, &known_cut_names, &mut report);
        }

        for action in self
            .actions
            .iter()
            .filter(|action| action.action_type == AiActionType::AddHist2D)
        {
            validate_histogram_name(
                action.name.trim(),
                HistogramKind::TwoD,
                snapshot,
                &mut seen_histogram_names,
                &mut report,
            );

            if action.x_column_name.trim().is_empty() {
                report
                    .errors
                    .push("2D histogram actions need a non-empty X column name.".to_owned());
            } else if !available_columns.contains(action.x_column_name.trim()) {
                report.errors.push(format!(
                    "2D histogram '{}' references unknown X column '{}'.",
                    action.name, action.x_column_name
                ));
            }

            if action.y_column_name.trim().is_empty() {
                report
                    .errors
                    .push("2D histogram actions need a non-empty Y column name.".to_owned());
            } else if !available_columns.contains(action.y_column_name.trim()) {
                report.errors.push(format!(
                    "2D histogram '{}' references unknown Y column '{}'.",
                    action.name, action.y_column_name
                ));
            }

            validate_range(
                &action.name,
                "2D histogram X range",
                action.x_range,
                &mut report,
            );
            validate_range(
                &action.name,
                "2D histogram Y range",
                action.y_range,
                &mut report,
            );
            if action.x_bins == 0 || action.y_bins == 0 {
                report.errors.push(format!(
                    "2D histogram '{}' needs both bin counts to be > 0.",
                    action.name
                ));
            }

            validate_histogram_cut_names(&action.name, &action.cuts, &known_cut_names, &mut report);
        }

        if report.clarification_questions.is_empty() && self.actions.is_empty() {
            report.warnings.push(
                "The AI response is valid, but it did not include any actions to apply.".to_owned(),
            );
        }

        report
    }

    fn apply_to(
        &self,
        histogram_script: &mut HistogramScript,
        active_histogram_cuts: &[ActiveHistogramCut],
    ) -> Result<ApplyReport, String> {
        let mut report = ApplyReport::default();

        for action in self
            .actions
            .iter()
            .filter(|action| action.action_type == AiActionType::AddColumn)
        {
            if upsert_derived_column(
                &mut histogram_script.configs,
                action.alias.trim(),
                action.expression.trim(),
            ) {
                report.replaced_columns += 1;
            } else {
                report.added_columns += 1;
            }
        }

        let mut available_cuts = active_histogram_cuts
            .iter()
            .map(|active_cut| (active_cut.cut.name().to_owned(), active_cut.cut.clone()))
            .collect::<HashMap<_, _>>();
        for cut in &histogram_script.configs.cuts.cuts {
            available_cuts.insert(cut.name().to_owned(), cut.clone());
        }

        for action in self
            .actions
            .iter()
            .filter(|action| action.action_type == AiActionType::AddCut1D)
        {
            let mut parsed_cut = Cut1D::new(action.name.trim(), action.expression.trim());
            parsed_cut.parse_conditions();
            let cut = Cut::Cut1D(parsed_cut);
            let cut_name = cut.name().to_owned();
            let replaced = upsert_general_cut(&mut histogram_script.configs, cut.clone());
            available_cuts.insert(cut_name, cut);
            if replaced {
                report.replaced_cuts += 1;
            } else {
                report.added_cuts += 1;
            }
        }

        for action in self
            .actions
            .iter()
            .filter(|action| action.action_type == AiActionType::AddHist1D)
        {
            let mut config = Hist1DConfig::new(
                action.name.trim(),
                action.column_name.trim(),
                (action.range[0], action.range[1]),
                action.bins,
            );
            config.cuts.cuts = resolve_cut_names(&action.cuts, &available_cuts)?;

            if upsert_histogram_config(&mut histogram_script.configs, Config::Hist1D(config)) {
                report.replaced_histograms += 1;
            } else {
                report.added_histograms += 1;
            }
        }

        for action in self
            .actions
            .iter()
            .filter(|action| action.action_type == AiActionType::AddHist2D)
        {
            let mut config = Hist2DConfig::new(
                action.name.trim(),
                action.x_column_name.trim(),
                action.y_column_name.trim(),
                (action.x_range[0], action.x_range[1]),
                (action.y_range[0], action.y_range[1]),
                (action.x_bins, action.y_bins),
            );
            config.cuts.cuts = resolve_cut_names(&action.cuts, &available_cuts)?;

            if upsert_histogram_config(&mut histogram_script.configs, Config::Hist2D(config)) {
                report.replaced_histograms += 1;
            } else {
                report.added_histograms += 1;
            }
        }

        Ok(report)
    }
}

fn validate_histogram_name(
    name: &str,
    kind: HistogramKind,
    snapshot: &AiContextSnapshot,
    seen_histogram_names: &mut BTreeSet<String>,
    report: &mut ValidationReport,
) {
    if name.is_empty() {
        report.errors.push(format!(
            "{} histogram actions need a non-empty name.",
            kind.label()
        ));
        return;
    }

    if !seen_histogram_names.insert(name.to_owned()) {
        report.errors.push(format!(
            "Histogram name '{name}' appears more than once in the AI response."
        ));
        return;
    }

    if let Some(existing_kind) = snapshot.histogram_kind_by_name(name) {
        if existing_kind == kind {
            report.warnings.push(format!(
                "{} histogram '{}' already exists and will be replaced.",
                kind.label(),
                name
            ));
        } else {
            report.errors.push(format!(
                "Histogram name '{name}' is already used by an existing {} histogram.",
                existing_kind.label()
            ));
        }
    }
}

fn validate_range(name: &str, label: &str, range: [f64; 2], report: &mut ValidationReport) {
    if !range[0].is_finite() || !range[1].is_finite() {
        report
            .errors
            .push(format!("{label} for '{name}' must contain finite numbers."));
    } else if range[0] >= range[1] {
        report
            .errors
            .push(format!("{label} for '{name}' must have min < max."));
    }
}

fn validate_histogram_cut_names(
    histogram_name: &str,
    cut_names: &[String],
    known_cut_names: &BTreeSet<String>,
    report: &mut ValidationReport,
) {
    let mut seen = BTreeSet::new();
    for cut_name in cut_names {
        let cut_name = cut_name.trim();
        if cut_name.is_empty() {
            report.errors.push(format!(
                "Histogram '{histogram_name}' contains an empty cut name."
            ));
            continue;
        }

        if !seen.insert(cut_name.to_owned()) {
            report.errors.push(format!(
                "Histogram '{histogram_name}' references cut '{cut_name}' more than once."
            ));
        }

        if !known_cut_names.contains(cut_name) {
            report.errors.push(format!(
                "Histogram '{histogram_name}' references unknown cut '{cut_name}'."
            ));
        }
    }
}

fn extract_expression_identifiers(expression: &str) -> Vec<String> {
    let re = regex::Regex::new(r"[A-Za-z_]\w*").expect("failed to build identifier regex");
    re.find_iter(expression)
        .map(|capture| capture.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn upsert_derived_column(configs: &mut Configs, alias: &str, expression: &str) -> bool {
    if let Some(existing) = configs
        .columns
        .iter_mut()
        .find(|(_, existing_alias)| existing_alias == alias)
    {
        *existing = (expression.to_owned(), alias.to_owned());
        true
    } else {
        configs
            .columns
            .push((expression.to_owned(), alias.to_owned()));
        false
    }
}

fn upsert_general_cut(configs: &mut Configs, cut: Cut) -> bool {
    if let Some(existing) = configs
        .cuts
        .cuts
        .iter_mut()
        .find(|existing| existing.name() == cut.name())
    {
        *existing = cut;
        true
    } else {
        configs.cuts.cuts.push(cut);
        false
    }
}

fn upsert_histogram_config(configs: &mut Configs, new_config: Config) -> bool {
    let target_name = match &new_config {
        Config::Hist1D(hist) => hist.name.as_str(),
        Config::Hist2D(hist) => hist.name.as_str(),
    };

    if let Some(existing) = configs.configs.iter_mut().find(|config| match config {
        Config::Hist1D(hist) => hist.name == target_name,
        Config::Hist2D(hist) => hist.name == target_name,
    }) {
        *existing = new_config;
        true
    } else {
        configs.configs.push(new_config);
        false
    }
}

fn resolve_cut_names(
    cut_names: &[String],
    available_cuts: &HashMap<String, Cut>,
) -> Result<Vec<Cut>, String> {
    cut_names
        .iter()
        .map(|cut_name| {
            let key = cut_name.trim();
            available_cuts.get(key).cloned().ok_or_else(|| {
                format!("Unable to resolve cut '{key}' while applying the AI preview.")
            })
        })
        .collect()
}

#[derive(Debug, Default, Clone)]
struct PromptHistogramHints {
    shared_bins: Option<usize>,
    shared_range: Option<[f64; 2]>,
    tab_name: Option<String>,
}

fn normalize_response(
    mut response: AiResponse,
    prompt: &str,
    snapshot: &AiContextSnapshot,
) -> AiResponse {
    let prompt_hints = parse_prompt_histogram_hints(prompt);

    response.summary = response.summary.trim().to_owned();
    response.clarification_questions = dedupe_strings(response.clarification_questions);
    response.notes = dedupe_strings(response.notes);

    let mut recovery_notes = Vec::new();
    response.actions = response
        .actions
        .into_iter()
        .filter_map(|mut action| {
            action.name = action.name.trim().to_owned();
            action.alias = action.alias.trim().to_owned();
            action.expression = action.expression.trim().to_owned();
            action.column_name = action.column_name.trim().to_owned();
            action.x_column_name = action.x_column_name.trim().to_owned();
            action.y_column_name = action.y_column_name.trim().to_owned();
            action.cuts = dedupe_strings(action.cuts);

            fill_histogram_action_defaults(
                &mut action,
                &prompt_hints,
                snapshot,
                &mut recovery_notes,
            );

            let keep = match action.action_type {
                AiActionType::AddColumn => {
                    !action.alias.is_empty() || !action.expression.is_empty()
                }
                AiActionType::AddCut1D => !action.name.is_empty() || !action.expression.is_empty(),
                AiActionType::AddHist1D => {
                    !action.name.is_empty() || !action.column_name.is_empty() || action.bins > 0
                }
                AiActionType::AddHist2D => {
                    !action.name.is_empty()
                        || !action.x_column_name.is_empty()
                        || !action.y_column_name.is_empty()
                        || action.x_bins > 0
                        || action.y_bins > 0
                }
            };

            keep.then_some(action)
        })
        .collect();

    response.notes.extend(recovery_notes);
    response.notes = dedupe_strings(response.notes);

    if !response.clarification_questions.is_empty() {
        if response.summary.is_empty() {
            response.summary =
                "Need clarification before applying any Spectrix actions.".to_owned();
        }
        if !response.actions.is_empty() {
            response.actions.clear();
            response.notes.push(
                "Discarded generated actions because the response also requested clarification."
                    .to_owned(),
            );
            response.notes = dedupe_strings(response.notes);
        }
    } else if response.summary.is_empty() {
        response.summary = "Generated Spectrix actions.".to_owned();
    }

    response
}

fn fill_histogram_action_defaults(
    action: &mut AiAction,
    prompt_hints: &PromptHistogramHints,
    snapshot: &AiContextSnapshot,
    recovery_notes: &mut Vec<String>,
) {
    match action.action_type {
        AiActionType::AddHist1D => {
            if let Some(resolved_column) = resolve_histogram_1d_column(action, snapshot) {
                if action.column_name != resolved_column {
                    action.column_name = resolved_column;
                    recovery_notes.push(
                        "Recovered missing or non-canonical 1D histogram columns from the available dataset columns."
                            .to_owned(),
                    );
                }
            }

            if action.name.is_empty() && !action.column_name.is_empty() {
                action.name = default_1d_histogram_name(&action.column_name, &action.cuts);
                recovery_notes.push(
                    "Filled missing 1D histogram names from the selected column names.".to_owned(),
                );
            }

            if let Some(tab_name) = &prompt_hints.tab_name {
                if !action.name.is_empty() && !action.name.contains('/') {
                    action.name = format!("{tab_name}/{}", action.name);
                    recovery_notes.push(
                        "Placed histogram names into the requested tab/container from the prompt."
                            .to_owned(),
                    );
                }
            }

            if action.bins == 0 {
                action.bins = prompt_hints.shared_bins.unwrap_or(DEFAULT_1D_BINS);
                recovery_notes.push(if prompt_hints.shared_bins.is_some() {
                    "Filled missing 1D histogram bin counts from the request text.".to_owned()
                } else {
                    "Filled missing 1D histogram bin counts with the Spectrix default of 512."
                        .to_owned()
                });
            }

            if range_needs_default(action.range) {
                action.range = prompt_hints.shared_range.unwrap_or(DEFAULT_1D_RANGE);
                recovery_notes.push(if prompt_hints.shared_range.is_some() {
                    "Filled missing 1D histogram ranges from the request text.".to_owned()
                } else {
                    "Filled missing 1D histogram ranges with the Spectrix default of [0, 4096]."
                        .to_owned()
                });
            }
        }
        AiActionType::AddHist2D => {
            if let Some((x_column_name, y_column_name)) =
                resolve_histogram_2d_columns(action, snapshot)
            {
                if action.x_column_name != x_column_name {
                    action.x_column_name = x_column_name;
                    recovery_notes.push(
                        "Recovered missing or non-canonical 2D histogram X columns from the available dataset columns."
                            .to_owned(),
                    );
                }

                if action.y_column_name != y_column_name {
                    action.y_column_name = y_column_name;
                    recovery_notes.push(
                        "Recovered missing or non-canonical 2D histogram Y columns from the available dataset columns."
                            .to_owned(),
                    );
                }
            }

            if action.name.is_empty()
                && !action.x_column_name.is_empty()
                && !action.y_column_name.is_empty()
            {
                action.name = default_2d_histogram_name(
                    &action.x_column_name,
                    &action.y_column_name,
                    &action.cuts,
                );
                recovery_notes.push(
                    "Filled missing 2D histogram names from the selected axis column names."
                        .to_owned(),
                );
            }

            if let Some(tab_name) = &prompt_hints.tab_name {
                if !action.name.is_empty() && !action.name.contains('/') {
                    action.name = format!("{tab_name}/{}", action.name);
                    recovery_notes.push(
                        "Placed histogram names into the requested tab/container from the prompt."
                            .to_owned(),
                    );
                }
            }

            if action.x_bins == 0 {
                action.x_bins = DEFAULT_2D_BINS;
                recovery_notes.push(
                    "Filled missing 2D histogram X bin counts with the Spectrix default of 512."
                        .to_owned(),
                );
            }

            if action.y_bins == 0 {
                action.y_bins = DEFAULT_2D_BINS;
                recovery_notes.push(
                    "Filled missing 2D histogram Y bin counts with the Spectrix default of 512."
                        .to_owned(),
                );
            }

            if range_needs_default(action.x_range) {
                action.x_range = DEFAULT_2D_RANGE;
                recovery_notes.push(
                    "Filled missing 2D histogram X ranges with the Spectrix default of [0, 4096]."
                        .to_owned(),
                );
            }

            if range_needs_default(action.y_range) {
                action.y_range = DEFAULT_2D_RANGE;
                recovery_notes.push(
                    "Filled missing 2D histogram Y ranges with the Spectrix default of [0, 4096]."
                        .to_owned(),
                );
            }
        }
        AiActionType::AddColumn | AiActionType::AddCut1D => {}
    }
}

fn default_1d_histogram_name(column_name: &str, cuts: &[String]) -> String {
    if cuts.is_empty() {
        column_name.to_owned()
    } else {
        format!("{column_name}-{}", cuts.join("-"))
    }
}

fn default_2d_histogram_name(x_column_name: &str, y_column_name: &str, cuts: &[String]) -> String {
    let base_name = format!("{y_column_name} vs {x_column_name}");
    if cuts.is_empty() {
        base_name
    } else {
        format!("{base_name}-{}", cuts.join("-"))
    }
}

fn range_needs_default(range: [f64; 2]) -> bool {
    !range[0].is_finite() || !range[1].is_finite() || range[0] >= range[1]
}

fn parse_prompt_histogram_hints(prompt: &str) -> PromptHistogramHints {
    PromptHistogramHints {
        shared_bins: parse_unique_bins(prompt),
        shared_range: parse_unique_range(prompt),
        tab_name: parse_requested_tab_name(prompt),
    }
}

fn resolve_histogram_1d_column(action: &AiAction, snapshot: &AiContextSnapshot) -> Option<String> {
    if let Some(column_name) = resolve_available_column(&action.column_name, snapshot) {
        return Some(column_name);
    }

    infer_column_from_histogram_name(&action.name, snapshot)
}

fn resolve_histogram_2d_columns(
    action: &AiAction,
    snapshot: &AiContextSnapshot,
) -> Option<(String, String)> {
    let x_column_name = resolve_available_column(&action.x_column_name, snapshot);
    let y_column_name = resolve_available_column(&action.y_column_name, snapshot);

    if let (Some(x_column_name), Some(y_column_name)) =
        (x_column_name.clone(), y_column_name.clone())
    {
        return Some((x_column_name, y_column_name));
    }

    let display_name = action
        .name
        .rsplit('/')
        .next()
        .unwrap_or(action.name.as_str())
        .trim();
    let Some((left, right)) = display_name.split_once(" vs ") else {
        return None;
    };

    let inferred_y = if let Some(y_column_name) = y_column_name {
        Some(y_column_name)
    } else {
        resolve_available_column(left, snapshot)
    };
    let inferred_x = if let Some(x_column_name) = x_column_name {
        Some(x_column_name)
    } else {
        resolve_available_column(right, snapshot)
    };

    match (inferred_x, inferred_y) {
        (Some(x_column_name), Some(y_column_name)) => Some((x_column_name, y_column_name)),
        _ => None,
    }
}

fn infer_column_from_histogram_name(name: &str, snapshot: &AiContextSnapshot) -> Option<String> {
    let display_name = name.rsplit('/').next().unwrap_or(name).trim();
    if display_name.is_empty() {
        return None;
    }

    if let Some(column_name) = resolve_available_column(display_name, snapshot) {
        return Some(column_name);
    }

    let prefix_matches = snapshot
        .available_columns()
        .into_iter()
        .filter(|column_name| display_name.starts_with(&format!("{column_name}-")))
        .collect::<Vec<_>>();

    if prefix_matches.len() == 1 {
        prefix_matches.into_iter().next()
    } else {
        None
    }
}

fn resolve_available_column(candidate: &str, snapshot: &AiContextSnapshot) -> Option<String> {
    let candidate = candidate.trim();
    if candidate.is_empty() {
        return None;
    }

    let available_columns = snapshot.available_columns();
    if available_columns.contains(candidate) {
        return Some(candidate.to_owned());
    }

    let lowercase_candidate = candidate.to_lowercase();
    let case_insensitive_matches = available_columns
        .iter()
        .filter(|column_name| column_name.to_lowercase() == lowercase_candidate)
        .cloned()
        .collect::<Vec<_>>();
    if case_insensitive_matches.len() == 1 {
        return case_insensitive_matches.into_iter().next();
    }

    let normalized_candidate = normalize_column_key(candidate);
    let normalized_matches = available_columns
        .iter()
        .filter(|column_name| normalize_column_key(column_name) == normalized_candidate)
        .cloned()
        .collect::<Vec<_>>();
    if normalized_matches.len() == 1 {
        normalized_matches.into_iter().next()
    } else {
        None
    }
}

fn normalize_column_key(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_unique_bins(prompt: &str) -> Option<usize> {
    let regex = regex::Regex::new(r"(?i)\b(\d+)\s*bins?\b").expect("failed to build bins regex");
    let values = regex
        .captures_iter(prompt)
        .filter_map(|captures| captures.get(1))
        .filter_map(|capture| capture.as_str().parse::<usize>().ok())
        .collect::<BTreeSet<_>>();

    if values.len() == 1 {
        values.into_iter().next()
    } else {
        None
    }
}

fn parse_unique_range(prompt: &str) -> Option<[f64; 2]> {
    let patterns = [
        r"(?ix)(?:range(?:\s+of)?|from)\s*([+-]?\s*\d+(?:\.\d+)?)\s*(?:to|through)\s*([+-]?\s*\d+(?:\.\d+)?)",
        r"(?ix)between\s*([+-]?\s*\d+(?:\.\d+)?)\s*and\s*([+-]?\s*\d+(?:\.\d+)?)",
    ];

    let mut ranges = Vec::new();
    for pattern in patterns {
        let regex = regex::Regex::new(pattern).expect("failed to build range regex");
        for captures in regex.captures_iter(prompt) {
            let Some(min_capture) = captures.get(1) else {
                continue;
            };
            let Some(max_capture) = captures.get(2) else {
                continue;
            };
            let Some(min) = parse_prompt_number(min_capture.as_str()) else {
                continue;
            };
            let Some(max) = parse_prompt_number(max_capture.as_str()) else {
                continue;
            };
            let range = [min, max];
            if !ranges.contains(&range) {
                ranges.push(range);
            }
        }
    }

    if ranges.len() == 1 {
        ranges.into_iter().next()
    } else {
        None
    }
}

fn parse_requested_tab_name(prompt: &str) -> Option<String> {
    let patterns = [
        r#"(?i)\b(?:tab|container)\s+(?:called|named)\s+["']?([A-Za-z0-9 _/\-]+?)["']?(?:[.,]|$)"#,
        r#"(?i)\bin\s+(?:a\s+)?(?:tab|container)\s+["']?([A-Za-z0-9 _/\-]+?)["']?(?:[.,]|$)"#,
    ];

    for pattern in patterns {
        let regex = regex::Regex::new(pattern).expect("failed to build tab-name regex");
        if let Some(captures) = regex.captures(prompt)
            && let Some(tab_name) = captures.get(1)
        {
            let tab_name = tab_name.as_str().trim().trim_matches(['"', '\'']);
            if !tab_name.is_empty() {
                return Some(tab_name.to_owned());
            }
        }
    }

    None
}

fn parse_prompt_number(token: &str) -> Option<f64> {
    token.replace(' ', "").parse::<f64>().ok()
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.to_lowercase()))
        .collect()
}

fn default_system_prompt() -> &'static str {
    DEFAULT_SYSTEM_PROMPT.trim()
}

fn normalize_help_response(mut response: AiResponse, prompt: &str) -> AiResponse {
    response.summary = response.summary.trim().to_owned();
    response.clarification_questions = dedupe_strings(response.clarification_questions);
    response.notes = dedupe_strings(response.notes);

    if !response.actions.is_empty() {
        response.actions.clear();
        response.notes.push(
            "AI action-taking in Spectrix is currently under development, so any returned actions were ignored."
                .to_owned(),
        );
    }

    if prompt_requests_actions(prompt) {
        response.actions.clear();
        response.clarification_questions.clear();
        response.summary = "AI action-taking in Spectrix is currently under development. Ask how to do it manually and I can walk you through the UI.".to_owned();
        response.notes.push(
            "The current assistant is limited to usage guidance and cannot create or apply Spectrix changes yet."
                .to_owned(),
        );
    } else if response.summary.is_empty() {
        response.summary = if response.clarification_questions.is_empty() {
            "I did not get a usable help answer back from the model. Try rephrasing the question."
                .to_owned()
        } else {
            "I need a little more detail to answer that clearly.".to_owned()
        };
    }

    response.notes = dedupe_strings(response.notes);
    response
}

fn prompt_requests_actions(prompt: &str) -> bool {
    let normalized = prompt.trim().to_lowercase();
    if normalized.is_empty() {
        return false;
    }

    let question_starters = [
        "how do",
        "how can",
        "what is",
        "what are",
        "where is",
        "where do",
        "why does",
        "can i",
        "could i",
        "does spectrix",
        "is there",
    ];
    if normalized.contains('?')
        || question_starters
            .iter()
            .any(|starter| normalized.starts_with(starter))
    {
        return false;
    }

    let action_starters = [
        "build ",
        "create ",
        "make ",
        "generate ",
        "add ",
        "apply ",
        "set up ",
        "configure ",
        "fit ",
        "plot ",
        "save ",
        "combine ",
        "filter ",
        "do ",
    ];

    action_starters
        .iter()
        .any(|starter| normalized.starts_with(starter))
}

fn build_follow_up_prompt_template(existing_prompt: &str, questions: &[String]) -> String {
    let mut template = String::new();
    let existing_prompt = existing_prompt.trim();
    if !existing_prompt.is_empty() {
        template.push_str(existing_prompt);
        template.push_str("\n\n");
    }

    template.push_str("Clarification answers:\n");
    for question in questions {
        template.push_str("- ");
        template.push_str(question.trim());
        template.push_str(": \n");
    }

    template
}

fn build_user_prompt(prompt: &str) -> Result<String, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Ask a question or describe what help you need in Spectrix.".to_owned());
    }

    let readme = README_HELP_TEXT.trim();

    Ok(format!(
        "User request:\n{trimmed}\n\nSpectrix help context summary:\n- Use the README below as the main reference for Spectrix behavior.\n- Prefer concrete UI steps and current UI labels from the README.\n- AI action-taking inside Spectrix is currently under development, so procedural questions should be answered with manual steps.\n\nSpectrix README.md:\n{readme}\n"
    ))
}

fn response_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["summary", "clarification_questions", "notes", "actions"],
        "properties": {
            "summary": { "type": "string" },
            "clarification_questions": {
                "type": "array",
                "items": { "type": "string" }
            },
            "notes": {
                "type": "array",
                "items": { "type": "string" }
            },
            "actions": {
                "type": "array",
                "items": {
                    "type": "object"
                },
                "maxItems": 0
            }
        }
    })
}

fn prettify_json(raw_json: &str) -> String {
    serde_json::from_str::<Value>(raw_json)
        .and_then(|value| serde_json::to_string_pretty(&value))
        .unwrap_or_else(|_| raw_json.to_owned())
}

fn extract_output_text(response_json: &Value) -> Result<String, String> {
    let output = response_json
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| "The OpenAI API response did not include an `output` array.".to_owned())?;

    for item in output {
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };

        for content_item in content {
            match content_item.get("type").and_then(Value::as_str) {
                Some("output_text") => {
                    if let Some(text) = content_item.get("text").and_then(Value::as_str) {
                        return Ok(text.to_owned());
                    }
                }
                Some("refusal") => {
                    let refusal = content_item
                        .get("refusal")
                        .and_then(Value::as_str)
                        .unwrap_or("The model refused to produce a response.");
                    return Err(format!("The model refused the request: {refusal}"));
                }
                _ => {}
            }
        }
    }

    Err("The OpenAI API response did not include any output text.".to_owned())
}

fn list_models(provider: AiProvider, base_url: &str) -> Result<Vec<String>, String> {
    let mut models = match provider {
        AiProvider::Ollama => list_ollama_models(base_url)
            .map_err(|error| format!("Ollama model listing failed: {error}"))?,
        AiProvider::OpenAiCompatible => list_openai_compatible_models(base_url, None)
            .map_err(|error| format!("OpenAI-compatible model listing failed: {error}"))?,
        AiProvider::OpenAi => {
            return Err("Local model listing is only available for local providers.".to_owned());
        }
    };

    models.sort();
    models.dedup();
    Ok(models)
}

fn call_openai_responses_api(
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    schema: &Value,
) -> PyResult<String> {
    let schema_json = serde_json::to_string(schema).map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Failed to serialize the Spectrix response schema: {error}"
        ))
    })?;

    Python::attach(|py| {
        let code = c_str!(
            r#"
import json
import urllib.error
import urllib.request


def create_response(base_url, api_key, model, system_prompt, user_prompt, schema_json):
    schema = json.loads(schema_json)
    payload = {
        "model": model,
        "store": False,
        "max_output_tokens": 2500,
        "input": [
            {
                "role": "system",
                "content": [
                    {
                        "type": "input_text",
                        "text": system_prompt,
                    }
                ],
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": user_prompt,
                    }
                ],
            },
        ],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "spectrix_ai_actions",
                "schema": schema,
                "strict": True,
            }
        },
    }

    request_url = base_url.rstrip("/") + "/responses"
    headers = {
        "Content-Type": "application/json",
    }
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    request = urllib.request.Request(
        request_url,
        data=json.dumps(payload).encode("utf-8"),
        headers=headers,
        method="POST",
    )

    try:
        with urllib.request.urlopen(request, timeout=90) as response:
            return response.read().decode("utf-8")
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Network error: {error}") from error
"#
        );

        let module = PyModule::from_code(py, code, c_str!("openai_api.py"), c_str!("openai_api"))?;
        module
            .getattr("create_response")?
            .call1((
                base_url,
                api_key,
                model,
                system_prompt,
                user_prompt,
                schema_json,
            ))?
            .extract()
    })
}

fn list_openai_compatible_models(base_url: &str, api_key: Option<&str>) -> PyResult<Vec<String>> {
    Python::attach(|py| {
        let code = c_str!(
            r#"
import json
import urllib.error
import urllib.request


def list_models(base_url, api_key):
    request_url = base_url.rstrip("/") + "/models"
    headers = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    request = urllib.request.Request(
        request_url,
        headers=headers,
        method="GET",
    )

    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Network error: {error}") from error

    data = body.get("data")
    if not isinstance(data, list):
        raise RuntimeError(f"Model list missing `data`: {body}")

    models = []
    for item in data:
        if isinstance(item, dict):
            model_id = item.get("id")
            if isinstance(model_id, str) and model_id:
                models.append(model_id)
    return models
"#
        );

        let module = PyModule::from_code(
            py,
            code,
            c_str!("openai_model_list.py"),
            c_str!("openai_model_list"),
        )?;
        module
            .getattr("list_models")?
            .call1((base_url, api_key))?
            .extract()
    })
}

fn list_ollama_models(base_url: &str) -> PyResult<Vec<String>> {
    Python::attach(|py| {
        let code = c_str!(
            r#"
import json
import urllib.error
import urllib.request


def list_models(base_url):
    request = urllib.request.Request(
        base_url.rstrip("/") + "/api/tags",
        headers={
            "Content-Type": "application/json",
        },
        method="GET",
    )

    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Network error: {error}") from error

    entries = body.get("models")
    if not isinstance(entries, list):
        raise RuntimeError(f"Model list missing `models`: {body}")

    models = []
    for item in entries:
        if isinstance(item, dict):
            name = item.get("name") or item.get("model")
            if isinstance(name, str) and name:
                models.append(name)
    return models
"#
        );

        let module = PyModule::from_code(
            py,
            code,
            c_str!("ollama_model_list.py"),
            c_str!("ollama_model_list"),
        )?;
        module.getattr("list_models")?.call1((base_url,))?.extract()
    })
}

fn call_ollama_generate_api(
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    schema: &Value,
) -> PyResult<String> {
    let schema_json = serde_json::to_string(schema).map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Failed to serialize the Spectrix response schema: {error}"
        ))
    })?;

    Python::attach(|py| {
        let code = c_str!(
            r#"
import json
import urllib.error
import urllib.request


def create_response(base_url, model, system_prompt, user_prompt, schema_json):
    payload = {
        "model": model,
        "system": system_prompt,
        "prompt": user_prompt,
        "stream": False,
        "format": json.loads(schema_json),
    }

    request = urllib.request.Request(
        base_url.rstrip("/") + "/api/generate",
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(request, timeout=90) as response:
            body = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Network error: {error}") from error

    output = body.get("response")
    if not isinstance(output, str):
        raise RuntimeError(f"Ollama response missing `response` text: {body}")
    return output
"#
        );

        let module = PyModule::from_code(py, code, c_str!("ollama_api.py"), c_str!("ollama_api"))?;
        module
            .getattr("create_response")?
            .call1((base_url, model, system_prompt, user_prompt, schema_json))?
            .extract()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_with_columns(columns: &[&str]) -> AiContextSnapshot {
        AiContextSnapshot {
            selected_files: Vec::new(),
            loaded_columns: columns.iter().map(|column| (*column).to_owned()).collect(),
            existing_derived_columns: Vec::new(),
            general_cut_names: Vec::new(),
            active_histogram_cut_names: Vec::new(),
            existing_histograms: Vec::new(),
        }
    }

    #[test]
    fn normalization_discards_actions_when_questions_exist() {
        let response = AiResponse {
            summary: String::new(),
            clarification_questions: vec!["Which X column?".to_owned()],
            notes: Vec::new(),
            actions: vec![AiAction {
                action_type: AiActionType::AddHist1D,
                name: "Xavg".to_owned(),
                column_name: "Xavg".to_owned(),
                range: [0.0, 4096.0],
                bins: 512,
                ..AiAction::default()
            }],
        };

        let normalized = normalize_response(response, "", &snapshot_with_columns(&["Xavg"]));
        assert!(normalized.actions.is_empty());
        assert_eq!(normalized.clarification_questions.len(), 1);
        assert!(!normalized.summary.is_empty());
    }

    #[test]
    fn normalization_recovers_missing_1d_histogram_fields_from_prompt() {
        let response = AiResponse {
            summary: "Build focal plane histograms.".to_owned(),
            clarification_questions: Vec::new(),
            notes: Vec::new(),
            actions: vec![
                AiAction {
                    action_type: AiActionType::AddHist1D,
                    column_name: "X1".to_owned(),
                    range: [-300.0, 300.0],
                    ..AiAction::default()
                },
                AiAction {
                    action_type: AiActionType::AddHist1D,
                    column_name: "X2".to_owned(),
                    range: [-300.0, 300.0],
                    ..AiAction::default()
                },
                AiAction {
                    action_type: AiActionType::AddHist1D,
                    column_name: "Xavg".to_owned(),
                    range: [-300.0, 300.0],
                    ..AiAction::default()
                },
            ],
        };

        let normalized = normalize_response(
            response,
            "build a 1d histogram of the focal plane channels (X1, X2, and Xavg) all with a range of - 300 to 300 with 600 bins. treat them as seperate histograms.",
            &snapshot_with_columns(&["X1", "X2", "Xavg"]),
        );

        assert_eq!(normalized.actions.len(), 3);
        assert_eq!(normalized.actions[0].name, "X1");
        assert_eq!(normalized.actions[1].name, "X2");
        assert_eq!(normalized.actions[2].name, "Xavg");
        assert!(normalized.actions.iter().all(|action| action.bins == 600));
        assert!(
            normalized
                .notes
                .iter()
                .any(|note| note.contains("bin counts from the request text"))
        );
    }

    #[test]
    fn normalization_applies_default_histogram_values_when_prompt_omits_them() {
        let response = AiResponse {
            summary: "Build one histogram.".to_owned(),
            clarification_questions: Vec::new(),
            notes: Vec::new(),
            actions: vec![AiAction {
                action_type: AiActionType::AddHist1D,
                column_name: "Xavg".to_owned(),
                ..AiAction::default()
            }],
        };

        let normalized = normalize_response(
            response,
            "build a 1d histogram of Xavg",
            &snapshot_with_columns(&["Xavg"]),
        );

        assert_eq!(normalized.actions.len(), 1);
        assert_eq!(normalized.actions[0].name, "Xavg");
        assert_eq!(normalized.actions[0].bins, DEFAULT_1D_BINS);
        assert_eq!(normalized.actions[0].range, DEFAULT_1D_RANGE);
    }

    #[test]
    fn normalization_recovers_histogram_columns_from_names_and_requested_tab() {
        let snapshot =
            snapshot_with_columns(&["CATRINA0Energy", "CATRINA1Energy", "CATRINA2Energy"]);
        let response = AiResponse {
            summary: "Build detector histograms.".to_owned(),
            clarification_questions: Vec::new(),
            notes: Vec::new(),
            actions: vec![
                AiAction {
                    action_type: AiActionType::AddHist1D,
                    name: "CATRINA0Energy".to_owned(),
                    ..AiAction::default()
                },
                AiAction {
                    action_type: AiActionType::AddHist1D,
                    name: "CATRINA1Energy".to_owned(),
                    ..AiAction::default()
                },
                AiAction {
                    action_type: AiActionType::AddHist1D,
                    name: "CATRINA2Energy".to_owned(),
                    ..AiAction::default()
                },
            ],
        };

        let normalized = normalize_response(
            response,
            "i want to view the energy of all the catrina detectors. place all the histograms in a tab called Detectors.",
            &snapshot,
        );

        assert_eq!(normalized.actions.len(), 3);
        assert_eq!(normalized.actions[0].column_name, "CATRINA0Energy");
        assert_eq!(normalized.actions[1].column_name, "CATRINA1Energy");
        assert_eq!(normalized.actions[2].column_name, "CATRINA2Energy");
        assert_eq!(normalized.actions[0].name, "Detectors/CATRINA0Energy");
        assert_eq!(normalized.actions[1].name, "Detectors/CATRINA1Energy");
        assert_eq!(normalized.actions[2].name, "Detectors/CATRINA2Energy");
    }

    #[test]
    fn validation_allows_cut_then_histogram_action_flow() {
        let snapshot = snapshot_with_columns(&["Xavg", "X1"]);
        let response = AiResponse {
            summary: "Build a gated histogram.".to_owned(),
            clarification_questions: Vec::new(),
            notes: Vec::new(),
            actions: vec![
                AiAction {
                    action_type: AiActionType::AddCut1D,
                    name: "X1 gate".to_owned(),
                    expression: "X1 > 10".to_owned(),
                    ..AiAction::default()
                },
                AiAction {
                    action_type: AiActionType::AddHist1D,
                    name: "Xavg gated".to_owned(),
                    column_name: "Xavg".to_owned(),
                    range: [0.0, 4096.0],
                    bins: 512,
                    cuts: vec!["X1 gate".to_owned()],
                    ..AiAction::default()
                },
            ],
        };

        let validation = response.validate(&snapshot);
        assert!(validation.errors.is_empty());
        assert!(validation.clarification_questions.is_empty());
    }

    #[test]
    fn serde_accepts_schema_action_names() {
        let raw = r#"{
            "summary": "Build one histogram.",
            "clarification_questions": [],
            "notes": [],
            "actions": [
                {
                    "type": "add_hist_1d",
                    "name": "Xavg",
                    "column_name": "Xavg",
                    "range": [0, 4096],
                    "bins": 512,
                    "cuts": []
                }
            ]
        }"#;

        let response: AiResponse =
            serde_json::from_str(raw).expect("schema action names should deserialize");
        assert_eq!(response.actions.len(), 1);
        assert_eq!(response.actions[0].action_type, AiActionType::AddHist1D);
    }

    #[test]
    fn build_user_prompt_includes_readme_context() {
        let prompt = build_user_prompt("How do I create a 1D cut in Spectrix?")
            .expect("help prompt should build");

        assert!(prompt.contains("Spectrix README.md:"));
        assert!(prompt.contains("# Spectrix"));
        assert!(prompt.contains("Histogram Script"));
    }

    #[test]
    fn apply_adds_actions_into_histogram_script() {
        let response = AiResponse {
            summary: "Build a gated histogram.".to_owned(),
            clarification_questions: Vec::new(),
            notes: Vec::new(),
            actions: vec![
                AiAction {
                    action_type: AiActionType::AddColumn,
                    alias: "Xsum".to_owned(),
                    expression: "X1 + X2".to_owned(),
                    ..AiAction::default()
                },
                AiAction {
                    action_type: AiActionType::AddCut1D,
                    name: "Xsum gate".to_owned(),
                    expression: "Xsum > 10".to_owned(),
                    ..AiAction::default()
                },
                AiAction {
                    action_type: AiActionType::AddHist1D,
                    name: "Xsum".to_owned(),
                    column_name: "Xsum".to_owned(),
                    range: [0.0, 4096.0],
                    bins: 512,
                    cuts: vec!["Xsum gate".to_owned()],
                    ..AiAction::default()
                },
            ],
        };

        let mut script = HistogramScript::default();
        let report = response
            .apply_to(&mut script, &[])
            .expect("actions should apply");

        assert_eq!(report.added_columns, 1);
        assert_eq!(report.added_cuts, 1);
        assert_eq!(report.added_histograms, 1);
        assert_eq!(script.configs.columns.len(), 1);
        assert_eq!(script.configs.cuts.cuts.len(), 1);
        assert_eq!(script.configs.configs.len(), 1);
    }
}
