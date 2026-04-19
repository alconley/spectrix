use super::context::AiContextSnapshot;
use super::prompt::{build_user_prompt, default_system_prompt};
use super::provider::{AiProvider, AiUsage, generate_text, list_models};
use super::response::{
    ConversationMessage, ConversationRole, format_assistant_message, normalize_help_response,
    parse_model_response, response_schema,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_CONVERSATION_MESSAGES: usize = 24;
const MAX_STORED_MESSAGE_CHARS: usize = 8_000;
// The provider abstraction remains in provider.rs; the current UI intentionally
// pins the assistant to Ollama until another provider is product-ready.
const ACTIVE_PROVIDER: AiProvider = AiProvider::Ollama;

type PendingGenerationResult = Result<PendingGeneration, String>;
type PendingModelList = Result<Vec<String>, String>;

#[derive(Debug)]
struct PendingGeneration {
    response_text: String,
    model: String,
    usage: Option<AiUsage>,
}

#[derive(Debug, Default)]
struct AiSessionUsage {
    requests: usize,
    cloud_requests: usize,
    prompt_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
    total_duration_ms: f64,
    load_duration_ms: f64,
    last_model: Option<String>,
    last_usage: Option<AiUsage>,
}

impl AiSessionUsage {
    fn record(&mut self, model: &str, usage: Option<AiUsage>) {
        self.requests += 1;
        if is_ollama_cloud_model(model) {
            self.cloud_requests += 1;
        }

        if let Some(usage) = usage {
            self.prompt_tokens += usage.prompt_tokens.unwrap_or(0);
            self.output_tokens += usage.output_tokens.unwrap_or(0);
            self.total_tokens += usage.total_tokens.unwrap_or_else(|| {
                usage.prompt_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0)
            });
            self.total_duration_ms += usage.total_duration_ms.unwrap_or(0.0);
            self.load_duration_ms += usage.load_duration_ms.unwrap_or(0.0);
            self.last_usage = Some(usage);
        } else {
            self.last_usage = None;
        }

        self.last_model = Some(model.to_owned());
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AiAssistant {
    provider: AiProvider,
    base_url: String,
    model: String,
    system_prompt: String,
    #[serde(skip)]
    input: String,
    #[serde(skip)]
    messages: Vec<ConversationMessage>,
    #[serde(skip)]
    discovered_models: Vec<String>,
    #[serde(skip)]
    last_error: Option<String>,
    #[serde(skip)]
    last_model_error: Option<String>,
    #[serde(skip)]
    generating: Arc<AtomicBool>,
    #[serde(skip)]
    pending_result: Arc<Mutex<Option<PendingGenerationResult>>>,
    #[serde(skip)]
    listing_models: Arc<AtomicBool>,
    #[serde(skip)]
    pending_models: Arc<Mutex<Option<PendingModelList>>>,
    #[serde(skip)]
    session_usage: AiSessionUsage,
    #[serde(skip)]
    initial_model_refresh_started: bool,
}

impl Default for AiAssistant {
    fn default() -> Self {
        let provider = ACTIVE_PROVIDER;
        Self {
            provider,
            base_url: provider.default_base_url().to_owned(),
            model: provider.default_model().to_owned(),
            system_prompt: default_system_prompt().to_owned(),
            input: String::new(),
            messages: Vec::new(),
            discovered_models: Vec::new(),
            last_error: None,
            last_model_error: None,
            generating: Arc::new(AtomicBool::new(false)),
            pending_result: Arc::new(Mutex::new(None)),
            listing_models: Arc::new(AtomicBool::new(false)),
            pending_models: Arc::new(Mutex::new(None)),
            session_usage: AiSessionUsage::default(),
            initial_model_refresh_started: false,
        }
    }
}

impl AiAssistant {
    pub fn is_busy(&self) -> bool {
        self.generating.load(Ordering::Relaxed) || self.listing_models.load(Ordering::Relaxed)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, snapshot: AiContextSnapshot) {
        self.ensure_ollama_provider();
        self.take_pending_result();
        self.take_pending_models();
        self.maybe_start_initial_model_refresh();

        ui.heading("Spectrix AI");
        ui.label(
            egui::RichText::new(
                "Ask usage questions. The assistant searches Spectrix source before answering.",
            )
            .weak()
            .small(),
        );

        ui.add_space(4.0);
        self.setup_ui(ui);

        ui.separator();
        self.composer_ui(ui, snapshot);

        ui.separator();
        self.conversation_ui(ui);

        if self.is_busy() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
    }

    fn setup_ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("AI Setup")
            .default_open(self.model.trim().is_empty())
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("Provider: {}", ACTIVE_PROVIDER.label())).strong(),
                );
                ui.label(
                    egui::RichText::new(ACTIVE_PROVIDER.help_text())
                        .weak()
                        .small(),
                );

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Ollama URL:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.base_url)
                            .hint_text(ACTIVE_PROVIDER.default_base_url())
                            .desired_width(260.0),
                    );
                });

                ui.add_space(4.0);
                self.model_listing_ui(ui);

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Model:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.model)
                            .hint_text(ACTIVE_PROVIDER.model_hint())
                            .desired_width(220.0),
                    );
                });

                ui.add_space(4.0);
                self.ollama_model_picker_ui(ui);

                if self.model.trim().is_empty() {
                    ui.add_space(4.0);
                    Self::getting_started_ui(ui);
                }
            });
    }

    fn model_listing_ui(&mut self, ui: &mut egui::Ui) {
        let list_models_enabled =
            !self.base_url.trim().is_empty() && !self.listing_models.load(Ordering::Relaxed);

        ui.horizontal_wrapped(|ui| {
            let response = ui.add_enabled(
                list_models_enabled,
                egui::Button::new("Refresh Ollama Models"),
            );
            let response = if list_models_enabled {
                response.on_hover_text(
                    "Query installed local models through the configured Ollama server.",
                )
            } else if self.base_url.trim().is_empty() {
                response.on_disabled_hover_text("Enter the Ollama URL first.")
            } else {
                response.on_disabled_hover_text("A model list request is already running.")
            };

            if response.clicked() {
                self.refresh_model_lists();
            }

            if self.listing_models.load(Ordering::Relaxed) {
                ui.label("Loading models...");
                ui.add(egui::widgets::Spinner::default());
            } else {
                ui.label(format!(
                    "{} installed local",
                    self.discovered_models
                        .iter()
                        .filter(|model| !is_ollama_cloud_model(model))
                        .count()
                ));
            }
        });

        if let Some(model_error) = &self.last_model_error {
            ui.colored_label(egui::Color32::LIGHT_RED, model_error);
        }
    }

    fn model_button_group(&mut self, ui: &mut egui::Ui, label: &str, models: &[String]) {
        ui.label(egui::RichText::new(label).strong().small());
        ui.horizontal_wrapped(|ui| {
            for model in models {
                let selected = self.model.trim() == model;
                if ui.selectable_label(selected, model).clicked() {
                    self.model = model.clone();
                }
            }
        });
    }

    fn ollama_model_picker_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Installed local models")
                .strong()
                .small(),
        );

        ui.horizontal_wrapped(|ui| {
            ui.hyperlink_to("Ollama settings", "https://ollama.com/settings");
            ui.hyperlink_to(
                "Browse Ollama models",
                "https://ollama.com/library?sort=popular",
            );
        });

        ui.label(
            egui::RichText::new(
                "Default cloud model: qwen3-coder:480b-cloud. This model was used for all testing of Spectrix. Use the Ollama library link to find another cloud model name if you want one.",
            )
            .weak()
            .small(),
        );

        let local_models = ollama_local_quick_pick_models(&self.discovered_models);

        if !local_models.is_empty() {
            self.model_button_group(ui, "Local models downloaded", &local_models);
        } else {
            ui.label(
                egui::RichText::new(
                    "No installed local models are listed yet. Click Refresh Ollama Models, or pull one with `ollama pull <model-name>`.",
                )
                .weak()
                .small(),
            );
        }

        if self.model.trim().is_empty() {
            ui.label(
                egui::RichText::new(
                    "Select an available quick pick, choose a discovered model, or type any Ollama model name.",
                )
                .weak()
                .small(),
            );
        } else if is_ollama_cloud_model(&self.model) {
            ui.label(
                egui::RichText::new(
                    "Ollama Cloud model selected. Run `ollama signin` before sending; for other cloud names, use the Ollama library link above.",
                )
                .weak()
                .small(),
            );
        } else if !self.discovered_models.is_empty()
            && !self
                .discovered_models
                .iter()
                .any(|model| model == self.model.trim())
        {
            ui.label(
                egui::RichText::new(
                    "This local model was typed manually and is not in the latest Ollama model list. If Send fails, pull it or click Refresh Ollama Models again.",
                )
                .weak()
                .small(),
            );
        }
    }

    fn getting_started_ui(ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("No AI model is selected yet.").strong());

        ui.label("To get started with Ollama:");
        ui.label("1. Install and start Ollama.");
        ui.label("2. Sign in to Ollama for the default cloud model.");
        ui.monospace("ollama signin");
        ui.label("3. Use qwen3-coder:480b-cloud, browse Ollama for another cloud model, or pull a local model.");
        ui.hyperlink_to(
            "Browse Ollama models",
            "https://ollama.com/library?sort=popular",
        );
        ui.monospace("ollama pull <model-name>");
        ui.label(
            "4. Click Refresh Ollama Models if you want to choose from models Ollama reports.",
        );
    }

    fn conversation_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Conversation").strong());
            if ui.button("Clear").clicked() {
                self.messages.clear();
                self.last_error = None;
            }
        });

        let conversation_height = ui.available_height() - 30.0;
        egui::Frame::NONE
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .inner_margin(egui::Margin::same(6))
            .show(ui, |ui| {
                ui.set_min_height(conversation_height);
                ui.set_max_height(conversation_height);
                egui::ScrollArea::vertical()
                    .id_salt("spectrix_ai_conversation_scroll")
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.messages.is_empty() {
                            ui.label(
                                egui::RichText::new(
                                    "Ask how to do something in Spectrix. I will search the Rust implementation source first, then answer from the current code.",
                                )
                                .weak(),
                            );
                        }

                        for message in &self.messages {
                            message_ui(ui, message);
                            ui.add_space(6.0);
                        }

                        if self.generating.load(Ordering::Relaxed) {
                            ui.horizontal(|ui| {
                                ui.label("Thinking...");
                                ui.add(egui::widgets::Spinner::default());
                            });
                        }
                    });
            });

        if let Some(error) = &self.last_error {
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    }

    fn composer_ui(&mut self, ui: &mut egui::Ui, snapshot: AiContextSnapshot) {
        ui.label("Ask a Question:");
        ui.add_sized(
            [ui.available_width(), 76.0],
            egui::TextEdit::multiline(&mut self.input)
                .hint_text("Example: How do I create a 1D cut with OR conditions?"),
        );

        ui.add_space(4.0);
        let can_generate = !self.input.trim().is_empty()
            && !self.system_prompt.trim().is_empty()
            && !self.base_url.trim().is_empty()
            && !self.model.trim().is_empty()
            && !self.generating.load(Ordering::Relaxed);
        let disabled_reason = self.generate_disabled_reason();

        ui.horizontal_wrapped(|ui| {
            let response = ui.add_enabled(can_generate, egui::Button::new("Send"));
            let response = if can_generate {
                response.on_hover_text("Search the Spectrix source and ask the selected model.")
            } else {
                response.on_disabled_hover_text(disabled_reason)
            };

            if response.clicked() {
                self.start_generation(snapshot);
            }

            if self.generating.load(Ordering::Relaxed) {
                ui.add(egui::widgets::Spinner::default());
            }
        });
    }

    fn take_pending_result(&mut self) {
        let result = self
            .pending_result
            .lock()
            .ok()
            .and_then(|mut pending| pending.take());

        match result {
            Some(Ok(pending)) => {
                self.session_usage
                    .record(&pending.model, pending.usage.clone());
                self.push_message(ConversationMessage::assistant(pending.response_text));
                self.last_error = None;
            }
            Some(Err(error)) => {
                self.last_error = Some(error.clone());
                self.push_message(ConversationMessage::assistant(
                    Self::error_message_for_user(&error),
                ));
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
                self.discovered_models = ollama_local_quick_pick_models(&models);
                self.last_model_error = None;
                if self.discovered_models.is_empty() {
                    self.last_model_error = Some(Self::no_local_models_message());
                } else if self.model.trim().is_empty() {
                    self.model = self.discovered_models.first().cloned().unwrap_or_default();
                }
            }
            Some(Err(error)) => {
                self.last_model_error = Some(error);
            }
            None => {}
        }
    }

    fn maybe_start_initial_model_refresh(&mut self) {
        if self.initial_model_refresh_started
            || self.provider != ACTIVE_PROVIDER
            || self.base_url.trim().is_empty()
        {
            return;
        }

        self.initial_model_refresh_started = true;
        self.start_model_listing();
    }

    fn ensure_ollama_provider(&mut self) {
        if self.provider == ACTIVE_PROVIDER {
            return;
        }

        self.provider = ACTIVE_PROVIDER;
        self.base_url = ACTIVE_PROVIDER.default_base_url().to_owned();
        self.model = ACTIVE_PROVIDER.default_model().to_owned();
        self.discovered_models.clear();
        self.last_model_error = None;
        self.initial_model_refresh_started = false;
    }

    fn refresh_model_lists(&mut self) {
        if self.provider != ACTIVE_PROVIDER || self.base_url.trim().is_empty() {
            return;
        }

        self.start_model_listing();
    }

    fn start_generation(&mut self, snapshot: AiContextSnapshot) {
        self.last_error = None;

        let prompt = self.input.trim().to_owned();
        if prompt.is_empty() {
            return;
        }

        let history = self.messages.clone();
        self.push_message(ConversationMessage::user(prompt.clone()));
        self.input.clear();

        let provider = ACTIVE_PROVIDER;
        let base_url = self.base_url.trim().to_owned();
        let model = self.model.trim().to_owned();
        let system_prompt = self.system_prompt.trim().to_owned();
        let generating = Arc::clone(&self.generating);
        let pending_result = Arc::clone(&self.pending_result);
        generating.store(true, Ordering::Relaxed);

        thread::spawn(move || {
            let result = build_user_prompt(&prompt, &history, &snapshot)
                .and_then(|user_prompt| {
                    let schema = response_schema();
                    generate_text(
                        provider,
                        &base_url,
                        &model,
                        &system_prompt,
                        &user_prompt,
                        &schema,
                    )
                })
                .map(|generation| {
                    let response = parse_model_response(&generation.text);
                    let response = normalize_help_response(response, &prompt);
                    PendingGeneration {
                        response_text: format_assistant_message(&response),
                        model,
                        usage: generation.usage,
                    }
                });

            if let Ok(mut pending) = pending_result.lock() {
                *pending = Some(result);
            }

            generating.store(false, Ordering::Relaxed);
        });
    }

    fn start_model_listing(&mut self) {
        if self.listing_models.load(Ordering::Relaxed) {
            return;
        }

        self.last_model_error = None;

        let provider = ACTIVE_PROVIDER;
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

    fn generate_disabled_reason(&self) -> &'static str {
        if self.generating.load(Ordering::Relaxed) {
            "An AI request is already running."
        } else if self.input.trim().is_empty() {
            "Ask a question about how to use Spectrix."
        } else if self.system_prompt.trim().is_empty() {
            "Enter a system prompt or reset it to the default."
        } else if self.base_url.trim().is_empty() {
            "Enter the Ollama URL."
        } else if self.model.trim().is_empty() {
            "Select or enter a model first. The setup section explains how to get started."
        } else {
            ""
        }
    }

    fn no_local_models_message() -> String {
        "No installed Ollama models were found. Spectrix can still use qwen3-coder:480b-cloud after `ollama signin`; use https://ollama.com/library?sort=popular to find other cloud model names. For local models, pull one with `ollama pull <model-name>`, then click Refresh Ollama Models again.".to_owned()
    }

    fn error_message_for_user(error: &str) -> String {
        if looks_like_local_setup_error(error) {
            return format!(
                "I could not reach a usable local AI model yet.\n\n{}\n\nTechnical detail: {error}",
                Self::local_setup_steps()
            );
        }

        format!("I could not get a model response.\n\nTechnical detail: {error}")
    }

    fn local_setup_steps() -> &'static str {
        "Start Ollama and run `ollama signin` for the default qwen3-coder:480b-cloud model. To use another cloud model, copy its name from https://ollama.com/library?sort=popular. For local models, pull one with `ollama pull <model-name>`, click Refresh Ollama Models, then choose the model."
    }

    fn push_message(&mut self, mut message: ConversationMessage) {
        message.content = truncate_text(&message.content, MAX_STORED_MESSAGE_CHARS);
        self.messages.push(message);

        let overflow = self
            .messages
            .len()
            .saturating_sub(MAX_CONVERSATION_MESSAGES);
        if overflow > 0 {
            self.messages.drain(0..overflow);
        }
    }
}

fn message_ui(ui: &mut egui::Ui, message: &ConversationMessage) {
    let (fill, stroke) = match message.role {
        ConversationRole::User => (
            ui.visuals().selection.bg_fill.gamma_multiply(0.35),
            ui.visuals().selection.stroke,
        ),
        ConversationRole::Assistant => (
            ui.visuals().faint_bg_color,
            ui.visuals().widgets.noninteractive.bg_stroke,
        ),
    };

    egui::Frame::NONE
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(message.role.label()).strong());
            ui.add(egui::Label::new(message.content.trim()).wrap());
        });
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let mut output = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        output.push_str("\n\n[Output shortened to keep the Spectrix UI responsive.]");
    }

    output
}

fn ollama_local_quick_pick_models(discovered_models: &[String]) -> Vec<String> {
    let mut local_models = Vec::new();

    for model in discovered_models {
        if !is_ollama_cloud_model(model) {
            push_unique_model(&mut local_models, model);
        }
    }

    local_models
}

fn push_unique_model(models: &mut Vec<String>, model: &str) {
    let model = model.trim();
    if model.is_empty() {
        return;
    }

    if !models.iter().any(|existing| existing == model) {
        models.push(model.to_owned());
    }
}

fn is_ollama_cloud_model(model: &str) -> bool {
    let model = model.trim().to_lowercase();
    model.contains("-cloud") || model.ends_with(":cloud")
}

fn looks_like_local_setup_error(error: &str) -> bool {
    let normalized = error.to_lowercase();
    [
        "network error",
        "connection refused",
        "failed to connect",
        "not found",
        "no ai model",
        "model",
        "404",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_cloud_model_names_are_detected() {
        assert!(is_ollama_cloud_model("qwen3-coder:480b-cloud"));
        assert!(is_ollama_cloud_model("glm-5:cloud"));
        assert!(is_ollama_cloud_model("  SOME-MODEL-CLOUD  "));
        assert!(!is_ollama_cloud_model("qwen3-coder:30b"));
        assert!(!is_ollama_cloud_model(""));
    }

    #[test]
    fn ollama_default_model_is_cloud_recommendation() {
        assert_eq!(AiProvider::Ollama.default_model(), "qwen3-coder:480b-cloud");
        assert!(is_ollama_cloud_model(AiProvider::Ollama.default_model()));
    }

    #[test]
    fn active_provider_is_ollama_only() {
        assert_eq!(ACTIVE_PROVIDER, AiProvider::Ollama);
    }

    #[test]
    fn saved_non_ollama_provider_falls_back_to_ollama() {
        for provider in [AiProvider::OpenAiCompatible, AiProvider::OpenAi] {
            let mut assistant = AiAssistant {
                provider,
                base_url: provider.default_base_url().to_owned(),
                model: provider.default_model().to_owned(),
                discovered_models: vec!["local-model".to_owned()],
                last_model_error: Some("previous error".to_owned()),
                initial_model_refresh_started: true,
                ..AiAssistant::default()
            };

            assistant.ensure_ollama_provider();

            assert_eq!(assistant.provider, ACTIVE_PROVIDER);
            assert_eq!(assistant.base_url, ACTIVE_PROVIDER.default_base_url());
            assert_eq!(assistant.model, ACTIVE_PROVIDER.default_model());
            assert!(assistant.discovered_models.is_empty());
            assert!(assistant.last_model_error.is_none());
            assert!(!assistant.initial_model_refresh_started);
        }
    }

    #[test]
    fn ollama_quick_picks_use_only_discovered_local_models() {
        let discovered_models = vec![
            "qwen2.5-coder:7b".to_owned(),
            "qwen3.5:cloud".to_owned(),
            "qwen2.5-coder:7b".to_owned(),
            "qwen3-coder:480b-cloud".to_owned(),
        ];
        let local_models = ollama_local_quick_pick_models(&discovered_models);

        assert_eq!(local_models, vec!["qwen2.5-coder:7b"]);
        assert!(!local_models.contains(&"qwen3-coder:30b".to_owned()));
    }

    #[test]
    fn session_usage_records_cloud_requests_and_tokens() {
        let mut usage = AiSessionUsage::default();
        usage.record(
            "qwen3-coder:480b-cloud",
            Some(AiUsage {
                prompt_tokens: Some(12),
                output_tokens: Some(8),
                total_tokens: Some(20),
                total_duration_ms: Some(1_250.0),
                load_duration_ms: Some(100.0),
            }),
        );

        assert_eq!(usage.requests, 1);
        assert_eq!(usage.cloud_requests, 1);
        assert_eq!(usage.prompt_tokens, 12);
        assert_eq!(usage.output_tokens, 8);
        assert_eq!(usage.total_tokens, 20);
        assert_eq!(usage.last_model.as_deref(), Some("qwen3-coder:480b-cloud"));
    }
}
