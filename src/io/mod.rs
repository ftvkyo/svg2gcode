use geo::{BooleanOps, Coord, Intersects, MultiPolygon, Simplify};
use geo_offset::Offset;

pub mod gcode;
pub mod svg_input;
pub mod svg_output;

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

pub struct FabData {
    pub contours: MultiPolygon,
    pub holes: Vec<Hole>,
}

impl FabData {
    pub fn new(contours: MultiPolygon, holes: Vec<Hole>) -> Self {
        let mut s = Self {
            contours,
            holes,
        };
        s.unite();
        s
    }

    fn unite(&mut self) {
        let mut contours_united = vec![];

        let find_next = |polygons: &MultiPolygon, current: &MultiPolygon| -> Option<usize> {
            polygons.iter()
                .enumerate()
                .find_map(|(i, p)| if p.intersects(current) { Some(i) } else { None })
        };

        while let Some(p_leader) = self.contours.0.pop() {
            let mut p_leader = MultiPolygon::from(p_leader);

            while let Some(pi) = find_next(&self.contours, &p_leader) {
                let p = self.contours.0.remove(pi);
                p_leader = p_leader.union(&p);
            }

            contours_united.extend(p_leader.into_iter());
        }

        self.contours = MultiPolygon::from(contours_united);
    }

    fn simplify(&mut self, simplification: f64) {
        for contour in &mut self.contours {
            *contour = contour.simplify(&simplification);
        }
    }

    pub fn offset(&mut self, distance: f64, resolution: f64) {
        let arc_resolution = geo_offset::ArcResolution::SegmentLength(resolution);

        let mut contours_offset = vec![];
        for contour in &self.contours {
            contours_offset.extend(contour.offset_with_arc_resolution(distance, arc_resolution).unwrap());
        }
        self.contours = MultiPolygon::from(contours_offset);

        for hole in &mut self.holes {
            hole.radius += distance;
        }

        self.unite();
        self.simplify(resolution / 5.0);
    }
}
