use polars::prelude::*;
use std::collections::HashMap;

#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct LazyFrameInfo {
    pub lfs: Vec<String>,
    pub columns: Vec<String>,
}

#[derive(Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct LazyFrames {
    #[serde(skip)]
    pub lfs: HashMap<String, LazyFrame>,
    pub info: LazyFrameInfo,
}

impl LazyFrames {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::all)]
    pub fn add_columns_to_lazyframe(&self, lf: &LazyFrame) -> LazyFrame {
        let lf = lf.clone().with_columns(vec![
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
            (col("DelayBackLeftTime") - col("AnodeBackTime"))
                .alias("DelayBackLeftTime_AnodeBackTime"),
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
            (col("DelayBackLeftTime") - col("ScintLeftTime"))
                .alias("DelayBackLeftTime_ScintLeftTime"),
            (col("DelayBackRightTime") - col("ScintLeftTime"))
                .alias("DelayBackRightTime_ScintLeftTime"),
            (col("ScintRightTime") - col("ScintLeftTime")).alias("ScintRightTime_ScintLeftTime"),
        ]);

        lf
    }

    pub fn filtered_lfs(&self, lf: LazyFrame) -> HashMap<String, LazyFrame> {
        let mut lfs = HashMap::new();
        let pid = lf
            .clone()
            .filter(col("ScintLeftEnergy").neq(lit(-1e6)))
            .filter(col("AnodeBackEnergy").neq(lit(-1e6)));

        let lf_bothplanes = lf
            .clone()
            .filter(col("X1").neq(lit(-1e6)))
            .filter(col("X2").neq(lit(-1e6)));

        let lf_only_x1_plane = lf
            .clone()
            .filter(col("X1").neq(lit(-1e6)))
            .filter(col("X2").eq(lit(-1e6)));

        let lf_only_x2_plane = lf
            .clone()
            .filter(col("X2").neq(lit(-1e6)))
            .filter(col("X1").eq(lit(-1e6)));

        lfs.insert("Raw".to_string(), lf);
        lfs.insert("BothPlanes".to_string(), lf_bothplanes);
        lfs.insert("OnlyX1Plane".to_string(), lf_only_x1_plane);
        lfs.insert("OnlyX2Plane".to_string(), lf_only_x2_plane);
        lfs.insert("PID".to_string(), pid);

        lfs
    }

    #[allow(clippy::all)]
    pub fn main_column_names(&self) -> Vec<String> {
        let main_sps_columns = vec![
            "AnodeFrontEnergy",
            "AnodeFrontShort",
            "AnodeFrontTime",
            "AnodeBackEnergy",
            "AnodeBackShort",
            "AnodeBackTime",
            "ScintLeftEnergy",
            "ScintLeftShort",
            "ScintLeftTime",
            "ScintRightEnergy",
            "ScintRightShort",
            "ScintRightTime",
            "CathodeEnergy",
            "CathodeShort",
            "CathodeTime",
            "DelayFrontLeftEnergy",
            "DelayFrontLeftShort",
            "DelayFrontLeftTime",
            "DelayFrontRightEnergy",
            "DelayFrontRightShort",
            "DelayFrontRightTime",
            "DelayBackLeftEnergy",
            "DelayBackLeftShort",
            "DelayBackLeftTime",
            "DelayBackRightEnergy",
            "DelayBackRightShort",
            "DelayBackRightTime",
            "X1",
            "X2",
            "Xavg",
            "Theta",
        ];

        let extra_sps_columns = vec![
            "DelayFrontAverageEnergy",
            "DelayBackAverageEnergy",
            "DelayFrontLeftTime_AnodeFrontTime",
            "DelayFrontRightTime_AnodeFrontTime",
            "DelayBackLeftTime_AnodeFrontTime",
            "DelayBackRightTime_AnodeFrontTime",
            "DelayFrontLeftTime_AnodeBackTime",
            "DelayFrontRightTime_AnodeBackTime",
            "DelayBackLeftTime_AnodeBackTime",
            "DelayBackRightTime_AnodeBackTime",
            "AnodeFrontTime_AnodeBackTime",
            "AnodeBackTime_AnodeFrontTime",
            "AnodeFrontTime_ScintLeftTime",
            "AnodeBackTime_ScintLeftTime",
            "DelayFrontLeftTime_ScintLeftTime",
            "DelayFrontRightTime_ScintLeftTime",
            "DelayBackLeftTime_ScintLeftTime",
            "DelayBackRightTime_ScintLeftTime",
            "ScintRightTime_ScintLeftTime",
        ];

        let mut columns = main_sps_columns
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();
        columns.extend(extra_sps_columns.iter().map(|&s| s.to_string()));

        columns
    }

    pub fn main_lfs_names(&self) -> Vec<String> {
        let lfs: Vec<String> = ["Raw", "BothPlanes", "OnlyX1Plane", "OnlyX2Plane", "PID"]
            .iter()
            .map(|&s| s.to_string())
            .collect();

        lfs
    }

    pub fn get_lf(&self, key: &str) -> Option<&LazyFrame> {
        self.lfs.get(key)
    }

    pub fn add_lf(&mut self, key: &str, lf: LazyFrame) -> &Self {
        self.lfs.insert(key.to_string(), lf);
        self
    }

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
}
