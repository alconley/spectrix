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
    let mut sections = Vec::new();

    if is_gaussian_fit_question(prompt) {
        sections.push(
            r#"Source-derived workflow hints for Gaussian fitting:
- Gaussian fitting is a 1D histogram workflow. The cursor must be inside the plot for keybinds to act at the cursor position.
- Marker keybinds: `P` adds a peak marker, `B` adds a background marker pair, `R` adds a region marker, `-` removes the nearest marker, and `Delete` clears temporary markers and temp fits.
- Fitting keybinds: `O` runs peak detection and places peak markers, `G` fits the current background model, `F` fits Gaussian peaks, and `S` stores the current temp fit.
- A common manual pipeline is: open a 1D histogram, add two `R` region markers around the fit interval, add one or more `B` background marker pairs in background-only areas, add peak markers with `P` or `O`, adjust options in the right-click `Fits` panel, press `G` if an explicit background fit is wanted, press `F`, inspect the Fit Panel, then press `S` to store the fit.
- `R` keeps at most two active region markers. Adding another region marker after two are present clears the old region markers and starts again.
- Peak markers, region markers, and background marker lines can be dragged directly on the plot. Background markers are stored as start/end pairs.
- The `Fits` panel contains background model selection, `Equal sigma`, `Free Position`, optional `Constrain sigma`, fit-line display toggles, UUID label controls, fit reports, modify/refit actions, save/load controls, and calibration controls.
- After fitting, Spectrix replaces peak markers with the fitted peak means when available and opens the Fit Panel for Gaussian results.
- Use `Store Fit`/`S` when the temp fit should become a stored fit for later comparison, calibration, saving, exporting, or refitting.
"#,
        );
    }

    if is_calibration_question(prompt) {
        sections.push(
            r#"Source-derived workflow hints for calibration:
- Spectrix has two different calibration workflows: calibrating Gaussian fit results/display on an existing 1D histogram, and creating a new calibrated event-data column in Histogram Script.
- For 1D histogram fit calibration, first fit and store Gaussian peaks. In the Fit Panel table, enter each peak's assigned energy and uncertainty; peaks with assigned energy `-1` are ignored as invalid calibration points.
- In the Fit Panel, enable `Calibration`. You can type coefficients directly as `a`, `b`, and `c` for `a*x^2 + b*x + c`, then click `Calibrate` to apply them to stored/temp fits.
- To derive coefficients from stored fits, use `Linear` for a linear calibration from at least 2 valid calibration points, or `Quadratic` for a quadratic calibration from at least 3 distinct valid points.
- Fit calibration affects the 1D histogram display/fit results when the calibration is safe over the histogram range. It attaches calibrated mean, sigma, and FWHM values to Gaussian parameters, while area and amplitude are copied through.
- Fit calibration does not create a new parquet/event column. It is for calibrated fit quantities and calibrated display behavior on that histogram.
- To create a calibrated column, open Histogram Script, use `Variables` for reusable coefficients if desired, then use `Column Creation` -> `+` -> `Builder`.
- A quadratic calibrated column can be built as three terms: coefficient `a` times source column with power `2`, plus coefficient `b` times source column with power `1`, plus constant `c`. The coefficients can be literal values or Variables.
- Give the computed column an alias such as `EnergyCalibrated`. Computed-column aliases are sanitized to letters, numbers, and underscores.
- Derived columns can then be selected like native columns in 1D/2D histogram definitions and cuts. To view the calibrated data as a new 1D histogram, add a `+1D` histogram using the calibrated-column alias, set calibrated range/bins, and calculate histograms.
"#,
        );
    }

    if is_2d_projection_question(prompt) {
        sections.push(
            r#"Source-derived workflow hints for 2D projections:
- 2D projections are created from an open 2D histogram. The cursor must be inside the plot for projection keybinds to act.
- Press `X` to toggle an X projection. It sums counts over a selected Y interval and opens a 1D histogram named `X-Projection of ...` with the 2D histogram's X column as its source column.
- Press `Y` to toggle a Y projection. It sums counts over a selected X interval and opens a 1D histogram named `Y-Projection of ...` with the 2D histogram's Y column as its source column.
- Projection regions initialize from the current visible axis bounds with a small inset from each edge.
- Drag a projection boundary line by its center dot to adjust the selected interval. Drag the filled span between projection lines to translate the whole projection window while keeping its width.
- You can also right-click the 2D plot and open `Projections` to toggle `Add X Projection` or `Add Y Projection`, edit X1/X2 or Y1/Y2 values, or set the projection span by width/range.
- The projection windows are normal 1D histogram windows, so 1D controls such as fitting, peak markers, statistics, rebinning, and 1D cuts can be used on the projected spectrum.
- 1D cuts created on projection windows inherit the projected axis column name automatically.
"#,
        );
    }

    if is_1d_cut_question(prompt) {
        if asks_definition(prompt) {
            sections.push(
                r#"Source-derived concept hints for 1D cuts:
- In Spectrix, a 1D cut is a one-variable gate: it keeps or rejects event rows by comparing numeric column values to one or more limits or literal values.
- In the code it is `Cut1D`. It stores a name, an expression string, an active flag, builder groups, and parsed condition groups.
- Conditions inside one builder group are combined with AND. Separate builder groups are combined with OR.
- When applied, Spectrix turns those parsed conditions into Polars boolean masks. Conditions in a group are ANDed, groups are ORed, and separate active cuts are combined with AND.
- Experimentally, a 1D cut is how the user gates on one event observable, such as an energy window, time window, position window, PID value, or validity flag, so only matching events contribute to histograms, fits, or saved filtered parquet output.
- An interactive 1D cut drawn from a 1D histogram is the same idea: two draggable vertical bounds are synced into an expression like `column >= low & column <= high`.
"#
            );
        } else {
            sections.push(
                r#"Source-derived workflow hints for 1D cuts:
- A 1D cut is a one-column event gate. It compares event values in a selected column against bounds or literal values, then uses the resulting boolean mask when calculating histograms or saving filtered parquet.
- The Histogram Script cut UI creates manual 1D cuts with the `+1D Manual` button and loads saved 1D cuts with `+1D Load`.
- A new manual 1D cut starts in Builder mode. In the 1D cut row, click `Builder` to open the dedicated editor.
- In the builder, each expression group combines conditions with AND. `Add Condition` adds another AND condition to the current group.
- `Add OR Expression` starts another expression group, so separate groups are combined with OR.
- Enable the cut checkbox when you want it used by histogram generation or parquet filtering.
- From an existing 1D histogram, pressing `C` or using the right-click Cuts menu can create an interactive draggable 1D cut if the histogram has a source column. Those appear under Active Histogram Cuts.
"#,
            );
        }
    }

    if is_2d_cut_question(prompt) {
        sections.push(
            r#"Source-derived workflow hints for 2D cuts:
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
        );
    }

    if sections.is_empty() {
        "Source-derived workflow hints: no special workflow hint matched; rely on the source excerpts below.".to_owned()
    } else {
        sections.join("\n")
    }
}

fn is_gaussian_fit_question(prompt: &str) -> bool {
    let normalized = prompt.to_lowercase();
    let mentions_fit = normalized.contains("fit")
        || normalized.contains("fitting")
        || normalized.contains("fitted");
    let mentions_gaussian = normalized.contains("gaussian");
    let mentions_peak = normalized.contains("peak");
    let mentions_marker = normalized.contains("marker");

    mentions_fit && (mentions_gaussian || mentions_peak || mentions_marker)
}

fn is_calibration_question(prompt: &str) -> bool {
    let normalized = prompt.to_lowercase();

    normalized.contains("calibrat")
}

fn is_2d_projection_question(prompt: &str) -> bool {
    let normalized = prompt.to_lowercase();
    let mentions_projection = normalized.contains("projection") || normalized.contains("project");
    let mentions_2d = normalized.contains("2d") || normalized.contains("two d");
    let mentions_histogram = normalized.contains("histogram") || normalized.contains("hist");

    mentions_projection && (mentions_2d || mentions_histogram)
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
        let hint_section = prompt
            .split("Codebase search results")
            .next()
            .expect("prompt should include hint section before codebase context");

        assert!(hint_section.contains("Source-derived concept hints for 1D cuts"));
        assert!(hint_section.contains("one-variable gate"));
        assert!(hint_section.contains("Polars boolean masks"));
        assert!(hint_section.contains("event observable"));
        assert!(!hint_section.contains("Source-derived workflow hints for 1D cuts"));
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

    #[test]
    fn user_prompt_includes_gaussian_fit_workflow_hints() {
        let prompt = build_user_prompt("How do I fit Gaussian peaks?", &[], &empty_snapshot())
            .expect("prompt should build");

        assert!(prompt.contains("Source-derived workflow hints for Gaussian fitting"));
        assert!(prompt.contains("`P` adds a peak marker"));
        assert!(prompt.contains("`B` adds a background marker pair"));
        assert!(prompt.contains("`R` adds a region marker"));
        assert!(prompt.contains("`F` fits Gaussian peaks"));
        assert!(prompt.contains("Store Fit"));
    }

    #[test]
    fn user_prompt_includes_calibration_workflow_hints() {
        let prompt = build_user_prompt(
            "How do I calibrate a 1D histogram and make a calibrated column?",
            &[],
            &empty_snapshot(),
        )
        .expect("prompt should build");

        assert!(prompt.contains("Source-derived workflow hints for calibration"));
        assert!(prompt.contains("creating a new calibrated event-data column"));
        assert!(prompt.contains("assigned energy `-1`"));
        assert!(prompt.contains("a*x^2 + b*x + c"));
        assert!(prompt.contains("Column Creation"));
        assert!(prompt.contains("EnergyCalibrated"));
    }

    #[test]
    fn user_prompt_includes_2d_projection_workflow_hints() {
        let prompt = build_user_prompt(
            "How do projections on 2D histograms work?",
            &[],
            &empty_snapshot(),
        )
        .expect("prompt should build");

        assert!(prompt.contains("Source-derived workflow hints for 2D projections"));
        assert!(prompt.contains("Press `X` to toggle an X projection"));
        assert!(prompt.contains("Press `Y` to toggle a Y projection"));
        assert!(prompt.contains("right-click the 2D plot"));
        assert!(prompt.contains("normal 1D histogram windows"));
    }

    #[test]
    fn user_prompt_can_include_multiple_workflow_hint_sections() {
        let prompt = build_user_prompt(
            "How do I fit Gaussian peaks, calibrate them, and make 2D projections?",
            &[],
            &empty_snapshot(),
        )
        .expect("prompt should build");

        assert!(prompt.contains("Source-derived workflow hints for Gaussian fitting"));
        assert!(prompt.contains("Source-derived workflow hints for calibration"));
        assert!(prompt.contains("Source-derived workflow hints for 2D projections"));
    }
}
