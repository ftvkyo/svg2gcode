use geo::{BooleanOps, Coord, Intersects, MultiPolygon, Simplify};
use geo_offset::Offset;

pub mod gcode;
pub mod svg_input;
pub mod svg_output;

#[derive(Debug)]
pub struct Hole {
    center: Coord,
    radius: f64,
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
pub enum FabData {
    Contours {
        contours: MultiPolygon,
        depths: Vec<f64>,
    },
    Plunges {
        holes: Vec<Hole>,
        depth: f64,
    },
    Spirals {
        holes: Vec<Hole>,
        depth: f64,
        bit_radius: f64,
    }
}

impl FabData {
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
