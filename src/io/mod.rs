use geo::{BooleanOps, Coord, Intersects, MultiPolygon, Simplify};

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

pub struct MachiningData {
    contours: MultiPolygon,
    holes: Vec<Hole>,
}

impl MachiningData {
    pub fn new(contours: MultiPolygon, holes: Vec<Hole>) -> Self {
        Self {
            contours,
            holes,
        }
    }

    pub fn contours(&self) -> &MultiPolygon {
        &self.contours
    }

    pub fn holes(&self) -> &Vec<Hole> {
        &self.holes
    }

    pub fn unite(&mut self) {
        let mut result = vec![];

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

            result.extend(p_leader.into_iter());
        }

        self.contours = MultiPolygon::from(result);
    }

    pub fn simplify(&mut self, simplification: f64) {
        for contour in &mut self.contours {
            *contour = contour.simplify(&simplification);
        }
    }
}
