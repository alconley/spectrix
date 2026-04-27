use crate::histoer::configs::Configs;
use crate::histoer::cuts::{Cut, Cuts};
use std::f64::consts::PI;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SPSOptions {
    pub focal_plane: bool,
    pub focal_plane_checks: bool,
    pub particle_identification: bool,
    pub particle_identification_vs_focal_plane: bool,
    pub delay_lines_vs_focal_plane: bool,
    pub timing: bool,
    pub calculate_no_cut_histograms: bool,
}

impl Default for SPSOptions {
    fn default() -> Self {
        Self {
            focal_plane: true,
            focal_plane_checks: true,
            particle_identification: true,
            particle_identification_vs_focal_plane: true,
            delay_lines_vs_focal_plane: true,
            timing: false,
            calculate_no_cut_histograms: true,
        }
    }
}

/*************************** SE-SPS Custom Struct ***************************/
#[derive(Default, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SPSConfig {
    pub active: bool,
    pub options: SPSOptions,
    pub sps_cuts: Cuts,
}

impl SPSConfig {
    pub fn new() -> Self {
        Self::default()
    }

    fn selected_cut(cut: &Cut) -> Cut {
        let mut cut = cut.clone();
        match &mut cut {
            Cut::Cut1D(cut1d) => cut1d.active = true,
            Cut::Cut2D(cut2d) => cut2d.active = true,
        }
        cut
    }

    fn sync_selected_cuts_from_available(&mut self, available_cuts: &Cuts) {
        self.sps_cuts.cuts = self
            .sps_cuts
            .cuts
            .iter()
            .filter_map(|selected_cut| {
                available_cuts
                    .cuts
                    .iter()
                    .find(|available_cut| available_cut.name() == selected_cut.name())
                    .map(Self::selected_cut)
            })
            .collect();
    }

    pub fn resolved_selected_cuts(&self, available_cuts: &Cuts, column_names: &[String]) -> Cuts {
        let selected_cuts = Cuts::new(
            self.sps_cuts
                .cuts
                .iter()
                .filter_map(|selected_cut| {
                    available_cuts
                        .cuts
                        .iter()
                        .find(|available_cut| available_cut.name() == selected_cut.name())
                        .map(Self::selected_cut)
                })
                .collect(),
        );
        selected_cuts.active_cuts_valid_for_columns(column_names, "SE-SPS custom configs")
    }

    pub fn configs(&self, available_cuts: &Cuts, column_names: &[String]) -> Configs {
        let selected_cuts = self.resolved_selected_cuts(available_cuts, column_names);
        let should_calculate_cut_histograms = !selected_cuts.is_empty();
        let mut configs = Configs::default();

        if should_calculate_cut_histograms {
            configs.merge(self.sps_configs(&Some(selected_cuts.clone())));
        }

        if self.options.calculate_no_cut_histograms || !should_calculate_cut_histograms {
            configs.merge(self.sps_configs(&None));
        }

        configs
    }

    fn cut_selector_ui(&mut self, ui: &mut egui::Ui, available_cuts: &Cuts) {
        ui.label("Global SE-SPS Cuts");
        ui.label(
            "Select the General cuts that should generate output under `SE-SPS/Cuts`. Selected cuts stay highlighted in the list below.",
        );

        if available_cuts.cuts.is_empty() {
            ui.label("No cuts are available in the General section right now.");
            return;
        }

        for available_cut in &available_cuts.cuts {
            let is_selected = self
                .sps_cuts
                .cuts
                .iter()
                .any(|selected_cut| selected_cut.name() == available_cut.name());

            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(is_selected, available_cut.name())
                    .clicked()
                {
                    if is_selected {
                        self.sps_cuts.remove_cut(available_cut.name());
                    } else {
                        self.sps_cuts.add_cut(Self::selected_cut(available_cut));
                    }
                }

                available_cut.info_button(ui, None);
            });
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, available_cuts: &Cuts) {
        self.sync_selected_cuts_from_available(available_cuts);

        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label("Histogram Options");
            ui.checkbox(&mut self.options.focal_plane, "Focal Plane");
            ui.checkbox(&mut self.options.focal_plane_checks, "Focal Plane Checks");
            ui.checkbox(
                &mut self.options.particle_identification,
                "Particle Identification",
            );
            ui.checkbox(
                &mut self.options.particle_identification_vs_focal_plane,
                "Particle Identification vs Focal Plane",
            );
            ui.checkbox(
                &mut self.options.delay_lines_vs_focal_plane,
                "Delay Lines vs Focal Plane",
            );
            ui.checkbox(&mut self.options.timing, "Timing");
        });

        ui.separator();

        // check if there are cuts, if so show check box
        if !available_cuts.cuts.is_empty() {
            ui.checkbox(
                &mut self.options.calculate_no_cut_histograms,
                "Calculate No-Cut Histograms",
            );
        } else {
            self.options.calculate_no_cut_histograms = true;
        }

        self.cut_selector_ui(ui, available_cuts);
    }

    #[rustfmt::skip]
    // #[expect(clippy::all)]
    pub fn sps_configs(&self, main_cuts: &Option<Cuts>) -> Configs {
        let mut configs = Configs::default();

        let base_path = if main_cuts.is_none() {
            "SE-SPS/Histograms"
        } else {
            "SE-SPS/Cuts"
        };

        let bothplanes_cut = Cut::new_1d("Both Planes", "X2 != -1e6 & X1 != -1e6");
        let only_x1_plane_cut = Cut::new_1d("Only X1 Plane", "X1 != -1e6 & X2 == -1e6");
        let only_x2_plane_cut = Cut::new_1d("Only X2 Plane", "X2 != -1e6 & X1 == -1e6");
        let valid_sps_timing = Cut::new_1d("Valid SPS Timing", "AnodeBackTime != -1e6 & ScintLeftTime != -1e6");

        configs.cuts.add_cut(bothplanes_cut.clone());
        configs.cuts.add_cut(only_x1_plane_cut.clone());
        configs.cuts.add_cut(only_x2_plane_cut.clone());
        configs.cuts.add_cut(valid_sps_timing.clone());


        // Typical 1d cuts for SPS
        let cut_bothplanes: Option<Cuts> = if let Some(mut main_cuts) = main_cuts.clone() {
            main_cuts.add_cut(bothplanes_cut);
            Some(main_cuts)
        } else {
            Some(Cuts::new(vec![bothplanes_cut.clone()]))
        };

        let cut_only_x1_plane: Option<Cuts> = if let Some(mut main_cuts) = main_cuts.clone() {
            main_cuts.add_cut(only_x1_plane_cut);
            Some(main_cuts)
        } else {
            Some(Cuts::new(vec![only_x1_plane_cut.clone()]))
        };

        let cut_only_x2_plane: Option<Cuts> = if let Some(mut main_cuts) = main_cuts.clone() {
            main_cuts.add_cut(only_x2_plane_cut);
            Some(main_cuts)
        } else {
            Some(Cuts::new(vec![only_x2_plane_cut.clone()]))
        };

        let cut_timing: Option<Cuts> = if let Some(mut main_cuts) = main_cuts.clone() {
            main_cuts.add_cut(valid_sps_timing);
            Some(main_cuts)
        } else {
            Some(Cuts::new(vec![valid_sps_timing.clone()]))
        };

        let fp_range = (-300.0, 300.0);
        let fp_bins = 600;

        let range = (0.0, 4096.0);
        let bins = 512;

        // Focal plane histograms
        if self.options.focal_plane {
            configs.hist1d(&format!("{base_path}/Focal Plane/X1"), "X1", fp_range, fp_bins, main_cuts);
            configs.hist1d(&format!("{base_path}/Focal Plane/X2"), "X2", fp_range, fp_bins, main_cuts);
            configs.hist1d(&format!("{base_path}/Focal Plane/Xavg"), "Xavg", fp_range, fp_bins, main_cuts);

            // configs.hist2d(&format!("{base_path}/Focal Plane/Both Planes- AnodeBack v ScintLeft"), "ScintLeftEnergy", "AnodeBackEnergy", range, range, (bins,bins), &cut_bothplanes);
            // configs.hist1d(&format!("{base_path}/Focal Plane/Xshap"), "Xshap", fp_range, fp_bins, main_cuts); //JCE 2026

            configs.hist2d(&format!("{base_path}/Focal Plane/X2 v X1"), "X1", "X2", fp_range, fp_range, (fp_bins, fp_bins), main_cuts);
            // configs.hist2d(&format!("{base_path}/Focal Plane/Theta v Xavg"), "Xavg", "Theta", fp_range, (0.0, PI), (fp_bins, fp_bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Focal Plane/Both Planes- AnodeBack v ScintLeft"), "ScintLeftEnergy", "AnodeBackEnergy", range, range, (bins,bins), &cut_bothplanes);

            if self.options.focal_plane_checks {
                configs.hist1d(&format!("{base_path}/Focal Plane/Checks/Xavg"), "Xavg", fp_range, fp_bins, main_cuts);
                configs.hist1d(&format!("{base_path}/Focal Plane/Checks/Raw- X1"), "X1", fp_range, fp_bins, main_cuts);
                configs.hist1d(&format!("{base_path}/Focal Plane/Checks/Both Planes- X1"), "X1", fp_range, fp_bins, &cut_bothplanes);
                configs.hist1d(&format!("{base_path}/Focal Plane/Checks/Only 1 Plane- X1"), "X1", fp_range, fp_bins, &cut_only_x1_plane);
                configs.hist1d(&format!("{base_path}/Focal Plane/Checks/Raw- X2"), "X2", fp_range, fp_bins, main_cuts);
                configs.hist1d(&format!("{base_path}/Focal Plane/Checks/Both Planes- X2"), "X2", fp_range, fp_bins, &cut_bothplanes);
                configs.hist1d(&format!("{base_path}/Focal Plane/Checks/Only 1 Plane- X2"), "X2", fp_range, fp_bins, &cut_only_x2_plane);
            }

            //configs.hist1d(&format!("{base_path}/Monitor/MonitorEnergy"), "MonitorEnergy", range, bins, main_cuts);

            // JCE 12/2025
            configs.hist2d(&format!("{base_path}/Focal Plane/Theta v Xavg"), "Xavg", "Theta", fp_range, (0.0, PI), (fp_bins, fp_bins), main_cuts);
            // configs.hist2d(&format!("{base_path}/Focal Plane/ThetaShap v Xshap"), "Xshap", "ThetaShap", fp_range, (0.0, PI), (fp_bins, fp_bins), main_cuts);

        }

        // Particle Identification histograms
        if self.options.particle_identification {
            configs.hist2d(&format!("{base_path}/Particle Identification/AnodeBack v ScintLeft"), "ScintLeftEnergy", "AnodeBackEnergy", range, range, (bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification/AnodeFront v ScintLeft"), "ScintLeftEnergy", "AnodeFrontEnergy", range, range, (bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification/Cathode v ScintLeft"), "ScintLeftEnergy", "CathodeEnergy", range, range, (bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification/AnodeBack v ScintRight"), "ScintRightEnergy", "AnodeBackEnergy", range, range, (bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification/AnodeFront v ScintRight"), "ScintRightEnergy", "AnodeFrontEnergy", range, range, (bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification/Cathode v ScintRight"), "ScintRightEnergy", "CathodeEnergy", range, range, (bins,bins), main_cuts);    
        }

        // Particle Identification vs Focal plane histograms
        if self.options.particle_identification_vs_focal_plane {
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/ScintLeft v X1"), "X1", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/ScintLeft v X2"), "X2", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/ScintLeft v Xavg"), "Xavg", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/ScintRight v X1"), "X1", "ScintRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/ScintRight v X2"), "X2", "ScintRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/ScintRight v Xavg"), "Xavg", "ScintRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/AnodeBack v X1"), "X1", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/AnodeBack v X2"), "X2", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/AnodeBack v Xavg"), "Xavg", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/AnodeFront v X1"), "X1", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/AnodeFront v X2"), "X2", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/AnodeFront v Xavg"), "Xavg", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/Cathode v X1"), "X1", "CathodeEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/Cathode v X2"), "X2", "CathodeEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Particle Identification v Focal Plane/Cathode v Xavg"), "Xavg", "CathodeEnergy", fp_range, range, (fp_bins,bins), main_cuts);
        }

        // Delay lines vs Focal plane histograms
        if self.options.delay_lines_vs_focal_plane {
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayFrontRight v X1"), "X1", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayFrontLeft v X1"), "X1", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayBackRight v X2"), "X2", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayBackLeft v X2"), "X2", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayBackRight v Xavg"), "Xavg", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayBackLeft v Xavg"), "Xavg", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayFrontRight v Xavg"), "Xavg", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayFrontLeft v Xavg"), "Xavg", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayBackRight v X1"), "X1", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayBackLeft v X1"), "X1", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayFrontRight v X2"), "X2", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/DelayFrontLeft v X2"), "X2", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), main_cuts);

            configs.columns.push(("( DelayFrontRightEnergy + DelayFrontLeftEnergy ) / 2.0".into(), "DelayFrontAverageEnergy".into()));
            configs.columns.push(("( DelayBackRightEnergy + DelayBackLeftEnergy ) / 2.0".into(), "DelayBackAverageEnergy".into()));

            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X1"), "X1", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/Averages/DelayBackAverage v X1"), "X1", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X2"), "X2", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/Averages/DelayBackAverage v X2"), "X2", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/Averages/DelayFrontAverage v Xavg"), "Xavg", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), main_cuts);
            configs.hist2d(&format!("{base_path}/Delay Lines v Focal Plane/Averages/DelayBackAverage v Xavg"), "Xavg", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), main_cuts);
        }


        // Delay timing relative to anodes histograms
        if self.options.timing {
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

            configs.columns.push(("DelayFrontLeftTime_AnodeFrontTime + DelayFrontRightTime_AnodeFrontTime".into(),"tsumF".into()));
            configs.columns.push(("DelayBackLeftTime_AnodeBackTime + DelayBackRightTime_AnodeBackTime".into(),"tsumB".into()));

            configs.hist2d(&format!("{base_path}/Timing/Timing Sums/tsumF vs X1"), "X1","tsumF", (-400.0, 400.0), (-1000.0, 3000.0), (800, 4000), main_cuts);
            configs.hist2d(&format!("{base_path}/Timing/Timing Sums/tsumB vs X2"), "X2","tsumB", (-400.0, 400.0), (-1000.0, 3000.0), (800, 4000), main_cuts);

            configs.hist1d(&format!("{base_path}/Timing/AnodeFrontTime-AnodeBackTime"), "AnodeFrontTime_AnodeBackTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/AnodeBackTime-AnodeFrontTime"), "AnodeBackTime_AnodeFrontTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/AnodeFrontTime-ScintLeftTime"), "AnodeFrontTime_ScintLeftTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/AnodeBackTime-ScintLeftTime"), "AnodeBackTime_ScintLeftTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/DelayFrontLeftTime-ScintLeftTime"), "DelayFrontLeftTime_ScintLeftTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/DelayFrontRightTime-ScintLeftTime"), "DelayFrontRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/DelayBackLeftTime-ScintLeftTime"), "DelayBackLeftTime_ScintLeftTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/DelayBackRightTime-ScintLeftTime"), "DelayBackRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/ScintRightTime-ScintLeftTime"), "ScintRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, &cut_timing);
            configs.hist2d(&format!("{base_path}/Timing/ScintTimeDif v Xavg"), "Xavg", "ScintRightTime_ScintLeftTime", fp_range, (-3200.0, 3200.0), (fp_bins, 12800), &cut_timing);
            configs.hist1d(&format!("{base_path}/Timing/Both Planes/DelayFrontLeftTime-AnodeFrontTime"), "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_bothplanes);
            configs.hist1d(&format!("{base_path}/Timing/Both Planes/DelayFrontRightTime-AnodeFrontTime"), "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_bothplanes);
            configs.hist1d(&format!("{base_path}/Timing/Both Planes/DelayBackLeftTime-AnodeBackTime"), "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_bothplanes);
            configs.hist1d(&format!("{base_path}/Timing/Both Planes/DelayBackRightTime-AnodeBackTime"), "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_bothplanes);
            configs.hist1d(&format!("{base_path}/Timing/Only X1 Plane/DelayFrontLeftTime-AnodeFrontTime"), "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_only_x1_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X1 Plane/DelayFrontRightTime-AnodeFrontTime"), "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_only_x1_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X1 Plane/DelayBackLeftTime-AnodeFrontTime"), "DelayBackLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_only_x1_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X1 Plane/DelayBackRightTime-AnodeFrontTime"), "DelayBackRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_only_x1_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X1 Plane/DelayFrontLeftTime-AnodeBackTime"), "DelayFrontLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_only_x1_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X1 Plane/DelayFrontRightTime-AnodeBackTime"), "DelayFrontRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_only_x1_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X1 Plane/DelayBackLeftTime-AnodeBackTime"), "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_only_x1_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X1 Plane/DelayBackRightTime-AnodeBackTime"), "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_only_x1_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X2 Plane/DelayFrontLeftTime-AnodeFrontTime"), "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_only_x2_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X2 Plane/DelayFrontRightTime-AnodeFrontTime"), "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_only_x2_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X2 Plane/DelayBackLeftTime-AnodeFrontTime"), "DelayBackLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_only_x2_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X2 Plane/DelayBackRightTime-AnodeFrontTime"), "DelayBackRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, &cut_only_x2_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X2 Plane/DelayFrontLeftTime-AnodeBackTime"), "DelayFrontLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_only_x2_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X2 Plane/DelayFrontRightTime-AnodeBackTime"), "DelayFrontRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_only_x2_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X2 Plane/DelayBackLeftTime-AnodeBackTime"), "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_only_x2_plane);
            configs.hist1d(&format!("{base_path}/Timing/Only X2 Plane/DelayBackRightTime-AnodeBackTime"), "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, &cut_only_x2_plane);
        }
        configs
    }
}
