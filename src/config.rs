use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub enum BitShape {
    V,
    Square { radius: f64 },
}

#[derive(Debug, Deserialize)]
pub enum JobKind {
    EngraveContours {
        depth: f64,
        offset: f64,
    },
    CutContours {
        depth: f64,
        depth_per_pass: f64,
    },
    DrillCircles {
        depth: f64,
        radius_min: Option<f64>,
        radius_max: Option<f64>,
    },
    BoreCircles {
        depth: f64,
        depth_per_turn: f64,
        radius_min: Option<f64>,
        radius_max: Option<f64>,
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

impl FabConfig {
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        Ok(serde_norway::from_reader(std::fs::File::open(&path)?)?)
    }

    pub fn relative_to(mut self, path: &std::path::Path) -> Self {
        self.outdir = path.join(&self.outdir);

        for job in &mut self.jobs {
            job.input = path.join(&job.input);
        }

        self
    }
}
