use std::f64::consts::TAU;

use geo::{Coord, LineString, Polygon};

use super::{CoordExt, ARC_RESOLUTION};


#[derive(Clone, Debug)]
pub struct Circle {
    center: Coord,
    radius: f64,
}

impl Circle {
    pub fn new(center: Coord, radius: f64) -> Self {
        assert!(radius > 0.0);
        Self {
            center,
            radius,
        }
    }
}

impl Into<Polygon> for Circle {
    fn into(self) -> Polygon {
        let circum = TAU * self.radius;
        let segments = (circum / ARC_RESOLUTION).ceil() as usize;
        let segments = segments.max(6);
        let angle = TAU / segments as f64;

        let mut boundary = vec![];
        let mut v = Coord { x: 0.0, y: self.radius };

        for _ in 0..=segments {
            boundary.push(self.center + v);
            v = v.rotate_ccwise(angle);
        }

        return Polygon::new(LineString::new(boundary), vec![]);
    }
}
