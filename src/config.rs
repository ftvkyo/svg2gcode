use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum EngravingBitShape {
    V45Deg,
}

#[derive(Debug, Deserialize)]
pub enum JobType {
    EngraveContours {
        depth: f64,
        bit_shape: EngravingBitShape,
    },
    CutContours {
        depth: f64,
        depth_per_pass: f64,
        bit_radius: f64,
    },
    DrillCircles {
        depth: f64,
    },
    BoreCircles {
        depth: f64,
        bit_radius: f64,
    },
}

#[derive(Debug, Deserialize)]
pub struct JobConfig {
    pub r#type: JobType,
    pub input: PathBuf,
}

impl JobConfig {
    pub fn offset(&self) -> Option<f64> {
        use EngravingBitShape::*;

        match self.r#type {
            JobType::EngraveContours { bit_shape: V45Deg, depth } => Some(depth),
            JobType::CutContours { bit_radius, .. } => Some(bit_radius),
            JobType::DrillCircles { .. } => None,
            JobType::BoreCircles { .. } => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SharedFabConfig {
    pub resolution: f64,
}

#[derive(Debug, Deserialize)]
pub struct FabConfig {
    pub name: String,
    pub jobs: Vec<JobConfig>,
    pub shared: SharedFabConfig,
}
