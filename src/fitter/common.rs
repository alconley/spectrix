#[derive(PartialEq, Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Data {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Parameter {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub initial_guess: f64,
    pub vary: bool,
    pub value: Option<f64>,
    pub uncertainty: Option<f64>,
}

impl Default for Parameter {
    fn default() -> Self {
        Parameter {
            name: String::new(),
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            initial_guess: 0.0,
            vary: true,
            value: None,
            uncertainty: None,
        }
    }
}

impl Parameter {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(&self.name);
        ui.add(
            egui::DragValue::new(&mut self.initial_guess).speed(0.1), // .prefix("Initial Guess: ")
                                                                      // .suffix(" a.u."),
        )
        .on_hover_text(format!("Initial guess for the {} parameter", self.name));

        ui.add(
            egui::DragValue::new(&mut self.min)
                .speed(0.1)
                // .prefix("Min: ")
                .range(f64::NEG_INFINITY..=self.max), // .suffix(" a.u."),
        )
        .on_hover_text(format!("Minimum value for the {} parameter", self.name));

        ui.add(
            egui::DragValue::new(&mut self.max)
                .speed(0.1)
                // .prefix("Max: ")
                .range(self.min..=f64::INFINITY), // .suffix(" a.u."),
        )
        .on_hover_text(format!("Maximum value for the {} parameter", self.name));

        ui.checkbox(&mut self.vary, "").on_hover_text(format!(
            "Allow the {} parameter to vary during the fitting process",
            self.name
        ));

        if let Some(value) = self.value {
            ui.separator();
            ui.label(format!("{:.3}", value));
            ui.label(format!("{:.3}", self.uncertainty.unwrap_or(0.0)));
        }
    }
}
