use crate::histoer::{configs::Config, cuts::Cut};
use polars::prelude::*;
use std::f64::consts::PI;

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

/// Helper function to get column names from a LazyFrame
pub fn get_column_names_from_lazyframe(lazyframe: &LazyFrame) -> Vec<String> {
    let lf: LazyFrame = lazyframe.clone().limit(1);
    let df: DataFrame = lf.collect().unwrap();
    let columns: Vec<String> = df
        .get_column_names_owned()
        .into_iter()
        .map(|name| name.to_string())
        .collect();

    columns
}

#[rustfmt::skip]
#[allow(clippy::all)]
pub fn sps_histograms() -> (Vec<(String, String)>, Vec<Config>, Vec<Cut>) {

    let mut new_columns = vec![];
    new_columns.push(("( DelayFrontRightEnergy + DelayFrontLeftEnergy ) / 2.0".into(), "DelayFrontAverageEnergy".into()));
    new_columns.push(("( DelayBackRightEnergy + DelayBackLeftEnergy ) / 2.0".into(), "DelayBackAverageEnergy".into()));
    new_columns.push(("DelayFrontLeftTime - AnodeFrontTime".into(), "DelayFrontLeftTime_AnodeFrontTime".into()));
    new_columns.push(("DelayFrontRightTime - AnodeFrontTime".into(), "DelayFrontRightTime_AnodeFrontTime".into()));
    new_columns.push(("DelayBackLeftTime - AnodeFrontTime".into(), "DelayBackLeftTime_AnodeFrontTime".into()));
    new_columns.push(("DelayBackRightTime - AnodeFrontTime".into(), "DelayBackRightTime_AnodeFrontTime".into()));
    new_columns.push(("DelayFrontLeftTime - AnodeBackTime".into(), "DelayFrontLeftTime_AnodeBackTime".into()));
    new_columns.push(("DelayFrontRightTime - AnodeBackTime".into(), "DelayFrontRightTime_AnodeBackTime".into()));
    new_columns.push(("DelayBackLeftTime - AnodeBackTime".into(), "DelayBackLeftTime_AnodeBackTime".into()));
    new_columns.push(("DelayBackRightTime - AnodeBackTime".into(), "DelayBackRightTime_AnodeBackTime".into()));
    new_columns.push(("AnodeFrontTime - AnodeBackTime".into(), "AnodeFrontTime_AnodeBackTime".into()));
    new_columns.push(("AnodeBackTime - AnodeFrontTime".into(), "AnodeBackTime_AnodeFrontTime".into()));
    new_columns.push(("AnodeFrontTime - ScintLeftTime".into(), "AnodeFrontTime_ScintLeftTime".into()));
    new_columns.push(("AnodeBackTime - ScintLeftTime".into(), "AnodeBackTime_ScintLeftTime".into()));
    new_columns.push(("DelayFrontLeftTime - ScintLeftTime".into(), "DelayFrontLeftTime_ScintLeftTime".into()));
    new_columns.push(("DelayFrontRightTime - ScintLeftTime".into(), "DelayFrontRightTime_ScintLeftTime".into()));
    new_columns.push(("DelayBackLeftTime - ScintLeftTime".into(), "DelayBackLeftTime_ScintLeftTime".into()));
    new_columns.push(("DelayBackRightTime - ScintLeftTime".into(), "DelayBackRightTime_ScintLeftTime".into()));
    new_columns.push(("ScintRightTime - ScintLeftTime".into(), "ScintRightTime_ScintLeftTime".into()));

    let mut cuts = vec![];

    let bothplanes_cut = Cut::new_1d("Both Planes", "X2 != -1e6 && X1 != -1e6");
    cuts.push(bothplanes_cut.clone());

    let only_x1_plane_cut = Cut::new_1d("Only X1 Plane", "X1 != -1e6 && X2 == -1e6");
    cuts.push(only_x1_plane_cut.clone());

    let only_x2_plane_cut = Cut::new_1d("Only X2 Plane", "X2 != -1e6 && X1 == -1e6");
    cuts.push(only_x2_plane_cut.clone());

    let mut histograms = vec![];

    let fp_range = (-300.0, 300.0);
    let fp_bins = 600;

    let range = (0.0, 4096.0);
    let bins = 512;

    // Focal plane histograms
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/X1", "X1", fp_range, fp_bins, None));
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/X2", "X2", fp_range, fp_bins, None));
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/Xavg", "Xavg", fp_range, fp_bins, None));
    histograms.push(Config::new_2d("SE-SPS/Focal Plane/X2 v X1", "X1", "X2", fp_range, fp_range, (fp_bins, fp_bins), None));
    histograms.push(Config::new_2d("SE-SPS/Focal Plane/Theta v Xavg", "Xavg", "Theta", fp_range, (0.0, PI), (fp_bins, fp_bins), None));
    histograms.push(Config::new_2d("SE-SPS/Focal Plane/Rays", "X", "Z", fp_range, (-50.0, 50.0), (fp_bins, 100), None));

    let cut_bothplanes = Some(vec![bothplanes_cut.clone()]);
    let cut_only_x1_plane = Some(vec![only_x1_plane_cut.clone()]);
    let cut_only_x2_plane = Some(vec![only_x2_plane_cut.clone()]);

    histograms.push(Config::new_1d("SE-SPS/Focal Plane/Checks/Xavg", "Xavg", fp_range, fp_bins, None));
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/Checks/Raw- X1", "X1", fp_range, fp_bins, None));
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/Checks/Both Planes- X1", "X1", fp_range, fp_bins, cut_bothplanes.clone()));
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/Checks/Only 1 Plane- X1", "X1", fp_range, fp_bins, cut_only_x1_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/Checks/Raw- X2", "X2", fp_range, fp_bins, None));
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/Checks/Both Planes- X2", "X2", fp_range, fp_bins, cut_bothplanes.clone()));
    histograms.push(Config::new_1d("SE-SPS/Focal Plane/Checks/Only 1 Plane- X2", "X2", fp_range, fp_bins, cut_only_x2_plane.clone()));

    // Particle Identification histograms
    histograms.push(Config::new_2d("SE-SPS/Particle Identification/AnodeBack v ScintLeft", "ScintLeftEnergy", "AnodeBackEnergy", range, range, (bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification/AnodeFront v ScintLeft", "ScintLeftEnergy", "AnodeFrontEnergy", range, range, (bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification/Cathode v ScintLeft", "ScintLeftEnergy", "CathodeEnergy", range, range, (bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification/AnodeBack v ScintRight", "ScintRightEnergy", "AnodeBackEnergy", range, range, (bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification/AnodeFront v ScintRight", "ScintRightEnergy", "AnodeFrontEnergy", range, range, (bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification/Cathode v ScintRight", "ScintRightEnergy", "CathodeEnergy", range, range, (bins,bins), None));

    // Particle Identification vs Focal plane histograms
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X1", "X1", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X2", "X2", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v Xavg", "Xavg", "ScintLeftEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v X1", "X1", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v X2", "X2", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v Xavg", "Xavg", "ScintRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X1", "X1", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X2", "X2", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v Xavg", "Xavg", "AnodeBackEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X1", "X1", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X2", "X2", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v Xavg", "Xavg", "AnodeFrontEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/Cathode v X1", "X1", "CathodeEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/Cathode v X2", "X2", "CathodeEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Particle Identification v Focal Plane/Cathode v Xavg", "Xavg", "CathodeEnergy", fp_range, range, (fp_bins,bins), None));

    // Delay lines vs Focal plane histograms
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X1", "X1", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X1", "X1", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X2", "X2", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X2", "X2", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v Xavg", "Xavg", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v Xavg", "Xavg", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v Xavg", "Xavg", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v Xavg", "Xavg", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X1", "X1", "DelayBackRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X1", "X1", "DelayBackLeftEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X2", "X2", "DelayFrontRightEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X2", "X2", "DelayFrontLeftEnergy", fp_range, range, (fp_bins,bins), None));

    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X1", "X1", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v X1", "X1", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X2", "X2", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v X2", "X2", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v Xavg", "Xavg", "DelayFrontAverageEnergy", fp_range, range, (fp_bins,bins), None));
    histograms.push(Config::new_2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v Xavg", "Xavg", "DelayBackAverageEnergy", fp_range, range, (fp_bins,bins), None));


    // Delay timing relative to anodes histograms
    let valid_sps_timing = Cut::new_1d("Valid SPS Timing", "AnodeBackTime != -1e6 && ScintLeftTime != -1e6");
    cuts.push(valid_sps_timing.clone());

    let cut_timing = Some(vec![valid_sps_timing.clone()]);

    histograms.push(Config::new_1d("SE-SPS/Timing/AnodeFrontTime-AnodeBackTime", "AnodeFrontTime_AnodeBackTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/AnodeBackTime-AnodeFrontTime", "AnodeBackTime_AnodeFrontTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/AnodeFrontTime-ScintLeftTime", "AnodeFrontTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/AnodeBackTime-ScintLeftTime", "AnodeBackTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/DelayFrontLeftTime-ScintLeftTime", "DelayFrontLeftTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/DelayFrontRightTime-ScintLeftTime", "DelayFrontRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/DelayBackLeftTime-ScintLeftTime", "DelayBackLeftTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/DelayBackRightTime-ScintLeftTime", "DelayBackRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/ScintRightTime-ScintLeftTime", "ScintRightTime_ScintLeftTime", (-3000.0, 3000.0), 1000, cut_timing.clone()));
    histograms.push(Config::new_2d("SE-SPS/Timing/ScintTimeDif v Xavg", "Xavg", "ScintRightTime_ScintLeftTime", fp_range, (-3200.0, 3200.0), (fp_bins, 12800), cut_timing.clone()));


    histograms.push(Config::new_1d("SE-SPS/Timing/Both Planes/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Both Planes/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Both Planes/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Both Planes/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_bothplanes.clone()));

    histograms.push(Config::new_1d("SE-SPS/Timing/Only X1 Plane/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X1 Plane/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X1 Plane/DelayBackLeftTime-AnodeFrontTime", "DelayBackLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X1 Plane/DelayBackRightTime-AnodeFrontTime", "DelayBackRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X1 Plane/DelayFrontLeftTime-AnodeBackTime", "DelayFrontLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X1 Plane/DelayFrontRightTime-AnodeBackTime", "DelayFrontRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X1 Plane/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X1 Plane/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x1_plane.clone()));

    histograms.push(Config::new_1d("SE-SPS/Timing/Only X2 Plane/DelayFrontLeftTime-AnodeFrontTime", "DelayFrontLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X2 Plane/DelayFrontRightTime-AnodeFrontTime", "DelayFrontRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X2 Plane/DelayBackLeftTime-AnodeFrontTime", "DelayBackLeftTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X2 Plane/DelayBackRightTime-AnodeFrontTime", "DelayBackRightTime_AnodeFrontTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X2 Plane/DelayFrontLeftTime-AnodeBackTime", "DelayFrontLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X2 Plane/DelayFrontRightTime-AnodeBackTime", "DelayFrontRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X2 Plane/DelayBackLeftTime-AnodeBackTime", "DelayBackLeftTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone()));
    histograms.push(Config::new_1d("SE-SPS/Timing/Only X2 Plane/DelayBackRightTime-AnodeBackTime", "DelayBackRightTime_AnodeBackTime", (-4000.0, 4000.0), 8000, cut_only_x2_plane.clone()));

    (new_columns, histograms, cuts)
}
/*
#[rustfmt::skip]
#[allow(clippy::all)]
pub fn pips1000(h: &mut Histogrammer, lf: LazyFrame) {
    let lf_pips = lf.with_columns( vec![
        // ( ( col("PIPS1000Energy") - col("PIPS1000Short") )/ col("PIPS1000Energy") ).alias("PIPS1000PSD"),
        (lit(-1.77049e-06)*col("PIPS1000Energy")*col("PIPS1000Energy") + lit(0.544755003513083)*col("PIPS1000Energy") + lit(-1.36822594543883)).alias("PIPS1000EnergyCalibrated") ]
    );

    h.add_fill_hist1d("PIPS1000/Energy", &lf_pips, "PIPS1000Energy", 16384, (0.0, 16384.0));
    // h.add_fill_hist2d("PIPS1000: PSD", &lf_pips, "PIPS1000Energy", "PIPS1000PSD", (512, 500), (range, (0.0, 1.0)));
    h.add_fill_hist1d("PIPS1000/Energy Calibrated", &lf_pips, "PIPS1000EnergyCalibrated", 600, (0.0, 1200.0));

}

pub struct TimeCut {
    pub mean: f64,
    pub low: f64,
    pub high: f64,
}

pub struct GainMatch {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

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
