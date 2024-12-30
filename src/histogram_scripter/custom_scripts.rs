use crate::histoer::{
    configs::Configs,
    cuts::{Cut, Cuts},
};
use std::f64::consts::PI;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct CustomConfigs {
    pub sps: SPSConfig,
}

impl Default for CustomConfigs {
    fn default() -> Self {
        Self {
            sps: SPSConfig::new(),
        }
    }
}

impl CustomConfigs {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Custom: ");
            ui.checkbox(&mut self.sps.active, "SPS");
        });

        if self.sps.active {
            ui.horizontal(|ui| {
                ui.collapsing("SE-SPS", |ui| {
                    if ui.button("Reset").clicked() {
                        self.sps = SPSConfig::new();
                        self.sps.active = true;
                    }
                    self.sps.ui(ui);
                });
            });
        }
    }

    pub fn merge_active_configs(&self) -> Configs {
        let mut configs = Configs::default();

        if self.sps.active {
            configs.merge(self.sps.configs.clone()); // Ensure `merge` handles in-place modifications
        }

        configs
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Calibration {
    pub name: String,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub bins: usize,
    pub range: (f64, f64),
    pub active: bool,
}

impl Calibration {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(self.name.clone());
            if self.active {
                ui.add(egui::DragValue::new(&mut self.a).speed(0.01).prefix("a: "));
                ui.add(egui::DragValue::new(&mut self.b).speed(0.01).prefix("b: "));
                ui.add(egui::DragValue::new(&mut self.c).speed(0.01).prefix("c: "));

                ui.separator();
                ui.add(
                    egui::DragValue::new(&mut self.bins)
                        .speed(1)
                        .prefix("Bins: "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.range.0)
                        .speed(1)
                        .prefix("Range: (")
                        .suffix(", "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.range.1)
                        .speed(1)
                        .suffix(") [keV]"),
                );

                ui.label(format!(
                    "keV/bin: {:.2}",
                    (self.range.1 - self.range.0) / self.bins as f64
                ));
            }
            ui.checkbox(&mut self.active, "Active");
        });
    }
}

/*************************** SE-SPS Custom Struct ***************************/
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct SPSConfig {
    active: bool,
    xavg: Calibration,
    cuts: Cuts,
    configs: Configs,
}

impl Default for SPSConfig {
    fn default() -> Self {
        Self {
            active: false,
            xavg: Calibration {
                name: "Xavg Energy Calibration:".into(),
                a: 0.0,
                b: 1.0,
                c: 0.0,
                bins: 512,
                range: (0.0, 4096.0),
                active: false,
            },
            cuts: Cuts::default(),
            configs: Configs::default(),
        }
    }
}

impl SPSConfig {
    pub fn new() -> Self {
        let mut sps = SPSConfig::default();
        sps.configs = sps.sps_configs();
        sps
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.heading("Calibration");

        self.xavg.ui(ui);

        ui.separator();

        self.cuts.ui(ui);

        ui.separator();

        egui::CollapsingHeader::new("Script")
            .default_open(false)
            .show(ui, |ui| {
                self.configs.ui(ui);
            });
    }

    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub fn sps_configs(&self) -> Configs {
        let mut configs = Configs::default();

        configs.columns.push(("( DelayFrontRightEnergy + DelayFrontLeftEnergy ) / 2.0".into(), "DelayFrontAverageEnergy".into()));
        configs.columns.push(("( DelayBackRightEnergy + DelayBackLeftEnergy ) / 2.0".into(), "DelayBackAverageEnergy".into()));
        configs.columns.push(("DelayFrontLeftTime - AnodeFrontTime".into(), "DelayFrontLeftTime_AnodeFrontTime".into()));
        configs.columns.push(("DelayFrontRightTime - AnodeFrontTime".into(), "DelayFrontRightTime_AnodeFrontTime".into()));
        configs.columns.push(("DelayBackLeftTime - AnodeFrontTime".into(), "DelayBackLeftTime_AnodeFrontTime".into()));
        configs.columns.push(("DelayBackRightTime - AnodeFrontTime".into(), "DelayBackRightTime_AnodeFrontTime".into()));
        configs.columns.push(("DelayFrontLeftTime - AnodeBackTime".into(), "DelayFrontLeftTime_AnodeBackTime".into()));
        configs.columns.push(("DelayFrontRightTime - AnodeBackTime".into(), "DelayFrontRightTime_AnodeBackTime".into()));
        configs.columns.push(("DelayBackLeftTime - AnodeBackTime".into(), "DelayBackLeftTime_AnodeBackTime".into()));
        configs.columns.push(("DelayBackRightTime - AnodeBackTime".into(), "DelayBackRightTime_AnodeBackTime".into()));
        configs.columns.push(("AnodeFrontTime - AnodeBackTime".into(), "AnodeFrontTime_AnodeBackTime".into()));
        configs.columns.push(("AnodeBackTime - AnodeFrontTime".into(), "AnodeBackTime_AnodeFrontTime".into()));
        configs.columns.push(("AnodeFrontTime - ScintLeftTime".into(), "AnodeFrontTime_ScintLeftTime".into()));
        configs.columns.push(("AnodeBackTime - ScintLeftTime".into(), "AnodeBackTime_ScintLeftTime".into()));
        configs.columns.push(("DelayFrontLeftTime - ScintLeftTime".into(), "DelayFrontLeftTime_ScintLeftTime".into()));
        configs.columns.push(("DelayFrontRightTime - ScintLeftTime".into(), "DelayFrontRightTime_ScintLeftTime".into()));
        configs.columns.push(("DelayBackLeftTime - ScintLeftTime".into(), "DelayBackLeftTime_ScintLeftTime".into()));
        configs.columns.push(("DelayBackRightTime - ScintLeftTime".into(), "DelayBackRightTime_ScintLeftTime".into()));
        configs.columns.push(("ScintRightTime - ScintLeftTime".into(), "ScintRightTime_ScintLeftTime".into()));

        if self.xavg.active {
            configs.columns.push((
                format!("({})*Xavg*Xavg + ({})*Xavg + ({})", self.xavg.a, self.xavg.b, self.xavg.c),
                "XavgEnergyCalibrated".into(),
            ));
        }

        let mut cuts = Cuts::default();

        let bothplanes_cut = Cut::new_1d("Both Planes", "X2 != -1e6 && X1 != -1e6");
        let only_x1_plane_cut = Cut::new_1d("Only X1 Plane", "X1 != -1e6 && X2 == -1e6");
        let only_x2_plane_cut = Cut::new_1d("Only X2 Plane", "X2 != -1e6 && X1 == -1e6");

        cuts.add_cut(bothplanes_cut.clone());
        cuts.add_cut(only_x1_plane_cut.clone());
        cuts.add_cut(only_x2_plane_cut.clone());

        let fp_range = (-300.0, 300.0);
        let fp_bins = 600;

        let range = (0.0, 4096.0);
        let bins = 512;

        // Focal plane histograms
        configs.hist1d("SE-SPS/Focal Plane/X1", "X1", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/X2", "X2", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/Xavg", "Xavg", fp_range, fp_bins, None);
        if self.xavg.active {
            configs.hist1d("SE-SPS/Focal Plane/Xavg Energy Calibrated", "XavgEnergyCalibrated", self.xavg.range, self.xavg.bins, None);
        }
        configs.hist2d("SE-SPS/Focal Plane/X2 v X1", "X1", "X2", fp_range, fp_range, (fp_bins, fp_bins), None);
        configs.hist2d("SE-SPS/Focal Plane/Theta v Xavg", "Xavg", "Theta", fp_range, (0.0, PI), (fp_bins, fp_bins), None);
        configs.hist2d("SE-SPS/Focal Plane/Rays", "X", "Z", fp_range, (-50.0, 50.0), (fp_bins, 100), None);

        let cut_bothplanes = Some(Cuts::new(vec![bothplanes_cut.clone()]));
        let cut_only_x1_plane = Some(Cuts::new(vec![only_x1_plane_cut]));
        let cut_only_x2_plane = Some(Cuts::new(vec![only_x2_plane_cut]));

        configs.hist1d("SE-SPS/Focal Plane/Checks/Xavg", "Xavg", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/Checks/Raw- X1", "X1", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/Checks/Both Planes- X1", "X1", fp_range, fp_bins, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Focal Plane/Checks/Only 1 Plane- X1", "X1", fp_range, fp_bins, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Focal Plane/Checks/Raw- X2", "X2", fp_range, fp_bins, None);
        configs.hist1d("SE-SPS/Focal Plane/Checks/Both Planes- X2", "X2", fp_range, fp_bins, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Focal Plane/Checks/Only 1 Plane- X2", "X2", fp_range, fp_bins, cut_only_x2_plane.clone());

        // Particle Identification histograms
        configs.hist2d("SE-SPS/Particle Identification/AnodeBack v ScintLeft", "ScintLeftEnergy", "AnodeBackEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/AnodeFront v ScintLeft", "ScintLeftEnergy", "AnodeFrontEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/Cathode v ScintLeft", "ScintLeftEnergy", "CathodeEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/AnodeBack v ScintRight", "ScintRightEnergy", "AnodeBackEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/AnodeFront v ScintRight", "ScintRightEnergy", "AnodeFrontEnergy", range, range, (bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification/Cathode v ScintRight", "ScintRightEnergy", "CathodeEnergy", range, range, (bins,bins), None);

        // Particle Identification vs Focal plane histograms
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X1", "X1", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X2", "X2", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v Xavg", "Xavg", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v X1", "X1", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v X2", "X2", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v Xavg", "Xavg", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X1", "X1", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X2", "X2", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v Xavg", "Xavg", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X1", "X1", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X2", "X2", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v Xavg", "Xavg", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v X1", "X1", "CathodeEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v X2", "X2", "CathodeEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v Xavg", "Xavg", "CathodeEnergy", fp_range, range, (fp_bins,bins), None);

        // Delay lines vs Focal plane histograms
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X1", "X1", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X1", "X1", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X2", "X2", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X2", "X2", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v Xavg", "Xavg", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v Xavg", "Xavg", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v Xavg", "Xavg", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v Xavg", "Xavg", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X1", "X1", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X1", "X1", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X2", "X2", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X2", "X2", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None);

        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X1", "X1", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v X1", "X1", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X2", "X2", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v X2", "X2", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v Xavg", "Xavg", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None);
        configs.hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v Xavg", "Xavg", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None);


        // Delay timing relative to anodes histograms
        let valid_sps_timing = Cut::new_1d("Valid SPS Timing", "AnodeBackTime != -1e6 && ScintLeftTime != -1e6");
        cuts.add_cut(valid_sps_timing.clone());

        let cut_timing = Some(Cuts::new(vec![valid_sps_timing.clone()]));

        configs.hist1d("SE-SPS/Timing/AnodeFrontTime-AnodeBackTime", "AnodeFrontTime_AnodeBackTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/AnodeBackTime-AnodeFrontTime", "AnodeBackTime_AnodeFrontTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/AnodeFrontTime-ScintLeftTime", "AnodeFrontTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/AnodeBackTime-ScintLeftTime", "AnodeBackTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/DelayFrontLeftTime-ScintLeftTime", "DelayFrontLeftTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/DelayFrontRightTime-ScintLeftTime", "DelayFrontRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/DelayBackLeftTime-ScintLeftTime", "DelayBackLeftTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/DelayBackRightTime-ScintLeftTime", "DelayBackRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist1d("SE-SPS/Timing/ScintRightTime-ScintLeftTime", "ScintRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone());
        configs.hist2d("SE-SPS/Timing/ScintTimeDif v Xavg", "Xavg", "ScintRightTime_ScintLeftTime", fp_range, (-3200.0, 3200.0), (fp_bins, 12800), cut_timing.clone());


        configs.hist1d("SE-SPS/Timing/Both Planes/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Timing/Both Planes/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Timing/Both Planes/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone());
        configs.hist1d("SE-SPS/Timing/Both Planes/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone());

        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayBackLeftTime-AnodeFrontTime", "DelayBackLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayBackRightTime-AnodeFrontTime", "DelayBackRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayFrontLeftTime-AnodeBackTime", "DelayFrontLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayFrontRightTime-AnodeBackTime", "DelayFrontRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X1 Plane/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone());

        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayBackLeftTime-AnodeFrontTime", "DelayBackLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayBackRightTime-AnodeFrontTime", "DelayBackRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayFrontLeftTime-AnodeBackTime", "DelayFrontLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayFrontRightTime-AnodeBackTime", "DelayFrontRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());
        configs.hist1d("SE-SPS/Timing/Only X2 Plane/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone());

        configs.cuts = cuts;

        configs
    }

    // fn update_configs_with_cuts(&self) -> Configs {
    // }
}

/*************************** CeBrA Cutsom Struct ***************************/

pub struct TimeCut {
    pub mean: f64,
    pub low: f64,
    pub high: f64,
    pub active: bool,
}

impl TimeCut {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Time Cut: ");
            if self.active {
                ui.add(
                    egui::DragValue::new(&mut self.mean)
                        .speed(0.01)
                        .prefix("Mean: "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.low)
                        .speed(0.01)
                        .prefix("Low: "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.high)
                        .speed(0.01)
                        .prefix("High: "),
                );
            }
            ui.checkbox(&mut self.active, "Active");
        });
    }
}

pub struct Cebr3 {
    pub detector_number: usize,
    pub sps: bool,
    pub timecut: TimeCut,
    pub gainmatch: Calibration,
    pub energy_calibration: Calibration,
}

// impl Cebr3 {
//     pub fn new(detector_number: usize, sps: bool) -> Self {
//         Self {
//             detector_number,
//             sps,
//             timecut: TimeCut {
//                 mean: 0.0,
//                 low: -3000.0,
//                 high: 3000.0,
//                 active: sps,
//             },
//             gainmatch: Calibration {
//                 name: format!("CeBr{} Gain Match:", detector_number),
//                 a: 0.0,
//                 b: 1.0,
//                 c: 0.0,
//                 bins: 512,
//                 range: (0.0, 4096.0),
//                 active: false,
//             },
//             energy_calibration: Calibration {
//                 name: format!("CeBr{} Energy Calibration:", detector_number),
//                 a: 0.0,
//                 b: 1.0,
//                 c: 0.0,
//                 bins: 512,
//                 range: (0.0, 4096.0),
//                 active: false,
//             },
//         }
//     }

//     pub fn calibration_ui(&mut self, ui: &mut egui::Ui) {
//         ui.vertical(|ui| {
//             self.timecut.ui(ui);
//             self.gainmatch.ui(ui);
//             self.energy_calibration.ui(ui);
//         });
//     }

//     pub fn config(&self) -> Configs {

//         let mut configs = Configs::default();

//         configs
//     }
// }

/*



pub struct EnergyCalibration {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub bins: usize,
    pub range: (f64, f64),
}

#[rustfmt::skip]
#[allow(clippy::all)]
pub fn cebra(h: &mut Histogrammer, lf: LazyFrame, detector_number: usize, timecut: Option<TimeCut>, gainmatch: Option<GainMatch>, energy_calibration: Option<EnergyCalibration>) {

    let i = detector_number;
    let lf_cebra = lf.filter(col(&format!("Cebra{}Energy", detector_number)).neq(lit(-1e6)));

    let mut column_names = get_column_names_from_lazyframe(&lf_cebra);

    // add the psd column if it doesn't exist
    let lf_cebra = lf_cebra.with_column(
            ( ( col(&format!("Cebra{}Energy", i)) - col(&format!("Cebra{}Short", i)) )/ col(&format!("Cebra{}Energy", i) ) ).alias(&format!("Cebra{}PSD", i))
        );

    h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}Energy", i, i), &lf_cebra, &format!("Cebra{}Energy", i), 512, range);
    h.add_fill_hist2d(&format!("CeBrA/Cebra{}/PSD v Energy", i), &lf_cebra, &format!("Cebra{}Energy", i), &format!("Cebra{}PSD", i), (512, 512), (range, (0.0, 1.0)));


    /*
    Gain Matching Values

    If the gain match column already exists, skip gain matching.
    This is done so if you gain match on a run by run basis, you don't need to the gain match values.
    Just make sure you the alias is the same when analyzing the data externally.
    */
    let lf_cebra = if column_names.contains(&format!("Cebra{}EnergyGM", i)) {
        log::warn!("Gain matched energy column already exists, skipping gain matching");
        lf_cebra
    } else {
        if let Some(ref gainmatch) = gainmatch {
                log::info!("Gain matching Cebra{}Energy", i);
            let lf = lf_cebra.with_column(
                (lit(gainmatch.a) * col(&format!("Cebra{}Energy", i)).pow(2.0)
                + lit(gainmatch.b) * col(&format!("Cebra{}Energy", i))
                + lit(gainmatch.c)).alias(&format!("Cebra{}EnergyGM", i))
            );

            // update column names to include the gain matched column
            column_names.push(format!("Cebra{}EnergyGM", i));

            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}EnergyGM", i, i), &lf, &format!("Cebra{}EnergyGM", i), 512, range);

            lf
        } else{
            lf_cebra
        }
    };

    // Energy Calibration logic -> if energy calibration is provided, apply it to the gain matched energy column if it exists, otherwise apply it to the raw energy column
    let lf_cebra = if let Some(ref ecal) = energy_calibration {
        let lf = if column_names.contains(&format!("Cebra{}EnergyGM", i)) {
            lf_cebra.with_column(
                (lit(ecal.a) * col(&format!("Cebra{}EnergyGM", i)).pow(2.0)
                + lit(ecal.b) * col(&format!("Cebra{}EnergyGM", i))
                + lit(ecal.c)).alias(&format!("Cebra{}EnergyCalibrated", i))
            )
        } else {
            lf_cebra.with_column(
                (lit(ecal.a) * col(&format!("Cebra{}Energy", i)).pow(2.0)
                + lit(ecal.b) * col(&format!("Cebra{}Energy", i))
                + lit(ecal.c)).alias(&format!("Cebra{}EnergyCalibrated", i))
            )
        };

        // Update column names to include the calibrated energy column
        column_names.push(format!("Cebra{}EnergyCalibrated", i));

        // Fill histogram for the calibrated energy
        h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}EnergyCalibrated", i, i), &lf, &format!("Cebra{}EnergyCalibrated", i), ecal.bins, ecal.range);

        lf
    } else {
        lf_cebra
    };


    let mut sps = false;

    // Check if ScintLeftTime exists and therefore SPS is present
    let lf_cebra = if column_names.contains(&"ScintLeftTime".to_string()) {
        sps = true;

        // Check if Cebra#RelTime exists, if not, create it as Cebra#Time - ScintLeftTime
        if !column_names.contains(&format!("Cebra{}RelTime", i)) {
            column_names.push(format!("Cebra{}RelTime", i));
            lf_cebra.with_column(
                (col(&format!("Cebra{}Time", i)) - col("ScintLeftTime")).alias(&format!("Cebra{}RelTime", i))
            )
        } else {
            lf_cebra
        }
    } else {
        lf_cebra
    };

    if sps {
        h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}RelTime", i, i), &lf_cebra, &format!("Cebra{}RelTime", i), 6400, (-3200.0, 3200.0));
        h.add_fill_hist2d(&format!("CeBrA/Cebra{}/Cebra{}Energy v Xavg", i, i), &lf_cebra, "Xavg", &format!("Cebra{}Energy", i), (600, 512), (fp_range, range));
        h.add_fill_hist2d(&format!("CeBrA/Cebra{}/Theta v Cebra{}RelTime ", i, i), &lf_cebra, &format!("Cebra{}RelTime", i), "Theta", (6400, 300), ((-3200.0, 3200.0), (0.0, PI)));
    }

    // Apply timecut and shift RelTime if timecut is provided
    if let Some(timecut) = timecut {
        if sps {
            // Shift RelTime by timecut.mean
            let lf_timecut = lf_cebra.with_column(
                (col(&format!("Cebra{}RelTime", i)) - lit(timecut.mean)).alias(&format!("Cebra{}RelTimeShifted", i))
            )
                .filter(col(&format!("Cebra{}RelTime", i)).gt(lit(timecut.low)))
                .filter(col(&format!("Cebra{}RelTime", i)).lt(lit(timecut.high)))
                .filter(col("AnodeBackTime").neq(lit(-1e6)));

            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}RelTime", i, i), &lf_timecut, &format!("Cebra{}RelTime", i), 6400, (-3200.0, 3200.0));
            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}RelTimeShifted", i, i), &lf_timecut, &format!("Cebra{}RelTimeShifted", i), 100, (-50.0, 50.0));
            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy", i, i), &lf_timecut, &format!("Cebra{}Energy", i), 512, range);
            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Xavg", i), &lf_timecut, "Xavg", 600, fp_range);

            h.add_fill_hist1d("CeBrA/Xavg", &lf_timecut, "Xavg", 600, fp_range);
            h.add_fill_hist1d("CeBrA/CebraRelTimeShifted_TimeCut", &lf_timecut, &format!("Cebra{}RelTimeShifted", i), 100, (-50.0, 50.0));

            h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy v Xavg", i, i), &lf_timecut, "Xavg", &format!("Cebra{}Energy", i), (600, 512), (fp_range, range));
            h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy v X1", i, i), &lf_timecut, "X1", &format!("Cebra{}Energy", i), (600, 512), (fp_range, range));

            if column_names.contains(&format!("Cebra{}EnergyGM", i)) {
                h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM", i, i), &lf_timecut, &format!("Cebra{}EnergyGM", i), 512, range);

                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM v Xavg", i, i), &lf_timecut, "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), (fp_range, range));
                h.add_fill_hist2d("CeBrA/CebraEnergyGM v Xavg: TimeCut", &lf_timecut, "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), (fp_range, range));

                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM v X1", i, i), &lf_timecut, "X1", &format!("Cebra{}EnergyGM", i), (600, 512), (fp_range, range));
                h.add_fill_hist2d("CeBrA/CebraEnergyGM v X1: TimeCut", &lf_timecut, "X1", &format!("Cebra{}EnergyGM", i), (600, 512), (fp_range, range));
            }

            if let Some(ref ecal) = energy_calibration {
                h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Calibrated/Cebra{}EnergyCalibrated", i, i), &lf_timecut, &format!("Cebra{}EnergyCalibrated", i), ecal.bins, ecal.range);

                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Calibrated/Cebra{}EnergyCalibrated v Xavg", i, i), &lf_timecut, "Xavg", &format!("Cebra{}EnergyCalibrated", i), (600, ecal.bins), (fp_range, ecal.range));
                h.add_fill_hist2d("CeBrA/CebraEnergyCalibrated v Xavg: TimeCut", &lf_timecut, "Xavg", &format!("Cebra{}EnergyCalibrated", i), (600, ecal.bins), (fp_range, ecal.range));

                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Calibrated/Cebra{}EnergyCalibrated v X1", i, i), &lf_timecut, "X1", &format!("Cebra{}EnergyCalibrated", i), (600, ecal.bins), (fp_range, ecal.range));
                h.add_fill_hist2d("CeBrA/CebraEnergyCalibrated v X1: TimeCut", &lf_timecut, "X1", &format!("Cebra{}EnergyCalibrated", i), (600, ecal.bins), (fp_range, ecal.range));
            }
        }
    };

}

#[rustfmt::skip]
#[allow(clippy::all)]
pub fn catrina(h: &mut Histogrammer, lf: LazyFrame, detector_number: usize) {
    let i = detector_number;

    h.add_fill_hist1d(&format!("Catrina/CATRINA{i}/Energy"), &lf, &format!("CATRINA{i}Energy"), 4096, range);
    h.add_fill_hist2d(&format!("Catrina/CATRINA{i}/PSD vs Energy"), &lf, &format!("CATRINA{i}Energy"), &format!("CATRINA{i}PSD"), (512, 500), (range, (0.0, 1.0)));
}


*/

// pips1000(h, lf);
//....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....

// sps_histograms(h, lf.clone());

// For 52Cr(d,pg)53Cr
// let det_0_timecut = TimeCut { mean: -1155.6, low: -1158.0, high: -1152.0};
// let det_1_timecut = TimeCut { mean: -1153.9, low: -1159.0, high: -1147.0};
// let det_2_timecut = TimeCut { mean: -1154.0, low: -1158.0, high: -1151.0};
// let det_3_timecut = TimeCut { mean: -1152.0, low: -1158.0, high: -1148.0};
// let det_4_timecut = TimeCut { mean: -1123.1, low: -1127.0, high: -1118.0};

// // These values were gain match to detector 0
// // let det_0_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
// // let det_1_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
// // let det_2_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
// // let det_3_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
// // let det_4_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};

// let det_0_energy_calibration = EnergyCalibration { a: 0.0, b: 1.7551059351549314, c: -12.273506897222896, bins: 1024, range: (0.0, 16384.0) };
// let det_1_energy_calibration = EnergyCalibration { a: 0.0, b: 1.9510278378962256, c: -16.0245754973971, bins: 1024, range: (0.0, 16384.0) };
// let det_2_energy_calibration = EnergyCalibration { a: 0.0, b: 1.917190081718234, c: 16.430212777833802, bins: 1024, range: (0.0, 16384.0) };
// let det_3_energy_calibration = EnergyCalibration { a: 0.0, b: 1.6931918955746692, c: 12.021258506937766, bins: 1024, range: (0.0, 16384.0) };
// let det_4_energy_calibration = EnergyCalibration { a: 0.0, b: 1.6373533248536343, c: 13.091030061910748, bins: 1024, range: (0.0, 16384.0) };

// cebra(h, lf.clone(), 0, Some(det_0_timecut), None, Some(det_0_energy_calibration));
// cebra(h, lf.clone(), 1, Some(det_1_timecut), None, Some(det_1_energy_calibration));
// cebra(h, lf.clone(), 2, Some(det_2_timecut), None, Some(det_2_energy_calibration));
// cebra(h, lf.clone(), 3, Some(det_3_timecut), None, Some(det_3_energy_calibration));
// cebra(h, lf.clone(), 4, Some(det_4_timecut), None, Some(det_4_energy_calibration));

// #[rustfmt::skip]
// #[allow(clippy::all)]
// pub fn pips1000(h: &mut Histogrammer, lf: LazyFrame) {
//     let lf_pips = lf.with_columns( vec![
//         // ( ( col("PIPS1000Energy") - col("PIPS1000Short") )/ col("PIPS1000Energy") ).alias("PIPS1000PSD"),
//         (lit(-1.77049e-06)*col("PIPS1000Energy")*col("PIPS1000Energy") + lit(0.544755003513083)*col("PIPS1000Energy") + lit(-1.36822594543883)).alias("PIPS1000EnergyCalibrated") ]
//     );

//     h.add_fill_hist1d("PIPS1000/Energy", &lf_pips, "PIPS1000Energy", 16384, (0.0, 16384.0));
//     // h.add_fill_hist2d("PIPS1000: PSD", &lf_pips, "PIPS1000Energy", "PIPS1000PSD", (512, 500), (range, (0.0, 1.0)));
//     h.add_fill_hist1d("PIPS1000/Energy Calibrated", &lf_pips, "PIPS1000EnergyCalibrated", 600, (0.0, 1200.0));

// }
