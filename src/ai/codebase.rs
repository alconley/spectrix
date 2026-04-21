use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const MAX_SOURCE_BYTES: u64 = 260_000;
const MAX_EXCERPTS: usize = 10;
const MAX_MATCHED_FILES: usize = 12;
const MAX_UI_CLUES: usize = 18;
const MAX_INTERACTION_CLUES: usize = 20;
const MAX_DATA_CLUES: usize = 18;
const MAX_CALL_CHAIN_CLUES: usize = 24;
const MAX_CALL_CHAIN_SEEDS: usize = 10;
const MAX_CALL_CHAIN_DEPTH: usize = 4;
const MAX_PROMPT_CHARS: usize = 24_000;
const EXCERPT_RADIUS: usize = 7;

#[derive(Debug, Clone)]
pub(crate) struct CodebaseContext {
    excerpts: Vec<CodeExcerpt>,
    matched_files: Vec<String>,
    ui_clues: Vec<SourceClue>,
    interaction_clues: Vec<SourceClue>,
    data_clues: Vec<SourceClue>,
    call_chain_clues: Vec<SourceClue>,
    searched_file_count: usize,
    warning: Option<String>,
}

#[derive(Debug, Clone)]
struct CodeDocument {
    relative_path: String,
    text: String,
}

#[derive(Debug, Clone)]
struct ScoredDocument {
    document: CodeDocument,
    score: usize,
}

#[derive(Debug, Clone)]
struct CodeExcerpt {
    relative_path: String,
    start_line: usize,
    end_line: usize,
    text: String,
}

#[derive(Debug, Clone)]
struct SourceClue {
    relative_path: String,
    line_number: usize,
    function_name: Option<String>,
    text: String,
}

impl CodebaseContext {
    pub(crate) fn for_query(query: &str) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut documents = Vec::new();
        let mut warning = None;

        if let Err(error) = collect_dir(root, &root.join("src"), &mut documents) {
            warning = Some(error);
        }

        let searched_file_count = documents.len();
        let terms = query_terms(query);
        let mut scored = documents
            .into_iter()
            .map(|document| {
                let score = score_document(&document, &terms);
                ScoredDocument { document, score }
            })
            .collect::<Vec<_>>();

        scored.sort_by(|left, right| {
            right.score.cmp(&left.score).then_with(|| {
                left.document
                    .relative_path
                    .cmp(&right.document.relative_path)
            })
        });

        let mut selected = scored
            .iter()
            .filter(|document| document.score > 0)
            .take(MAX_EXCERPTS)
            .map(|document| excerpt_for_document(&document.document, &terms))
            .collect::<Vec<_>>();

        if selected.is_empty() {
            selected = scored
                .iter()
                .filter(|document| is_high_signal_file(&document.document.relative_path))
                .take(MAX_EXCERPTS)
                .map(|document| excerpt_for_document(&document.document, &terms))
                .collect();
        }

        let relevant_documents = relevant_documents(&scored);
        let matched_files = relevant_documents
            .iter()
            .take(MAX_MATCHED_FILES)
            .map(|document| document.document.relative_path.clone())
            .collect::<Vec<_>>();
        let ui_clues = extract_ui_clues(&relevant_documents, &terms);
        let interaction_clues = extract_interaction_clues(&relevant_documents, &terms);
        let data_clues = extract_data_clues(&relevant_documents, &terms);
        let call_chain_clues = extract_call_chain_clues(&scored, &relevant_documents, &terms);

        Self {
            excerpts: selected,
            matched_files,
            ui_clues,
            interaction_clues,
            data_clues,
            call_chain_clues,
            searched_file_count,
            warning,
        }
    }

    pub(crate) fn to_prompt_section(&self) -> String {
        let mut section = format!(
            "Codebase search results from Spectrix source ({} files searched before answering):",
            self.searched_file_count
        );

        if let Some(warning) = &self.warning {
            section.push_str("\nSearch note: ");
            section.push_str(warning);
        }

        section.push_str("\n\nMatched source files:\n");
        if self.matched_files.is_empty() {
            section.push_str("- none\n");
        } else {
            for file in &self.matched_files {
                section.push_str("- ");
                section.push_str(file);
                section.push('\n');
            }
        }

        section.push_str("\nUI controls and entry-point clues found in matched source:\n");
        append_clues(&mut section, &self.ui_clues);

        section.push_str(
            "\nInteraction and gesture clues found in matched source (click, right-click, hover, drag, keybinds, context menus):\n",
        );
        append_clues(&mut section, &self.interaction_clues);

        section.push_str(
            "\nCall-chain and UI reachability clues found by following matched functions back toward callers:\n",
        );
        append_clues(&mut section, &self.call_chain_clues);

        section.push_str("\nData and experiment operation clues found in matched source:\n");
        append_clues(&mut section, &self.data_clues);

        if self.excerpts.is_empty() {
            section.push_str("\nNo source excerpts were available. Say that the implementation source search did not find a matching code path.");
            return section;
        }

        let mut used_chars = section.len();
        for excerpt in &self.excerpts {
            let header = format!(
                "\n\nFile: {} lines {}-{}\n```rust\n",
                excerpt.relative_path, excerpt.start_line, excerpt.end_line
            );
            let footer = "\n```";
            let next_len = header.len() + excerpt.text.len() + footer.len();
            if used_chars + next_len > MAX_PROMPT_CHARS {
                section.push_str(
                    "\n\nAdditional source excerpts were omitted to keep the prompt small.",
                );
                break;
            }

            section.push_str(&header);
            section.push_str(&excerpt.text);
            section.push_str(footer);
            used_chars += next_len;
        }

        section
    }
}

fn relevant_documents(scored: &[ScoredDocument]) -> Vec<&ScoredDocument> {
    let mut relevant = scored
        .iter()
        .filter(|document| document.score > 0)
        .take(MAX_MATCHED_FILES)
        .collect::<Vec<_>>();

    if relevant.is_empty() {
        relevant = scored
            .iter()
            .filter(|document| is_high_signal_file(&document.document.relative_path))
            .take(MAX_MATCHED_FILES)
            .collect();
    }

    relevant
}

fn collect_dir(root: &Path, dir: &Path, documents: &mut Vec<CodeDocument>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("Could not read {}: {error}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            collect_dir(root, &path, documents)?;
        } else if should_index_file(&path) {
            collect_file(root, &path, documents)?;
        }
    }

    Ok(())
}

fn collect_file(root: &Path, path: &Path, documents: &mut Vec<CodeDocument>) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?;
    if metadata.len() > MAX_SOURCE_BYTES {
        return Ok(());
    }

    let text = fs::read_to_string(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let relative_path = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    documents.push(CodeDocument {
        relative_path,
        text,
    });
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "target" | ".git" | ".venv"))
}

fn should_index_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "rs")
}

fn score_document(document: &CodeDocument, terms: &[String]) -> usize {
    if terms.is_empty() {
        return usize::from(is_high_signal_file(&document.relative_path));
    }

    let path = document.relative_path.to_lowercase();
    let text = document.text.to_lowercase();

    let base_score = terms
        .iter()
        .map(|term| path.matches(term).count() * 8 + text.matches(term).count())
        .sum::<usize>();

    base_score + workflow_file_boost(&path, terms)
}

fn excerpt_for_document(document: &CodeDocument, terms: &[String]) -> CodeExcerpt {
    let lines = document.text.lines().collect::<Vec<_>>();
    let best_index = best_line_index(&lines, terms);
    let start = best_index.saturating_sub(EXCERPT_RADIUS);
    let end = (best_index + EXCERPT_RADIUS + 1).min(lines.len());
    let text = lines[start..end]
        .iter()
        .enumerate()
        .map(|(index, line)| format!("{:>4}: {}", start + index + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    CodeExcerpt {
        relative_path: document.relative_path.clone(),
        start_line: start + 1,
        end_line: end,
        text,
    }
}

fn best_line_index(lines: &[&str], terms: &[String]) -> usize {
    if lines.is_empty() {
        return 0;
    }

    let mut best_index = 0;
    let mut best_score = 0;

    for (index, line) in lines.iter().enumerate() {
        let line = line.to_lowercase();
        let score = terms
            .iter()
            .map(|term| line.matches(term).count())
            .sum::<usize>();
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }

    best_index
}

fn extract_ui_clues(documents: &[&ScoredDocument], terms: &[String]) -> Vec<SourceClue> {
    extract_clues(documents, terms, is_ui_source_line, MAX_UI_CLUES)
}

fn extract_interaction_clues(documents: &[&ScoredDocument], terms: &[String]) -> Vec<SourceClue> {
    extract_clues(
        documents,
        terms,
        is_interaction_source_line,
        MAX_INTERACTION_CLUES,
    )
}

fn extract_data_clues(documents: &[&ScoredDocument], terms: &[String]) -> Vec<SourceClue> {
    extract_clues(
        documents,
        terms,
        is_data_or_experiment_source_line,
        MAX_DATA_CLUES,
    )
}

fn extract_call_chain_clues(
    all_documents: &[ScoredDocument],
    relevant_documents: &[&ScoredDocument],
    terms: &[String],
) -> Vec<SourceClue> {
    let seed_functions = seed_functions_from_documents(relevant_documents, terms);
    if seed_functions.is_empty() {
        return Vec::new();
    }

    let mut frontier = seed_functions.into_iter().collect::<BTreeSet<_>>();
    let mut visited_functions = frontier.clone();
    let mut clues = Vec::new();
    let mut seen_clues = BTreeSet::new();

    for depth in 0..MAX_CALL_CHAIN_DEPTH {
        let mut next_frontier = BTreeSet::new();

        for document in all_documents {
            let mut current_function = None;

            for (line_index, line) in document.document.text.lines().enumerate() {
                let trimmed = line.trim();

                if let Some(function_name) = function_name_from_line(trimmed) {
                    current_function = Some(function_name);
                }

                let Some(called_function) = frontier
                    .iter()
                    .find(|function_name| line_calls_function(trimmed, function_name))
                else {
                    continue;
                };

                let clue = SourceClue {
                    relative_path: document.document.relative_path.clone(),
                    line_number: line_index + 1,
                    function_name: current_function.clone(),
                    text: compact_source_line(trimmed),
                };

                let dedupe_key =
                    format!("{}:{}:{}", clue.relative_path, clue.line_number, clue.text);
                if seen_clues.insert(dedupe_key) {
                    clues.push((
                        call_chain_clue_score(&clue, terms, called_function, depth),
                        clue,
                    ));
                }

                if let Some(caller) = &current_function
                    && caller != called_function
                    && is_traceable_function(caller)
                    && visited_functions.insert(caller.clone())
                {
                    next_frontier.insert(caller.clone());
                }
            }
        }

        if next_frontier.is_empty() {
            break;
        }

        frontier = next_frontier;
    }

    clues.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.relative_path.cmp(&right.1.relative_path))
            .then_with(|| left.1.line_number.cmp(&right.1.line_number))
    });

    clues
        .into_iter()
        .map(|(_, clue)| clue)
        .take(MAX_CALL_CHAIN_CLUES)
        .collect()
}

fn seed_functions_from_documents(documents: &[&ScoredDocument], terms: &[String]) -> Vec<String> {
    let mut function_scores = BTreeMap::<String, usize>::new();

    for document in documents {
        let mut current_function = None;

        for line in document.document.text.lines() {
            let trimmed = line.trim();

            if let Some(function_name) = function_name_from_line(trimmed) {
                current_function = is_traceable_function(&function_name).then_some(function_name);

                if let Some(function_name) = &current_function {
                    let name_score = score_text_for_terms(function_name, terms);
                    if name_score > 0 {
                        *function_scores.entry(function_name.clone()).or_default() += name_score
                            * 12
                            + usize::from(is_high_signal_file(&document.document.relative_path));
                    }
                }
            }

            let Some(function_name) = &current_function else {
                continue;
            };

            let line_score = score_text_for_terms(trimmed, terms);
            if line_score == 0 {
                continue;
            }

            let source_kind_boost = usize::from(is_ui_source_line(trimmed))
                + usize::from(is_data_or_experiment_source_line(trimmed));
            *function_scores.entry(function_name.clone()).or_default() +=
                line_score * 5 + source_kind_boost * 4;
        }
    }

    let mut scored_functions = function_scores
        .into_iter()
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();

    scored_functions.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    scored_functions
        .into_iter()
        .map(|(function_name, _)| function_name)
        .take(MAX_CALL_CHAIN_SEEDS)
        .collect()
}

fn extract_clues(
    documents: &[&ScoredDocument],
    terms: &[String],
    predicate: fn(&str) -> bool,
    max_clues: usize,
) -> Vec<SourceClue> {
    let mut clues = Vec::new();
    let mut seen = BTreeSet::new();

    for document in documents {
        let mut current_function = None;
        for (line_index, line) in document.document.text.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(function_name) = function_name_from_line(trimmed) {
                current_function = Some(function_name);
            }

            if !predicate(trimmed) {
                continue;
            }

            let clue = SourceClue {
                relative_path: document.document.relative_path.clone(),
                line_number: line_index + 1,
                function_name: current_function.clone(),
                text: compact_source_line(trimmed),
            };
            let dedupe_key = format!("{}:{}:{}", clue.relative_path, clue.line_number, clue.text);
            if seen.insert(dedupe_key) {
                clues.push((clue_score(&clue, terms), clue));
            }
        }
    }

    clues.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.relative_path.cmp(&right.1.relative_path))
            .then_with(|| left.1.line_number.cmp(&right.1.line_number))
    });

    clues
        .into_iter()
        .map(|(_, clue)| clue)
        .take(max_clues)
        .collect()
}

fn is_ui_source_line(line: &str) -> bool {
    UI_SOURCE_PATTERNS
        .iter()
        .any(|pattern| line.contains(pattern))
}

fn is_interaction_source_line(line: &str) -> bool {
    let lowercase = line.to_lowercase();
    INTERACTION_SOURCE_PATTERNS
        .iter()
        .any(|pattern| lowercase.contains(pattern))
}

fn is_data_or_experiment_source_line(line: &str) -> bool {
    let lowercase = line.to_lowercase();
    DATA_SOURCE_PATTERNS
        .iter()
        .any(|pattern| lowercase.contains(pattern))
}

fn function_name_from_line(line: &str) -> Option<String> {
    let marker_index = line.find("fn ")?;
    let after_marker = &line[marker_index + 3..];
    let name = after_marker
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();

    (!name.is_empty()).then_some(name)
}

fn is_traceable_function(function_name: &str) -> bool {
    function_name.len() >= 4
        && !matches!(
            function_name,
            "main" | "new" | "ui" | "fmt" | "clone" | "default" | "from" | "into"
        )
}

fn line_calls_function(line: &str, function_name: &str) -> bool {
    if line.contains(&format!("fn {function_name}")) {
        return false;
    }

    for (index, _) in line.match_indices(function_name) {
        let Some(after) = line[index + function_name.len()..].chars().next() else {
            continue;
        };

        if after != '(' {
            continue;
        }

        let before = line[..index].chars().next_back();
        if before.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_') {
            continue;
        }

        return true;
    }

    false
}

fn score_text_for_terms(text: &str, terms: &[String]) -> usize {
    let lowercase = text.to_lowercase();
    terms
        .iter()
        .map(|term| lowercase.matches(term).count())
        .sum()
}

fn clue_score(clue: &SourceClue, terms: &[String]) -> usize {
    let text = clue.text.to_lowercase();
    let path = clue.relative_path.to_lowercase();
    let function_name = clue
        .function_name
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();

    let term_score = terms
        .iter()
        .map(|term| {
            text.matches(term).count() * 5
                + path.matches(term).count() * 3
                + function_name.matches(term).count() * 4
        })
        .sum::<usize>();

    term_score + usize::from(is_high_signal_file(&clue.relative_path))
}

fn call_chain_clue_score(
    clue: &SourceClue,
    terms: &[String],
    called_function: &str,
    depth: usize,
) -> usize {
    let depth_boost = MAX_CALL_CHAIN_DEPTH.saturating_sub(depth) * 20;
    let called_function_boost = score_text_for_terms(called_function, terms) * 8;
    let ui_boost = usize::from(is_ui_source_line(&clue.text)) * 80
        + clue
            .function_name
            .as_deref()
            .map(|function_name| {
                usize::from(function_name.contains("ui") || function_name.contains("menu")) * 40
            })
            .unwrap_or_default();

    clue_score(clue, terms) + depth_boost + called_function_boost + ui_boost
}

fn compact_source_line(line: &str) -> String {
    const MAX_LINE_CHARS: usize = 180;

    let compact = line.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compact.chars();
    let mut output = chars.by_ref().take(MAX_LINE_CHARS).collect::<String>();
    if chars.next().is_some() {
        output.push_str("...");
    }
    output
}

fn append_clues(section: &mut String, clues: &[SourceClue]) {
    if clues.is_empty() {
        section.push_str("- none found in the matched files\n");
        return;
    }

    for clue in clues {
        section.push_str("- ");
        section.push_str(&clue.relative_path);
        section.push(':');
        section.push_str(&clue.line_number.to_string());
        if let Some(function_name) = &clue.function_name {
            section.push_str(" (");
            section.push_str(function_name);
            section.push(')');
        }
        section.push_str(": ");
        section.push_str(&clue.text);
        section.push('\n');
    }
}

fn query_terms(query: &str) -> Vec<String> {
    let mut terms = query
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .map(str::trim)
        .filter(|term| term.len() > 2 || term.chars().any(|character| character.is_ascii_digit()))
        .map(str::to_lowercase)
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .collect::<BTreeSet<_>>();

    let original_terms = terms.iter().cloned().collect::<Vec<_>>();
    for term in original_terms {
        if let Some(synonyms) = synonyms_for(&term) {
            terms.extend(synonyms.iter().map(|synonym| (*synonym).to_owned()));
        }
    }

    terms.into_iter().collect()
}

fn synonyms_for(term: &str) -> Option<&'static [&'static str]> {
    match term {
        "hist" | "histogram" | "histograms" => Some(&["histogram", "histograms", "histogrammer"]),
        "1d" => Some(&["1d", "cut1d", "hist1d", "interactivecut1d"]),
        "2d" => Some(&["2d", "cut2d", "hist2d"]),
        "gate" | "gates" => Some(&["cut", "cuts", "gate", "polygon"]),
        "cut" | "cuts" => Some(&[
            "cut",
            "cuts",
            "active",
            "builder",
            "+1d manual",
            "add or expression",
            "active histogram cuts",
        ]),
        "column" | "columns" => Some(&["column", "columns", "computed", "builder"]),
        "fit" | "fits" | "fitting" => Some(&["fit", "fits", "fitter", "gaussian", "lmfit"]),
        "root" => Some(&["root", "uproot"]),
        "parquet" => Some(&["parquet", "lazyframe", "polars"]),
        "click" | "clicked" => Some(&[
            "click",
            "clicked",
            "click_and_drag",
            "interactive_clicking",
            "context_menu",
        ]),
        "right" => Some(&["right", "secondary_clicked", "context_menu"]),
        "hover" | "hovered" => Some(&["hover", "hovered", "on_hover_text"]),
        "drag" | "dragging" => {
            Some(&["drag", "dragging", "click_and_drag", "interactive_dragging"])
        }
        "double" => Some(&["double", "double-click", "allow_double_click_reset"]),
        "keybind" | "keybinds" => Some(&["keybind", "keybinds", "press"]),
        "save" | "export" => Some(&["save", "export", "file"]),
        "open" | "load" => Some(&["open", "load", "file"]),
        "plot" | "view" => Some(&["plot", "view", "histogram"]),
        "ai" | "assistant" => Some(&["ai", "assistant", "model", "provider"]),
        "analysis" => Some(&["analysis", "se_sps", "cross", "section", "yield", "theta"]),
        "calibrate" | "calibration" => Some(&["calibration", "energy", "calibrated", "fit"]),
        "peak" | "peaks" => Some(&["peak", "peaks", "gaussian", "fit", "centroid"]),
        "filter" | "filtered" => Some(&["filter", "filtered", "cut", "mask", "save"]),
        _ => None,
    }
}

fn workflow_file_boost(path: &str, terms: &[String]) -> usize {
    let has_cut = terms.iter().any(|term| term.contains("cut"));
    let has_1d = terms.iter().any(|term| {
        matches!(
            term.as_str(),
            "1d" | "cut1d" | "hist1d" | "interactivecut1d"
        )
    });
    let has_2d = terms.iter().any(|term| {
        matches!(
            term.as_str(),
            "2d" | "cut2d" | "hist2d" | "polygon" | "graphical"
        )
    });

    if has_cut && has_1d {
        return match path {
            "src/histoer/cuts.rs" => 600,
            "src/histoer/histo1d/context_menu.rs" => 350,
            "src/histoer/histo1d/interactive_cuts.rs" => 300,
            "src/histoer/configs.rs" => 200,
            _ => 0,
        };
    }

    if has_cut && has_2d {
        return match path {
            "src/histoer/histo2d/context_menu.rs" => 700,
            "src/histoer/cuts.rs" => 600,
            "src/egui_plot_stuff/egui_polygon.rs" => 350,
            "src/histoer/histo2d/plot_settings.rs" => 250,
            "src/histoer/configs.rs" => 150,
            _ => 0,
        };
    }

    0
}

fn is_high_signal_file(path: &str) -> bool {
    matches!(
        path,
        "src/util/processer.rs"
            | "src/histogram_scripter/histogram_script.rs"
            | "src/histoer/configs.rs"
            | "src/histoer/cuts.rs"
            | "src/histoer/histogrammer.rs"
            | "src/histoer/histo1d/context_menu.rs"
            | "src/histoer/histo2d/context_menu.rs"
            | "src/ai/assistant.rs"
    )
}

const STOP_WORDS: &[&str] = &[
    "about", "after", "again", "all", "and", "are", "ask", "but", "can", "could", "does", "for",
    "from", "get", "have", "how", "into", "like", "make", "one", "spectrix", "that", "the", "then",
    "this", "through", "use", "user", "want", "what", "when", "where", "with", "would", "you",
];

const UI_SOURCE_PATTERNS: &[&str] = &[
    "ui.button",
    "Button::new",
    "ui.checkbox",
    "ui.menu_button",
    "SubMenuButton::new",
    "ComboBox::",
    "selectable_label",
    "selectable_value",
    "ui.heading",
    "ui.label",
    "TextEdit::",
];

const INTERACTION_SOURCE_PATTERNS: &[&str] = &[
    ".clicked(",
    ".secondary_clicked(",
    ".on_hover_text(",
    ".on_disabled_hover_text(",
    ".context_menu(",
    "hovered(",
    "hover_pos",
    "drag",
    "click_and_drag",
    "interactive_dragging",
    "interactive_clicking",
    "double-click",
    "double_click",
    "keybind",
    "press `",
    "sense(egui::sense::click_and_drag())",
];

const DATA_SOURCE_PATTERNS: &[&str] = &[
    "active_filter_cuts",
    "add_hist",
    "add_vertex",
    "area",
    "background",
    "bins",
    "calibration",
    "centroid",
    "contains(",
    "counts",
    "create_mask",
    "cross_section",
    "energy",
    "eval_uncertainty",
    "filter",
    "fit",
    "gaussian",
    "histogram",
    "is_inside",
    "lazyframe",
    "lmfit",
    "mask",
    "mean",
    "parquet",
    "polygon",
    "projection",
    "range",
    "required_columns",
    "sigma",
    "theta",
    "valid(",
    "vertices",
    "yield",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codebase_context_includes_relevant_source_for_cut_question() {
        let context = CodebaseContext::for_query("How do I create a 1D cut with OR conditions?");
        let prompt_section = context.to_prompt_section();

        assert!(prompt_section.contains("files searched"));
        assert!(!prompt_section.contains("README.md"));
        assert!(!prompt_section.contains("Cargo.toml"));
        assert!(prompt_section.contains("src/histoer/cuts.rs"));
        assert!(
            prompt_section.contains("Add OR Expression")
                || prompt_section.contains("+1D Manual")
                || prompt_section.contains("Cut1D")
        );
    }

    #[test]
    fn codebase_context_includes_relevant_source_for_2d_cut_question() {
        let context = CodebaseContext::for_query("How can I create a 2D cut?");
        let prompt_section = context.to_prompt_section();

        assert!(prompt_section.contains("src/histoer/histo2d/context_menu.rs"));
        assert!(
            prompt_section.contains("new_cut") || prompt_section.contains("interactive_clicking")
        );
    }

    #[test]
    fn codebase_context_includes_general_ui_and_data_clues() {
        let context = CodebaseContext::for_query("How do I save filtered files?");
        let prompt_section = context.to_prompt_section();

        assert!(prompt_section.contains("UI controls and entry-point clues"));
        assert!(prompt_section.contains("Interaction and gesture clues"));
        assert!(prompt_section.contains("Call-chain and UI reachability clues"));
        assert!(prompt_section.contains("Data and experiment operation clues"));
        assert!(prompt_section.contains("Save Filtered Files"));
        assert!(
            prompt_section.contains("active_filter_cuts")
                || prompt_section.contains("filter_selected_files_and_save")
        );
    }

    #[test]
    fn codebase_context_includes_interaction_clues_for_mouse_and_context_menu_queries() {
        let context =
            CodebaseContext::for_query("How do I right click and drag on a 2D histogram?");
        let prompt_section = context.to_prompt_section();

        assert!(prompt_section.contains("Interaction and gesture clues"));
        assert!(
            prompt_section.contains("context_menu")
                || prompt_section.contains("interactive_dragging")
                || prompt_section.contains("hover_pos")
        );
    }

    #[test]
    fn call_chain_clues_follow_implementation_function_to_ui_caller() {
        let scored = vec![ScoredDocument {
            score: 10,
            document: CodeDocument {
                relative_path: "src/histoer/histo2d/context_menu.rs".to_owned(),
                text: r#"
impl Histogram2D {
    pub fn context_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("+").clicked() {
            self.new_cut();
        }
    }

    pub fn new_cut(&mut self) {
        self.plot_settings.cuts.push(Cut2D::default());
    }
}
"#
                .to_owned(),
            },
        }];
        let relevant_documents = vec![&scored[0]];
        let terms = vec!["cut".to_owned()];

        let clues = extract_call_chain_clues(&scored, &relevant_documents, &terms);

        assert!(clues.iter().any(|clue| {
            clue.function_name.as_deref() == Some("context_menu")
                && clue.text.contains("self.new_cut()")
        }));
    }
}
