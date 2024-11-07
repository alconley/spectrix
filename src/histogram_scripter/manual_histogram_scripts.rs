use super::histogram_script::HistoConfig;
use crate::histoer::histogrammer::{Histo1DConfig, Histo2DConfig};
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
pub fn sps_histograms() -> Vec<HistoConfig> {

    let mut histograms = vec![];

    // Focal plane histograms
    histograms.push(HistoConfig::Histo1D(Histo1DConfig::new("SE-SPS/Focal Plane/X1", "X1", (-300.0, 300.0), 600)));
    histograms.push(HistoConfig::Histo1D(Histo1DConfig::new("SE-SPS/Focal Plane/X2", "X2", (-300.0, 300.0), 600)));
    histograms.push(HistoConfig::Histo1D(Histo1DConfig::new("SE-SPS/Focal Plane/Xavg", "Xavg", (-300.0, 300.0), 600)));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Focal Plane/X2 v X1", "X1", "X2", (-300.0, 300.0), (-300.0, 300.0), (600, 600))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Focal Plane/Theta v Xavg", "Xavg", "Theta", (-300.0, 300.0), (0.0, PI), (600, 600))));

    // Particle Identification histograms
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification/AnodeBack v ScintLeft", "ScintLeftEnergy", "AnodeBackEnergy", (0.0, 4096.0), (0.0, 4096.0), (512,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification/AnodeFront v ScintLeft", "ScintLeftEnergy", "AnodeFrontEnergy", (0.0, 4096.0), (0.0, 4096.0), (512,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification/Cathode v ScintLeft", "ScintLeftEnergy", "CathodeEnergy", (0.0, 4096.0), (0.0, 4096.0), (512,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification/AnodeBack v ScintRight", "ScintRightEnergy", "AnodeBackEnergy", (0.0, 4096.0), (0.0, 4096.0), (512,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification/AnodeFront v ScintRight", "ScintRightEnergy", "AnodeFrontEnergy", (0.0, 4096.0), (0.0, 4096.0), (512,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification/Cathode v ScintRight", "ScintRightEnergy", "CathodeEnergy", (0.0, 4096.0), (0.0, 4096.0), (512,512))));

    // Particle Identification vs Focal plane histograms
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X1", "X1", "ScintLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X2", "X2", "ScintLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/ScintLeft v Xavg", "Xavg", "ScintLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/ScintRight v X1", "X1", "ScintRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/ScintRight v X2", "X2", "ScintRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/ScintRight v Xavg", "Xavg", "ScintRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X1", "X1", "AnodeBackEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X2", "X2", "AnodeBackEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/AnodeBack v Xavg", "Xavg", "AnodeBackEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X1", "X1", "AnodeFrontEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X2", "X2", "AnodeFrontEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/AnodeFront v Xavg", "Xavg", "AnodeFrontEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/Cathode v X1", "X1", "CathodeEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/Cathode v X2", "X2", "CathodeEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Particle Identification v Focal Plane/Cathode v Xavg", "Xavg", "CathodeEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));

    // Delay lines vs Focal plane histograms
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X1", "X1", "DelayFrontRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X1", "X1", "DelayFrontLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X2", "X2", "DelayBackRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X2", "X2", "DelayBackLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v Xavg", "Xavg", "DelayBackRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v Xavg", "Xavg", "DelayBackLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v Xavg", "Xavg", "DelayFrontRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v Xavg", "Xavg", "DelayFrontLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X1", "X1", "DelayBackRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X1", "X1", "DelayBackLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X2", "X2", "DelayFrontRightEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));
    histograms.push(HistoConfig::Histo2D(Histo2DConfig::new("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X2", "X2", "DelayFrontLeftEnergy", (-300.0, 300.0), (0.0, 4096.0), (600,512))));


    histograms
}
/*

    let lf_sps = lf.with_columns(vec![
            (col("DelayFrontRightEnergy") + col("DelayFrontLeftEnergy") / lit(2.0)).alias("DelayFrontAverageEnergy"),
            (col("DelayBackRightEnergy") + col("DelayBackLeftEnergy") / lit(2.0)).alias("DelayBackAverageEnergy"),
            (col("DelayFrontLeftTime") - col("AnodeFrontTime")).alias("DelayFrontLeftTime_AnodeFrontTime"),
            (col("DelayFrontRightTime") - col("AnodeFrontTime")).alias("DelayFrontRightTime_AnodeFrontTime"),
            (col("DelayBackLeftTime") - col("AnodeFrontTime")).alias("DelayBackLeftTime_AnodeFrontTime"),
            (col("DelayBackRightTime") - col("AnodeFrontTime")).alias("DelayBackRightTime_AnodeFrontTime"),
            (col("DelayFrontLeftTime") - col("AnodeBackTime")).alias("DelayFrontLeftTime_AnodeBackTime"),
            (col("DelayFrontRightTime") - col("AnodeBackTime")).alias("DelayFrontRightTime_AnodeBackTime"),
            (col("DelayBackLeftTime") - col("AnodeBackTime")).alias("DelayBackLeftTime_AnodeBackTime"),
            (col("DelayBackRightTime") - col("AnodeBackTime")).alias("DelayBackRightTime_AnodeBackTime"),
            (col("AnodeFrontTime") - col("AnodeBackTime")).alias("AnodeFrontTime_AnodeBackTime"),
            (col("AnodeBackTime") - col("AnodeFrontTime")).alias("AnodeBackTime_AnodeFrontTime"),
            (col("AnodeFrontTime") - col("ScintLeftTime")).alias("AnodeFrontTime_ScintLeftTime"),
            (col("AnodeBackTime") - col("ScintLeftTime")).alias("AnodeBackTime_ScintLeftTime"),
            (col("DelayFrontLeftTime") - col("ScintLeftTime")).alias("DelayFrontLeftTime_ScintLeftTime"),
            (col("DelayFrontRightTime") - col("ScintLeftTime")).alias("DelayFrontRightTime_ScintLeftTime"),
            (col("DelayBackLeftTime") - col("ScintLeftTime")).alias("DelayBackLeftTime_ScintLeftTime"),
            (col("DelayBackRightTime") - col("ScintLeftTime")).alias("DelayBackRightTime_ScintLeftTime"),
            (col("ScintRightTime") - col("ScintLeftTime")).alias("ScintRightTime_ScintLeftTime"),
            // (lit(-0.013139237615)*col("Xavg")*col("Xavg") + lit(-13.80004977)*col("Xavg") + lit(9790.048149635)).alias("XavgEnergyCalibrated")
        ]);

        let lf_bothplanes = lf_sps.clone().filter(col("X1").neq(lit(-1e6))).filter(col("X2").neq(lit(-1e6)));
        let lf_only_x1_plane = lf_sps.clone().filter(col("X1").neq(lit(-1e6))).filter(col("X2").eq(lit(-1e6)));
        let lf_only_x2_plane = lf_sps.clone().filter(col("X2").neq(lit(-1e6))).filter(col("X1").eq(lit(-1e6)));


        // // // //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
        // Delay lines vs Focal plane histograms
        let lf_time_rel_backanode = lf_sps.clone().filter(col("AnodeBackTime").neq(lit(-1e6))).filter(col("ScintLeftTime").neq(lit(-1e6)));
        h.add_fill_hist1d("SE-SPS/Timing/AnodeFrontTime-AnodeBackTime", &lf_time_rel_backanode, "AnodeFrontTime_AnodeBackTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist1d("SE-SPS/Timing/AnodeBackTime-AnodeFrontTime", &lf_time_rel_backanode, "AnodeBackTime_AnodeFrontTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist1d("SE-SPS/Timing/AnodeFrontTime-ScintLeftTime", &lf_time_rel_backanode, "AnodeFrontTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist1d("SE-SPS/Timing/AnodeBackTime-ScintLeftTime", &lf_time_rel_backanode, "AnodeBackTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist1d("SE-SPS/Timing/DelayFrontLeftTime-ScintLeftTime", &lf_time_rel_backanode, "DelayFrontLeftTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist1d("SE-SPS/Timing/DelayFrontRightTime-ScintLeftTime", &lf_time_rel_backanode, "DelayFrontRightTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist1d("SE-SPS/Timing/DelayBackLeftTime-ScintLeftTime", &lf_time_rel_backanode, "DelayBackLeftTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist1d("SE-SPS/Timing/DelayBackRightTime-ScintLeftTime", &lf_time_rel_backanode, "DelayBackRightTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist1d("SE-SPS/Timing/ScintRightTime-ScintLeftTime", &lf_time_rel_backanode, "ScintRightTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
        h.add_fill_hist2d("SE-SPS/Timing/ScintTimeDif v Xavg", &lf_time_rel_backanode, "Xavg", "ScintRightTime_ScintLeftTime", (600, 12800), ((-300.0, 300.0), (-3200.0, 3200.0)));

        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X1", &lf_sps, "X1", "DelayFrontAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v X1", &lf_sps, "X1", "DelayBackAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v X2", &lf_sps, "X2", "DelayFrontAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v X2", &lf_sps, "X2", "DelayBackAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayFrontAverage v Xavg", &lf_sps, "Xavg", "DelayFrontAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/Averages/DelayBackAverage v Xavg", &lf_sps, "Xavg", "DelayBackAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

        //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
        // Delay timing relative to anodes histograms
        h.add_fill_hist1d("SE-SPS/Timing/Bothplanes/DelayFrontLeftTime-AnodeFrontTime: bothplanes", &lf_bothplanes, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Bothplanes/DelayFrontRightTime-AnodeFrontTime: bothplanes", &lf_bothplanes, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Bothplanes/DelayBackLeftTime-AnodeBackTime: bothplanes", &lf_bothplanes, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Bothplanes/DelayBackRightTime-AnodeBackTime: bothplanes", &lf_bothplanes, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));

        h.add_fill_hist1d("SE-SPS/Timing/Only X1/DelayFrontLeftTime-AnodeFrontTime: onlyX1", &lf_only_x1_plane, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X1/DelayFrontRightTime-AnodeFrontTime: onlyX1", &lf_only_x1_plane, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X1/DelayBackLeftTime-AnodeFrontTime: onlyX1", &lf_only_x1_plane, "DelayBackLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X1/DelayBackRightTime-AnodeFrontTime: onlyX1", &lf_only_x1_plane, "DelayBackRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X1/DelayFrontLeftTime-AnodeBackTime: onlyX1", &lf_only_x1_plane, "DelayFrontLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X1/DelayFrontRightTime-AnodeBackTime: onlyX1", &lf_only_x1_plane, "DelayFrontRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X1/DelayBackLeftTime-AnodeBackTime: onlyX1", &lf_only_x1_plane, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X1/DelayBackRightTime-AnodeBackTime: onlyX1", &lf_only_x1_plane, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));

        h.add_fill_hist1d("SE-SPS/Timing/Only X2/DelayFrontLeftTime-AnodeFrontTime: onlyX2", &lf_only_x2_plane, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X2/DelayFrontRightTime-AnodeFrontTime: onlyX2", &lf_only_x2_plane, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X2/DelayBackLeftTime-AnodeFrontTime: onlyX2", &lf_only_x2_plane, "DelayBackLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X2/DelayBackRightTime-AnodeFrontTime: onlyX2", &lf_only_x2_plane, "DelayBackRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X2/DelayFrontLeftTime-AnodeBackTime: onlyX2", &lf_only_x2_plane, "DelayFrontLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X2/DelayFrontRightTime-AnodeBackTime: onlyX2", &lf_only_x2_plane, "DelayFrontRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X2/DelayBackLeftTime-AnodeBackTime: onlyX2", &lf_only_x2_plane, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
        h.add_fill_hist1d("SE-SPS/Timing/Only X2/DelayBackRightTime-AnodeBackTime: onlyX2", &lf_only_x2_plane, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
}

#[rustfmt::skip]
#[allow(clippy::all)]
pub fn pips1000(h: &mut Histogrammer, lf: LazyFrame) {
    let lf_pips = lf.with_columns( vec![
        // ( ( col("PIPS1000Energy") - col("PIPS1000Short") )/ col("PIPS1000Energy") ).alias("PIPS1000PSD"),
        (lit(-1.77049e-06)*col("PIPS1000Energy")*col("PIPS1000Energy") + lit(0.544755003513083)*col("PIPS1000Energy") + lit(-1.36822594543883)).alias("PIPS1000EnergyCalibrated") ]
    );

    h.add_fill_hist1d("PIPS1000/Energy", &lf_pips, "PIPS1000Energy", 16384, (0.0, 16384.0));
    // h.add_fill_hist2d("PIPS1000: PSD", &lf_pips, "PIPS1000Energy", "PIPS1000PSD", (512, 500), ((0.0, 4096.0), (0.0, 1.0)));
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

    h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}Energy", i, i), &lf_cebra, &format!("Cebra{}Energy", i), 512, (0.0, 4096.0));
    h.add_fill_hist2d(&format!("CeBrA/Cebra{}/PSD v Energy", i), &lf_cebra, &format!("Cebra{}Energy", i), &format!("Cebra{}PSD", i), (512, 512), ((0.0, 4096.0), (0.0, 1.0)));


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

            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}EnergyGM", i, i), &lf, &format!("Cebra{}EnergyGM", i), 512, (0.0, 4096.0));

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
        h.add_fill_hist2d(&format!("CeBrA/Cebra{}/Cebra{}Energy v Xavg", i, i), &lf_cebra, "Xavg", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
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
            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy", i, i), &lf_timecut, &format!("Cebra{}Energy", i), 512, (0.0, 4096.0));
            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Xavg", i), &lf_timecut, "Xavg", 600, (-300.0, 300.0));

            h.add_fill_hist1d("CeBrA/Xavg", &lf_timecut, "Xavg", 600, (-300.0, 300.0));
            h.add_fill_hist1d("CeBrA/CebraRelTimeShifted_TimeCut", &lf_timecut, &format!("Cebra{}RelTimeShifted", i), 100, (-50.0, 50.0));

            h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy v Xavg", i, i), &lf_timecut, "Xavg", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
            h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy v X1", i, i), &lf_timecut, "X1", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

            if column_names.contains(&format!("Cebra{}EnergyGM", i)) {
                h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM", i, i), &lf_timecut, &format!("Cebra{}EnergyGM", i), 512, (0.0, 4096.0));

                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM v Xavg", i, i), &lf_timecut, "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
                h.add_fill_hist2d("CeBrA/CebraEnergyGM v Xavg: TimeCut", &lf_timecut, "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM v X1", i, i), &lf_timecut, "X1", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
                h.add_fill_hist2d("CeBrA/CebraEnergyGM v X1: TimeCut", &lf_timecut, "X1", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
            }

            if let Some(ref ecal) = energy_calibration {
                h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Calibrated/Cebra{}EnergyCalibrated", i, i), &lf_timecut, &format!("Cebra{}EnergyCalibrated", i), ecal.bins, ecal.range);

                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Calibrated/Cebra{}EnergyCalibrated v Xavg", i, i), &lf_timecut, "Xavg", &format!("Cebra{}EnergyCalibrated", i), (600, ecal.bins), ((-300.0, 300.0), ecal.range));
                h.add_fill_hist2d("CeBrA/CebraEnergyCalibrated v Xavg: TimeCut", &lf_timecut, "Xavg", &format!("Cebra{}EnergyCalibrated", i), (600, ecal.bins), ((-300.0, 300.0), ecal.range));

                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Calibrated/Cebra{}EnergyCalibrated v X1", i, i), &lf_timecut, "X1", &format!("Cebra{}EnergyCalibrated", i), (600, ecal.bins), ((-300.0, 300.0), ecal.range));
                h.add_fill_hist2d("CeBrA/CebraEnergyCalibrated v X1: TimeCut", &lf_timecut, "X1", &format!("Cebra{}EnergyCalibrated", i), (600, ecal.bins), ((-300.0, 300.0), ecal.range));
            }
        }
    };

}

#[rustfmt::skip]
#[allow(clippy::all)]
pub fn catrina(h: &mut Histogrammer, lf: LazyFrame, detector_number: usize) {
    let i = detector_number;

    h.add_fill_hist1d(&format!("Catrina/CATRINA{i}/Energy"), &lf, &format!("CATRINA{i}Energy"), 4096, (0.0, 4096.0));
    h.add_fill_hist2d(&format!("Catrina/CATRINA{i}/PSD vs Energy"), &lf, &format!("CATRINA{i}Energy"), &format!("CATRINA{i}PSD"), (512, 500), ((0.0, 4096.0), (0.0, 1.0)));
}


*/
