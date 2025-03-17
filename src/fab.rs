use anyhow::{bail, Result};
use geo::{BooleanOps, Coord, Intersects, MultiPolygon, Simplify};
use geo_offset::Offset;

use crate::{config::{JobConfig, SharedFabConfig}, io::svg_input::SvgPrimitives};

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
pub enum FabDataKind {
    Contours {
        contours: MultiPolygon,
        depths: Vec<f64>,
    },
    Drilling {
        holes: Vec<Hole>,
        depth: f64,
    },
    Boring {
        holes: Vec<Hole>,
        depth: f64,
        bit_radius: f64,
    }
}

impl FabDataKind {
    pub fn contours_with_offset(contours: MultiPolygon, depths: Vec<f64>, offset: f64, resolution: f64) -> Self {
        let arc_resolution = geo_offset::ArcResolution::SegmentLength(resolution);

        let mut contours_offset = vec![];
        for contour in &contours {
            contours_offset.extend(contour.offset_with_arc_resolution(offset, arc_resolution).unwrap());
        }
        let mut contours = MultiPolygon::from(contours_offset);

        let find_next = |polygons: &MultiPolygon, current: &MultiPolygon| -> Option<usize> {
            polygons.iter()
                .enumerate()
                .find_map(|(i, p)| if p.intersects(current) { Some(i) } else { None })
        };

        let mut contours_united = vec![];
        while let Some(p_leader) = contours.0.pop() {
            let mut p_leader = MultiPolygon::from(p_leader);

            while let Some(pi) = find_next(&contours, &p_leader) {
                let p = contours.0.remove(pi);
                p_leader = p_leader.union(&p);
            }

            contours_united.extend(p_leader.into_iter());
        }
        let mut contours = MultiPolygon::from(contours_united);

        for contour in &mut contours {
            *contour = contour.simplify(&(resolution / 5.0));
        }

        Self::Contours { contours, depths }
    }
}

#[derive(Debug)]
pub struct FabData {
    pub kind: FabDataKind,
    pub feed: f64,
    pub rpm: f64,
}

impl FabData {
    pub fn new(config: &SharedFabConfig, job: JobConfig, primitives: SvgPrimitives) -> Result<Self> {
        use crate::config::{BitShape, JobKind::*};

        match job.kind {
            EngraveContours { depth } => {
                let offset = match job.bit_shape {
                    BitShape::V45Deg => depth,
                    _ => bail!("Unsupported bit shape: {:?}", job.bit_shape),
                };

                let contours = primitives.contours(config.resolution);
                return Ok(FabData {
                    kind: FabDataKind::contours_with_offset(contours, vec![depth], offset, config.resolution),
                    feed: job.feed,
                    rpm: job.rpm,
                });
            },
            CutContours { depth, depth_per_pass } => {
                let offset = match job.bit_shape {
                    BitShape::Square { radius } => radius,
                    _ => bail!("Unsupported bit shape: {:?}", job.bit_shape),
                };

                let passes = (depth / depth_per_pass.ceil()) as usize;

                let mut depths: Vec<_> = (0..passes).map(|pass| depth_per_pass * pass as f64).collect();
                if let Some(last) = depths.last().copied() {
                    if (depth - last).abs() > 0.01 {
                        depths.push(depth);
                    }
                } else {
                    depths.push(depth);
                }

                let contours = primitives.contours(config.resolution);
                return Ok(FabData {
                    kind: FabDataKind::contours_with_offset(contours, depths, offset, config.resolution),
                    feed: job.feed,
                    rpm: job.rpm,
                });
            },
            DrillCircles { depth } => {
                let radius = match job.bit_shape {
                    BitShape::Square { radius } => radius,
                    _ => bail!("Unsupported bit shape: {:?}", job.bit_shape),
                };

                let holes = primitives.holes()
                    .into_iter()
                    .map(|mut h| {
                        h.radius = radius;
                        h
                    })
                    .collect();

                let fd = FabData {
                    kind: FabDataKind::Drilling { holes, depth },
                    feed: job.feed,
                    rpm: job.rpm,
                };

                return Ok(fd);
            },
            BoreCircles { depth } => {
                let bit_radius = match job.bit_shape {
                    BitShape::Square { radius } => radius,
                    _ => bail!("Unsupported bit shape: {:?}", job.bit_shape),
                };

                let holes = primitives.holes();
                // Only bore holes if the bit is large enough to do helical movement inside of the hole
                let holes = holes.into_iter().filter(|hole| (hole.radius - bit_radius) > 0.1).collect();

                let fd = FabData {
                    kind: FabDataKind::Boring { holes, depth, bit_radius },
                    feed: job.feed,
                    rpm: job.rpm,
                };

                return Ok(fd);
            },
        }
    }
}
