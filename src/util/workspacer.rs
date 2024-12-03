use super::egui_file_dialog::FileDialog;
use std::path::PathBuf;

#[derive(Default, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WorkspacerOptions {
    pub root: bool,
}

#[derive(Default, Debug)]
pub struct Workspacer {
    pub selected_files: Vec<PathBuf>,
    pub file_dialog: Option<FileDialog>, // Not serialized or cloned
    pub options: WorkspacerOptions,
}

impl Workspacer {
    pub fn new() -> Self {
        Self {
            selected_files: Vec::new(),
            options: WorkspacerOptions::default(),
            file_dialog: None,
        }
    }

    pub fn open_file_dialog(&mut self) {
        self.file_dialog = Some(FileDialog::open_file(None).multi_select(true));
        if let Some(dialog) = &mut self.file_dialog {
            dialog.open(); // Modify the dialog in-place to open it
        }
    }

    /// Renders the workspace UI
    pub fn workspace_ui(&mut self, ui: &mut egui::Ui) {
        // ui.collapsing("Workspace", |ui| {

            // put this in a bottom panel
            egui::TopBottomPanel::top("workspace_bottom_panel").show_inside(ui, |ui| {
                // // Display selected files
                // if !self.selected_files.is_empty() {
                //     ui.label("Selected Files:");
                //     for file in &self.selected_files {
                //         ui.label(file.display().to_string());
                //     }
                // }

            });

            ui.checkbox(&mut self.options.root, "Root Files");

            if self.file_dialog.is_none() {
                self.open_file_dialog();
            }

            if let Some(dialog) = &mut self.file_dialog {
                dialog.ui_embeded(ui);
                self.selected_files = dialog.selected_file_paths();
            } 
            

            // ui.checkbox(&mut self.options.root, "Root Files");

            // if self.file_dialog.is_none() {
            //     self.open_file_dialog();
            // }

            // if let Some(dialog) = &mut self.file_dialog {
            //     dialog.ui_embeded(ui);
            //     self.selected_files = dialog.selected_file_paths();
            // } 
            
        // });
    }
}