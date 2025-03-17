use std::iter::once;

use anyhow::{bail, ensure, Result};
use geo::{BooleanOps, Coord, Intersects, LineString, MultiPolygon, Polygon, Simplify};
use geo_offset::Offset;
use log::debug;

use crate::{config::{JobConfig, SharedFabConfig}, io::svg_input::SvgPrimitives, shape::EPSILON};

#[derive(Debug)]
pub struct Hole {
    pub center: Coord,
    pub radius: f64,
}

impl Hole {
    pub fn new(center: Coord, radius: f64) -> Self {
        Self {
            center,
            radius,
        }
    }
}

#[derive(Debug)]
pub struct FabContourData {
    pub contours: Vec<LineString>,
    pub depths: Vec<f64>,
}

impl FabContourData {
    pub fn new(polygons: MultiPolygon, depths: Vec<f64>, offset: f64, resolution: f64) -> Self {
        let arc_resolution = geo_offset::ArcResolution::SegmentLength(resolution);

        let mut polygons_offset = vec![];
        for polygon in polygons {
            polygons_offset.extend(polygon.offset_with_arc_resolution(offset, arc_resolution).unwrap());
        }
        // let mut contours = MultiPolygon::from(polygons_offset);

        let find_next = |polygons: &Vec<Polygon>, current: &MultiPolygon| -> Option<usize> {
            polygons.iter()
                .enumerate()
                .find_map(|(i, p)| if p.intersects(current) { Some(i) } else { None })
        };

        let mut polygons_united = vec![];
        while let Some(p_leader) = polygons_offset.pop() {
            let mut p_leader = MultiPolygon::from(p_leader);

            while let Some(pi) = find_next(&polygons_offset, &p_leader) {
                let p = polygons_offset.remove(pi);
                p_leader = p_leader.union(&p);
            }

            p_leader = p_leader.simplify(&(resolution / 5.0));

            polygons_united.extend(p_leader.into_iter());
        }

        let mut contours = vec![];

        for polygon in polygons_united {
            contours.extend(polygon.interiors().into_iter().cloned());
            contours.push(polygon.exterior().clone());
        }

        Self {
            contours,
            depths,
        }
    }
}

#[derive(Debug)]
pub struct FabHoleData {
    pub holes: Vec<Hole>,
    pub depth: f64,
}

impl FabHoleData {
    pub fn new(holes: Vec<Hole>, depth: f64) -> Self {
        Self {
            holes,
            depth,
        }
    }
}

#[derive(Debug)]
pub enum FabOperation {
    Engrave(FabContourData),
    Cut(FabContourData),
    Drilling(FabHoleData),
    Boring {
        data: FabHoleData,
        depth_per_turn: f64,
        bit_radius: f64,
    }
}

impl FabOperation {
    pub fn engrave_v45_deg(polygons: MultiPolygon, depth: f64, resolution: f64) -> Self {
        Self::Engrave(FabContourData::new(polygons, vec![depth], depth, resolution))
    }

    pub fn cut(polygons: MultiPolygon, depth: f64, depth_per_pass: f64, bit_radius: f64, resolution: f64) -> Self {
        let passes = (depth / depth_per_pass).floor() as usize;

        debug!("Cutting in {passes} passes: depth {depth}, depth_per_pass: {depth_per_pass}");

        let depths: Vec<_> = (0..passes)
            .map(|pass| depth_per_pass * pass as f64)
            .chain(once(depth))
            .collect();

        Self::Cut(FabContourData::new(polygons, depths, bit_radius, resolution))
    }

    pub fn drill(holes: Vec<Hole>, depth: f64) -> Self {
        Self::Drilling(FabHoleData::new(holes, depth))
    }

    pub fn bore(holes: Vec<Hole>, depth: f64, depth_per_turn: f64, bit_radius: f64) -> Self {
        Self::Boring {
            data: FabHoleData::new(holes, depth),
            bit_radius,
            depth_per_turn,
        }
    }
}

fn hole_filter(hole: &Hole, radius_min: Option<f64>, radius_max: Option<f64>) -> bool {
    if let Some(rmin) = radius_min {
        if hole.radius < rmin {
            return false;
        }
    }

    if let Some(rmax) = radius_max {
        if hole.radius > rmax {
            return false;
        }
    }

    return true;
}

#[derive(Debug)]
pub struct FabData {
    pub feed: f64,
    pub rpm: f64,
    pub operation: FabOperation,
}

impl FabData {
    pub fn new(config: &SharedFabConfig, job: JobConfig, primitives: SvgPrimitives) -> Result<Self> {
        use crate::config::{BitShape, JobKind::*};

        let JobConfig {
            feed,
            rpm,
            kind,
            bit_shape,
            ..
        } = job;

        match kind {
            EngraveContours { depth } => {
                ensure!(bit_shape == BitShape::V45Deg, "Unsupported bit shape: {:?}", bit_shape);

                let polygons = primitives.polygons(config.resolution);
                return Ok(FabData {
                    operation: FabOperation::engrave_v45_deg(polygons, depth, config.resolution),
                    feed,
                    rpm
                });
            },
            CutContours { depth, depth_per_pass } => {
                let offset = match bit_shape {
                    BitShape::Square { radius } => radius,
                    _ => bail!("Unsupported bit shape: {:?}", bit_shape),
                };

                let polygons = primitives.polygons(config.resolution);
                return Ok(FabData {
                    feed,
                    rpm,
                    operation: FabOperation::cut(polygons, depth, depth_per_pass, offset, config.resolution),
                });
            },
            DrillCircles { depth, radius_min, radius_max } => {
                let bit_radius = match bit_shape {
                    BitShape::Square { radius } => radius,
                    _ => bail!("Unsupported bit shape: {:?}", bit_shape),
                };

                let holes: Vec<_> = primitives.holes()
                    .filter(|h| hole_filter(h, radius_min, radius_max))
                    .collect();

                let holes = holes
                    .into_iter()
                    .map(|hole| Hole {
                        radius: bit_radius,
                        ..hole
                    })
                    .collect();

                return Ok(FabData {
                    feed,
                    rpm,
                    operation: FabOperation::drill(holes, depth),
                });
            },
            BoreCircles { depth, depth_per_turn, radius_min, radius_max } => {
                let bit_radius = match bit_shape {
                    BitShape::Square { radius } => radius,
                    _ => bail!("Unsupported bit shape: {:?}", bit_shape),
                };

                let holes: Vec<_> = primitives.holes()
                    .filter(|h| hole_filter(h, radius_min, radius_max))
                    .collect();

                for hole in &holes {
                    let hole_radius = hole.radius;
                    ensure!(bit_radius + 0.1 <= hole_radius + EPSILON, "The hole (r={hole_radius}) is too small for boring with the bit (r={bit_radius}), consider increasing radius_min");
                    ensure!(bit_radius * 2.0 >= hole_radius - EPSILON, "The hole (r={hole_radius}) is too big for boring with the bit (r={bit_radius}), consider reducing radius_max");
                }

                return Ok(FabData {
                    feed,
                    rpm,
                    operation: FabOperation::bore(holes, depth, depth_per_turn, bit_radius),
                });
            },
        }
    }
}
