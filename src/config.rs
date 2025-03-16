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
        feed_xy: f64,
    },
    CutContours {
        depth: f64,
        depth_per_pass: f64,
        feed_xy: f64,
    },
    DrillCircles {
        depth: f64,
        feed_z: f64,
    },
    BoreCircles {
        depth: f64,
        feed_z: f64,
        feed_xy: f64,
    },
}

#[derive(Debug, Deserialize)]
pub struct JobConfig {
    pub input: PathBuf,
    pub kind: JobKind,
    pub bit_shape: BitShape,
}

#[derive(Debug, Deserialize)]
pub struct SharedFabConfig {
    pub resolution: f64,
    pub rapid_feed: f64,
    pub rapid_height: f64,
}

#[derive(Debug, Deserialize)]
pub struct FabConfig {
    pub name: String,
    pub jobs: Vec<JobConfig>,
    pub shared: SharedFabConfig,
}
