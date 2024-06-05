use super::histogrammer::Histogrammer;
use polars::prelude::*;
use std::f64::consts::PI;

pub fn add_histograms(lf: LazyFrame) -> Result<Histogrammer, PolarsError> {
    let mut h = Histogrammer::new();

    let lf = lf.with_columns(vec![
        (col("DelayFrontRightEnergy") + col("DelayFrontLeftEnergy") / lit(2.0))
            .alias("DelayFrontAverageEnergy"),
        (col("DelayBackRightEnergy") + col("DelayBackLeftEnergy") / lit(2.0))
            .alias("DelayBackAverageEnergy"),
        (col("DelayFrontLeftTime") - col("AnodeFrontTime"))
            .alias("DelayFrontLeftTime_AnodeFrontTime"),
        (col("DelayFrontRightTime") - col("AnodeFrontTime"))
            .alias("DelayFrontRightTime_AnodeFrontTime"),
        (col("DelayBackLeftTime") - col("AnodeFrontTime"))
            .alias("DelayBackLeftTime_AnodeFrontTime"),
        (col("DelayBackRightTime") - col("AnodeFrontTime"))
            .alias("DelayBackRightTime_AnodeFrontTime"),
        (col("DelayFrontLeftTime") - col("AnodeBackTime"))
            .alias("DelayFrontLeftTime_AnodeBackTime"),
        (col("DelayFrontRightTime") - col("AnodeBackTime"))
            .alias("DelayFrontRightTime_AnodeBackTime"),
        (col("DelayBackLeftTime") - col("AnodeBackTime")).alias("DelayBackLeftTime_AnodeBackTime"),
        (col("DelayBackRightTime") - col("AnodeBackTime"))
            .alias("DelayBackRightTime_AnodeBackTime"),
        (col("AnodeFrontTime") - col("AnodeBackTime")).alias("AnodeFrontTime_AnodeBackTime"),
        (col("AnodeBackTime") - col("AnodeFrontTime")).alias("AnodeBackTime_AnodeFrontTime"),
        (col("AnodeFrontTime") - col("ScintLeftTime")).alias("AnodeFrontTime_ScintLeftTime"),
        (col("AnodeBackTime") - col("ScintLeftTime")).alias("AnodeBackTime_ScintLeftTime"),
        (col("DelayFrontLeftTime") - col("ScintLeftTime"))
            .alias("DelayFrontLeftTime_ScintLeftTime"),
        (col("DelayFrontRightTime") - col("ScintLeftTime"))
            .alias("DelayFrontRightTime_ScintLeftTime"),
        (col("DelayBackLeftTime") - col("ScintLeftTime")).alias("DelayBackLeftTime_ScintLeftTime"),
        (col("DelayBackRightTime") - col("ScintLeftTime"))
            .alias("DelayBackRightTime_ScintLeftTime"),
        (col("ScintRightTime") - col("ScintLeftTime")).alias("ScintRightTime_ScintLeftTime"),
    ]);

    // h.add_fill_hist1d("Cebra0Energy", &lf, "Cebra0Energy", 512, (0.0, 4096.0));
    // h.add_fill_hist1d("Cebra1Energy", &lf, "Cebra1Energy", 512, (0.0, 4096.0));
    // h.add_fill_hist1d("Cebra2Energy", &lf, "Cebra2Energy", 512, (0.0, 4096.0));
    // h.add_fill_hist1d("Cebra3Energy", &lf, "Cebra3Energy", 512, (0.0, 4096.0));
    // h.add_fill_hist1d("Cebra4Energy", &lf, "Cebra4Energy", 512, (0.0, 4096.0));
    /*

    h.add_fill_hist1d("X1", &lf, "X1", 600, (-300.0, 300.0));
    h.add_fill_hist1d("X2", &lf, "X2", 600, (-300.0, 300.0));
    h.add_fill_hist2d(
        "X2 v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "X2",
        600,
        (-300.0, 300.0),
    );
    h.add_fill_hist2d(
        "DelayBackRight v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "DelayBackRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayBackLeft v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "DelayBackLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontRight v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "DelayFrontRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontLeft v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "DelayFrontLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayBackRight v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "DelayBackRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayBackLeft v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "DelayBackLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontRight v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "DelayFrontRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontLeft v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "DelayFrontLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayBackRight v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "DelayBackRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayBackLeft v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "DelayBackLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontRight v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "DelayFrontRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontLeft v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "DelayFrontLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontAverage v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "DelayFrontAverageEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayBackAverage v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "DelayBackAverageEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontAverage v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "DelayFrontAverageEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayBackAverage v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "DelayBackAverageEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayFrontAverage v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "DelayFrontAverageEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "DelayBackAverage v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "DelayBackAverageEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeBack v ScintLeft",
        &lf,
        "ScintLeftEnergy",
        512,
        (0.0, 4096.0),
        "AnodeBackEnergy",
        512,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeFront v ScintLeft",
        &lf,
        "ScintLeftEnergy",
        256,
        (0.0, 4096.0),
        "AnodeFrontEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "Cathode v ScintLeft",
        &lf,
        "ScintLeftEnergy",
        256,
        (0.0, 4096.0),
        "CathodeEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeBack v ScintRight",
        &lf,
        "ScintRightEnergy",
        256,
        (0.0, 4096.0),
        "AnodeBackEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeFront v ScintRight",
        &lf,
        "ScintRightEnergy",
        256,
        (0.0, 4096.0),
        "AnodeFrontEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "Cathode v ScintRight",
        &lf,
        "ScintRightEnergy",
        256,
        (0.0, 4096.0),
        "CathodeEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "ScintLeft v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "ScintLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "ScintLeft v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "ScintLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "ScintLeft v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "ScintLeftEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "ScintRight v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "ScintRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "ScintRight v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "ScintRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "ScintRight v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "ScintRightEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeBack v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "AnodeBackEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeBack v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "AnodeBackEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeBack v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "AnodeBackEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeFront v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "AnodeFrontEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeFront v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "AnodeFrontEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "AnodeFront v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "AnodeFrontEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "Cathode v X1",
        &lf,
        "X1",
        600,
        (-300.0, 300.0),
        "CathodeEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "Cathode v X2",
        &lf,
        "X2",
        600,
        (-300.0, 300.0),
        "CathodeEnergy",
        256,
        (0.0, 4096.0),
    );
    h.add_fill_hist2d(
        "Cathode v Xavg",
        &lf,
        "Xavg",
        600,
        (-300.0, 300.0),
        "CathodeEnergy",
        256,
        (0.0, 4096.0),
    );
    */

    // Both planes histograms
    let lf_bothplanes = lf
        .clone()
        .filter(col("X1").neq(lit(-1e6)))
        .filter(col("X2").neq(lit(-1e6)));

    // h.add_fill_hist1d("X1: bothplanes", &lf_bothplanes, "X1", 600, (-300.0, 300.0));
    // h.add_fill_hist1d("X2: bothplanes", &lf_bothplanes, "X2", 600, (-300.0, 300.0));

    h.add_fill_hist1d(
        "Xavg: bothplanes",
        &lf_bothplanes,
        "Xavg",
        600,
        (-300.0, 300.0),
    );

    h.add_fill_hist2d(
        "Theta v Xavg: bothplanes",
        &lf_bothplanes,
        "Xavg",
        "Theta",
        (600, 300),
        ((-300.0, 300.0), (0.0, PI / 2.0)),
    );
    /*
    // h.add_fill_hist1d("DelayFrontLeftTime_relTo_AnodeFrontTime_bothplanes", &lf_bothplanes, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayFrontRightTime_relTo_AnodeFrontTime_bothplanes", &lf_bothplanes, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackLeftTime_relTo_AnodeBackTime_bothplanes", &lf_bothplanes, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackRightTime_relTo_AnodeBackTime_bothplanes", &lf_bothplanes, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));

    // Only 1 plane: X1
    let lf_only_x1_plane = lf
        .clone()
        .filter(col("X1").neq(lit(-1e6)))
        .filter(col("X2").eq(lit(-1e6)));

    h.add_fill_hist1d(
        "X1: only1plane",
        &lf_only_x1_plane,
        "X1",
        600,
        (-300.0, 300.0),
    );
    // h.add_fill_hist1d("DelayFrontLeftTime_relTo_AnodeFrontTime_noX2", &lf_only_x1_plane, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayFrontRightTime_relTo_AnodeFrontTime_noX2", &lf_only_x1_plane, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackLeftTime_relTo_AnodeFrontTime_noX2", &lf_only_x1_plane, "DelayBackLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackRightTime_relTo_AnodeFrontTime_noX2", &lf_only_x1_plane, "DelayBackRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayFrontLeftTime_relTo_AnodeBackTime_noX2", &lf_only_x1_plane, "DelayFrontLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayFrontRightTime_relTo_AnodeBackTime_noX2", &lf_only_x1_plane, "DelayFrontRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackLeftTime_relTo_AnodeBackTime_noX2", &lf_only_x1_plane, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackRightTime_relTo_AnodeBackTime_noX2", &lf_only_x1_plane, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));

    // Only 1 plane: X2
    let lf_only_x2_plane = lf
        .clone()
        .filter(col("X2").neq(lit(-1e6)))
        .filter(col("X1").eq(lit(-1e6)));

    h.add_fill_hist1d(
        "X2: only1plane",
        &lf_only_x2_plane,
        "X2",
        600,
        (-300.0, 300.0),
    );
    // h.add_fill_hist1d("DelayFrontLeftTime_relTo_AnodeFrontTime_noX1", &lf_only_x2_plane, "DelayFrontLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayFrontRightTime_relTo_AnodeFrontTime_noX1", &lf_only_x2_plane, "DelayFrontRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackLeftTime_relTo_AnodeFrontTime_noX1", &lf_only_x2_plane, "DelayBackLeftTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackRightTime_relTo_AnodeFrontTime_noX1", &lf_only_x2_plane, "DelayBackRightTime_AnodeFrontTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayFrontLeftTime_relTo_AnodeBackTime_noX1", &lf_only_x2_plane, "DelayFrontLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayFrontRightTime_relTo_AnodeBackTime_noX1", &lf_only_x2_plane, "DelayFrontRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackLeftTime_relTo_AnodeBackTime_noX1", &lf_only_x2_plane, "DelayBackLeftTime_AnodeBackTime", 8000, (-4000.0, 4000.0));
    // h.add_fill_hist1d("DelayBackRightTime_relTo_AnodeBackTime_noX1", &lf_only_x2_plane, "DelayBackRightTime_AnodeBackTime", 8000, (-4000.0, 4000.0));

    // Time relative to Back Anode

    let lf_time_rel_backanode = lf
        .clone()
        .filter(col("AnodeBackTime").neq(lit(-1e6)))
        .filter(col("ScintLeftTime").neq(lit(-1e6)));

    h.add_fill_hist1d(
        "AnodeFrontTime-AnodeBackTime",
        &lf_time_rel_backanode,
        "AnodeFrontTime_AnodeBackTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist1d(
        "AnodeBackTime-AnodeFrontTime",
        &lf_time_rel_backanode,
        "AnodeBackTime_AnodeFrontTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist1d(
        "AnodeFrontTime-ScintLeftTime",
        &lf_time_rel_backanode,
        "AnodeFrontTime_ScintLeftTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist1d(
        "AnodeBackTime-ScintLeftTime",
        &lf_time_rel_backanode,
        "AnodeBackTime_ScintLeftTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist1d(
        "DelayFrontLeftTime-ScintLeftTime",
        &lf_time_rel_backanode,
        "DelayFrontLeftTime_ScintLeftTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist1d(
        "DelayFrontRightTime-ScintLeftTime",
        &lf_time_rel_backanode,
        "DelayFrontRightTime_ScintLeftTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist1d(
        "DelayBackLeftTime-ScintLeftTime",
        &lf_time_rel_backanode,
        "DelayBackLeftTime_ScintLeftTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist1d(
        "DelayBackRightTime-ScintLeftTime",
        &lf_time_rel_backanode,
        "DelayBackRightTime_ScintLeftTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist1d(
        "ScintRightTime-ScintLeftTime",
        &lf_time_rel_backanode,
        "ScintRightTime_ScintLeftTime",
        1000,
        (-3000.0, 3000.0),
    );
    h.add_fill_hist2d(
        "ScintTimeDif v Xavg",
        &lf_time_rel_backanode,
        "Xavg",
        600,
        (-300.0, 300.0),
        "ScintRightTime_ScintLeftTime",
        12800,
        (-3200.0, 3200.0),
    );
    */
    Ok(h)
}
