use crate::histoer::histogrammer::Histogrammer;

use polars::prelude::*;
use std::f64::consts::PI;

#[rustfmt::skip]
#[allow(clippy::all)]
pub fn manual_add_histograms(h: &mut Histogrammer, lf: LazyFrame) {

    
    // h.add_fill_hist1d("CATRINA0", &lf, "CATRINA0Energy", 4096, (0.0, 4096.0), None);
    // h.add_fill_hist1d("CATRINA1", &lf, "CATRINA1Energy", 4096, (0.0, 4096.0), None);
    // h.add_fill_hist1d("CATRINA2", &lf, "CATRINA2Energy", 4096, (0.0, 4096.0), None);

    // h.add_fill_hist2d("CATRINA0 PSD", &lf, "CATRINA0Energy", "CATRINA0PSD", (512, 500), ((0.0, 4096.0), (0.0, 1.0)), None);
    // h.add_fill_hist2d("CATRINA1 PSD", &lf, "CATRINA1Energy", "CATRINA1PSD", (512, 500), ((0.0, 4096.0), (0.0, 1.0)), None);
    // h.add_fill_hist2d("CATRINA2 PSD", &lf, "CATRINA2Energy", "CATRINA2PSD", (512, 500), ((0.0, 4096.0), (0.0, 1.0)), None);

    sps_histograms(h, lf.clone());
    // pips1000(h, lf);
    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....


    // For 52Cr(d,pg)53Cr
    let det_0_timecut = TimeCut { mean: -1155.6, low: -1158.0, high: -1152.0};
    let det_1_timecut = TimeCut { mean: -1153.9, low: -1159.0, high: -1147.0};
    let det_2_timecut = TimeCut { mean: -1154.0, low: -1158.0, high: -1151.0};
    let det_3_timecut = TimeCut { mean: -1152.0, low: -1158.0, high: -1148.0};
    let det_4_timecut = TimeCut { mean: -1123.1, low: -1127.0, high: -1118.0};

    let det_0_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
    let det_1_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
    let det_2_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
    let det_3_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};
    let det_4_gain_match_values = GainMatch { a: 0.0, b: 1.0, c: 0.0};

    let det_0_energy_calibration = EnergyCalibration { a: 0.0, b: 1.7551059351549314, c: -12.273506897222896};
    let det_1_energy_calibration = EnergyCalibration { a: 0.0, b: 1.9510278378962256, c: -16.0245754973971};
    let det_2_energy_calibration = EnergyCalibration { a: 0.0, b: 1.917190081718234, c: 16.430212777833802};
    let det_3_energy_calibration = EnergyCalibration { a: 0.0, b: 1.6931918955746692, c: 12.021258506937766};
    let det_4_energy_calibration = EnergyCalibration { a: 0.0, b: 1.6373533248536343, c: 13.091030061910748};

    cebra(h, lf.clone(), 0, Some(det_0_timecut), None);
    cebra(h, lf.clone(), 1, Some(det_1_timecut), None);
    cebra(h, lf.clone(), 2, Some(det_2_timecut), None);
    cebra(h, lf.clone(), 3, Some(det_3_timecut), None);
    cebra(h, lf.clone(), 4, Some(det_4_timecut), None);


    /*
    // CeBrA plots
    // declare the gain matched energy columns, and time to scint left columns

    let time_shits = vec![
        1158.5,
        1162.5,
        1160.5,
        1160.5,
        1135.5,
        1130.5,
        1129.5,
        1128.5,
        1136.5,
    ];

    let gain_match_values = vec![
        (-2.72156900878375e-19, 1.0, -5.6843418860808e-14),
        (-4.97165415075812e-06, 1.14338957961188, -0.445011719623494),
        (-2.35730449363139e-05, 0.996173496729512, 1.7443869915702),
        (-1.32712886414671e-05, 0.999818365276875, 0.941072453624997),
        (-2.76775763630926e-05, 0.860232802761158, -0.459548374380745),
        (-2.22330292983274e-05, 1.2289729811605, 0.283037829606826),
        (-4.12687972463716e-07, 0.701053103610418, 0.211631359048795),
        (-3.57953438680052e-05, 1.29207015301957, 1.07805724372673),
        (-3.03544968865592e-05, 0.957884129953653, -1.25796690476597),
    ];

    let time_cut_width = 6.0;

    let mut det_lfs = Vec::new();
    let mut det_time_lfs = Vec::new();
    let mut det_time_cut_lfs = Vec::new();
    for i in 0..9 {
        let det_lf = lf.clone()
            .filter(col(&format!("Cebra{}Energy", i)).neq(lit(-1e6)))
            .with_column(
                (lit(gain_match_values[i].0)*col(&format!("Cebra{}Energy", i))*col(&format!("Cebra{}Energy", i)) 
                    + lit(gain_match_values[i].1)*col(&format!("Cebra{}Energy", i)) 
                    + lit(gain_match_values[i].2)
                ).alias(&format!("Cebra{}EnergyGM", i)))
            .with_column((col(&format!("Cebra{}Time", i)) - col("ScintLeftTime")).alias(&format!("Cebra{}Time_ScintLeftTime", i)));

        let det_time_lf = det_lf.clone()
            .filter(col(&format!("Cebra{}Time", i)).neq(lit(-1e6)))
            .filter(col("ScintLeftTime").neq(lit(-1e6)))
            .filter(col("AnodeBackTime").neq(lit(-1e6)))
            .with_column((col(&format!("Cebra{}Time_ScintLeftTime", i)) + lit(time_shits[i])).alias(&format!("Cebra{}Time_ScintLeftTime_Shifted", i)));
    
        let det_time_cut_lf = det_time_lf.clone()
            .filter(col(&format!("Cebra{}Time_ScintLeftTime_Shifted", i)).gt(-time_cut_width))
            .filter(col(&format!("Cebra{}Time_ScintLeftTime_Shifted", i)).lt(time_cut_width))
            .with_column(( lit(2.598957532)*col(&format!("Cebra{}EnergyGM", i)) + lit(-12.36847047833) ).alias(&format!("Cebra{}EnergyECal", i)));

        det_lfs.push(det_lf);
        det_time_lfs.push(det_time_lf);
        det_time_cut_lfs.push(det_time_cut_lf);
    }

    h.add_hist1d("CeBrA Gain Matched", 512, (0.0, 4096.0));
    h.add_hist1d("CeBrA Time to Scint Shifted with TCut", 100, (-50.0, 50.0));
    h.add_hist2d("CeBrA Gain Matched vs Xavg with TCut", (600,512), ((-300.0, 300.0),(0.0, 4096.0)));
    h.add_hist2d("CeBrA vs Xavg: Energy Calibrated", (4096,512), ((0.0, 16384.0),(0.0, 8192.0)));

    
    for i in 0..9 {
        // Raw CeBrA Histograms
        h.add_fill_hist1d(&format!("Cebra{}Energy", i), &det_lfs[i], &format!("Cebra{}Energy", i), 512, (0.0, 4096.0));
        h.add_fill_hist2d(&format!("Cebra{}Energy v Xavg", i), &det_lfs[i], "Xavg", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist1d(&format!("Cebra{}Time-ScintLeftTime", i), &det_time_lfs[i], &format!("Cebra{}Time_ScintLeftTime", i), 3200, (-1600.0, 1600.0));
        h.add_fill_hist1d(&format!("Cebra{}Time-ScintLeftTime Shifted", i), &det_time_lfs[i], &format!("Cebra{}Time_ScintLeftTime_Shifted", i), 3200, (-1600.0, 1600.0));

        // Gain Matched Histograms
        h.add_fill_hist1d(&format!("Cebra{}EnergyGM", i), &det_lfs[i], &format!("Cebra{}EnergyGM", i), 512, (0.0, 4096.0));
        h.add_fill_hist2d(&format!("Cebra{}EnergyGainMatched v Xavg", i), &det_lfs[i], "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

        // Time Cut Histograms
        h.add_fill_hist1d(&format!("Cebra{}Time-ScintLeftTime Shifted Time Cut", i), &det_time_cut_lfs[i], &format!("Cebra{}Time_ScintLeftTime_Shifted", i), 100, (-50.0, 50.0));
        h.add_fill_hist1d(&format!("Cebra{}Energy Time Cut", i), &det_time_cut_lfs[i], &format!("Cebra{}Energy", i), 512, (0.0, 4096.0));
        h.add_fill_hist2d(&format!("Cebra{}Energy v Xavg Time Cut", i), &det_time_cut_lfs[i], "Xavg", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

        h.add_fill_hist2d(&format!("Cebra{}EnergyGM v Xavg Time Cut", i), &det_time_cut_lfs[i], "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        
        let cebra_det_panes_name_strings: Vec<String> = vec![
            format!("Cebra{}Energy", i), 
            format!("Cebra{}Energy v Xavg", i), 
            format!("Cebra{}Time-ScintLeftTime", i), 
            format!("Cebra{}Time-ScintLeftTime Shifted", i),
            format!("Cebra{}EnergyGM", i), 
            format!("Cebra{}EnergyGainMatched v Xavg", i),
            format!("Cebra{}Time-ScintLeftTime Shifted Time Cut", i), 
            format!("Cebra{}Energy Time Cut", i), 
            format!("Cebra{}Energy v Xavg Time Cut", i),
            format!("Cebra{}EnergyGM v Xavg Time Cut", i)
        ];

        let cebra_det_panes_name: Vec<&str> = cebra_det_panes_name_strings.iter().map(|s| s.as_str()).collect();

        let cebra_det_panes = h.get_panes(cebra_det_panes_name);

        h.tabs.insert(format!("CeBr3 Detector {}", i).to_string(), cebra_det_panes);

        // fill the combined histograms
        h.fill_hist1d(&format!("CeBrA Gain Matched"), &det_lfs[i], &format!("Cebra{}EnergyGM", i));
        h.fill_hist1d(&format!("CeBrA Time to Scint Shifted with TCut"), &det_time_cut_lfs[i], &format!("Cebra{}Time_ScintLeftTime_Shifted", i));
        h.fill_hist2d(&format!("CeBrA Gain Matched vs Xavg with TCut"), &det_time_cut_lfs[i], "Xavg", &format!("Cebra{}EnergyGM", i));
        h.fill_hist2d(&format!("CeBrA vs Xavg: Energy Calibrated"), &det_time_cut_lfs[i], "XavgEnergyCalibrated", &format!("Cebra{}EnergyECal", i));
    }

    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
    let cebra_panes_names = vec![
        "CeBrA Gain Matched", "CeBrA Time to Scint Shifted with TCut", 
        "CeBrA Gain Matched vs Xavg with TCut", "CeBrA vs Xavg: Energy Calibrated"
    ];

    let cebra_panes = h.get_panes(cebra_panes_names);
    h.tabs.insert("CeBrA".to_string(), cebra_panes);

    */

    // */
}


pub fn sps_histograms(h: &mut Histogrammer, lf: LazyFrame) {
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


        // ....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
        // Focal Plane histograms
        h.add_fill_hist1d("SE-SPS/Focal Plane/X1", &lf_sps, "X1", 600, (-300.0, 300.0));
        h.add_fill_hist1d("SE-SPS/Focal Plane/X1: only1plane", &lf_only_x1_plane, "X1", 600, (-300.0, 300.0));
        h.add_fill_hist1d("SE-SPS/Focal Plane/X1: bothplanes", &lf_bothplanes, "X1", 600, (-300.0, 300.0));
        h.add_fill_hist1d("SE-SPS/Focal Plane/X2", &lf_sps, "X2", 600, (-300.0, 300.0));
        h.add_fill_hist1d("SE-SPS/Focal Plane/X2: only1plane", &lf_only_x2_plane, "X2", 600, (-300.0, 300.0));
        h.add_fill_hist1d("SE-SPS/Focal Plane/X2: bothplanes", &lf_bothplanes, "X2", 600, (-300.0, 300.0));
        h.add_fill_hist2d("SE-SPS/Focal Plane/X2 v X1", &lf_sps, "X1", "X2", (600, 600), ((-300.0, 300.0), (-300.0, 300.0)));
        h.add_fill_hist1d("SE-SPS/Focal Plane/Xavg: bothplanes", &lf_bothplanes, "Xavg", 600, (-300.0, 300.0));
        h.add_fill_hist2d("SE-SPS/Focal Plane/Theta v Xavg: bothplanes", &lf_bothplanes, "Xavg", "Theta", (600, 300), ((-300.0, 300.0), (0.0, PI)));
        // h.add_fill_hist1d("XavgEnergyCalibrated", &lf, "XavgEnergyCalibrated", 4096, (0.0, 16384.0), fp_grid);

        // ....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
        // Particle Identification histograms
        h.add_fill_hist2d("SE-SPS/Particle Identification/AnodeBack v ScintLeft", &lf_sps, "ScintLeftEnergy", "AnodeBackEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification/AnodeFront v ScintLeft", &lf_sps, "ScintLeftEnergy", "AnodeFrontEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification/Cathode v ScintLeft", &lf_sps, "ScintLeftEnergy", "CathodeEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification/AnodeBack v ScintRight", &lf_sps, "ScintRightEnergy", "AnodeBackEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification/AnodeFront v ScintRight", &lf_sps, "ScintRightEnergy", "AnodeFrontEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification/Cathode v ScintRight", &lf_sps, "ScintRightEnergy", "CathodeEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
        
        // ....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
        // Particle Identification vs Focal plane histograms
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X1", &lf_sps, "X1", "ScintLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v X2", &lf_sps, "X2", "ScintLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/ScintLeft v Xavg", &lf_sps, "Xavg", "ScintLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v X1", &lf_sps, "X1", "ScintRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v X2", &lf_sps, "X2", "ScintRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/ScintRight v Xavg", &lf_sps, "Xavg", "ScintRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X1", &lf_sps, "X1", "AnodeBackEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v X2", &lf_sps, "X2", "AnodeBackEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeBack v Xavg", &lf_sps, "Xavg", "AnodeBackEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X1", &lf_sps, "X1", "AnodeFrontEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v X2", &lf_sps, "X2", "AnodeFrontEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/AnodeFront v Xavg", &lf_sps, "Xavg", "AnodeFrontEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v X1", &lf_sps, "X1", "CathodeEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v X2", &lf_sps, "X2", "CathodeEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Particle Identification v Focal Plane/Cathode v Xavg", &lf_sps, "Xavg", "CathodeEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

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

        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X1", &lf_sps, "X1", "DelayFrontRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X1", &lf_sps, "X1", "DelayFrontLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X2", &lf_sps, "X2", "DelayBackRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X2", &lf_sps, "X2", "DelayBackLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v Xavg", &lf_sps, "Xavg", "DelayBackRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v Xavg", &lf_sps, "Xavg", "DelayBackLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v Xavg", &lf_sps, "Xavg", "DelayFrontRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v Xavg", &lf_sps, "Xavg", "DelayFrontLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackRight v X1", &lf_sps, "X1", "DelayBackRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayBackLeft v X1", &lf_sps, "X1", "DelayBackLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontRight v X2", &lf_sps, "X2", "DelayFrontRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d("SE-SPS/Delay Lines v Focal Plane/DelayFrontLeft v X2", &lf_sps, "X2", "DelayFrontLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
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

pub fn pips1000(h: &mut Histogrammer, lf: LazyFrame) {
    let lf_pips = lf.with_columns( vec![
        // ( ( col("PIPS1000Energy") - col("PIPS1000Short") )/ col("PIPS1000Energy") ).alias("PIPS1000PSD"),
        (lit(-1.77049e-06)*col("PIPS1000Energy")*col("PIPS1000Energy") + lit(0.544755003513083)*col("PIPS1000Energy") + lit(-1.36822594543883)).alias("PIPS1000EnergyCalibrated") ]
    );

    h.add_fill_hist1d("PIPS1000: Energy", &lf_pips, "PIPS1000Energy", 16384, (0.0, 16384.0));
    // h.add_fill_hist2d("PIPS1000: PSD", &lf_pips, "PIPS1000Energy", "PIPS1000PSD", (512, 500), ((0.0, 4096.0), (0.0, 1.0)));
    h.add_fill_hist1d("PIPS1000: Energy Calibrated", &lf_pips, "PIPS1000EnergyCalibrated", 600, (0.0, 1200.0));


    
}

pub struct TimeCut {
    pub mean: f64,
    pub low: f64,
    pub high: f64
}

pub struct GainMatch {
    pub a: f64,
    pub b: f64,
    pub c: f64
}

pub struct EnergyCalibration {
    pub a: f64,
    pub b: f64,
    pub c: f64
}

pub fn cebra(h: &mut Histogrammer, lf: LazyFrame, detector_number: usize, timecut: Option<TimeCut>, gainmatch: Option<GainMatch>) {

    let i = detector_number;
    let lf_cebra = lf.filter(col(&format!("Cebra{}Energy", detector_number)).neq(lit(-1e6)));

    let column_names = get_column_names_from_lazyframe(&lf_cebra);
    
    // add the psd column if it doesn't exist
    let lf_cebra = lf_cebra.with_column(
            ( ( col(&format!("Cebra{}Energy", i)) - col(&format!("Cebra{}Short", i)) )/ col(&format!("Cebra{}Energy", i) ) ).alias(&format!("Cebra{}PSD", i))
        );

    h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}Energy", i, i), &lf_cebra, &format!("Cebra{}Energy", i), 512, (0.0, 4096.0));
    h.add_fill_hist2d(&format!("CeBrA/Cebra{}/PSD v Energy", i), &lf_cebra, &format!("Cebra{}Energy", i), &format!("Cebra{}PSD", i), (512, 512), ((0.0, 4096.0), (0.0, 1.0)));
    

    // Apply gain match if provided
    let lf_cebra = if let Some(ref gainmatch) = &gainmatch {
        let lf = lf_cebra.with_column(
            (lit(gainmatch.a) * col(&format!("Cebra{}Energy", i)).pow(2.0)
            + lit(gainmatch.b) * col(&format!("Cebra{}Energy", i))
            + lit(gainmatch.c)).alias(&format!("Cebra{}EnergyGM", i))
        );

        h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}EnergyGM", i, i), &lf, &format!("Cebra{}EnergyGM", i), 512, (0.0, 4096.0));

        lf

    } else {
        lf_cebra
    };

    let mut sps = false;

    // Check if ScintLeftTime exists
    let lf_cebra = if column_names.contains(&"ScintLeftTime".to_string()) {
        sps = true;
    
        // Check if Cebra#RelTime exists, if not, create it as Cebra#Time - ScintLeftTime
        if !column_names.contains(&format!("Cebra{}RelTime", i)) {
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
        h.add_fill_hist1d(&format!("CeBrA/Cebra{}/Cebra{}RelTime", i, i), &lf_cebra, &format!("Cebra{}RelTime", i), 3400, (-3200.0, 3200.0));
        h.add_fill_hist2d(&format!("CeBrA/Cebra{}/Cebra{}Energy v Xavg", i, i), &lf_cebra, "Xavg", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d(&format!("CeBrA/Cebra{}/Theta v Cebra{}RelTime ", i, i), &lf_cebra, &format!("Cebra{}RelTime", i), "Theta", (3400, 300), ((-3200.0, 3200.0), (0.0, PI)));
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

            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}RelTime", i, i), &lf_timecut, &format!("Cebra{}RelTime", i), 3400, (-3200.0, 3200.0));
            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}RelTimeShifted", i, i), &lf_timecut, &format!("Cebra{}RelTimeShifted", i), 100, (-50.0, 50.0));
            h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy", i, i), &lf_timecut, &format!("Cebra{}Energy", i), 512, (0.0, 4096.0));

            h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy v Xavg", i, i), &lf_timecut, "Xavg", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
            h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/Cebra{}Energy v X1", i, i), &lf_timecut, "X1", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

            if gainmatch.is_some() {
                h.add_fill_hist1d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM", i, i), &lf_timecut, &format!("Cebra{}EnergyGM", i), 512, (0.0, 4096.0));
                
                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM v Xavg", i, i), &lf_timecut, "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
                h.add_fill_hist2d(&"CeBrA/CebraEnergyGM v Xavg: TimeCut", &lf_timecut, "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
                
                h.add_fill_hist2d(&format!("CeBrA/Cebra{}/TimeCut/GainMatched/Cebra{}EnergyGM v X1", i, i), &lf_timecut, "X1", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
                h.add_fill_hist2d(&"CeBrA/CebraEnergyGM v X1: TimeCut", &lf_timecut, "X1", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
            }

        }
    };

}

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