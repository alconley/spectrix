use super::histogrammer::Histogrammer;

use polars::prelude::*;
use std::f64::consts::PI;

#[rustfmt::skip]
#[allow(clippy::all)]
pub fn add_histograms(lf: LazyFrame) -> Result<Histogrammer, PolarsError> {
    let mut h = Histogrammer::new();

    // Declare all the lazyframes that will be used
    let lf = lf.with_columns(vec![
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
        (col("ScintRightTime") - col("ScintLeftTime")).alias("ScintRightTime_ScintLeftTime")
    ]);

    let lf_bothplanes = lf.clone().filter(col("X1").neq(lit(-1e6))).filter(col("X2").neq(lit(-1e6)));
    let lf_only_x1_plane = lf.clone().filter(col("X1").neq(lit(-1e6))).filter(col("X2").eq(lit(-1e6)));
    let lf_only_x2_plane = lf.clone().filter(col("X2").neq(lit(-1e6))).filter(col("X1").eq(lit(-1e6)));


    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....

    // Focal Plane histograms
    h.add_fill_hist1d("X1", &lf, "X1", 600, (-300.0, 300.0));
    h.add_fill_hist1d("X1: only1plane", &lf_only_x1_plane, "X1", 600, (-300.0, 300.0));
    h.add_fill_hist1d("X1: bothplanes", &lf_bothplanes, "X1", 600, (-300.0, 300.0));

    h.add_fill_hist1d("X2", &lf, "X2", 600, (-300.0, 300.0));
    h.add_fill_hist1d("X2: only1plane", &lf_only_x2_plane, "X2", 600, (-300.0, 300.0));
    h.add_fill_hist1d("X2: bothplanes", &lf_bothplanes, "X2", 600, (-300.0, 300.0));

    h.add_fill_hist2d("X2 v X1", &lf, "X1", "X2", (600, 600), ((-300.0, 300.0), (-300.0, 300.0)));
    h.add_fill_hist1d("Xavg: bothplanes", &lf_bothplanes, "Xavg", 600, (-300.0, 300.0));
    h.add_fill_hist2d("Theta v Xavg: bothplanes", &lf_bothplanes, "Xavg", "Theta", (600, 300), ((-300.0, 300.0), (0.0, PI / 2.0)));

    // this is how you can pick which tabs the histograms will be displayed in
    let focal_plane_panes_names = vec![
        "X1", "X2", "X2 v X1", 
        "X1: bothplanes", "X2: bothplanes", "Xavg: bothplanes", 
        "X1: only1plane", "X2: only1plane", "Theta v Xavg: bothplanes"
    ];

    let focal_plane_panes = h.get_panes(focal_plane_panes_names);

    h.tabs.insert("Focal Plane".to_string(), focal_plane_panes);

    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....

    // Particle Identification histograms
    h.add_fill_hist2d("AnodeBack v ScintLeft", &lf, "ScintLeftEnergy", "AnodeBackEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeFront v ScintLeft", &lf, "ScintLeftEnergy", "AnodeFrontEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
    h.add_fill_hist2d("Cathode v ScintLeft", &lf, "ScintLeftEnergy", "CathodeEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeBack v ScintRight", &lf, "ScintRightEnergy", "AnodeBackEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeFront v ScintRight", &lf, "ScintRightEnergy", "AnodeFrontEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
    h.add_fill_hist2d("Cathode v ScintRight", &lf, "ScintRightEnergy", "CathodeEnergy", (512, 512), ((0.0, 4096.0), (0.0, 4096.0)));
    
    let particle_id_panes_names = vec![
        "AnodeBack v ScintLeft", "AnodeFront v ScintLeft", "Cathode v ScintLeft",
        "AnodeBack v ScintRight", "AnodeFront v ScintRight", "Cathode v ScintRight"
    ];

    let particle_id_panes = h.get_panes(particle_id_panes_names);

    h.tabs.insert("Particle Identification".to_string(), particle_id_panes);

    let pid_pane = h.get_panes(vec!["AnodeBack v ScintLeft"]);
    h.tabs.insert("PID".to_string(), pid_pane);


    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
    
    // Particle Identification vs Focal plane histograms
    h.add_fill_hist2d("ScintLeft v X1", &lf, "X1", "ScintLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("ScintLeft v X2", &lf, "X2", "ScintLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("ScintLeft v Xavg", &lf, "Xavg", "ScintLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("ScintRight v X1", &lf, "X1", "ScintRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("ScintRight v X2", &lf, "X2", "ScintRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("ScintRight v Xavg", &lf, "Xavg", "ScintRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeBack v X1", &lf, "X1", "AnodeBackEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeBack v X2", &lf, "X2", "AnodeBackEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeBack v Xavg", &lf, "Xavg", "AnodeBackEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeFront v X1", &lf, "X1", "AnodeFrontEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeFront v X2", &lf, "X2", "AnodeFrontEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("AnodeFront v Xavg", &lf, "Xavg", "AnodeFrontEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("Cathode v X1", &lf, "X1", "CathodeEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("Cathode v X2", &lf, "X2", "CathodeEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("Cathode v Xavg", &lf, "Xavg", "CathodeEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

    let particle_id_vs_focal_plane_panes_names = vec![
        "ScintLeft v X1", "ScintLeft v X2", "ScintLeft v Xavg",
        "ScintRight v X1", "ScintRight v X2", "ScintRight v Xavg",
        "AnodeBack v X1", "AnodeBack v X2", "AnodeBack v Xavg",
        "AnodeFront v X1", "AnodeFront v X2", "AnodeFront v Xavg",
        "Cathode v X1", "Cathode v X2", "Cathode v Xavg"
    ];

    let particle_id_vs_focal_plane_panes = h.get_panes(particle_id_vs_focal_plane_panes_names);

    h.tabs.insert("Particle Identification v Focal Plane".to_string(), particle_id_vs_focal_plane_panes);

    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....

    // Delay lines vs Focal plane histograms

    h.add_fill_hist2d("DelayBackRight v X1", &lf, "X1", "DelayBackRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayBackLeft v X1", &lf, "X1", "DelayBackLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontRight v X1", &lf, "X1", "DelayFrontRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontLeft v X1", &lf, "X1", "DelayFrontLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayBackRight v X2", &lf, "X2", "DelayBackRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayBackLeft v X2", &lf, "X2", "DelayBackLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontRight v X2", &lf, "X2", "DelayFrontRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontLeft v X2", &lf, "X2", "DelayFrontLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayBackRight v Xavg", &lf, "Xavg", "DelayBackRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayBackLeft v Xavg", &lf, "Xavg", "DelayBackLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontRight v Xavg", &lf, "Xavg", "DelayFrontRightEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontLeft v Xavg", &lf, "Xavg", "DelayFrontLeftEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontAverage v X1", &lf, "X1", "DelayFrontAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayBackAverage v X1", &lf, "X1", "DelayBackAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontAverage v X2", &lf, "X2", "DelayFrontAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayBackAverage v X2", &lf, "X2", "DelayBackAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayFrontAverage v Xavg", &lf, "Xavg", "DelayFrontAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    h.add_fill_hist2d("DelayBackAverage v Xavg", &lf, "Xavg", "DelayBackAverageEnergy", (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
    
    let delay_lines_panes_names = vec![
        "DelayBackRight v X1", "DelayBackLeft v X1", "DelayFrontRight v X1", "DelayFrontLeft v X1",
        "DelayBackRight v X2", "DelayBackLeft v X2", "DelayFrontRight v X2", "DelayFrontLeft v X2",
        "DelayBackRight v Xavg", "DelayBackLeft v Xavg", "DelayFrontRight v Xavg", "DelayFrontLeft v Xavg",
        "DelayFrontAverage v X1", "DelayBackAverage v X1", "DelayFrontAverage v X2", "DelayBackAverage v X2",
        "DelayFrontAverage v Xavg", "DelayBackAverage v Xavg"
    ];

    let delay_lines_panes = h.get_panes(delay_lines_panes_names);

    h.tabs.insert("Delay Lines v Focal Plane".to_string(), delay_lines_panes);

    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
    /* 
    // Delay timing relative to anodes histograms

    h.add_fill_hist1d("DelayFrontLeftTime-AnodeFrontTime: bothplanes", &lf_bothplanes, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayFrontRightTime-AnodeFrontTime: bothplanes", &lf_bothplanes, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackLeftTime-AnodeBackTime: bothplanes", &lf_bothplanes, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackRightTime-AnodeBackTime: bothplanes", &lf_bothplanes, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));

    h.add_fill_hist1d("DelayFrontLeftTime-AnodeFrontTime: onlyX1", &lf_only_x1_plane, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayFrontRightTime-AnodeFrontTime: onlyX1", &lf_only_x1_plane, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackLeftTime-AnodeFrontTime: onlyX1", &lf_only_x1_plane, "DelayBackLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackRightTime-AnodeFrontTime: onlyX1", &lf_only_x1_plane, "DelayBackRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayFrontLeftTime-AnodeBackTime: onlyX1", &lf_only_x1_plane, "DelayFrontLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayFrontRightTime-AnodeBackTime: onlyX1", &lf_only_x1_plane, "DelayFrontRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackLeftTime-AnodeBackTime: onlyX1", &lf_only_x1_plane, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackRightTime-AnodeBackTime: onlyX1", &lf_only_x1_plane, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));

    h.add_fill_hist1d("DelayFrontLeftTime-AnodeFrontTime: onlyX2", &lf_only_x2_plane, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayFrontRightTime-AnodeFrontTime: onlyX2", &lf_only_x2_plane, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackLeftTime-AnodeFrontTime: onlyX2", &lf_only_x2_plane, "DelayBackLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackRightTime-AnodeFrontTime: onlyX2", &lf_only_x2_plane, "DelayBackRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayFrontLeftTime-AnodeBackTime: onlyX2", &lf_only_x2_plane, "DelayFrontLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayFrontRightTime-AnodeBackTime: onlyX2", &lf_only_x2_plane, "DelayFrontRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackLeftTime-AnodeBackTime: onlyX2", &lf_only_x2_plane, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    h.add_fill_hist1d("DelayBackRightTime-AnodeBackTime: onlyX2", &lf_only_x2_plane, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));

    let lf_time_rel_backanode = lf.clone().filter(col("AnodeBackTime").neq(lit(-1e6))).filter(col("ScintLeftTime").neq(lit(-1e6)));
    h.add_fill_hist1d("AnodeFrontTime-AnodeBackTime", &lf_time_rel_backanode, "AnodeFrontTime_AnodeBackTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist1d("AnodeBackTime-AnodeFrontTime", &lf_time_rel_backanode, "AnodeBackTime_AnodeFrontTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist1d("AnodeFrontTime-ScintLeftTime", &lf_time_rel_backanode, "AnodeFrontTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist1d("AnodeBackTime-ScintLeftTime", &lf_time_rel_backanode, "AnodeBackTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist1d("DelayFrontLeftTime-ScintLeftTime", &lf_time_rel_backanode, "DelayFrontLeftTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist1d("DelayFrontRightTime-ScintLeftTime", &lf_time_rel_backanode, "DelayFrontRightTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist1d("DelayBackLeftTime-ScintLeftTime", &lf_time_rel_backanode, "DelayBackLeftTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist1d("DelayBackRightTime-ScintLeftTime", &lf_time_rel_backanode, "DelayBackRightTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist1d("ScintRightTime-ScintLeftTime", &lf_time_rel_backanode, "ScintRightTime_ScintLeftTime", 1000, (-3000.0, 3000.0));
    h.add_fill_hist2d("ScintTimeDif v Xavg", &lf_time_rel_backanode, "Xavg", "ScintRightTime_ScintLeftTime", (600, 12800), ((-300.0, 300.0), (-3200.0, 3200.0)));

    let delay_timing_panes_names = vec![
        "DelayFrontLeftTime-AnodeFrontTime: bothplanes", "DelayFrontRightTime-AnodeFrontTime: bothplanes",
        "DelayBackLeftTime-AnodeBackTime: bothplanes", "DelayBackRightTime-AnodeBackTime: bothplanes",
        "DelayFrontLeftTime-AnodeFrontTime: onlyX1", "DelayFrontRightTime-AnodeFrontTime: onlyX1",
        "DelayBackLeftTime-AnodeFrontTime: onlyX1", "DelayBackRightTime-AnodeFrontTime: onlyX1",
        "DelayFrontLeftTime-AnodeBackTime: onlyX1", "DelayFrontRightTime-AnodeBackTime: onlyX1",
        "DelayBackLeftTime-AnodeBackTime: onlyX1", "DelayBackRightTime-AnodeBackTime: onlyX1",
        "DelayFrontLeftTime-AnodeFrontTime: onlyX2", "DelayFrontRightTime-AnodeFrontTime: onlyX2",
        "DelayBackLeftTime-AnodeFrontTime: onlyX2", "DelayBackRightTime-AnodeFrontTime: onlyX2",
        "DelayFrontLeftTime-AnodeBackTime: onlyX2", "DelayFrontRightTime-AnodeBackTime: onlyX2",
        "DelayBackLeftTime-AnodeBackTime: onlyX2", "DelayBackRightTime-AnodeBackTime: onlyX2",
        "AnodeFrontTime-AnodeBackTime", "AnodeBackTime-AnodeFrontTime",
        "AnodeFrontTime-ScintLeftTime", "AnodeBackTime-ScintLeftTime",
        "DelayFrontLeftTime-ScintLeftTime", "DelayFrontRightTime-ScintLeftTime",
        "DelayBackLeftTime-ScintLeftTime", "DelayBackRightTime-ScintLeftTime",
        "ScintRightTime-ScintLeftTime", "ScintTimeDif v Xavg"
    ];

    let delay_timing_panes = h.get_panes(delay_timing_panes_names);

    h.tabs.insert("Delay Timing".to_string(), delay_timing_panes);
    */
    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....

    // CeBrA plots
    // declare the gain matched energy columns, and time to scint left columns
    let lf = lf.with_columns(vec![
        (col("Cebra0Energy") * lit(1.0) + lit(0.0)).alias("Cebra0EnergyGM"),
        (col("Cebra1Energy") * lit(0.99398158) + lit(0.8603429)).alias("Cebra1EnergyGM"),
        (col("Cebra2Energy") * lit(0.97459773) + lit(5.16109648)).alias("Cebra2EnergyGM"),
        (col("Cebra3Energy") * lit(0.98075836) + lit(4.03824526)).alias("Cebra3EnergyGM"),
        (col("Cebra4Energy") * lit(0.83952309) + lit(3.9492804)).alias("Cebra4EnergyGM"),
        (col("Cebra5Energy") * lit(0.83752932) + lit(1.99392618)).alias("Cebra5EnergyGM"),
        (col("Cebra6Energy") * lit(0.73315054) + lit(3.67471898)).alias("Cebra6EnergyGM"),
        (col("Cebra7Energy") * lit(0.9575986) + lit(5.40847873)).alias("Cebra7EnergyGM"),
        (col("Cebra8Energy") * lit(0.99018772) + lit(0.03972928)).alias("Cebra8EnergyGM"),
        (col("Cebra0Time") - col("ScintLeftTime")).alias("Cebra0Time_ScintLeftTime"),
        (col("Cebra1Time") - col("ScintLeftTime")).alias("Cebra1Time_ScintLeftTime"),
        (col("Cebra2Time") - col("ScintLeftTime")).alias("Cebra2Time_ScintLeftTime"),
        (col("Cebra3Time") - col("ScintLeftTime")).alias("Cebra3Time_ScintLeftTime"),
        (col("Cebra4Time") - col("ScintLeftTime")).alias("Cebra4Time_ScintLeftTime"),
        (col("Cebra5Time") - col("ScintLeftTime")).alias("Cebra5Time_ScintLeftTime"),
        (col("Cebra6Time") - col("ScintLeftTime")).alias("Cebra6Time_ScintLeftTime"),
        (col("Cebra7Time") - col("ScintLeftTime")).alias("Cebra7Time_ScintLeftTime"),
        (col("Cebra8Time") - col("ScintLeftTime")).alias("Cebra8Time_ScintLeftTime"),

        (col("Cebra0Time") / lit(1.0e9)).alias("Cebra0Time_Seconds"),
        (col("Cebra1Time") / lit(1.0e9)).alias("Cebra1Time_Seconds"),
        (col("Cebra2Time") / lit(1.0e9)).alias("Cebra2Time_Seconds"),
        (col("Cebra3Time") / lit(1.0e9)).alias("Cebra3Time_Seconds"),
        (col("Cebra4Time") / lit(1.0e9)).alias("Cebra4Time_Seconds"),
        (col("Cebra5Time") / lit(1.0e9)).alias("Cebra5Time_Seconds"),
        (col("Cebra6Time") / lit(1.0e9)).alias("Cebra6Time_Seconds"),
        (col("Cebra7Time") / lit(1.0e9)).alias("Cebra7Time_Seconds"),
        (col("Cebra8Time") / lit(1.0e9)).alias("Cebra8Time_Seconds"),
    
    ]);

    let time_shits = vec![
        1148.0, 1152.0, 1151.0, 1151.0, 1125.0, 1121.0, 1119.0, 1118.0, 1126.0
    ];

    let time_cut_width = 6.0;

    let mut time_lf = Vec::new();
    let mut time_cut_lf = Vec::new();
    for i in 0..9 {
        let det_time_lf = lf.clone()
            .filter(col(&format!("Cebra{}Time", i)).neq(lit(-1e6)))
            .filter(col("ScintLeftTime").neq(lit(-1e6)))
            .filter(col("AnodeBackTime").neq(lit(-1e6)))
            .with_column((col(&format!("Cebra{}Time_ScintLeftTime", i)) + lit(time_shits[i])).alias(&format!("Cebra{}Time_ScintLeftTime_Shifted", i)));
    

        let det_time_cut_lf = det_time_lf.clone()
            .filter(col(&format!("Cebra{}Time_ScintLeftTime_Shifted", i)).gt(-time_cut_width))
            .filter(col(&format!("Cebra{}Time_ScintLeftTime_Shifted", i)).lt(time_cut_width));

        time_lf.push(det_time_lf);
        time_cut_lf.push(det_time_cut_lf);
    }

    h.add_hist1d("CeBrA Gain Matched", 512, (0.0, 4096.0));
    h.add_hist1d("CeBrA Time to Scint Shifted with TCut", 100, (-50.0, 50.0));
    h.add_hist2d("CeBrA Gain Matched vs Xavg with TCut", (600,512), ((-300.0, 300.0),(0.0, 4096.0)));
    
    for i in 0..1 {
        // Raw CeBrA Histograms
        h.add_fill_hist1d(&format!("Cebra{}Energy", i), &lf, &format!("Cebra{}Energy", i), 512, (0.0, 4096.0));
        h.add_fill_hist2d(&format!("Cebra{}Energy v Xavg", i), &lf, "Xavg", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        h.add_fill_hist2d(&format!("Cebra{}Energy v Cebra{}Time", i, i), &lf, &format!("Cebra{}Time_Seconds", i), &format!("Cebra{}Energy", i), (3600, 512), ((0.0, 3600.0), (0.0, 4096.0)));
        h.add_fill_hist1d(&format!("Cebra{}Time-ScintLeftTime", i), &time_lf[i], &format!("Cebra{}Time_ScintLeftTime", i), 3200, (-1600.0, 1600.0));
        h.add_fill_hist1d(&format!("Cebra{}Time-ScintLeftTime Shifted", i), &time_lf[i], &format!("Cebra{}Time_ScintLeftTime_Shifted", i), 3200, (-1600.0, 1600.0));

        // Gain Matched Histograms
        h.add_fill_hist1d(&format!("Cebra{}EnergyGM", i), &lf, &format!("Cebra{}EnergyGM", i), 512, (0.0, 4096.0));
        h.add_fill_hist2d(&format!("Cebra{}EnergyGainMatched v Xavg", i), &lf, "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

        // Time Cut Histograms
        h.add_fill_hist1d(&format!("Cebra{}Time-ScintLeftTime Shifted Time Cut", i), &time_cut_lf[i], &format!("Cebra{}Time_ScintLeftTime_Shifted", i), 100, (-50.0, 50.0));
        h.add_fill_hist1d(&format!("Cebra{}Energy Time Cut", i), &time_cut_lf[i], &format!("Cebra{}Energy", i), 512, (0.0, 4096.0));
        h.add_fill_hist2d(&format!("Cebra{}Energy v Xavg Time Cut", i), &time_cut_lf[i], "Xavg", &format!("Cebra{}Energy", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));

        h.add_fill_hist1d(&format!("Cebra{}EnergyGM Time Cut", i), &time_cut_lf[i], &format!("Cebra{}EnergyGM", i), 512, (0.0, 4096.0));
        h.add_fill_hist2d(&format!("Cebra{}EnergyGM v Xavg Time Cut", i), &time_cut_lf[i], "Xavg", &format!("Cebra{}EnergyGM", i), (600, 512), ((-300.0, 300.0), (0.0, 4096.0)));
        
        let cebra_det_panes_name_strings: Vec<String> = vec![
            format!("Cebra{}Energy", i), 
            format!("Cebra{}Energy v Xavg", i), 
            format!("Cebra{}Energy v Cebra{}Time", i, i),
            format!("Cebra{}Time-ScintLeftTime", i), 
            format!("Cebra{}Time-ScintLeftTime Shifted", i),
            format!("Cebra{}EnergyGM", i), 
            format!("Cebra{}EnergyGainMatched v Xavg", i),
            format!("Cebra{}Time-ScintLeftTime Shifted Time Cut", i), 
            format!("Cebra{}Energy Time Cut", i), 
            format!("Cebra{}Energy v Xavg Time Cut", i),
            format!("Cebra{}EnergyGM Time Cut", i), 
            format!("Cebra{}EnergyGM v Xavg Time Cut", i)
        ];

        let cebra_det_panes_name: Vec<&str> = cebra_det_panes_name_strings.iter().map(|s| s.as_str()).collect();

        let cebra_det_panes = h.get_panes(cebra_det_panes_name);

        h.tabs.insert(format!("CeBr3 Detector {}", i).to_string(), cebra_det_panes);

        // fill the combined histograms
        h.fill_hist1d(&format!("CeBrA Gain Matched"), &lf, &format!("Cebra{}EnergyGM", i));
        h.fill_hist1d(&format!("CeBrA Time to Scint Shifted with TCut"), &time_cut_lf[i], &format!("Cebra{}Time_ScintLeftTime_Shifted", i));
        h.fill_hist2d(&format!("CeBrA Gain Matched vs Xavg with TCut"), &time_cut_lf[i], "Xavg", &format!("Cebra{}EnergyGM", i));
    }

    //....oooOO0OOooo........oooOO0OOooo........oooOO0OOooo........oooOO0OOooo....
    let cebra_panes_names = vec![
        "CeBrA Gain Matched", "CeBrA Time to Scint Shifted with TCut", "CeBrA Gain Matched vs Xavg with TCut"
    ];

    let cebra_panes = h.get_panes(cebra_panes_names);
    h.tabs.insert("CeBrA".to_string(), cebra_panes);

    Ok(h)
}
