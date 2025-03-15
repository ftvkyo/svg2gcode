mod circle;
mod line;

use std::f64::consts::PI;

use geo::{Coord, Line, Polygon, Vector2DOps};
use log::debug;

pub use circle::*;
pub use line::*;

pub const EPSILON: f64 = 0.000000001;


pub trait CoordExt: Sized {
    fn rotate_ccwise(&self, angle_rad: f64) -> Self;

    /// Find the smaller of two angles
    fn find_angle(&self, other: &Self) -> f64;
}


impl CoordExt for Coord {
    fn rotate_ccwise(&self, angle_rad: f64) -> Self {
        let sin = angle_rad.sin();
        let cos = angle_rad.cos();

        Self {
            x: cos * self.x - sin * self.y,
            y: sin * self.x + cos * self.y,
        }
    }

    fn find_angle(&self, other: &Self) -> f64 {
        let m1 = self.magnitude();
        let m2 = other.magnitude();

        let cos = self.dot_product(*other) / m1 / m2;

        if (cos - 1.0).abs() < EPSILON {
            return 0.0;
        }

        if (cos + 1.0).abs() < EPSILON {
            return PI;
        }

        cos.acos()
    }
}


pub trait LineExt: Sized {
    fn shift_right(&self, offset: f64) -> Self;

    /// Find a counter-clockwise arc that will connect `self` to `other` around `axis`
    fn find_arc(&self, other: &Self, axis: Coord, resolution: f64) -> impl Iterator<Item = Coord>;
}


impl LineExt for Line {
    fn shift_right(&self, offset: f64) -> Self {
        let shift = Coord { x: self.dy(), y: - self.dx() }
            .try_normalize()
            .expect("Could not normalize the shift direction")
            * offset;

        geo::Line::new(self.start + shift, self.end + shift)
    }

    fn find_arc(&self, other: &Self, axis: Coord, resolution: f64) -> impl Iterator<Item = Coord> {
        let a = self.end;
        let b = other.start;

        let va = a - axis;
        let vb = b - axis;

        assert!((va.magnitude() - vb.magnitude()) < EPSILON);

        let radius = (a - axis).magnitude();
        let arc_angle = self.delta().find_angle(&other.delta());
        let arc_circum = radius * arc_angle;
        let arc_segments = (arc_circum / resolution).ceil() as usize;
        let arc_rot = arc_angle / arc_segments as f64;

        debug!("Arc angle: {arc_angle}, segments: {arc_segments}, angle_per_segment: {arc_rot}");

        let mut vp = va;
        let mut arc = vec![];

        for _ in 0..arc_segments {
            arc.push(axis + vp);
            vp = vp.rotate_ccwise(arc_rot);
        }

        arc.push(b);
        arc.into_iter()
    }
}


pub trait IntoPolygon {
    fn into_polygon(self, resolution: f64) -> Polygon;
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_shift() {
        let l = Line::new(Coord { x: 0.0, y: 0.0 }, Coord { x: 0.0, y: 1.0 });
        let ls = l.shift_right(1.0);

        assert!((ls.start - Coord { x: 1.0, y: 0.0 }).magnitude() < EPSILON);
        assert!((ls.end - Coord { x: 1.0, y: 1.0 }).magnitude() < EPSILON);

        let l = Line::new(Coord { x: 0.0, y: 0.0 }, Coord { x: 1.0, y: 1.0 });
        let ls = l.shift_right(2.0f64.sqrt());

        let e1 = Coord { x: 1.0, y: -1.0 };
        let e2 = Coord { x: 2.0, y: 0.0 };

        assert!((ls.start - e1).magnitude() < EPSILON);
        assert!((ls.end - e2).magnitude() < EPSILON);
    }
}
