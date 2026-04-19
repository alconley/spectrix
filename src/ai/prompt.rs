use super::codebase::CodebaseContext;
use super::context::AiContextSnapshot;
use super::response::{ConversationMessage, ConversationRole};

const DEFAULT_SYSTEM_PROMPT: &str = r#"
You are Spectrix's in-app help assistant.

Your job is to answer questions about how to use Spectrix as it exists today. Return only valid JSON that matches the provided schema.

Response rules:
- Put the main answer in `summary`.
- Use `clarification_questions` only if a short follow-up is required to answer correctly.
- Use `notes` for short extra tips, caveats, or UI labels that may help the user.

Behavior rules:
- Before answering, use the provided codebase investigation packet: matched files, UI controls, data/experiment operation clues, and source excerpts.
- Reconstruct the user's workflow by tracing UI labels and entry points in order. Prefer real labels from the UI clues over generic wording.
- Explain what the code does to the data when the user asks "what is", "why", "what happens", or any experiment/data question.
- For "what is" questions, lead with the concept and experiment/data meaning before giving UI steps.
- Interpret Spectrix as an experimental event-data analysis tool. In experiments such as nuclear physics, users are often interested in observables like energy, time, position, PID, or calibrated derived values. Histograms are binned event counts; cuts/gates are event-selection masks; fits extract peak centroids, widths, areas/yields; calibration maps detector/channel observables to physical units where supported.
- Trust implementation source excerpts and UI/data clues over docs, prior assumptions, or generic app behavior.
- If the codebase investigation did not find enough evidence, say what was missing and answer only what the source supports.
- Do not switch to the Analysis module unless the user asks about Analysis, SE-SPS, cross sections, or post-fit analysis.
- Do not ask clarification when the source excerpts already show the likely workflow. For broad "how do I..." questions, explain the common path and mention alternatives.
- The current user request is authoritative. Do not reuse a previous answer topic if the current request asks about a different Spectrix concept.
- If the user asks how to do something in Spectrix, answer with practical UI steps.
- Refer to real Spectrix UI names when helpful, such as Processor, Histogram Script, Variables, Column Creation, 1D Cuts, Builder, Calculate Histograms, Selected File Settings, fitting controls, and active cuts.
- If the user asks you to create, apply, edit, calculate, configure, analyze, or otherwise do something for them, clearly say that AI action-taking and AI analysis in Spectrix are currently under development.
- If Spectrix does not currently support what the user wants, say so plainly.
- Keep answers concise, concrete, and focused on the current app.
"#;

const MAX_HISTORY_MESSAGES: usize = 8;

pub(crate) fn default_system_prompt() -> &'static str {
    DEFAULT_SYSTEM_PROMPT.trim()
}

pub(crate) fn build_user_prompt(
    prompt: &str,
    conversation_history: &[ConversationMessage],
    snapshot: &AiContextSnapshot,
) -> Result<String, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Ask a question about how to use Spectrix.".to_owned());
    }

    let codebase_context = CodebaseContext::for_query(trimmed);

    Ok(format!(
        "Current user request:\n{trimmed}\n\nConversation so far:\n{}\n\nCurrent Spectrix session state:\n{}\n\n{}\n\n{}\n\nExperimental-data interpretation guide:\n{}\n\nAnswering reminder:\n- First identify what feature the user is asking about from the matched source files.\n- If the user asks what something is, answer the definition and experiment/data meaning first.\n- Then trace back the UI path using UI controls and source excerpts.\n- Then explain what the code does to the user's experimental data when relevant.\n- Give practical steps using exact Spectrix UI names.\n- Do not answer about the Analysis module unless the request is actually about Analysis.\n- Do not claim you changed Spectrix state or performed analysis.",
        conversation_history_section(conversation_history),
        snapshot.to_prompt_section(),
        source_derived_workflow_hints(trimmed),
        codebase_context.to_prompt_section(),
        experimental_data_interpretation_guide(),
    ))
}

fn experimental_data_interpretation_guide() -> &'static str {
    "- Parquet columns represent event-by-event experimental observables, such as energy, time, position, detector IDs, or calibrated derived values.\n- A histogram bins one or two observables and displays counts/yields per bin.\n- A cut/gate creates a boolean event-selection mask. A 1D cut compares one column to bounds or values. A 2D cut tests whether an event's `(x, y)` point lies inside a polygon gate.\n- Applying cuts before histogramming means only events passing the enabled gates contribute to the displayed histogram, fit, or saved filtered parquet file.\n- Gaussian fitting measures peak centroids, widths, amplitudes, and areas. In experiments such as nuclear physics, those are commonly used for energy/time/position calibration checks, resolution estimates, and yield extraction.\n- Calibration code maps detector/channel coordinates to physical units where the source code supports it.\n- Cross-section analysis, where present, combines fitted yields with beam/current/target/solid-angle style metadata; do not imply it is available for unrelated workflows unless source excerpts show it."
}

fn source_derived_workflow_hints(prompt: &str) -> String {
    if is_1d_cut_question(prompt) {
        if asks_definition(prompt) {
            return r#"Source-derived concept hints for 1D cuts:
- In Spectrix, a 1D cut is a one-variable gate: it keeps or rejects event rows by comparing numeric column values to one or more limits or literal values.
- In the code it is `Cut1D`. It stores a name, an expression string, an active flag, builder groups, and parsed condition groups.
- Conditions inside one builder group are combined with AND. Separate builder groups are combined with OR.
- When applied, Spectrix turns those parsed conditions into Polars boolean masks. Conditions in a group are ANDed, groups are ORed, and separate active cuts are combined with AND.
- Experimentally, a 1D cut is how the user gates on one event observable, such as an energy window, time window, position window, PID value, or validity flag, so only matching events contribute to histograms, fits, or saved filtered parquet output.
- An interactive 1D cut drawn from a 1D histogram is the same idea: two draggable vertical bounds are synced into an expression like `column >= low & column <= high`.
"#
            .to_owned();
        }

        return r#"Source-derived workflow hints for 1D cuts:
- A 1D cut is a one-column event gate. It compares event values in a selected column against bounds or literal values, then uses the resulting boolean mask when calculating histograms or saving filtered parquet.
- The Histogram Script cut UI creates manual 1D cuts with the `+1D Manual` button and loads saved 1D cuts with `+1D Load`.
- A new manual 1D cut starts in Builder mode. In the 1D cut row, click `Builder` to open the dedicated editor.
- In the builder, each expression group combines conditions with AND. `Add Condition` adds another AND condition to the current group.
- `Add OR Expression` starts another expression group, so separate groups are combined with OR.
- Enable the cut checkbox when you want it used by histogram generation or parquet filtering.
- From an existing 1D histogram, pressing `C` or using the right-click Cuts menu can create an interactive draggable 1D cut if the histogram has a source column. Those appear under Active Histogram Cuts.
"#
        .to_owned();
    }

    if is_2d_cut_question(prompt) {
        return r#"Source-derived workflow hints for 2D cuts:
- In Spectrix, a 2D cut is a graphical polygon gate tied to an X column and a Y column. In the code it is `Cut2D`, which stores polygon vertices and checks whether each row's `(x, y)` point is inside the polygon.
- To create a new 2D cut, first create/open a 2D histogram so Spectrix knows the X/Y columns.
- In the 2D plot, press `C` or right-click the plot and use `Cuts` -> `+` to start a new polygon cut.
- Click on the 2D plot to add polygon vertices. Double-click to finish adding vertices.
- After creation, drag vertices to edit the polygon.
- The cut name defaults to `Y v X Cut N`; the X/Y column names come from the current 2D histogram axes. Verify them in the Cuts menu/info.
- Use `Save` in the cut menu to write the 2D cut to JSON for reuse.
- The Histogram Script `+2D` button loads a saved 2D cut JSON. It is not the main path for drawing a new blank 2D polygon.
- Unsaved 2D cuts from a plot can still appear under Active Histogram Cuts and can be enabled for histogram generation or parquet filtering.
"#
        .to_owned();
    }

    "Source-derived workflow hints: no special workflow hint matched; rely on the source excerpts below.".to_owned()
}

fn is_1d_cut_question(prompt: &str) -> bool {
    let normalized = prompt.to_lowercase();
    let mentions_cut = normalized.contains("cut") || normalized.contains("gate");
    let mentions_1d = normalized.contains("1d") || normalized.contains("one d");

    mentions_cut && mentions_1d
}

fn is_2d_cut_question(prompt: &str) -> bool {
    let normalized = prompt.to_lowercase();
    let mentions_cut = normalized.contains("cut") || normalized.contains("gate");
    let mentions_2d = normalized.contains("2d") || normalized.contains("two d");

    mentions_cut && mentions_2d
}

fn asks_definition(prompt: &str) -> bool {
    let normalized = prompt.trim().to_lowercase();
    normalized.starts_with("what is")
        || normalized.starts_with("what are")
        || normalized.starts_with("explain")
        || normalized.starts_with("define")
}

fn conversation_history_section(conversation_history: &[ConversationMessage]) -> String {
    if conversation_history.is_empty() {
        return "No previous messages in this AI conversation.".to_owned();
    }

    let start = conversation_history
        .len()
        .saturating_sub(MAX_HISTORY_MESSAGES);
    conversation_history[start..]
        .iter()
        .map(|message| match message.role {
            ConversationRole::User => format!("User: {}", message.content.trim()),
            ConversationRole::Assistant => format!("Assistant: {}", message.content.trim()),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::context::AiContextSnapshot;

    fn empty_snapshot() -> AiContextSnapshot {
        AiContextSnapshot {
            selected_files: Vec::new(),
            loaded_columns: Vec::new(),
            existing_derived_columns: Vec::new(),
            general_cut_names: Vec::new(),
            active_histogram_cut_names: Vec::new(),
            existing_histograms: Vec::new(),
        }
    }

    #[test]
    fn user_prompt_includes_source_code_context() {
        let prompt = build_user_prompt("How do I create a 1D cut?", &[], &empty_snapshot())
            .expect("prompt should build");

        let workflow_index = prompt
            .find("Source-derived workflow hints for 1D cuts")
            .expect("workflow hints should be included");
        let codebase_index = prompt
            .find("Codebase search results")
            .expect("codebase search should be included");

        assert!(workflow_index < codebase_index);
        assert!(prompt.contains("+1D Manual"));
        assert!(prompt.contains("Add OR Expression"));
        assert!(prompt.contains("Current Spectrix session state"));
        assert!(prompt.contains("Experimental-data interpretation guide"));
        assert!(prompt.contains("trace back the UI path"));
        assert!(!prompt.contains("## Analysis"));
        assert!(!prompt.contains("README"));
    }

    #[test]
    fn user_prompt_prioritizes_1d_cut_definition_for_what_is_question() {
        let prompt = build_user_prompt("What is a 1D cut?", &[], &empty_snapshot())
            .expect("prompt should build");

        assert!(prompt.contains("Source-derived concept hints for 1D cuts"));
        assert!(prompt.contains("one-variable gate"));
        assert!(prompt.contains("Polars boolean masks"));
        assert!(prompt.contains("event observable"));
        assert!(!prompt.contains("Source-derived workflow hints for 1D cuts"));
    }

    #[test]
    fn user_prompt_includes_2d_cut_workflow_hints() {
        let prompt = build_user_prompt("What is a 2D cut?", &[], &empty_snapshot())
            .expect("prompt should build");

        assert!(prompt.contains("Source-derived workflow hints for 2D cuts"));
        assert!(prompt.contains("Cut2D"));
        assert!(prompt.contains("right-click the plot"));
        assert!(prompt.contains("Double-click"));
        assert!(prompt.contains("`+2D` button loads a saved 2D cut JSON"));
    }
}
