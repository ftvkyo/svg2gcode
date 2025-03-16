use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub enum BitShape {
    V45Deg,
    Square { radius: f64 },
}

#[derive(Debug, Deserialize)]
pub enum JobKind {
    EngraveContours {
        depth: f64,
    },
    CutContours {
        depth: f64,
        depth_per_pass: f64,
    },
    DrillCircles {
        depth: f64,
    },
    BoreCircles {
        depth: f64,
    },
}

#[derive(Debug, Deserialize)]
pub struct JobConfig {
    pub input: PathBuf,
    pub kind: JobKind,
    pub bit_shape: BitShape,
    pub feed: f64,
    pub rpm: f64,
}

#[derive(Debug, Deserialize)]
pub struct SharedFabConfig {
    pub resolution: f64,
    pub safe_height: f64,
}

#[derive(Debug, Deserialize)]
pub struct FabConfig {
    pub name: String,
    pub outdir: PathBuf,
    pub shared: SharedFabConfig,
    pub jobs: Vec<JobConfig>,
}
