use create::fitter::common::Parameter;

pub struct Runs {
    file_name: String,
    bci_scale: u32, 
    bci_scaler: f64,
    angle: f64, // lab degrees
    slits: f64, // msr
    normalization_factor: Option<f64>,
}

impl Runs {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(&mut self.bci_scale).speed(1).prefix("BCI Scale: "));
        ui.add(egui::DragValue::new(&mut self.bci_scaler).speed(0.01).prefix("BCI Scaler: "));
        ui.add(egui::DragValue::new(&mut self.angle).speed(1.0).prefix("Angle (deg): "));
        ui.add(egui::DragValue::new(&mut self.slits).speed(0.1).prefix("Slits (msr): "));
    }

}

pub struct FitUUIDMap {
    pub uuid: usize,
    pub energy: (f64, f64),
}

impl FitUUIDMap {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::DragValue::new(&mut self.uuid).speed(1));
        ui.add(egui::DragValue::new(&mut self.energy.0).speed(0.001).suffix("± "));
        ui.add(egui::DragValue::new(&mut self.energy.1).speed(0.001));
    }
}

pub struct SPSAnalysis {
    target_thickness: f64,  // in ug/cm^2
    target_molar_mass: f64, // in g/mol
    beam_energy: f64,       // in MeV
    beam_z: u32,          // atomic number
    runs: Vec<Runs>,
    fit_uuid_map: Vec<FitUUIDMap>,
}

impl SPSAnalysis {

}